.PHONY: wasm-demo wasm-ei

wasm-demo:
	wasm-pack build --target web --features wasm && python3 -m http.server 8080

wasm-ei:
	@if [ -z "$(APP)" ]; then echo "Usage: make wasm-ei APP=examples/demo.ei [NAME=demo]"; exit 1; fi
	@if [ -n "$(NAME)" ]; then \
		./scripts/build_wasm_ei_app.sh "$(APP)" "$(NAME)"; \
	else \
		./scripts/build_wasm_ei_app.sh "$(APP)"; \
	fi
