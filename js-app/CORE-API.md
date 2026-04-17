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
  Создаёт web worker, **на главном потоке** `fetch`-ит `wasm_bindings_bg.wasm` (cache-bust) и `assets/entities.yaml`, передаёт в worker `ArrayBuffer` WASM и текст YAML, воркер вызывает `init(wasmBuffer)` и `new JsWorld(entitiesYaml)`. Возвращает `Promise<void>`. Вызывать один раз до `tick` / `spawnRandomAt` / `moveSelectedTo`.
- **`isWasmReady()`** — `true` после успешного `initWasm()`.

### Мир и тик с delta time (через worker)

- **`tick(dt): Promise<EntitySnapshot[]>`** — отправить в worker один тик симуляции; `dt` в секундах. Резолвится снапшотом сущностей для отрисовки.
- **`spawnRandomAt(typeName, faction?): Promise<EntitySnapshot[]>`** — спавн по имени типа из YAML в **случайных** координатах (диапазон задаётся в `wasm.ts`, ориентир — размер мира визуализации). Опциональный `faction` передаётся в worker как `spawn_at` и в Rust — `JsWorld::spawn_at(..., faction)` (компонент `Faction`). Резолвится актуальным снапшотом.
- **`moveSelectedTo(entityIds, point): Promise<EntitySnapshot[]>`** — приказ «идти в точку мира» для списка id из снапшота; `point` — `{ x, y }` в мировых координатах (тот же смысл, что у `Position` / `JsPosition`). В worker вызывается `JsWorld::order_move_to(entityIds, new JsPosition(...))` без `tick`. Пустой `entityIds` — ошибка. Сообщение worker: `{ type: "move_to", entityIds, point }`.

В worker живёт один экземпляр **`JsWorld`** (Rust/WASM): `world.tick(dt)`, `world.spawn_at(...)`, `world.order_move_to(...)`, `world.get_entities()`. Delta time в Rust: ресурс `DeltaTime(dt)`, `move_system` делает `position += velocity * dt`.

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
| `src/core/ecs-worker.ts` | Web worker: `init(wasmBuffer)`, `new JsWorld(yaml)`, обработка `init` / `tick` / `spawn_at` / `move_to`. |
| `src/core/wasm.ts` | Обёртка главного потока: `initWasm()`, `isWasmReady()`, `tick(dt)`, `spawnRandomAt(...)`, `moveSelectedTo(...)` — всё уходит в worker. |
| `src/core/types.ts` | Типы приложения (`EntitySnapshot` и др.). |
| `src/visualization/render.ts` | Список сущностей в DOM (дополнение к канвасу). |
| `src/visualization/pixi-canvas.ts` | PixiJS: мир, выбор юнитов, клик по пустой земле → `moveSelectedTo`. |
| `src/visualization/coords.ts` | Преобразование экран ↔ мир. |
| `src/visualization/selection-logic.ts` | Логика выбора (без привязки к рендеру). |
| `src/main.ts` | Точка входа: `initWasm`, игровой цикл, UI, связка с Pixi и `render.ts`. |

## Дальнейшее развитие API

1. **Инициализация**: `init()`, `new JsWorld(yaml)` — уже есть (в js-app YAML приходит с главного потока в `init`).
2. **Тик**: `world.tick(dt)` — реализовано.
3. **Чтение состояния**: `world.get_entities()` — реализовано (`id`, `entityType`, `pos`, `velocity`, `faction`).
4. **Ввод**: `applyInput(playerId, input)` или `queueCommand(...)` — TS только передаёт события, логика в WASM.

Типы для новых функций и структур описывать в `core/` (или использовать сгенерированные wasm-pack `.d.ts`).
