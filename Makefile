# Makefile for open-entities Rust Workspace

.PHONY: all build test clippy fmt run-wasm-todo clean check docs js-app

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

# Сборка WASM (wasm-pack) и копирование в js-app/public для dev-сервера.
# Скрипт использует абсолютные пути, чтобы public всегда обновлялся.
wasm:
	cd js-app && ./build-wasm.sh

# Запуск dev-сервера js-app (Vite; WASM собирается автоматически)
js-app:
	cd js-app && npm run dev

# Чистка проекта (target/ и wasm-bindings/pkg/)
clean:
	cargo clean
	rm -rf target
	rm -rf wasm-bindings/pkg

# Запуск всех проверок (для CI)
ci: check clippy fmt test
