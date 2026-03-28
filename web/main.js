import init, { EiriadRuntime } from "../pkg/eiriad.js";

const statusEl = document.getElementById("status");
const outEl = document.getElementById("out");
const sourceEl = document.getElementById("source");
const runBtn = document.getElementById("runBtn");
const resetBtn = document.getElementById("resetBtn");

let runtime;

async function boot() {
  try {
    await init();
    runtime = new EiriadRuntime();
    statusEl.textContent = "Runtime ready";
  } catch (err) {
    statusEl.textContent = `Failed to load wasm runtime: ${err}`;
  }
}

runBtn.addEventListener("click", () => {
  if (!runtime) {
    statusEl.textContent = "Runtime not ready yet";
    return;
  }
  try {
    const result = runtime.eval(sourceEl.value);
    outEl.textContent = result;
  } catch (err) {
    outEl.textContent = `error: ${err}`;
  }
});

resetBtn.addEventListener("click", () => {
  if (!runtime) {
    return;
  }
  runtime.reset();
  outEl.textContent = "(state cleared)";
});

boot();
