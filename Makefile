.PHONY: wasm-demo

wasm-demo:
	wasm-pack build --target web --features wasm && python3 -m http.server 8080
