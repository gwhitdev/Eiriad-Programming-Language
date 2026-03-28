use wasm_bindgen::prelude::*;

use crate::{parse_program, Runtime};

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

    // Initializes TODO state in the runtime.
    pub fn todo_init(&mut self) -> Result<(), JsError> {
        self.eval_in_runtime("mut todo_text = \"\"")?;
        Ok(())
    }

    // Adds one TODO item and returns the current list text.
    pub fn todo_add(&mut self, item: &str) -> Result<String, JsError> {
        let clean = escape_for_eiriad_string(item);
        if clean.is_empty() {
            return self.todo_list();
        }

        let source = format!("todo_text = todo_text + \"- {}\\n\"\ntodo_text", clean);
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
