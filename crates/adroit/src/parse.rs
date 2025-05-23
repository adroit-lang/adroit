use enumset::EnumSet;
use index_vec::{IndexVec, define_index_type};

use crate::lex::{
    TokenId,
    TokenKind::{self, *},
    Tokens,
};

define_index_type! {
    pub struct TypeId = u32;
}

define_index_type! {
    pub struct ParamId = u32;
}

define_index_type! {
    pub struct StmtId = u32;
}

define_index_type! {
    pub struct ExprId = u32;
}

define_index_type! {
    pub struct ArgId = u32;
}

define_index_type! {
    pub struct ImportId = u16;
}

define_index_type! {
    pub struct FuncId = u16;
}

#[derive(Clone, Copy, Debug)]
pub struct IdRange<T> {
    start: T,
    end: T,
}

impl<T> IdRange<T> {
    fn new(start: T, end: T) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Type {
    Name(TokenId),
    Vector(TypeId),
    Matrix(TypeId),
}

#[derive(Clone, Copy, Debug)]
pub struct Param {
    pub name: TokenId,
    pub ty: TypeId,
}

#[derive(Clone, Copy, Debug)]
pub enum Lhs {
    Name(TokenId),
    Vector {
        vec: ExprId,
        index: ExprId,
    },
    Matrix {
        mat: ExprId,
        row: ExprId,
        col: ExprId,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum SetKind {
    Set,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Copy, Debug)]
pub enum Stmt {
    Let {
        name: TokenId,
        rhs: ExprId,
    },
    Var {
        name: TokenId,
        rhs: ExprId,
    },
    For {
        name: TokenId,
        start: ExprId,
        end: ExprId,
        body: IdRange<StmtId>,
    },
    Set {
        lhs: Lhs,
        kind: SetKind,
        rhs: ExprId,
    },
    Expr(ExprId),
}

#[derive(Clone, Copy, Debug)]
pub enum Unop {
    Negative,
}

#[derive(Clone, Copy, Debug)]
pub enum Binop {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Copy, Debug)]
pub enum Expr {
    Paren(ExprId),
    Name(TokenId),
    Int(TokenId),
    Float(TokenId),
    New {
        ty: TypeId,
        args: IdRange<ArgId>,
    },
    Vector {
        vec: ExprId,
        index: ExprId,
    },
    Matrix {
        mat: ExprId,
        row: ExprId,
        col: ExprId,
    },
    Function {
        name: TokenId,
        args: IdRange<ArgId>,
    },
    Method {
        obj: ExprId,
        name: TokenId,
        args: IdRange<ArgId>,
    },
    Unary {
        op: Unop,
        arg: ExprId,
    },
    Binary {
        lhs: ExprId,
        op: Binop,
        rhs: ExprId,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Arg {
    pub name: Option<TokenId>,
    pub expr: ExprId,
}

#[derive(Clone, Copy, Debug)]
pub struct Signature {
    pub name: TokenId,
    pub params: IdRange<ParamId>,
    pub ret: TypeId,
}

#[derive(Clone, Copy, Debug)]
pub struct Function {
    pub sig: Signature,
    pub body: IdRange<StmtId>,
    pub ret: Option<ExprId>,
}

#[derive(Debug, Default)]
pub struct Tree {
    pub types: IndexVec<TypeId, Type>,
    pub params: IndexVec<ParamId, Param>,
    pub stmts: IndexVec<StmtId, Stmt>,
    pub exprs: IndexVec<ExprId, Expr>,
    pub args: IndexVec<ArgId, Arg>,
    pub imports: IndexVec<ImportId, Signature>,
    pub funcs: IndexVec<FuncId, Function>,
}

impl Tree {
    pub fn params(&self, range: IdRange<ParamId>) -> &[Param] {
        &self.params[range.start..range.end].raw
    }

    pub fn stmts(&self, range: IdRange<StmtId>) -> &[Stmt] {
        &self.stmts[range.start..range.end].raw
    }

    pub fn args(&self, range: IdRange<ArgId>) -> &[Arg] {
        &self.args[range.start..range.end].raw
    }
}

#[derive(Debug)]
struct Block {
    /// The `StmtId` here is fake, be warned!
    stmts: IndexVec<StmtId, Stmt>,
    expr: Option<Expr>,
}

#[derive(Debug)]
pub enum ParseError {
    Expected {
        id: TokenId,
        kinds: EnumSet<TokenKind>,
    },
}

impl ParseError {
    pub fn message(&self) -> String {
        match self {
            ParseError::Expected { id: _, kinds } => format!(
                "expected {}",
                itertools::join(kinds.into_iter().map(|kind| kind.to_string()), " or ")
            ),
        }
    }
}

type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug)]
struct Parser<'a> {
    tokens: &'a Tokens,
    id: TokenId,
    tree: Tree,
}

