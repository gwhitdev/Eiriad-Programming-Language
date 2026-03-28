use wasm_bindgen::prelude::*;

use crate::{parse_program, Runtime};

fn escape_for_eiriad_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "")
        .replace('\n', " ")
}

#[wasm_bindgen]
pub struct EiriadRuntime {
    inner: Runtime,
}

#[wasm_bindgen]
impl EiriadRuntime {
    fn eval_in_runtime(&mut self, source: &str) -> Result<crate::ExecResult, JsError> {
        let program = parse_program(source).map_err(|e| JsError::new(&e.to_string()))?;
        self.inner
            .exec_program(&program)
            .map_err(|e| JsError::new(&e.to_string()))
    }

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
        let result = self.eval_in_runtime(source)?;

        let mut lines = result.output;
        lines.push(format!("=> {}", result.last_value));
        Ok(lines.join("\n"))
    }

    // Returns only the final expression value as a string.
    pub fn eval_value(&mut self, source: &str) -> Result<String, JsError> {
        let result = self.eval_in_runtime(source)?;
        Ok(result.last_value.to_string())
    }

    // Calls a zero-arg Eiriad function and returns its final value.
    pub fn call0(&mut self, fn_name: &str) -> Result<String, JsError> {
        let source = format!("{}()", fn_name.trim());
        self.eval_value(&source)
    }

    // Calls a one-arg Eiriad function with a safely escaped string argument.
    pub fn call1(&mut self, fn_name: &str, arg: &str) -> Result<String, JsError> {
        let source = format!(
            "{}(\"{}\")",
            fn_name.trim(),
            escape_for_eiriad_string(arg)
        );
        self.eval_value(&source)
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
