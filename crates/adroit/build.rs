use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    mem::take,
    path::Path,
};

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer};

fn flag<'de, D: Deserializer<'de>>(de: D) -> Result<bool, D::Error> {
    type Unit = ();
    Unit::deserialize(de)?;
    Ok(true)
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Contents<'a> {
    Uint {
        uint: (),
    },
    Int32 {
        int32: (),
    },
    Float64 {
        float64: (),
    },
    String {
        string: (),
    },
    Type {
        #[serde(borrow, rename = "type")]
        name: &'a str,
    },
    List {
        #[serde(borrow, rename = "list")]
        element: Box<Type<'a>>,
    },
    Record {
        #[serde(borrow, rename = "record")]
        fields: IndexMap<&'a str, Box<Type<'a>>>,
    },
    Enum {
        #[serde(borrow, rename = "enum")]
        variants: IndexMap<&'a str, Box<Type<'a>>>,
    },
}

#[derive(Debug, Deserialize)]
struct Type<'a> {
    #[serde(default, deserialize_with = "flag")]
    optional: bool,

    #[serde(borrow)]
    name: Option<&'a str>,

    doc: Option<String>,

    #[serde(borrow, flatten)]
    data: Option<Contents<'a>>,
}

#[derive(Debug, Deserialize)]
struct Types<'a> {
    #[serde(borrow)]
    types: IndexMap<&'a str, Type<'a>>,
}

#[derive(Debug)]
struct Writer<'a, W> {
    w: W,
    types: Vec<(&'a str, &'a Type<'a>)>,
}

impl<'a, W: Write> Writer<'a, W> {
    fn indent(&mut self, indent: usize) -> io::Result<()> {
        for _ in 0..indent {
            write!(self.w, "    ")?;
        }
        Ok(())
    }

    fn doc_comment(&mut self, indent: usize, doc: &'a Option<String>) -> io::Result<()> {
        if let Some(string) = doc {
            for line in string.lines() {
                self.indent(indent)?;
                if line.is_empty() {
                    writeln!(self.w, "///")?;
                } else {
                    writeln!(self.w, "/// {line}")?;
                }
            }
        }
        Ok(())
    }

    fn ty(&mut self, ty: &'a Type) -> io::Result<()> {
        match &ty.data {
            None => unimplemented!(),
            Some(contents) => match contents {
                Contents::Uint { uint: () } => write!(self.w, "usize")?,
                Contents::Int32 { int32: () } => write!(self.w, "i32")?,
                Contents::Float64 { float64: () } => write!(self.w, "f64")?,
                Contents::String { string: () } => write!(self.w, "Box<str>")?,
                Contents::Type { name } => write!(self.w, "{name}")?,
                Contents::List { element } => {
                    write!(self.w, "Box<[")?;
                    self.ty(element)?;
                    write!(self.w, "]>")?;
                }
                Contents::Record { fields: _ } | Contents::Enum { variants: _ } => {
                    let name = ty.name.unwrap();
                    write!(self.w, "{name}")?;
                    self.types.push((name, ty));
                }
            },
        }
        Ok(())
    }

    fn field(&mut self, indent: usize, public: bool, name: &str, ty: &'a Type) -> io::Result<()> {
        self.doc_comment(indent, &ty.doc)?;
        let rename = match name {
            "return" => Some("ret"),
            "type" => Some("ty"),
            _ => None,
        };
        if rename.is_some() {
            self.indent(indent)?;
            writeln!(self.w, "#[serde(rename = {name:?})]")?;
        }
        if ty.optional {
            self.indent(indent)?;
            writeln!(
                self.w,
                "#[serde(skip_serializing_if = \"Option::is_none\")]"
            )?;
        }
        self.indent(indent)?;
        if public {
            write!(self.w, "pub ")?;
        }
        write!(self.w, "{}", rename.unwrap_or(name))?;
        write!(self.w, ": ")?;
        if ty.optional {
            write!(self.w, "Option<")?;
        }
        self.ty(ty)?;
        if ty.optional {
            write!(self.w, ">")?;
        }
        writeln!(self.w, ",")?;
        Ok(())
    }

    fn variant(&mut self, name: &str, ty: &'a Type) -> io::Result<()> {
        self.doc_comment(1, &ty.doc)?;
        write!(self.w, "    {name}")?;
        match &ty.data {
            None => writeln!(self.w, ",")?,
            Some(contents) => match contents {
                Contents::Record { fields } => {
                    writeln!(self.w, " {{")?;
                    let mut first = true;
                    for (name, ty) in fields {
                        if !first {
                            writeln!(self.w)?;
                        }
                        first = false;
                        self.field(2, false, name, ty)?;
                    }
                    writeln!(self.w, "    }},")?;
                }
                _ => todo!(),
            },
        }
        Ok(())
    }

    fn toplevel(&mut self, name: &str, ty: &'a Type) -> io::Result<()> {
        self.doc_comment(0, &ty.doc)?;
        match &ty.data {
            None => unimplemented!(),
            Some(contents) => match contents {
                Contents::Uint { uint: () }
                | Contents::Int32 { int32: () }
                | Contents::Float64 { float64: () }
                | Contents::String { string: () } => {
                    writeln!(self.w, "#[derive(Serialize)]")?;
                    write!(self.w, "pub struct {name}(pub ")?;
                    self.ty(ty)?;
                    writeln!(self.w, ");")?;
                }
                Contents::Type { name: _ } => unimplemented!(),
                Contents::List { element } => {
                    write!(self.w, "pub type {name} = Box<[")?;
                    self.ty(element)?;
                    writeln!(self.w, "]>;")?;
                }
                Contents::Record { fields } => {
                    writeln!(self.w, "#[derive(Serialize)]")?;
                    writeln!(self.w, "pub struct {name} {{")?;
                    let mut first = true;
                    for (name, ty) in fields {
                        if !first {
                            writeln!(self.w)?;
                        }
                        first = false;
                        self.field(1, true, name, ty)?;
                    }
                    writeln!(self.w, "}}")?;
                }
                Contents::Enum { variants } => {
                    writeln!(self.w, "#[derive(Serialize)]")?;
                    writeln!(self.w, "#[serde(tag = \"kind\")]")?;
                    writeln!(self.w, "pub enum {name} {{")?;
                    let mut first = true;
                    for (name, ty) in variants {
                        if !first {
                            writeln!(self.w)?;
                        }
                        first = false;
                        self.variant(name, ty)?;
                    }
                    writeln!(self.w, "}}")?;
                }
            },
        }
        Ok(())
    }

    fn all(&mut self) -> io::Result<()> {
        writeln!(self.w, "use serde::Serialize;").unwrap();
        writeln!(self.w)?;
        let mut first = true;
        loop {
            let types = take(&mut self.types);
            if types.is_empty() {
                return Ok(());
            }
            for (name, ty) in types {
                if !first {
                    writeln!(self.w)?;
                }
                first = false;
                self.toplevel(name, ty)?;
            }
        }
    }
}

fn main() {
    let src = "gradbench.yml";
    println!("cargo::rerun-if-changed={src}");
    let string = fs::read_to_string(src).unwrap();
    let types: Types = serde_yaml::from_str(&string).unwrap();
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("ir.rs");
    let mut writer = Writer {
        w: File::create(out).unwrap(),
        types: types.types.iter().map(|(&k, v)| (k, v)).collect(),
    };
    writer.all().unwrap();
}
