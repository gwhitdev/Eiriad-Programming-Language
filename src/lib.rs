mod ast;
mod checker;
pub mod cli_common;
mod error;
mod lexer;
mod parser;
mod runtime;
#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
mod wasm_api;

pub use error::{EiriadError, EiriadResult};
pub use runtime::{ExecResult, Runtime, Value};

use checker::Checker;
use lexer::Lexer;
use parser::Parser;

pub fn parse_program(source: &str) -> EiriadResult<Vec<ast::Stmt>> {
    let tokens = Lexer::new(source).lex()?;
    Parser::new(tokens).parse_program()
}

pub fn eval_source(runtime: &mut Runtime, source: &str) -> EiriadResult<ExecResult> {
    let program = parse_program(source)?;
    Checker::new().check_program(&program)?;
    runtime.exec_program(&program)
}
