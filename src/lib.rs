mod ast;
mod checker;
pub mod cli_common;
mod error;
mod lexer;
mod parser;
mod runtime;

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

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
mod wasm_api {
    use std::cell::RefCell;
    use std::rc::Rc;

    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{window, Document, HtmlElement, HtmlInputElement, KeyboardEvent};

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

    struct TodoUiState {
        runtime: EiriadRuntime,
        status_el: HtmlElement,
        list_el: HtmlElement,
        trace_el: HtmlElement,
        input_el: HtmlInputElement,
    }

    impl TodoUiState {
        fn paint_todos(&self, raw_text: &str) {
            self.list_el.set_inner_html("");

            let rows: Vec<&str> = raw_text
                .split('\n')
                .map(str::trim)
                .filter(|row| !row.is_empty())
                .collect();

            let document = self
                .list_el
                .owner_document()
                .expect("list element should have a document");

            if rows.is_empty() {
                if let Ok(li) = document.create_element("li") {
                    li.set_text_content(Some("No tasks yet."));
                    let _ = li.set_attribute("style", "color: #6e6458");
                    let _ = self.list_el.append_child(&li);
                }
                return;
            }

            for row in rows {
                if let Ok(li) = document.create_element("li") {
                    li.set_text_content(Some(row));
                    let _ = self.list_el.append_child(&li);
                }
            }
        }

        fn refresh_todos(&mut self) -> Result<(), JsError> {
            let list_text = self.runtime.todo_list()?;
            self.paint_todos(&list_text);
            self.trace_el
                .set_text_content(Some(&format!("todo_list() => {:?}", list_text)));
            Ok(())
        }

        fn add_todo(&mut self) -> Result<(), JsError> {
            let clean = self.input_el.value().trim().to_string();
            if clean.is_empty() {
                self.status_el
                    .set_text_content(Some("Enter a task before adding."));
                return Ok(());
            }

            let list_text = self.runtime.todo_add(&clean)?;
            self.paint_todos(&list_text);
            self.trace_el
                .set_text_content(Some(&format!("todo_add({:?}) => {:?}", clean, list_text)));
            self.input_el.set_value("");
            self.status_el
                .set_text_content(Some(&format!("Added: {}", clean)));
            Ok(())
        }

        fn clear_todos(&mut self) -> Result<(), JsError> {
            let list_text = self.runtime.todo_clear()?;
            self.paint_todos(&list_text);
            self.trace_el
                .set_text_content(Some(&format!("todo_clear() => {:?}", list_text)));
            self.status_el.set_text_content(Some("All tasks cleared."));
            Ok(())
        }
    }

    fn document() -> Result<Document, JsError> {
        window()
            .ok_or_else(|| JsError::new("Window not available"))?
            .document()
            .ok_or_else(|| JsError::new("Document not available"))
    }

    fn html_el_by_id(document: &Document, id: &str) -> Result<HtmlElement, JsError> {
        document
            .get_element_by_id(id)
            .ok_or_else(|| JsError::new(&format!("Missing element with id '{}'", id)))?
            .dyn_into::<HtmlElement>()
            .map_err(|_| JsError::new(&format!("Element '{}' is not HtmlElement", id)))
    }

    fn input_el_by_id(document: &Document, id: &str) -> Result<HtmlInputElement, JsError> {
        document
            .get_element_by_id(id)
            .ok_or_else(|| JsError::new(&format!("Missing input with id '{}'", id)))?
            .dyn_into::<HtmlInputElement>()
            .map_err(|_| JsError::new(&format!("Element '{}' is not HtmlInputElement", id)))
    }

    fn status_error(status_el: &HtmlElement, message: &str) {
        status_el.set_text_content(Some(message));
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

    #[wasm_bindgen]
    pub fn start_todo_app() -> Result<(), JsError> {
        let document = document()?;

        let status_el = html_el_by_id(&document, "status")?;
        let list_el = html_el_by_id(&document, "todoList")?;
        let trace_el = html_el_by_id(&document, "trace")?;
        let add_btn = html_el_by_id(&document, "addBtn")?;
        let refresh_btn = html_el_by_id(&document, "refreshBtn")?;
        let clear_btn = html_el_by_id(&document, "clearBtn")?;
        let input_el = input_el_by_id(&document, "todoInput")?;

        let mut runtime = EiriadRuntime::new();
        runtime.todo_init()?;

        let state = Rc::new(RefCell::new(TodoUiState {
            runtime,
            status_el: status_el.clone(),
            list_el,
            trace_el,
            input_el: input_el.clone(),
        }));

        {
            let mut s = state.borrow_mut();
            s.trace_el.set_text_content(Some("todo_init() => ok"));
            s.refresh_todos()?;
            s.status_el.set_text_content(Some("Runtime ready"));
        }

        let add_state = Rc::clone(&state);
        let add_handler = Closure::wrap(Box::new(move || {
            let status = add_state.borrow().status_el.clone();
            if let Err(err) = add_state.borrow_mut().add_todo() {
                status_error(&status, &format!("Failed to add TODO: {:?}", err));
            }
        }) as Box<dyn FnMut()>);
        add_btn.set_onclick(Some(add_handler.as_ref().unchecked_ref()));
        add_handler.forget();

        let refresh_state = Rc::clone(&state);
        let refresh_handler = Closure::wrap(Box::new(move || {
            let status = refresh_state.borrow().status_el.clone();
            if let Err(err) = refresh_state.borrow_mut().refresh_todos() {
                status_error(&status, &format!("Failed to refresh TODOs: {:?}", err));
            }
        }) as Box<dyn FnMut()>);
        refresh_btn.set_onclick(Some(refresh_handler.as_ref().unchecked_ref()));
        refresh_handler.forget();

        let clear_state = Rc::clone(&state);
        let clear_handler = Closure::wrap(Box::new(move || {
            let status = clear_state.borrow().status_el.clone();
            if let Err(err) = clear_state.borrow_mut().clear_todos() {
                status_error(&status, &format!("Failed to clear TODOs: {:?}", err));
            }
        }) as Box<dyn FnMut()>);
        clear_btn.set_onclick(Some(clear_handler.as_ref().unchecked_ref()));
        clear_handler.forget();

        let key_state = Rc::clone(&state);
        let key_handler = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            if event.key() != "Enter" {
                return;
            }
            event.prevent_default();
            let status = key_state.borrow().status_el.clone();
            if let Err(err) = key_state.borrow_mut().add_todo() {
                status_error(&status, &format!("Failed to add TODO: {:?}", err));
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);
        input_el.set_onkeydown(Some(key_handler.as_ref().unchecked_ref()));
        key_handler.forget();

        Ok(())
    }
}
