# Roadmap Step 4 Handoff

## Что сделано

- Усилена проверка input-flow на уровне `pixi-canvas` (интеграционные тесты с pointer-событиями), не только pure helper.
- Закрыты сценарии:
  - plain click по пустой земле с активным selection -> move order отправляется;
  - modified click (`Shift`) по пустоте -> move order не отправляется;
  - minimap click с модификатором (`Ctrl`) -> move order не отправляется;
  - right-click -> selection очищается.
- Обновлена документация семантики ввода в `js-app/CORE-API.md`.

## Изменённые файлы

- `js-app/src/visualization/pixi-canvas.input-flow.test.ts` (новый)
- `js-app/CORE-API.md`

## Проверки Step 4

- `cargo test -p open-entities-lib`
- `npm --prefix "/Users/random/restore/open-entities/js-app" run typecheck`
- `npm --prefix "/Users/random/restore/open-entities/js-app" run build`
- `npm --prefix "/Users/random/restore/open-entities/js-app" run test:run`

## Риски / хвосты

- Сценарии `Esc` и UI clear покрыты в runtime-коде (`main.ts`) и требуют ручной browser-проверки в полноценной среде DOM + canvas.
