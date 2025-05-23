mod codegen;
mod lex;
mod parse;

use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use clap::Parser;
use line_index::{LineIndex, TextSize};
use parse::ParseError;

#[derive(Debug, Parser)]
struct Cli {
    filename: PathBuf,

    #[arg(short)]
    output: Option<PathBuf>,
}

fn main() {
    let args = Cli::parse();
    let src = fs::read_to_string(args.filename).unwrap();
    let tokens = lex::lex(&src).unwrap();
    let tree = match parse::parse(&tokens) {
        Ok(tree) => tree,
        Err(error) => {
            eprintln!("{}", error.message());
            let lines = LineIndex::new(&src);
            match error {
                ParseError::Expected { id, kinds: _ } => {
                    let line_col = lines.line_col(TextSize::new(u32::from(tokens[id].start)));
                    eprintln!("gmm.adroit:{}:{}", line_col.line + 1, line_col.col + 1);
                }
            }
            return;
        }
    };
    let bytes = codegen::codegen(&src, &tokens, &tree).finish();
    match args.output {
        Some(filename) => fs::write(filename, bytes).unwrap(),
        None => io::stdout().write_all(&bytes).unwrap(),
    }
}
