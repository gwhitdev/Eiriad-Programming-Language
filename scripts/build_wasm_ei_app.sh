#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 <path/to/file.ei> [app-name]" >&2
  exit 1
fi

SOURCE_FILE="$1"
APP_NAME="${2:-$(basename "${SOURCE_FILE%.ei}")}"

if [[ ! -f "$SOURCE_FILE" ]]; then
  echo "Source file not found: $SOURCE_FILE" >&2
  exit 1
fi

if [[ "${SOURCE_FILE##*.}" != "ei" ]]; then
  echo "Expected a .ei source file, got: $SOURCE_FILE" >&2
  exit 1
fi

if [[ -z "$APP_NAME" ]]; then
  echo "Unable to determine app name. Pass one explicitly as second argument." >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$REPO_ROOT/web/generated/$APP_NAME"

mkdir -p "$OUT_DIR"

echo "Building Eiriad WASM runtime..."
(
  cd "$REPO_ROOT"
  wasm-pack build --target web --features wasm
)

ENCODED_SOURCE="$(base64 < "$SOURCE_FILE" | tr -d '\n')"

cat > "$OUT_DIR/index.html" <<EOF
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Eiriad WASM App: $APP_NAME</title>
    <style>
      :root {
        --bg: #f7f5ef;
        --panel: #fffdf8;
        --ink: #231c13;
        --line: #e8dece;
        --accent: #0f766e;
      }
      body {
        margin: 0;
        min-height: 100vh;
        background: radial-gradient(circle at top right, #e8f7f5, var(--bg));
        color: var(--ink);
        font-family: "IBM Plex Sans", "Segoe UI", sans-serif;
      }
      .shell {
        max-width: 960px;
        margin: 32px auto;
        padding: 0 14px;
      }
      .card {
        background: var(--panel);
        border: 1px solid var(--line);
        border-radius: 14px;
        padding: 14px;
        box-shadow: 0 12px 24px rgba(39, 31, 20, 0.07);
      }
      h1 {
        margin-top: 0;
      }
      .row {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 12px;
      }
      @media (max-width: 780px) {
        .row {
          grid-template-columns: 1fr;
        }
      }
      pre {
        margin: 0;
        min-height: 260px;
        overflow: auto;
        border-radius: 10px;
        border: 1px solid var(--line);
        padding: 10px;
        font-family: "Iosevka", "JetBrains Mono", monospace;
        font-size: 13px;
      }
      #source {
        background: #fff;
      }
      #output {
        background: #111a25;
        color: #d9f4ff;
        border-color: #1f3044;
      }
      .controls {
        display: flex;
        gap: 10px;
        flex-wrap: wrap;
        margin-top: 10px;
      }
      button {
        border: 0;
        border-radius: 9px;
        background: var(--accent);
        color: #fff;
        font-weight: 700;
        padding: 9px 12px;
        cursor: pointer;
      }
      .muted {
        color: #665c50;
        margin-top: 10px;
        font-size: 12px;
      }
      code {
        font-family: "Iosevka", "JetBrains Mono", monospace;
      }
    </style>
  </head>
  <body>
    <main class="shell">
      <section class="card">
        <h1>Eiriad WASM App: $APP_NAME</h1>
        <div class="row">
          <pre id="source"></pre>
          <pre id="output">Loading wasm runtime...</pre>
        </div>
        <div class="controls">
          <button id="run">Run App</button>
          <button id="reset">Reset Runtime</button>
        </div>
        <div class="muted">
          Generated from <code>$SOURCE_FILE</code>
        </div>
      </section>
    </main>
    <script type="module" src="./main.js"></script>
  </body>
</html>
EOF

cat > "$OUT_DIR/main.js" <<EOF
import init, { EiriadRuntime } from "../../../pkg/eiriad.js";

const encoded = "$ENCODED_SOURCE";
const source = atob(encoded);

const sourceEl = document.getElementById("source");
const outputEl = document.getElementById("output");
const runBtn = document.getElementById("run");
const resetBtn = document.getElementById("reset");

sourceEl.textContent = source;

let runtime;

function runProgram() {
  if (!runtime) {
    outputEl.textContent = "Runtime not ready";
    return;
  }

  try {
    const out = runtime.eval(source);
    outputEl.textContent = out;
  } catch (error) {
    outputEl.textContent = "error: " + error;
  }
}

async function boot() {
  try {
    await init();
    runtime = new EiriadRuntime();
    runProgram();
  } catch (error) {
    outputEl.textContent = "Failed to load wasm runtime: " + error;
  }
}

runBtn.addEventListener("click", runProgram);
resetBtn.addEventListener("click", () => {
  if (!runtime) {
    return;
  }
  runtime.reset();
  outputEl.textContent = "Runtime state reset. Click 'Run App' to run again.";
});

boot();
EOF

echo "Generated WASM app: $OUT_DIR/index.html"
