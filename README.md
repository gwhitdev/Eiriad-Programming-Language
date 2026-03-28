# EIRIAD Rust Runtime + REPL

This project contains a baseline EIRIAD runtime implemented in Rust with two interfaces:

- Command-line REPL (`eiriad-repl` binary)
- Browser runtime via WebAssembly (`EiriadRuntime` binding)

## What this MVP supports

- Variable declarations: `let` and `mut`
- Assignment to mutable bindings
- Expressions with precedence: `+ - * / % ^`, comparisons, `&&`, `||`, unary `!` and `-`
- Function calls to built-ins: `print`, `len`, `sqrt`, `typeof`
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
cargo run --bin eiriad-repl -- examples/demo.vx
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

## Notes

- This is an interpreter-first runtime aligned with the spec's current tree-walk phase.
- The parser intentionally ignores type annotations for now, so declarations like `let x: Int = 1` still run.
- Many spec features (async, classes, traits, full static inference with generics, reactivity) are still roadmap items.
