# EIRIAD Programming Language

[![Rust CI](https://github.com/gwhitdev/Eiriad-Programming-Language/actions/workflows/ci.yml/badge.svg)](https://github.com/gwhitdev/Eiriad-Programming-Language/actions/workflows/ci.yml)
[![Deploy WASM Demo to Pages](https://github.com/gwhitdev/Eiriad-Programming-Language/actions/workflows/deploy-pages.yml/badge.svg)](https://github.com/gwhitdev/Eiriad-Programming-Language/actions/workflows/deploy-pages.yml)
[![Live Demo](https://img.shields.io/badge/Live%20Demo-TODO%20App-0f766e)](https://gwhitdev.github.io/Eiriad-Programming-Language/web/todo.html)

EIRIAD is an interpreter-first programming language runtime implemented in Rust.
It runs in two places:

- CLI (`eiriad` command)
- Browser (WebAssembly runtime)

## Contents

- [What Is EIRIAD](#what-is-eiriad)
- [Language Features](#language-features)
- [Quick Start](#quick-start)
- [Linux Installation](#linux-installation)
- [CLI Usage](#cli-usage)
- [Core Server API](#core-server-api)
- [Browser and WASM Usage](#browser-and-wasm-usage)
- [Tutorial: Write and Run an EIRIAD WASM App](#tutorial-write-and-run-an-eiriad-wasm-app)
- [HTTP in EIRIAD](#http-in-eiriad)
- [Examples](#examples)
- [Language and Runtime Architecture](#language-and-runtime-architecture)
- [Extending EIRIAD](#extending-eiriad)
- [GitHub Pages Demo Deployment](#github-pages-demo-deployment)
- [Project Notes and Roadmap](#project-notes-and-roadmap)

## What Is EIRIAD

EIRIAD currently focuses on a tree-walk interpreter pipeline with a semantic
checker and a shared runtime for both CLI and browser/WASM execution.

## Language Features

- Mutable and immutable bindings (`mut`, `let`)
- Assignment for mutable bindings
- Numeric and boolean expressions with precedence
- Operators: `+ - * / % ^ == != < <= > >= && || !`
- User functions: `fn name(params) { expr }`
- Lambdas and closures: `(a, b) -> expr`
- Pipe operator: `value |> fn(args...)`
- Option/Result-style values: `None`, `Some(x)`, `Ok(x)`, `Err(x)`
- Option/Result helpers: `unwrap_or`, `is_some`, `is_none`, `is_ok`, `is_err`
- Match expressions with patterns: `Some(_)`, `None`, `Ok(_)`, `Err(_)`, `_`
- Built-ins: `print`, `len`, `sqrt`, `typeof`, `fetch`, HTTP built-ins
- Core server built-ins (native/CLI): `serve`, `request_method`, `request_path`, `request_body`, `response`
- Terminators: newline or `;`
- Trailing `\` line continuation

### Language Syntax at a Glance

| Concept | Syntax | Example |
|---|---|---|
| Immutable binding | `let <name> = <expr>` | `let radius = 9` |
| Mutable binding | `mut <name> = <expr>` | `mut total = 0` |
| Assignment | `<name> = <expr>` | `total = total + 1` |
| Function declaration | `fn name(params) { expr }` | `fn sq(x) { x * x }` |
| Lambda | `(a, b) -> expr` | `let add = (a, b) -> a + b` |
| Pipe | `value |> fn(args...)` | `9 |> sqrt()` |
| Match | `match value { pattern -> expr }` | `match r { Ok(v) -> v Err(e) -> e }` |
| Option/Result constructors | `Some(x)`, `None`, `Ok(x)`, `Err(x)` | `let maybe = Some(42)` |
| Print | `print(value)` | `print("hello")` |
| HTTP GET | `http_get(url)` | `http_get("https://httpbin.org/get")` |
| HTTP with body | `http_post(url, body)` | `http_post("https://httpbin.org/post", "{\"x\":1}")` |
| Start server | `serve(port, handler)` | `serve(3000, app)` |
| Build response | `response(status, body)` | `response(200, "ok")` |

## Quick Start

Install and run:

```bash
cargo install --path . --force
eiriad
eiriad examples/demo.ei
```

## Linux Installation

### One-line installer (from GitHub)

```bash
curl -fsSL https://raw.githubusercontent.com/gwhitdev/Eiriad-Programming-Language/main/scripts/install_linux.sh | bash
```

### One-line installer (explicit method)

```bash
curl -fsSL https://raw.githubusercontent.com/gwhitdev/Eiriad-Programming-Language/main/scripts/install_linux.sh | bash -s -- --method git
```

### Install from local source checkout

```bash
./scripts/install_linux.sh --method source
```

What the installer does:

1. Ensures Linux is the current OS
2. Installs Rust/cargo via rustup if missing
3. Installs the `eiriad` CLI with `cargo install`
4. Prints PATH guidance if `~/.cargo/bin` is not active yet

## CLI Usage

Run REPL:

```bash
eiriad
```

Run a file:

```bash
eiriad examples/demo.ei
```

Other modes:

```bash
eiriad -c 'print("hello")'
cat examples/demo.ei | eiriad -
```

Development fallback without install:

```bash
cargo run --bin eiriad -- examples/demo.ei
```

Compatibility binary:

- `eiriad-repl` delegates to the same shared CLI implementation.

Shebang support:

```ei
#!/usr/bin/env eiriad
print("hello from shebang")
```

```bash
chmod +x hello.ei
./hello.ei
```

REPL commands:

- `:quit` / `:q`
- `:env`
- `:reset`

## Core Server API

Eiriad now includes a built-in native server API inspired by Node.js/Bun style
ergonomics for quick backend scripts.

Built-ins:

- `serve(port, handler)`
- `request_method(req)`
- `request_path(req)`
- `request_body(req)`
- `response(status, body)`

Example (`examples/server_basic.ei`):

```ei
fn app(req) {
	let method = request_method(req)
	let path = request_path(req)
	let body = request_body(req)
	response(200, "method=" + method + " path=" + path + " body=" + body)
}

print("listening on http://127.0.0.1:3000")
serve(3000, app)
```

Run it:

```bash
eiriad examples/server_basic.ei
```

Then test in another terminal:

```bash
curl -X POST http://127.0.0.1:3000/hello -d 'ping'
```

Notes:

- `serve` is currently native/CLI only.
- Browser/WASM runtime cannot open listening sockets.
- Handler can return:
  - `response(status, body)`
  - `Str` (treated as HTTP 200)
  - `()` (treated as HTTP 204)

## Browser and WASM Usage

Build WASM and serve locally:

```bash
wasm-pack build --target web --features wasm
python3 -m http.server 8080
```

Open:

- `http://localhost:8080/web/` (browser REPL)
- `http://localhost:8080/web/todo.html` (WASM TODO app)

One-line command:

```bash
wasm-pack build --target web --features wasm && python3 -m http.server 8080
```

Make target:

```bash
make wasm-demo
```

Package a `.ei` file as a WASM web app:

```bash
make wasm-ei APP=examples/demo.ei
```

Custom output name:

```bash
make wasm-ei APP=examples/http_methods.ei NAME=http-methods
```

Generated output goes under `web/generated/<name>/`.

## Tutorial: Write and Run an EIRIAD WASM App

### 1) Create a source file

Create `examples/hello_web.ei`:

```ei
print("Hello from Eiriad WASM")

mut count = 41
count = count + 1

print("count = " + count)
count
```

### 2) Generate web app files from `.ei`

```bash
make wasm-ei APP=examples/hello_web.ei NAME=hello-web
```

This builds the WASM runtime and creates:

- `web/generated/hello-web/index.html`
- `web/generated/hello-web/main.js`

### 3) Serve the repository root

```bash
python3 -m http.server 8080
```

### 4) Open the app

Open:

`http://localhost:8080/web/generated/hello-web/`

### 5) Iterate

Edit `examples/hello_web.ei`, regenerate, then hard refresh:

```bash
make wasm-ei APP=examples/hello_web.ei NAME=hello-web
```

## HTTP in EIRIAD

Example:

```ei
let response = http_get("https://httpbin.org/get")

let body = match response {
	Ok(text) -> text
	Err(e) -> "request failed: " + e
}

print(body)
```

Other methods:

```ei
let created = http_post("https://httpbin.org/post", "{\"name\":\"eiriad\"}")
let replaced = http_put("https://httpbin.org/put", "replace")
let changed = http_patch("https://httpbin.org/patch", "patch")
let removed = http_delete("https://httpbin.org/delete")
let headers_only = http_head("https://httpbin.org/get")
let options = http_options("https://httpbin.org/get")
```

## Examples

Run examples:

```bash
eiriad examples/<file>.ei
```

Available examples:

- `examples/demo.ei`
- `examples/fn_decl.ei`
- `examples/lambda_closure.ei`
- `examples/match_option_result.ei`
- `examples/option_result_helpers.ei`
- `examples/http_get_fetch.ei`
- `examples/http_methods.ei`
- `examples/line_continuation.ei`

## Language and Runtime Architecture

Pipeline:

1. Lexer
2. Parser
3. Semantic checker
4. Runtime evaluator

Core code locations:

- `src/lexer.rs`
- `src/parser.rs`
- `src/checker.rs`
- `src/runtime.rs`
- `src/lib.rs`

## Extending EIRIAD

Typical change flow:

1. Add AST representation
2. Add lexer/parser support
3. Add checker rules
4. Add runtime execution behavior
5. Add an example under `examples/`

Useful extension points:

- Built-ins in `src/runtime.rs` and `src/checker.rs`
- New syntax nodes in `src/ast.rs` and parser hooks
- WASM API surface in `src/lib.rs` (`wasm` feature)

## GitHub Pages Demo Deployment

Deployment workflow:

- `.github/workflows/deploy-pages.yml`

Publish steps:

1. Push to `main`
2. In GitHub settings, enable Pages with source set to GitHub Actions
3. Wait for Pages workflow completion

Live URL patterns:

- `https://<your-user>.github.io/<your-repo>/`
- `https://<your-user>.github.io/<your-repo>/web/todo.html`
- `https://<your-user>.github.io/<your-repo>/web/generated/<name>/`

## Project Notes and Roadmap

- Interpreter-first implementation (tree-walk)
- Parser currently ignores type annotations in declarations
- Browser HTTP requests use `XmlHttpRequest` and are subject to CORS
- Broader language features (async/classes/traits/generics/reactivity) remain roadmap work
