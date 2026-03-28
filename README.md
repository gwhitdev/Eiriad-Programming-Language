# EIRIAD Rust Runtime + REPL

[![Rust CI](https://github.com/gwhitdev/Eiriad-Programming-Language/actions/workflows/ci.yml/badge.svg)](https://github.com/gwhitdev/Eiriad-Programming-Language/actions/workflows/ci.yml)
[![Deploy WASM Demo to Pages](https://github.com/gwhitdev/Eiriad-Programming-Language/actions/workflows/deploy-pages.yml/badge.svg)](https://github.com/gwhitdev/Eiriad-Programming-Language/actions/workflows/deploy-pages.yml)
[![Live Demo](https://img.shields.io/badge/Live%20Demo-TODO%20App-0f766e)](https://gwhitdev.github.io/Eiriad-Programming-Language/web/todo.html)

This project contains a baseline EIRIAD runtime implemented in Rust with two interfaces:

- Command-line runtime (`eiriad` binary)
- Browser runtime via WebAssembly (`EiriadRuntime` binding)

## Quick Start

```bash
# Install the CLI once
cargo install --path . --force

# Start the REPL
eiriad

# Run a script
eiriad examples/demo.ei
```

### WASM Quick Start

```bash
# Build and serve the browser runtime
make wasm-demo
```

Open `http://localhost:8080/web/`.

## What this MVP supports

- Variable declarations: `let` and `mut`
- Assignment to mutable bindings
- Expressions with precedence: `+ - * / % ^`, comparisons, `&&`, `||`, unary `!` and `-`
- Function calls to built-ins: `print`, `len`, `sqrt`, `typeof`, `fetch`, `http_get`, `http_post`, `http_put`, `http_patch`, `http_delete`, `http_head`, `http_options`
- User-defined functions: `fn name(params) { expr }`
- Lambda expressions and closures: `(a, b) -> expr`
- Pipe operator: `value |> fn(args...)`
- Newlines or `;` as statement terminators
- Trailing `\` line continuation
- Option/Result-style values: `None`, `Some(x)`, `Ok(x)`, `Err(x)`, `unwrap_or`, `is_some`, `is_none`, `is_ok`, `is_err`
- Match expressions for Option/Result patterns: `Some(_)`, `None`, `Ok(_)`, `Err(_)`, `_`
- Semantic checker pass before evaluation (unknown names, immutable assignment, operand/type checks)

## CLI usage

Install the `eiriad` command once:

```bash
cargo install --path . --force
```

Run REPL:

```bash
eiriad
```

Execute a file:

```bash
eiriad examples/demo.ei
```

Other useful modes:

```bash
eiriad -c 'print("hello")'
cat examples/demo.ei | eiriad -
```

Development fallback (without install):

```bash
cargo run --bin eiriad -- examples/demo.ei
```

`eiriad-repl` is a compatibility entrypoint that delegates to the same shared
CLI implementation as `eiriad` (REPL and file mode only).

Shebang usage:

```ei
#!/usr/bin/env eiriad
print("hello from shebang")
```

Make executable and run:

```bash
chmod +x hello.ei
./hello.ei
```

REPL commands:

- `:quit` / `:q` exit
- `:env` show variables
- `:reset` clear state

## Browser (WASM) usage

1. Install `wasm-pack` if needed.
2. Build the WASM package:

```bash
wasm-pack build --target web --features wasm
```

3. Serve the workspace root (or any static server that can read `pkg/` and `web/`):

```bash
python3 -m http.server 8080
```

4. Open `http://localhost:8080/web/`.

WASM TODO example app:

- `http://localhost:8080/web/todo.html`
- Uses EIRIAD runtime state (`todo_text`) in wasm for add/list/clear actions.

One-line wasm demo command:

```bash
wasm-pack build --target web --features wasm && python3 -m http.server 8080
```

Or via Make:

```bash
make wasm-demo
```

### Build a `.ei` file into a WASM web app

You can package any EIRIAD source file as a runnable browser app (using the
WASM runtime) with:

```bash
make wasm-ei APP=examples/demo.ei
```

This generates:

- `web/generated/demo/index.html`
- `web/generated/demo/main.js`

Then serve the repo and open:

`http://localhost:8080/web/generated/demo/`

Custom app name:

```bash
make wasm-ei APP=examples/http_methods.ei NAME=http-methods
```

This outputs to `web/generated/http-methods/`.

## Tutorial: Write and Run an Eiriad WASM App

This walkthrough shows the full flow from writing a `.ei` file to running it in
the browser.

### 1. Create a new app file

Create `examples/hello_web.ei`:

```ei
print("Hello from Eiriad WASM")

mut count = 41
count = count + 1

print("count = " + count)
count
```

The final expression (`count`) is shown as the last `=>` value in the web app.

### 2. Generate a WASM web app from the `.ei` file

```bash
make wasm-ei APP=examples/hello_web.ei NAME=hello-web
```

This command does two things:

1. Builds the Eiriad WASM runtime package (`pkg/`)
2. Generates a runnable web app at `web/generated/hello-web/`

Generated files:

- `web/generated/hello-web/index.html`
- `web/generated/hello-web/main.js`

### 3. Serve the repository root

```bash
python3 -m http.server 8080
```

### 4. Open the app in your browser

Open:

`http://localhost:8080/web/generated/hello-web/`

You will see:

- Source panel with your `.ei` code
- Output panel with `print(...)` output and final result
- `Run App` button to run the program again
- `Reset Runtime` button to clear runtime state

### 5. Edit and re-run (developer loop)

After changing your `.ei` file, regenerate and refresh:

```bash
make wasm-ei APP=examples/hello_web.ei NAME=hello-web
```

Then hard refresh the browser page (`Ctrl+Shift+R`).

### 6. Publish to GitHub Pages

Commit and push your changes to `main`. The Pages workflow builds WASM and
publishes `web/` and `pkg/`.

Your generated app URL pattern is:

`https://<your-user>.github.io/<your-repo>/web/generated/<name>/`

For this example:

`https://<your-user>.github.io/<your-repo>/web/generated/hello-web/`

## Notes

- This is an interpreter-first runtime aligned with the spec's current tree-walk phase.
- The parser intentionally ignores type annotations for now, so declarations like `let x: Int = 1` still run.
- HTTP built-ins work on CLI via `reqwest` and on wasm via browser `XmlHttpRequest`.
- In browser/wasm mode, requests are subject to CORS and browser security policies.
- Many spec features (async, classes, traits, full static inference with generics, reactivity) are still roadmap items.

## HTTP Example

```ei
let response = http_get("https://httpbin.org/get")

let body = match response {
	Ok(text) -> text
	Err(e) -> "request failed: " + e
}

print(body)
```

Additional method examples:

```ei
let created = http_post("https://httpbin.org/post", "{\"name\":\"eiriad\"}")
let replaced = http_put("https://httpbin.org/put", "replace")
let changed = http_patch("https://httpbin.org/patch", "patch")
let removed = http_delete("https://httpbin.org/delete")
let headers_only = http_head("https://httpbin.org/get")
let options = http_options("https://httpbin.org/get")
```

## Examples Index

Run any example with:

```bash
eiriad examples/<file>.ei
```

Examples for each major functionality added:

- Function declarations: `examples/fn_decl.ei`
- Lambdas and closures: `examples/lambda_closure.ei`
- Match with Option/Result patterns: `examples/match_option_result.ei`
- Option/Result helpers (`unwrap_or`, `is_*`): `examples/option_result_helpers.ei`
- HTTP GET + fetch alias: `examples/http_get_fetch.ei`
- Full HTTP methods (`POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`): `examples/http_methods.ei`
- Line continuation with trailing `\`: `examples/line_continuation.ei`
- Combined language walkthrough: `examples/demo.ei`

## Extending EIRIAD

EIRIAD is organized as a small interpreter pipeline, so most language features follow the same pattern:

1. Add syntax representation in AST (`src/ast.rs`)
2. Add tokenization/parsing (`src/lexer.rs`, `src/parser.rs`)
3. Add static validation (`src/checker.rs`)
4. Add runtime behavior (`src/runtime.rs`)
5. Add an example script (`examples/*.ei`) and run it in REPL

### Example 1: Add a new built-in function

Suppose you want to add a new built-in `upper(Str) -> Str`.

Add checker support in `src/checker.rs` inside `check_builtin`:

```rust
"upper" => {
	expect_arg_count(name, args, 1)?;
	if args[0] == Type::Str || args[0] == Type::Unknown {
		Ok(Type::Str)
	} else {
		Err(EiriadError::new("upper expects Str"))
	}
}
```

Add runtime behavior in `src/runtime.rs` inside `call_builtin`:

```rust
"upper" => {
	if args.len() != 1 {
		return Err(EiriadError::new("upper expects exactly 1 argument"));
	}
	match &args[0] {
		Value::Str(s) => Ok(Value::Str(s.to_uppercase())),
		_ => Err(EiriadError::new("upper expects Str")),
	}
}
```

Try it:

```ei
print(upper("eiriad"))
```

### Example 2: Add a new expression form

Suppose you want to add an `if` expression.

1. Add a new AST variant in `src/ast.rs`, for example:

```rust
If {
	cond: Box<Expr>,
	then_expr: Box<Expr>,
	else_expr: Box<Expr>,
}
```

2. Add lexer keywords/tokens in `src/lexer.rs` (`if`, `else`).
3. Parse the grammar in `src/parser.rs` (`parse_if_expr` and call it from `parse_primary`).
4. Type-check in `src/checker.rs`:
   - condition must be `Bool`
   - then/else branch types must be compatible
5. Evaluate in `src/runtime.rs` by evaluating the condition and selected branch.

This same workflow applies to any new language feature (loops, map/list literals, pattern variants, etc).

### Example 3: Expose new behavior to browser WASM

The wasm API lives in `src/lib.rs` under the `wasm` feature. If you add runtime capabilities, they are automatically available when `eval` uses `eval_source`.

Build and run the browser demo:

```bash
make wasm-demo
```

Then open `http://localhost:8080/web/`.

To open the TODO sample directly after build/serve, visit:

`http://localhost:8080/web/todo.html`

## Demonstrate On GitHub

This repository includes a GitHub Pages deploy workflow at [`.github/workflows/deploy-pages.yml`](.github/workflows/deploy-pages.yml).

How to publish the WASM TODO demo:

1. Push your branch to `main`.
2. In GitHub, open repository settings and enable Pages with source set to **GitHub Actions**.
3. Wait for the **Deploy WASM Demo to Pages** workflow to finish.

Demo URLs after deployment:

- `https://<your-user>.github.io/<your-repo>/` (redirects to TODO demo)
- `https://<your-user>.github.io/<your-repo>/web/todo.html`
- `https://<your-user>.github.io/<your-repo>/web/` (REPL page)

For a stronger demo in your README, add a short screen recording (GIF/MP4) and link the live URL near the top of the document.
