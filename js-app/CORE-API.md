# Core API (WASM ↔ TypeScript)

Контракт между Rust/WASM-ядром и слоем визуализации (JS/TS).

**Первая установка:** если папки `wasm-bindings/pkg/` ещё нет, сначала соберите WASM: `npm run build:wasm`, затем `npm install`.

## Общая схема

- **Ядро (WASM)** выполняется в **web worker**: симуляция, ECS-логика, состояние мира. Не знает о DOM/Canvas.
- **Визуализация (TS)** в главном потоке: рендер, ввод, UI. Общается с ядром через `postMessage` (см. `core/worker-types.ts`).

Граница — только данные и вызовы функций; никаких общих мутабельных структур.

## Текущий API (wasm-bindings)

### Инициализация

- **`initWasm()`** (из `core/wasm.ts`)  
  Создаёт web worker, загружает в нём WASM и создаёт `JsWorld`. Возвращает `Promise<void>`. Вызывать один раз до `tick`/`spawn`.
- **`isWasmReady()`** — `true` после успешного `initWasm()`.

### Мир и тик с delta time (через worker)

- **`tick(dt): Promise<EntitySnapshot[]>`** — отправить в worker один тик симуляции; `dt` в секундах. Резолвится снапшотом сущностей для отрисовки.
- **`spawn(x, y, vx, vy): Promise<EntitySnapshot[]>`** — создать сущность в worker; резолвится актуальным снапшотом сущностей.

В worker живёт один экземпляр **`JsWorld`** (Rust/WASM): `world.tick(dt)`, `world.spawn(...)`, `world.get_entities()`. Delta time в Rust: ресурс `DeltaTime(dt)`, `move_system` делает `position += velocity * dt`.

### 2. Стабильные ID сущностей

- **Стабильный id:** в Rust для каждой сущности в снапшот передаётся `Entity::to_bits()` как **десятичная строка** (`wasm-bindings`: поле `id` в объекте из `get_entities()`). Пока сущность жива, биты не меняются между кадрами; визуализация может ключевать спрайты/DOM по `EntitySnapshot.id`. Строка нужна, чтобы не терять точность u64 в JS `Number` (безопасны только целые до 2⁵³−1).
- Worker сериализует `Array.from(world.get_entities())` в сообщения как `EntitySnapshot[]` (см. `core/types.ts`).

### Legacy (по желанию)

- **`JsPosition`** / **`JsVelocity`** — обёртки для компонентов.
- **`move_position(pos, vel): JsPosition`** — один тик без dt (для совместимости).

## Где что лежит в js-app

| Путь | Назначение |
|------|------------|
| `src/core/wasm-types.d.ts` | Объявления типов для модуля `open-entities-wasm`. |
| `src/core/worker-types.ts` | Типы сообщений main ↔ worker (`WorkerInMessage`, `WorkerOutMessage`). |
| `src/core/ecs-worker.ts` | Web worker: загрузка WASM, `JsWorld`, обработка `init`/`tick`/`spawn`. |
| `src/core/wasm.ts` | Обёртка главного потока: `initWasm()`, `isWasmReady()`, `tick(dt)`, `spawn(...)` (все вызовы уходят в worker). |
| `src/core/types.ts` | Типы приложения (например, `EntitySnapshot`). |
| `src/visualization/render.ts` | Отрисовка состояния в DOM (или в будущем Canvas/WebGL). |
| `src/main.ts` | Точка входа: инит, цикл, кнопки. |

## Дальнейшее развитие API

1. **Инициализация**: `init()`, `new JsWorld()` — уже есть.
2. **Тик**: `world.tick(dt)` — реализовано.
3. **Чтение состояния**: `world.get_entities()` — реализовано.
4. **Ввод**: `applyInput(playerId, input)` или `queueCommand(...)` — TS только передаёт события, логика в WASM.

Типы для новых функций и структур описывать в `core/` (или использовать сгенерированные wasm-pack `.d.ts`).
