.PHONY: test example example-world-json

test:
	cargo test

EXAMPLE ?= spawn_entity

example:
	cargo run -p open_entities --example $(EXAMPLE)

