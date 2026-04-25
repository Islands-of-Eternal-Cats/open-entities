# Makefile for open-entities Rust Workspace

.PHONY: all build test full-check clippy fmt run-wasm-todo clean check docs js-app help

# По умолчанию - список доступных команд
all: help

# Показать краткий список команд
help:
	@echo "Доступные команды:"
	@echo "  build      - Сборка проекта в режиме отладки"
	@echo "  release    - Сборка проекта в релизном режиме"
	@echo "  test       - Запуск тестов"
	@echo "  full-check - Rust-тесты + TypeScript проверки"
	@echo "  clippy     - Статический анализ с Clippy"
	@echo "  fmt        - Форматирование кода"
	@echo "  check      - Проверка без сборки"
	@echo "  docs       - Генерация документации"
	@echo "  wasm       - Сборка WASM"
	@echo "  js-app     - Запуск dev-сервера js-app"
	@echo "  clean      - Очистка проекта"
	@echo "  ci         - Запуск всех проверок (CI)"
	@echo "  help       - Показать этот список"

# Сборка проекта в режиме отладки
build:
	cargo build

# Сборка проекта в релизном режиме
release:
	cargo build --release

# Запуск тестов
test:
	cargo test

# Rust-тесты + проверка TypeScript (tsc) + vitest (нужны зависимости: cd js-app && npm ci)
full-check: test
	cd js-app && npm run typecheck
	cd js-app && npm run test:run

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

# Запуск dev-сервера js-app (Vite). Сначала пересобирает WASM, затем стартует сервер.
js-app:
	$(MAKE) wasm
	cd js-app && npm run dev

# Чистка проекта (target/ и wasm-bindings/pkg/)
clean:
	cargo clean
	rm -rf target
	rm -rf wasm-bindings/pkg

# Запуск всех проверок (для CI)
ci: check clippy fmt test
