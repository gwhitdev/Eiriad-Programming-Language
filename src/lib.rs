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

    fn escape_for_eiriad_string(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\r', "")
            .replace('\n', " ")
            .trim()
            .to_string()
    }

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

        // Returns only the final expression value as a string.
        pub fn eval_value(&mut self, source: &str) -> Result<String, JsError> {
            let result = eval_source(&mut self.inner, source)
                .map_err(|e| JsError::new(&e.to_string()))?;
            Ok(result.last_value.to_string())
        }

        // Initializes TODO state in the runtime.
        pub fn todo_init(&mut self) -> Result<(), JsError> {
            eval_source(&mut self.inner, "let mut todo_text = \"\"")
                .map_err(|e| JsError::new(&e.to_string()))?;
            Ok(())
        }

        // Adds one TODO item and returns the current list text.
        pub fn todo_add(&mut self, item: &str) -> Result<String, JsError> {
            let clean = escape_for_eiriad_string(item);
            if clean.is_empty() {
                return self.todo_list();
            }

            let source = format!(
                "todo_text = todo_text + \"- {}\\n\"\ntodo_text",
                clean
            );
            self.eval_value(&source)
        }

        // Clears TODO list and returns the current list text.
        pub fn todo_clear(&mut self) -> Result<String, JsError> {
            self.eval_value("todo_text = \"\"\ntodo_text")
        }

        // Reads TODO list text from runtime state.
        pub fn todo_list(&mut self) -> Result<String, JsError> {
            self.eval_value("todo_text")
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
