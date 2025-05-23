use std::{collections::HashMap, f64::consts::PI, ops::Index};

use index_vec::{IndexVec, define_index_type};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use wasm_encoder::{
    CodeSection, EntityType, Function, FunctionSection, ImportSection, InstructionSink, Module,
    TypeSection, ValType,
};

use crate::{
    lex::{TokenId, Tokens},
    parse::{self, ArgId, Expr, ExprId, IdRange, Param, Signature, Stmt, Tree},
};

define_index_type! {
    struct TypeId = u32;
}

define_index_type! {
    struct LocalId = u32;
    IMPL_RAW_CONVERSIONS = true;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Type {
    Int,
    Float,
    Vector(TypeId),
    Matrix(TypeId),
}

#[derive(Debug)]
struct Types {
    set: IndexSet<Type>,
}

impl Types {
    fn new() -> Self {
        Self {
            set: IndexSet::new(),
        }
    }

    fn push(&mut self, ty: Type) -> TypeId {
        let (i, _) = self.set.insert_full(ty);
        TypeId::from_usize(i)
    }
}

impl Index<TypeId> for Types {
    type Output = Type;

    fn index(&self, index: TypeId) -> &Self::Output {
        &self.set[index.index()]
    }
}

#[derive(Debug)]
struct Funcs<'a> {
    map: IndexMap<&'a str, Signature>,
}

impl<'a> Funcs<'a> {
    fn new() -> Self {
        Self {
            map: IndexMap::new(),
        }
    }

    fn push(&mut self, name: &'a str, sig: Signature) -> u32 {
        let (i, _) = self.map.insert_full(name, sig);
        u32::try_from(i).unwrap()
    }

    fn get(&self, name: &'a str) -> (u32, Signature) {
        let i = self.map.get_index_of(name).unwrap();
        (u32::try_from(i).unwrap(), self.map[i])
    }
}

#[derive(Debug)]
struct Codegen<'a> {
    src: &'a str,
    tokens: &'a Tokens,
    tree: &'a Tree,
    types: Types,
}

impl Codegen<'_> {
    fn string(&self, token: TokenId) -> &str {
        &self.src[self.tokens[token].byte_range()]
    }

    fn val_types(&self, ty: parse::TypeId) -> Vec<ValType> {
        // TODO: can we avoid allocating memory here?
        match self.tree.types[ty] {
            parse::Type::Name(token) => match self.string(token) {
                "Int" => vec![ValType::I32],
                "Float" => vec![ValType::F64],
                s => panic!("{s}"),
            },
            parse::Type::Vector(_) => vec![ValType::I32, ValType::I32],
            parse::Type::Matrix(_) => vec![ValType::I32, ValType::I32, ValType::I32],
        }
    }

    fn signature(&self, types: &mut TypeSection, sig: Signature) -> u32 {
        let t = types.len();
        types.ty().function(
            self.tree
                .params(sig.params)
                .iter()
                .flat_map(|param| self.val_types(param.ty))
                .collect::<Vec<_>>(),
            self.val_types(sig.ret),
        );
        t
    }

    fn module(self) -> Module {
        let mut types = Types::new();
        let mut type_section = TypeSection::new();
        let mut imports = ImportSection::new();
        let mut func_section = FunctionSection::new();
        let mut codes = CodeSection::new();
        let mut funcs = Funcs::new();
        for &sig in &self.tree.imports {
            let name = self.string(sig.name);
            let ty = self.signature(&mut type_section, sig);
            imports.import("", name, EntityType::Function(ty));
            funcs.push(name, sig);
        }
        for func in self.tree.funcs.iter() {
            func_section.function(self.signature(&mut type_section, func.sig));
            funcs.push(self.string(func.sig.name), func.sig);
        }
        for func in &self.tree.funcs {
            let mut fun = Fun::new(self.src, self.tokens, self.tree, types, &funcs);
            for &param in self.tree.params(func.sig.params) {
                fun.param(param);
            }
            let param_locals = fun.locals.len();
            for &stmt in self.tree.stmts(func.body) {
                fun.stmt(stmt);
            }
            let mut f = Function::new_with_locals_types(fun.locals.into_iter().skip(param_locals));
            f.raw(fun.body);
            f.instructions().end();
            codes.function(&f);
            types = fun.types;
        }
        let mut module = Module::new();
        module.section(&type_section);
        module.section(&imports);
        module.section(&func_section);
        module.section(&codes);
        module
    }
}

#[derive(Clone, Copy, Debug)]
enum Value {
    Int,
    Float,
    Vector {
        ty: TypeId,
        len: LocalId,
        ptr: LocalId,
    },
    Matrix {
        ty: TypeId,
        rows: LocalId,
        cols: LocalId,
        ptr: LocalId,
    },
}

