# Сравнение размера: ecs-only vs dev (с bevy_app)

Сборка: `CARGO_TARGET_DIR=target cargo build --release -p open-entities-lib`  
Базовая ветка (с bevy_app): dev, отдельная сборка в `target_dev/`.

## Библиотека (rlib)

| Вариант    | Размер (bytes) | Размер (KB) |
|-----------|----------------|-------------|
| dev (bevy_app) | 529 216 | ~517 KB |
| ecs-only       | 549 184 | ~536 KB |

**Итог:** без bevy_app rlib получился **на ~20 KB больше**. Причина: вместо тонкого слоя App используется явная работа с `World` и `Schedule` (больше нашего кода в systems.rs).

## Нативный бинарник (example run_ecs)

Доступен только на ветке ecs-only (пример использует `setup_world_with_yaml` с `assets/entities.yaml`).

- **run_ecs:** 936 816 bytes (~914 KB), release.

## Зависимости

- Удаление bevy_app убирает из дерева зависимостей крейты: bevy_app, bevy_derive, toml_edit, toml_datetime, winnow (~5 крейтов, см. обсуждение в чате).
- WASM-сборка wasm-bindings на dev не проходила из‑за getrandom/wasm без дополнительных флагов, поэтому сравнение размера .wasm не выполнялось.
