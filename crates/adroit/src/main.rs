mod cli;
mod compile;
mod fetch;
mod graph;
mod ir;
mod lex;
mod lower;
mod lsp;
mod parse;
mod pprint;
mod range;
mod typecheck;
mod util;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::cli() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
