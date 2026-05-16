.PHONY: test example

test:
	cargo test

example:
	cargo run -p open_entities --example hello
