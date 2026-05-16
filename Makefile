.PHONY: test example example-world-json

test:
	cargo test

EXAMPLE ?= hello

example:
	cargo run -p open_entities --example $(EXAMPLE)

example-world-json:
	$(MAKE) example EXAMPLE=world_json
