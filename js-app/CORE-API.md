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

### Типы (экспортируемые классы)

- **`JsPosition`**  
  - `new JsPosition(x: number, y: number)`  
  - `x(): number`, `set_x(x: number): void`  
  - `y(): number`, `set_y(y: number): void`

- **`JsVelocity`**  
  - `new JsVelocity(vx: number, vy: number)`  
  - `vx(): number`, `set_vx(vx: number): void`  
  - `vy(): number`, `set_vy(vy: number): void`

### Функции

- **`move_position(pos: JsPosition, vel: JsVelocity): JsPosition`**  
  Возвращает новую позицию после одного тика: `(x + vx, y + vy)`. Входы не мутируются.

## Где что лежит в js-app

| Путь | Назначение |
|------|------------|
| `src/core/wasm-types.d.ts` | Объявления типов для модуля `open-entities-wasm`. |
| `src/core/wasm.ts` | Обёртка: `initWasm()`, `isWasmReady()`, реэкспорт API. |
| `src/core/types.ts` | Типы приложения (например, `GameEntity`). |
| `src/visualization/render.ts` | Отрисовка состояния в DOM (или в будущем Canvas/WebGL). |
| `src/main.ts` | Точка входа: инит, цикл, кнопки. |

## Рекомендуемое развитие API для большой игры

1. **Инициализация**: `init()`, опционально `createWorld(options?)`.
2. **Тик**: `tick(deltaMs?: number)` — вызов из TS каждый кадр/интервал; вся симуляция в WASM.
3. **Чтение состояния**: например `getEntities()` / `getEntitiesWithPosition()` — возврат типизированных структур для отрисовки.
4. **Ввод**: `applyInput(playerId, input)` или `queueCommand(...)` — TS только передаёт события, логика в WASM.

Типы для новых функций и структур описывать в `core/` (или использовать сгенерированные wasm-pack `.d.ts`).
