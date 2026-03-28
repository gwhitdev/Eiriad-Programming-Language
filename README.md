# EIRIAD Rust Runtime + REPL

This project contains a baseline EIRIAD runtime implemented in Rust with two interfaces:

- Command-line REPL (`eiriad-repl` binary)
- Browser runtime via WebAssembly (`EiriadRuntime` binding)

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

Build and run REPL:

```bash
cargo run --bin eiriad-repl
```

Execute a file:

```bash
cargo run --bin eiriad-repl -- examples/demo.ei
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

One-line wasm demo command:

```bash
wasm-pack build --target web --features wasm && python3 -m http.server 8080
```

Or via Make:

```bash
make wasm-demo
```

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
cargo run --bin eiriad-repl -- examples/<file>.ei
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
