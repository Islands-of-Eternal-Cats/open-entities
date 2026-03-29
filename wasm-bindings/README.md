# `wasm-bindings` (WASM ↔ JS/TS API)

Этот crate экспортирует минимальный API через `wasm-bindgen`, чтобы использовать ECS-ядро из JavaScript/TypeScript.

## Быстрый старт (ESM)

Сборка делается через `wasm-pack` (см. `js-app/build-wasm.sh` / `js-app` scripts).

В JS/TS проекте обычно используется пакет, который создаёт `wasm-pack` в `wasm-bindings/pkg/`:

```ts
import init, { JsWorld } from "open-entities-wasm";

// 1) Инициализировать WASM модуль (обязательно до любых вызовов)
await init();

// 2) Создать мир из YAML-строки с определениями типов сущностей
const entitiesYaml = `
entities:
  mover:
    position: { x: 0.0, y: 0.0 }
    base_move_speed: 45.0
  static_obstacle:
    position: { x: 10.0, y: 10.0 }
`;
const world = new JsWorld(entitiesYaml);

// 3) Спавн по имени типа из YAML
world.spawn("mover");

// 3b) Спавн по имени типа, но в заданных координатах (без Velocity)
world.spawn_at("mover", 100.0, 200.0);

// 4) Тик симуляции (dt в секундах)
world.tick(1 / 60);

// 5) Снапшот для рендера
const entities = Array.from(world.get_entities());
```

Если вы используете web worker (рекомендуется для UI), посмотрите готовую интеграцию в `js-app`:

- `js-app/src/core/ecs-worker.ts` — загрузка WASM внутри воркера и вызовы `JsWorld`
- `js-app/src/core/wasm.ts` — API главного потока: `initWasm()`, `tick(dt)`, `spawn(typeName)`
- `js-app/CORE-API.md` — контракт WASM ↔ TS

## Публичный API

Источник: `wasm-bindings/src/lib.rs` (экспортируется через `#[wasm_bindgen]`).

### `init()` (default export, wasm-pack)

`wasm-pack` генерирует default export `init(...)`, который нужно вызвать **перед** использованием классов/функций.

- В `js-app` используется вариант, когда главный поток сам `fetch`-ит `.wasm` и передаёт `ArrayBuffer` воркеру, а воркер вызывает `initWasmModule(wasmBuffer)`.
- Для простых случаев достаточно `await init()` без аргументов (зависит от bundler окружения).

Также экспортируется `wasm_init()` как `#[wasm_bindgen(start)]` (ставит panic hook); обычно вручную его вызывать не нужно — он запускается при инициализации модуля.

### `class JsWorld`

Мир ECS, который хранит состояние и выполняет тики симуляции.

- `new JsWorld(entitiesYaml: string)`
  - Парсит YAML и создаёт `World + Schedule`.
  - При ошибке парсинга/формата бросает исключение (как `JsValue`).
- `spawn(typeName: string): void`
  - Создаёт одну сущность по имени типа из загруженных `entities` YAML.
  - Если `typeName` неизвестен — бросает исключение.
- `spawn_at(typeName: string, x: number, y: number): void`
  - Создаёт сущность по имени типа, но с `Position = {x, y}` из аргументов.
  - Компонент `Velocity` **не создаётся** (появится при приказе движения); для подвижных типов (`base_move_speed` > 0) `BaseMoveSpeed` из YAML всё равно задаётся.
- `tick(dt: number): void`
  - Запускает один тик симуляции с delta time в секундах.
- `get_entities(): Array<{ id, pos, velocity }>`
  - Возвращает снапшот для рендера.

#### Формат снапшота `get_entities()`

Каждый элемент массива:

```ts
{
  id: string,
  pos: { x: number, y: number },
  velocity: { vx: number, vy: number } | null
}
```

Примечания:

- `id` — **строка** (это `Entity::to_bits()` в виде десятичной строки), чтобы не терять точность u64 в JS (в `Number` безопасны целые только до \(2^{53}-1\)).
- `velocity: null` у статичных сущностей (которые имеют `Position`, но не имеют `Velocity`).

### Legacy API (опционально)

- `class JsPosition`, `class JsVelocity` — простые обёртки для компонентов.
- `move_position(pos, vel) -> JsPosition` — “один тик без dt” (устаревший хелпер; предпочтительнее `JsWorld.tick(dt)`).

## Memory / lifetime (важно для долгих сессий)

`wasm-bindgen`-классы (`JsWorld`, `JsPosition`, `JsVelocity`) владеют WASM-ресурсами. В сгенерированном `.d.ts` также будут методы:

- `.free(): void`
- `[Symbol.dispose](): void`

Если вы создаёте много временных объектов, освобождайте их явно (или используйте `using` в современных окружениях).

## YAML формат

Минимальный формат (root key **`entities`**):

```yaml
entities:
  mover:
    position: { x: 0.0, y: 0.0 }
    base_move_speed: 45.0
  static_obstacle:
    position: { x: 10.0, y: 10.0 }
```

`base_move_speed` > 0 помечает подвижный тип; без поля или 0 — статика (только позиция).

