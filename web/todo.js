import init, { start_todo_app } from "../pkg/eiriad.js";

const statusEl = document.getElementById("status");

async function boot() {
  try {
    await init();
    start_todo_app();
    statusEl.textContent = "Runtime ready (WASM UI wiring)";
  } catch (error) {
    statusEl.textContent = `Failed to load wasm runtime: ${error}`;
  }
}

boot();
