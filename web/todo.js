import init, { EiriadRuntime } from "../pkg/eiriad.js";

const statusEl = document.getElementById("status");
const inputEl = document.getElementById("todoInput");
const addBtn = document.getElementById("addBtn");
const refreshBtn = document.getElementById("refreshBtn");
const clearBtn = document.getElementById("clearBtn");
const listEl = document.getElementById("todoList");
const traceEl = document.getElementById("trace");

let runtime;

const BOOTSTRAP_SOURCE = 'let mut todo_text = ""';

function escapeEiriadString(value) {
  return value
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/\r/g, "")
    .replace(/\n/g, " ")
    .trim();
}

function extractEvalValue(evalOutput) {
  const marker = "=> ";
  const markerIndex = evalOutput.lastIndexOf(marker);
  if (markerIndex < 0) {
    return "";
  }
  return evalOutput.slice(markerIndex + marker.length);
}

function paintTodos(rawText) {
  listEl.innerHTML = "";
  const rows = rawText
    .split("\n")
    .map((item) => item.trim())
    .filter(Boolean);

  if (rows.length === 0) {
    const li = document.createElement("li");
    li.textContent = "No tasks yet.";
    li.style.color = "#6e6458";
    listEl.appendChild(li);
    return;
  }

  for (const row of rows) {
    const li = document.createElement("li");
    li.textContent = row;
    listEl.appendChild(li);
  }
}

function runEval(source) {
  const output = runtime.eval(source);
  traceEl.textContent = output;
  return output;
}

function refreshTodos() {
  const out = runEval("todo_text");
  paintTodos(extractEvalValue(out));
}

function addTodo() {
  if (!runtime) {
    statusEl.textContent = "Runtime not ready yet";
    return;
  }

  const clean = escapeEiriadString(inputEl.value);
  if (!clean) {
    statusEl.textContent = "Enter a task before adding.";
    return;
  }

  const script = `todo_text = todo_text + "- ${clean}\\n"\ntodo_text`;
  runEval(script);
  inputEl.value = "";
  refreshTodos();
  statusEl.textContent = `Added: ${clean}`;
}

function clearTodos() {
  if (!runtime) {
    return;
  }
  runEval('todo_text = ""\ntodo_text');
  refreshTodos();
  statusEl.textContent = "All tasks cleared.";
}

async function boot() {
  try {
    await init();
    runtime = new EiriadRuntime();
    runEval(BOOTSTRAP_SOURCE);
    refreshTodos();
    statusEl.textContent = "Runtime ready";
  } catch (error) {
    statusEl.textContent = `Failed to load wasm runtime: ${error}`;
  }
}

addBtn.addEventListener("click", addTodo);
refreshBtn.addEventListener("click", refreshTodos);
clearBtn.addEventListener("click", clearTodos);

inputEl.addEventListener("keydown", (event) => {
  if (event.key === "Enter") {
    event.preventDefault();
    addTodo();
  }
});

boot();
