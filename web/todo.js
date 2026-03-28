import init, { start_todo_app } from "../pkg/eiriad.js";

const statusEl = document.getElementById("status");

async function boot() {
  try {
    await init();
    start_todo_app();
  } catch (error) {
    statusEl.textContent = `Failed to load wasm runtime: ${error}`;
  }
}

boot();
