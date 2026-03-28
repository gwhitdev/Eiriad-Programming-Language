import init, { start_todo_app_from_ei } from "../pkg/eiriad.js";

const statusEl = document.getElementById("status");

async function boot() {
  try {
    await init();
    start_todo_app_from_ei("./todo.ei");
  } catch (error) {
    statusEl.textContent = `Failed to load wasm runtime: ${error}`;
  }
}

boot();
