mod ast;
mod checker;
pub mod cli_common;
mod error;
mod lexer;
mod parser;
mod runtime;

pub use error::{EiriadError, EiriadResult};
pub use runtime::{ExecResult, Runtime, Value};

use lexer::Lexer;
use parser::Parser;
use checker::Checker;

pub fn parse_program(source: &str) -> EiriadResult<Vec<ast::Stmt>> {
    let tokens = Lexer::new(source).lex()?;
    Parser::new(tokens).parse_program()
}

pub fn eval_source(runtime: &mut Runtime, source: &str) -> EiriadResult<ExecResult> {
    let program = parse_program(source)?;
    Checker::new().check_program(&program)?;
    runtime.exec_program(&program)
}

#[cfg(feature = "wasm")]
mod wasm_api {
    use wasm_bindgen::prelude::*;

    use crate::{eval_source, Runtime};

    #[wasm_bindgen]
    pub struct EiriadRuntime {
        inner: Runtime,
    }

    #[wasm_bindgen]
    impl EiriadRuntime {
        #[wasm_bindgen(constructor)]
        pub fn new() -> EiriadRuntime {
            EiriadRuntime {
                inner: Runtime::new(),
            }
        }

        pub fn reset(&mut self) {
            self.inner.reset();
        }

        // Returns newline-joined printed output and the final expression value.
        pub fn eval(&mut self, source: &str) -> Result<String, JsError> {
            let result = eval_source(&mut self.inner, source)
                .map_err(|e| JsError::new(&e.to_string()))?;

            let mut lines = result.output;
            lines.push(format!("=> {}", result.last_value));
            Ok(lines.join("\n"))
        }

        pub fn env_snapshot(&self) -> String {
            self.inner
                .snapshot_env()
                .into_iter()
                .map(|(_, row)| row)
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}