impl Parser<'_> {
    fn get(&self, id: TokenId) -> TokenKind {
        self.tokens[id].kind
    }

    fn peek(&self) -> TokenKind {
        self.get(self.id)
    }

    fn next(&mut self) -> TokenId {
        if let Eof = self.peek() {
            panic!("unexpected end of file");
        }
        let id = self.id;
        self.id += 1;
        id
    }

    fn err(&self, kinds: EnumSet<TokenKind>) -> ParseError {
        ParseError::Expected { id: self.id, kinds }
    }

    fn expect(&mut self, kind: TokenKind) -> ParseResult<TokenId> {
        let id = self.id;
        if self.peek() == kind {
            self.next();
            Ok(id)
        } else {
            Err(self.err(EnumSet::only(kind)))
        }
    }

    fn ty(&mut self) -> ParseResult<Type> {
        match self.peek() {
            Ident => Ok(Type::Name(self.next())),
            LBracket => {
                self.next();
                match self.peek() {
                    LBracket => {
                        self.next();
                        self.expect(RBracket)?;
                        self.expect(RBracket)?;
                        Ok(Type::Matrix(self.ty_id()?))
                    }
                    RBracket => {
                        self.next();
                        Ok(Type::Vector(self.ty_id()?))
                    }
                    _ => Err(self.err(LBracket | RBracket)),
                }
            }
            _ => Err(self.err(Ident | LBracket)),
        }
    }

    fn ty_id(&mut self) -> ParseResult<TypeId> {
        let ty = self.ty()?;
        Ok(self.tree.types.push(ty))
    }

    fn expr_atom(&mut self) -> ParseResult<Expr> {
        match self.peek() {
            Ident => Ok(Expr::Name(self.next())), // Is this dead code?
            Int => Ok(Expr::Int(self.next())),
            Float => Ok(Expr::Float(self.next())),
            LParen => {
                self.next();
                let inner = self.expr_id()?;
                self.expect(RParen)?;
                Ok(Expr::Paren(inner))
            }
            LBracket => {
                let ty = self.ty_id()?;
                let args = self.args()?;
                Ok(Expr::New { ty, args })
            }
            _ => Err(self.err(Ident | Int | Float | LParen | LBracket)),
        }
    }

    fn expr_factor(&mut self) -> ParseResult<Expr> {
        let mut unops = Vec::new();
        while let Minus = self.peek() {
            self.next();
            unops.push(Unop::Negative);
        }
        let mut expr = match self.peek() {
            Ident => {
                let name = self.next();
                match self.peek() {
                    LParen => {
                        let args = self.args()?;
                        Expr::Function { name, args }
                    }
                    _ => Expr::Name(name),
                }
            }
            _ => self.expr_atom()?,
        };
        loop {
            expr = match self.peek() {
                LBracket => {
                    self.next();
                    let index = self.expr_id()?;
                    match self.peek() {
                        RBracket => {
                            self.next();
                            let vec = self.tree.exprs.push(expr);
                            Expr::Vector { vec, index }
                        }
                        Comma => {
                            self.next();
                            let mat = self.tree.exprs.push(expr);
                            let row = index;
                            let col = self.expr_id()?;
                            self.expect(RBracket)?;
                            Expr::Matrix { mat, row, col }
                        }
                        _ => return Err(self.err(RBracket | Comma)),
                    }
                }
                Dot => {
                    self.next();
                    let obj = self.tree.exprs.push(expr);
                    let name = self.expect(Ident)?;
                    let args = self.args()?;
                    Expr::Method { obj, name, args }
                }
                _ => break,
            };
        }
        Ok(unops.into_iter().rfold(expr, |expr, op| {
            let arg = self.tree.exprs.push(expr);
            Expr::Unary { op, arg }
        }))
    }

    fn expr_factor_id(&mut self) -> ParseResult<ExprId> {
        let expr = self.expr_factor()?;
        Ok(self.tree.exprs.push(expr))
    }

    fn expr_term(&mut self) -> ParseResult<Expr> {
        let mut expr = self.expr_factor()?;
        loop {
            let op = match self.peek() {
                Times => Binop::Multiply,
                Divide => Binop::Divide,
                _ => break,
            };
            self.next();
            let lhs = self.tree.exprs.push(expr);
            let rhs = self.expr_factor_id()?;
            expr = Expr::Binary { lhs, op, rhs };
        }
        Ok(expr)
    }

    fn expr_term_id(&mut self) -> ParseResult<ExprId> {
        let expr = self.expr_term()?;
        Ok(self.tree.exprs.push(expr))
    }

    fn expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.expr_term()?;
        loop {
            let op = match self.peek() {
                Plus => Binop::Add,
                Minus => Binop::Subtract,
                _ => break,
            };
            self.next();
            let lhs = self.tree.exprs.push(expr);
            let rhs = self.expr_term_id()?;
            expr = Expr::Binary { lhs, op, rhs };
        }
        Ok(expr)
    }

    fn expr_id(&mut self) -> ParseResult<ExprId> {
        let expr = self.expr()?;
        Ok(self.tree.exprs.push(expr))
    }

    fn args(&mut self) -> ParseResult<IdRange<ArgId>> {
        self.expect(LParen)?;
        let mut args = IndexVec::new();
        let start = self.tree.args.next_idx();
        loop {
            if let RParen = self.peek() {
                self.next();
                break;
            }
            let expr = self.expr()?;
            let (name, expr) = match self.peek() {
                Equals => match expr {
                    Expr::Name(name) => {
                        self.next();
                        (Some(name), self.expr_id()?)
                    }
                    _ => return Err(self.err(Comma | LParen)),
                },
                _ => (None, self.tree.exprs.push(expr)),
            };
            args.push(Arg { name, expr });
            if let Comma = self.peek() {
                self.next();
            }
        }
        self.tree.args.append(&mut args);
        let end = self.tree.args.next_idx();
        Ok(IdRange::new(start, end))
    }

    fn block(&mut self) -> ParseResult<Block> {
        let mut stmts = IndexVec::new();
        self.expect(LBrace)?;
        let expr = loop {
            let stmt = match self.peek() {
                RBrace => break None,
                Let => {
                    self.next();
                    let name = self.expect(Ident)?;
                    self.expect(Equals)?;
                    let rhs = self.expr_id()?;
                    self.expect(Semicolon)?;
                    Stmt::Let { name, rhs }
                }
                Var => {
                    self.next();
                    let name = self.expect(Ident)?;
                    self.expect(Equals)?;
                    let rhs = self.expr_id()?;
                    self.expect(Semicolon)?;
                    Stmt::Var { name, rhs }
                }
                For => {
                    self.next();
                    let name = self.expect(Ident)?;
                    self.expect(In)?;
                    let start = self.expr_id()?;
                    self.expect(DotDot)?;
                    let end = self.expr_id()?;
                    let Block { mut stmts, expr } = self.block()?;
                    assert!(expr.is_none()); // TODO
                    let body_start = self.tree.stmts.next_idx();
                    self.tree.stmts.append(&mut stmts);
                    let body_end = self.tree.stmts.next_idx();
                    Stmt::For {
                        name,
                        start,
                        end,
                        body: IdRange::new(body_start, body_end),
                    }
                }
                _ => {
                    let expr = self.expr()?;
                    let kind = match self.peek() {
                        RBrace => break Some(expr),
                        Semicolon => {
                            stmts.push(Stmt::Expr(self.tree.exprs.push(expr)));
                            continue;
                        }
                        Equals => SetKind::Set,
                        PlusEquals => SetKind::Add,
                        MinusEquals => SetKind::Subtract,
                        TimesEquals => SetKind::Multiply,
                        DivideEquals => SetKind::Divide,
                        _ => {
                            return Err(self.err(
                                Semicolon
                                    | Equals
                                    | PlusEquals
                                    | MinusEquals
                                    | TimesEquals
                                    | DivideEquals
                                    | RBrace,
                            ));
                        }
                    };
                    let lhs = match expr {
                        Expr::Name(name) => Lhs::Name(name),
                        Expr::Vector { vec, index } => Lhs::Vector { vec, index },
                        Expr::Matrix { mat, row, col } => Lhs::Matrix { mat, row, col },
                        _ => return Err(self.err(Semicolon | RBrace)),
                    };
                    self.next();
                    let rhs = self.expr_id()?;
                    self.expect(Semicolon)?;
                    Stmt::Set { lhs, kind, rhs }
                }
            };
            stmts.push(stmt);
        };
        self.expect(RBrace)?;
        Ok(Block { stmts, expr })
    }

    fn signature(&mut self) -> ParseResult<Signature> {
        self.expect(Func)?;
        let name = self.expect(Ident)?;
        self.expect(LParen)?;
        let param_start = self.tree.params.next_idx();
        loop {
            if let RParen = self.peek() {
                self.next();
                break;
            }
            let name = self.expect(Ident)?;
            self.expect(Colon)?;
            let ty = self.ty_id()?;
            self.tree.params.push(Param { name, ty });
            if let Comma = self.peek() {
                self.next();
            }
        }
        let param_end = self.tree.params.next_idx();
        let params = IdRange::new(param_start, param_end);
        self.expect(Colon)?;
        let ret = self.ty_id()?;
        Ok(Signature { name, params, ret })
    }

    fn import(&mut self) -> ParseResult<Signature> {
        self.expect(Import)?;
        let sig = self.signature()?;
        self.expect(Semicolon)?;
        Ok(sig)
    }

    fn func(&mut self) -> ParseResult<Function> {
        let sig = self.signature()?;
        let Block { mut stmts, expr } = self.block()?;
        let body_start = self.tree.stmts.next_idx();
        self.tree.stmts.append(&mut stmts);
        let body_end = self.tree.stmts.next_idx();
        Ok(Function {
            sig,
            body: IdRange::new(body_start, body_end),
            ret: expr.map(|ret| self.tree.exprs.push(ret)),
        })
    }

    fn module(mut self) -> ParseResult<Tree> {
        loop {
            match self.peek() {
                Import => {
                    let ty = self.import()?;
                    self.tree.imports.push(ty);
                }
                Func => {
                    let func = self.func()?;
                    self.tree.funcs.push(func);
                }
                Eof => return Ok(self.tree),
                _ => return Err(self.err(Import | Func | Eof)),
            }
        }
    }
}

pub fn parse(tokens: &Tokens) -> Result<Tree, ParseError> {
    let id = TokenId::from(0);
    let tree = Tree::default();
    Parser { tokens, id, tree }.module()
}
