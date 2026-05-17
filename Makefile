.PHONY: test example example-world-json wasm-demo

test:
	cargo test

EXAMPLE ?= spawn_entity

example:
	cargo run -p open_entities --example $(EXAMPLE)

# Prerequisites: rustup target add wasm32-unknown-unknown; cargo install wasm-pack
wasm-demo:
	@command -v wasm-pack >/dev/null 2>&1 || { echo "wasm-pack not found. Install with: cargo install wasm-pack"; exit 1; }
	wasm-pack build wasm-bindings --target nodejs
	cd wasm-bindings && node demo/run.mjs
