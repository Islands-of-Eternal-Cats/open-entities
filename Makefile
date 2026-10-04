.PHONY: test float-check bench example example-world-json wasm-demo wasm-test wasm-check wasm-bench

test:
	cargo test

# Simulation modules must not mention f32/f64; CI runs this too.
float-check:
	./scripts/check-no-floats.sh

# Criterion benches of `Api::step` at 100k units; timings are local only.
bench:
	cargo bench -p open_entities --bench step

EXAMPLE ?= spawn_entity

example:
	cargo run -p open_entities --example $(EXAMPLE)

# Prerequisites: rustup target add wasm32-unknown-unknown; cargo install wasm-pack
wasm-demo:
	@command -v wasm-pack >/dev/null 2>&1 || { echo "wasm-pack not found. Install with: cargo install wasm-pack"; exit 1; }
	wasm-pack build wasm-bindings --target nodejs
	cd wasm-bindings && node demo/run.mjs

wasm-test:
	@command -v wasm-pack >/dev/null 2>&1 || { echo "wasm-pack not found. Install with: cargo install wasm-pack"; exit 1; }
	wasm-pack test --node wasm-bindings

wasm-check: wasm-demo wasm-test

# `step()` and `writeFrame()` at 100k movers under wasm32 in Node; the wasm half of the budget.
wasm-bench:
	@command -v wasm-pack >/dev/null 2>&1 || { echo "wasm-pack not found. Install with: cargo install wasm-pack"; exit 1; }
	wasm-pack build wasm-bindings --target nodejs
	node wasm-bindings/demo/bench.mjs
