# Core API (WASM ↔ TypeScript)

Контракт между Rust/WASM-ядром и слоем визуализации (JS/TS).

**Первая установка:** если папки `wasm-bindings/pkg/` ещё нет, сначала соберите WASM: `npm run build:wasm`, затем `npm install`.

## Общая схема

- **Ядро (WASM)**: симуляция, ECS-логика, состояние мира. Не знает о DOM/Canvas.
- **Визуализация (TS)**: рендер, ввод, UI. Вызывает ядро через этот API.

Граница — только данные и вызовы функций; никаких общих мутабельных структур.

## Текущий API (wasm-bindings)

### Инициализация

- **`init()`** (default export)  
  Инициализирует WASM и panic hook. Должна быть вызвана один раз до любого другого вызова.

### Мир и тик с delta time

- **`JsWorld`**  
  - `new JsWorld()` — пустой мир (только move_system в расписании).
  - `world.spawn(x, y, vx, vy)` — создать сущность с Position и Velocity.
  - `world.tick(dt)` — один тик симуляции; `dt` в секундах (например из `requestAnimationFrame`).
  - `world.get_entities()` — снапшот всех сущностей: массив `{ x, y, vx, vy }` для отрисовки.

Delta time связан с движком: в Rust в мир перед тиком кладётся ресурс `DeltaTime(dt)`, `move_system` делает `position += velocity * dt`.

### Legacy (по желанию)

- **`JsPosition`** / **`JsVelocity`** — обёртки для компонентов.
- **`move_position(pos, vel): JsPosition`** — один тик без dt (для совместимости).

## Где что лежит в js-app

| Путь | Назначение |
|------|------------|
| `src/core/wasm-types.d.ts` | Объявления типов для модуля `open-entities-wasm`. |
| `src/core/wasm.ts` | Обёртка: `initWasm()`, `isWasmReady()`, реэкспорт API. |
| `src/core/types.ts` | Типы приложения (например, `EntitySnapshot`). |
| `src/visualization/render.ts` | Отрисовка состояния в DOM (или в будущем Canvas/WebGL). |
| `src/main.ts` | Точка входа: инит, цикл, кнопки. |

## Дальнейшее развитие API

1. **Инициализация**: `init()`, `new JsWorld()` — уже есть.
2. **Тик**: `world.tick(dt)` — реализовано.
3. **Чтение состояния**: `world.get_entities()` — реализовано.
4. **Ввод**: `applyInput(playerId, input)` или `queueCommand(...)` — TS только передаёт события, логика в WASM.

Типы для новых функций и структур описывать в `core/` (или использовать сгенерированные wasm-pack `.d.ts`).
