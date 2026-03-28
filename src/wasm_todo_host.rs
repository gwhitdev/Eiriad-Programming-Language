use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, Document, HtmlElement, HtmlInputElement, KeyboardEvent, XmlHttpRequest};

use super::runtime_exports::EiriadRuntime;

struct TodoUiState {
    runtime: EiriadRuntime,
    status_el: HtmlElement,
    list_el: HtmlElement,
    trace_el: HtmlElement,
    input_el: HtmlInputElement,
    empty_placeholder: String,
    status_need_input: String,
    status_cleared: String,
}

impl TodoUiState {
    fn render_list_text(&self, list_text: &str) {
        if list_text.trim().is_empty() {
            self.list_el.set_text_content(Some(&self.empty_placeholder));
        } else {
            self.list_el.set_text_content(Some(list_text));
        }
    }

    fn refresh_todos(&mut self) -> Result<(), JsError> {
        let list_text = self.runtime.call0("todo_list")?;
        self.render_list_text(&list_text);
        self.trace_el
            .set_text_content(Some(&format!("todo_list() => {:?}", list_text)));
        Ok(())
    }

    fn add_todo(&mut self) -> Result<(), JsError> {
        let clean = self.input_el.value().trim().to_string();
        if clean.is_empty() {
            self.status_el.set_text_content(Some(&self.status_need_input));
            return Ok(());
        }

        let list_text = self.runtime.call1("todo_add", &clean)?;
        self.render_list_text(&list_text);
        self.trace_el
            .set_text_content(Some(&format!("todo_add({:?}) => {:?}", clean, list_text)));
        self.input_el.set_value("");

        let status = self.runtime.call1("todo_status_added", &clean)?;
        self.status_el.set_text_content(Some(&status));
        Ok(())
    }

    fn clear_todos(&mut self) -> Result<(), JsError> {
        let list_text = self.runtime.call0("todo_clear")?;
        self.render_list_text(&list_text);
        self.trace_el
            .set_text_content(Some(&format!("todo_clear() => {:?}", list_text)));
        self.status_el.set_text_content(Some(&self.status_cleared));
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

fn load_text_via_xhr(path: &str) -> Result<String, JsError> {
    let xhr = XmlHttpRequest::new().map_err(|_| JsError::new("Failed to create XmlHttpRequest"))?;
    xhr.open_with_async("GET", path, false)
        .map_err(|_| JsError::new("Failed to open request for .ei source"))?;
    xhr.send()
        .map_err(|_| JsError::new("Failed to send request for .ei source"))?;

    let status = xhr
        .status()
        .map_err(|_| JsError::new("Unable to read XHR status"))?;
    if status != 200 {
        return Err(JsError::new(&format!(
            "Failed to load .ei source '{}' (HTTP {})",
            path, status
        )));
    }

    xhr.response_text()
        .map_err(|_| JsError::new("Unable to read .ei response text"))?
        .ok_or_else(|| JsError::new(".ei response body was empty"))
}

#[wasm_bindgen]
pub fn start_todo_app_from_ei(source_path: &str) -> Result<(), JsError> {
    let document = document()?;

    let status_el = html_el_by_id(&document, "status")?;
    let list_el = html_el_by_id(&document, "todoList")?;
    let trace_el = html_el_by_id(&document, "trace")?;
    let add_btn = html_el_by_id(&document, "addBtn")?;
    let refresh_btn = html_el_by_id(&document, "refreshBtn")?;
    let clear_btn = html_el_by_id(&document, "clearBtn")?;
    let input_el = input_el_by_id(&document, "todoInput")?;

    let source = load_text_via_xhr(source_path)?;

    let mut runtime = EiriadRuntime::new();
    let init_trace = runtime.eval(&source)?;
    trace_el.set_text_content(Some(&format!("load {} => {:?}", source_path, init_trace)));

    let empty_placeholder = runtime.call0("todo_empty_placeholder")?;
    let status_need_input = runtime.call0("todo_status_need_input")?;
    let status_cleared = runtime.call0("todo_status_cleared")?;
    let status_ready = runtime.call0("todo_status_ready")?;

    let state = Rc::new(RefCell::new(TodoUiState {
        runtime,
        status_el: status_el.clone(),
        list_el,
        trace_el,
        input_el: input_el.clone(),
        empty_placeholder,
        status_need_input,
        status_cleared,
    }));

    {
        let mut s = state.borrow_mut();
        s.refresh_todos()?;
        s.status_el.set_text_content(Some(&status_ready));
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
