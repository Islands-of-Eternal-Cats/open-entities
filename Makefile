# Makefile for open-entities Rust Workspace

.PHONY: all build test clippy fmt run-wasm-todo clean check docs

# По умолчанию - сборка всего
all: build

# Сборка проекта в режиме отладки
build:
	cargo build

# Сборка проекта в релизном режиме
release:
	cargo build --release

# Запуск тестов
test:
	cargo test

# Статический анализ с Clippy
clippy:
	cargo clippy --all-targets --all-features -- -D warnings

# Форматирование кода
fmt:
	cargo fmt --all

# Проверка без сборки (check)
check:
	cargo check --all-targets --all-features

# Документация
docs:
	cargo doc --no-deps --open

# Сборка WASM в релизном режиме
wasm:
	cargo build --target wasm32-unknown-unknown --release -p wasm-bindings

# Чистка проекта (target/ и wasm-bindings/pkg/)
clean:
	cargo clean
	rm -rf target
	rm -rf wasm-bindings/pkg

# Запуск всех проверок (для CI)
ci: check clippy fmt test
