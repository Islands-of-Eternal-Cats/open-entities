# OpenEntities

Rust workspace with the core library crate `open_entities` in `open-entities-lib/`.

## Requirements

- Rust **1.85+** (edition 2024)

Check your toolchain:

```bash
rustc --version
cargo --version
```

## Build

From the repository root:

```bash
cargo build --workspace
```

Build only the library crate:

```bash
cargo build -p open_entities
```

## Test

```bash
make test
```

Or directly:

```bash
cargo test
```

## Run example

The hello-world demo prints a greeting to stdout:

```bash
make example
```

Or directly:

```bash
cargo run -p open_entities --example hello
```

Expected output:

```
Hello, world!
```