impl Value {
    fn ty(&self) -> Type {
        match *self {
            Value::Int => Type::Int,
            Value::Float => Type::Float,
            Value::Vector { ty, .. } => Type::Vector(ty),
            Value::Matrix { ty, .. } => Type::Matrix(ty),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Binding {
    Int(LocalId),
    Float(LocalId),
    Vector {
        ty: TypeId,
        len: LocalId,
        ptr: LocalId,
    },
    Matrix {
        ty: TypeId,
        rows: LocalId,
        cols: LocalId,
        ptr: LocalId,
    },
}

fn insn(body: &mut Vec<u8>) -> InstructionSink {
    InstructionSink::new(body)
}

#[derive(Debug)]
struct Fun<'a> {
    src: &'a str,
    tokens: &'a Tokens,
    tree: &'a Tree,
    types: Types,
    funcs: &'a Funcs<'a>,
    locals: IndexVec<LocalId, ValType>, // TODO: can we use a smaller type instead?
    names: HashMap<&'a str, Binding>,
    body: Vec<u8>,
}

impl<'a> Fun<'a> {
    fn new(
        src: &'a str,
        tokens: &'a Tokens,
        tree: &'a Tree,
        types: Types,
        funcs: &'a Funcs,
    ) -> Self {
        Self {
            src,
            tokens,
            tree,
            types,
            funcs,
            locals: IndexVec::new(),
            names: HashMap::new(),
            body: Vec::new(),
        }
    }

    fn string(&self, token: TokenId) -> &'a str {
        &self.src[self.tokens[token].byte_range()]
    }

    fn int(&mut self) -> LocalId {
        self.locals.push(ValType::I32)
    }

    fn float(&mut self) -> LocalId {
        self.locals.push(ValType::F64)
    }

    fn ty(&mut self, t: parse::TypeId) -> Type {
        match self.tree.types[t] {
            parse::Type::Name(token) => match self.string(token) {
                "Int" => Type::Int,
                "Float" => Type::Float,
                s => panic!("{s}"),
            },
            parse::Type::Vector(inner) => Type::Vector(self.ty_id(inner)),
            parse::Type::Matrix(inner) => Type::Matrix(self.ty_id(inner)),
        }
    }

    fn ty_id(&mut self, t: parse::TypeId) -> TypeId {
        let ty = self.ty(t);
        self.types.push(ty)
    }

    fn bind_ty(&mut self, ty: Type) -> Binding {
        match ty {
            Type::Int => Binding::Int(self.int()),
            Type::Float => Binding::Float(self.int()),
            Type::Vector(ty) => {
                let len = self.int();
                let ptr = self.int();
                Binding::Vector { ty, len, ptr }
            }
            Type::Matrix(ty) => {
                let rows = self.int();
                let cols = self.int();
                let ptr = self.int();
                Binding::Matrix {
                    ty,
                    rows,
                    cols,
                    ptr,
                }
            }
        }
    }

    fn param(&mut self, param: Param) {
        let ty = self.ty(param.ty);
        let binding = self.bind_ty(ty);
        let prev = self.names.insert(self.string(param.name), binding);
        assert!(prev.is_none());
    }

    fn val(&mut self, binding: Binding) -> Value {
        match binding {
            Binding::Int(local) => {
                insn(&mut self.body).local_get(local.into());
                Value::Int
            }
            Binding::Float(local) => {
                insn(&mut self.body).local_get(local.into());
                Value::Float
            }
            Binding::Vector { ty, len, ptr } => Value::Vector { ty, len, ptr },
            Binding::Matrix {
                ty,
                rows,
                cols,
                ptr,
            } => Value::Matrix {
                ty,
                rows,
                cols,
                ptr,
            },
        }
    }

    fn bind(&mut self, val: Value) -> Binding {
        match val {
            Value::Int => {
                let local = self.int();
                insn(&mut self.body).local_set(local.into());
                Binding::Int(local)
            }
            Value::Float => {
                let local = self.float();
                insn(&mut self.body).local_set(local.into());
                Binding::Float(local)
            }
            Value::Vector { ty, len, ptr } => Binding::Vector { ty, len, ptr },
            Value::Matrix {
                ty,
                rows,
                cols,
                ptr,
            } => Binding::Matrix {
                ty,
                rows,
                cols,
                ptr,
            },
        }
    }

    fn expr(&mut self, expr: ExprId) -> Value {
        match self.tree.exprs[expr] {
            Expr::Paren(inner) => self.expr(inner),
            Expr::Name(token) => {
                let name = self.string(token);
                match self.names.get(name) {
                    Some(&binding) => self.val(binding),
                    None => match name {
                        "pi" => {
                            insn(&mut self.body).f64_const(PI.into());
                            Value::Float
                        }
                        _ => panic!("{name}"),
                    },
                }
            }
            Expr::Int(token) => {
                let val = self.string(token).parse().unwrap();
                insn(&mut self.body).i32_const(val);
                Value::Int
            }
            Expr::Float(token) => {
                let val: f64 = self.string(token).parse().unwrap();
                insn(&mut self.body).f64_const(val.into());
                Value::Float
            }
            Expr::New { ty, args } => match self.ty(ty) {
                Type::Int => panic!(),
                Type::Float => panic!(),
                Type::Vector(ty) => match self.types[ty] {
                    Type::Int => todo!(),
                    Type::Float => todo!(),
                    Type::Vector(_) => todo!(),
                    Type::Matrix(_) => todo!(),
                },
                Type::Matrix(ty) => match self.types[ty] {
                    Type::Int => todo!(),
                    Type::Float => {
                        let rows = self.int();
                        let cols = self.int();
                        let ptr = self.int();
                        Value::Matrix {
                            ty,
                            rows,
                            cols,
                            ptr,
                        }
                    }
                    Type::Vector(_) => todo!(),
                    Type::Matrix(_) => todo!(),
                },
            },
            Expr::Vector { vec, index } => todo!(),
            Expr::Matrix { mat, row, col } => todo!(),
            Expr::Function { name, args } => {
                let (funcidx, sig) = self.funcs.get(self.string(name));
                let types: Vec<_> = self
                    .tree
                    .params(sig.params)
                    .iter()
                    .map(|param| self.ty(param.ty))
                    .collect();
                self.args(args, types);
                insn(&mut self.body).call(funcidx);
                let ty = self.ty(sig.ret); // TODO: don't convert types at every callsite.
                let binding = self.bind_ty(ty);
                self.val(binding)
            }
            Expr::Method { obj, name, args } => {
                let t = self.expr(obj).ty();
                let s = self.string(name);
                match t {
                    Type::Int => match s {
                        "float" => {
                            self.args(args, []);
                            Value::Float
                        }
                        _ => unimplemented!("{s}"),
                    },
                    Type::Float => match s {
                        "log" | "sqr" | "sqrt" => {
                            self.args(args, []);
                            Value::Float
                        }
                        _ => unimplemented!("{s}"),
                    },
                    Type::Vector(type_id) => todo!(),
                    Type::Matrix(type_id) => todo!(),
                }
            }
            Expr::Unary { op, arg } => {
                let t = self.expr(arg).ty();
                match (t, op) {
                    (Type::Int, parse::Unop::Negative) => {
                        let tmp = self.int();
                        insn(&mut self.body)
                            .local_set(tmp.into())
                            .i32_const(0)
                            .local_get(tmp.into())
                            .i32_sub();
                        Value::Int
                    }
                    (Type::Float, parse::Unop::Negative) => {
                        insn(&mut self.body).f64_neg();
                        Value::Float
                    }
                    _ => panic!(),
                }
            }
            Expr::Binary { lhs, op, rhs } => {
                let t = self.expr(lhs).ty();
                let t2 = self.expr(rhs).ty();
                if t2 != t {
                    panic!("{t:?} {op:?} {t2:?}");
                }
                let mut insn = insn(&mut self.body);
                match (t, op) {
                    (Type::Int, parse::Binop::Add) => insn.i32_add(),
                    (Type::Int, parse::Binop::Subtract) => insn.i32_sub(),
                    (Type::Int, parse::Binop::Multiply) => insn.i32_mul(),
                    (Type::Int, parse::Binop::Divide) => insn.i32_div_s(),
                    (Type::Float, parse::Binop::Add) => insn.f64_add(),
                    (Type::Float, parse::Binop::Subtract) => insn.f64_sub(),
                    (Type::Float, parse::Binop::Multiply) => insn.f64_mul(),
                    (Type::Float, parse::Binop::Divide) => insn.f64_div(),
                    _ => panic!(),
                };
                match t {
                    Type::Int => Value::Int,
                    Type::Float => Value::Float,
                    _ => panic!(),
                }
            }
        }
    }

    fn args(&mut self, args: IdRange<ArgId>, types: impl IntoIterator<Item = Type>) {
        // TODO: handle named arguments properly.
        for (&arg, ty) in self.tree.args(args).iter().zip_eq(types) {
            assert_eq!(self.expr(arg.expr).ty(), ty, "{:?}", arg);
        }
    }

    fn stmt(&mut self, stmt: Stmt) {
        match stmt {
            Stmt::Let { name, rhs } => {
                let val = self.expr(rhs);
                let binding = self.bind(val);
                let prev = self.names.insert(self.string(name), binding);
                assert!(prev.is_none());
            }
            Stmt::Var { name, rhs } => {
                let val = self.expr(rhs);
                let binding = self.bind(val);
                let prev = self.names.insert(self.string(name), binding);
                assert!(prev.is_none());
            }
            Stmt::For {
                name,
                start,
                end,
                body,
            } => {
                // TODO
            }
            Stmt::Set { lhs, kind, rhs } => todo!(),
            Stmt::Expr(expr_id) => todo!(),
        }
    }
}

pub fn codegen(src: &str, tokens: &Tokens, tree: &Tree) -> Module {
    Codegen {
        src,
        tokens,
        tree,
        types: Types::new(),
    }
    .module()
}
