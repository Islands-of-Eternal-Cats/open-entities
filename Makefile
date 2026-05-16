.PHONY: test example example-world-json

test:
	cargo test

EXAMPLE ?= world_json

example:
	cargo run -p open_entities --example $(EXAMPLE)

