# Design: YAML Component Spawn Import

**Date:** 2026-05-16  
**Status:** Approved (brainstorming)  
**Depends on:** [RTS Components and World JSON Export v2](2026-05-16-rts-components-export-v2-design.md)  
**Scope:** Add `import` module: preload a YAML templates file, then `Api::spawn_yaml(name)` to instantiate one entity by template name. Strict validation, symmetric field shapes with export v2. No template inheritance, world import, or JSON import.

## Summary

Introduce `open-entities-lib/src/import/` mirroring `export/`. Workflow:

1. **`load_templates_yaml(yaml)`** — parse and store named entity templates (`entities: { scout: ..., base: ... }`) on `Api`. Does not spawn.
2. **`spawn_yaml(template_name)`** — look up a loaded template by name, spawn one `Entity` in the world.

Unknown component keys in the templates file fail load. Unknown template name fails spawn. Template field inheritance via `template:` is **deferred** — see [idea doc](../ideas/2026-05-16-yaml-template-inheritance.md).

No inline YAML spawn (raw component map without `entities` wrapper). No batch spawn API; call `spawn_yaml` per instance.

## Goals

- Define typical RTS entities once in a YAML file, spawn many instances by name.
- Reuse the same component field shapes as JSON export v2.
- Keep `Core` / `Api` facade; import in a dedicated module.
- Testable contract via unit tests in `import/mod.rs`.

## Non-Goals (This Increment)

- **`template` inheritance** between named entries ([ideas](../ideas/2026-05-16-yaml-template-inheritance.md)).
- Inline / ad-hoc `spawn_yaml` from a raw component map string.
- `spawn_all_templates` or batch spawn in one call.
- World-level YAML (`version`, export-shaped rows).
- Preserving or assigning `Entity` ids from YAML.
- JSON import.
- Ignoring unknown component keys (strict mode only).
- Reading files from disk inside the library (`include_str!` / `std::fs` in caller or examples).
- `wasm-bindings`, examples, Makefile, README (optional follow-up).
- Simulation systems, `Api::tick`, new component types.

## Decisions (Brainstorming)

| Topic | Choice | Rationale |
|-------|--------|-----------|
| Templates storage | `Api.templates: Option<BTreeMap<String, EntitySpawnYaml>>` | Parse once; no separate file/registry types |
| Spawn API | `spawn_yaml(&str) -> Entity` by template name only | User direction; no redundant methods |
| Templates file | Root key `entities` → map name → component map | Named templates |
| Template inheritance | Deferred (`template` key) | [ideas](../ideas/2026-05-16-yaml-template-inheritance.md) |
| Component keys | `position`, `velocity`, `faction`, `move_target` | Match export v2 |
| Unknown component keys in file | Error on load | Strict |
| Unknown template name on spawn | Error; no spawn | Strict |
| Spawn without prior load | Error (`TemplatesNotLoaded`) | Explicit lifecycle |
| Empty template `{}` | Valid; spawns entity with no components | Consistent |
| Empty `entities: {}` | Valid load → empty map; spawn fails with `UnknownTemplate` | Map may be empty |
| Reload | `load_templates_yaml` replaces entire map | Simple semantics |
| Parsing | `serde_yaml` in `load_templates_yaml` only | YAML root wrapper is parse detail, not stored |
| Spawn mechanics | `spawn_empty()` + `insert` per component | Bevy 0.18 |
| Component serde | `Deserialize` on all four components | Symmetric with export |

## Repository Layout

```text
open-entities-lib/src/
├── api.rs                  # Api { core, templates: Option<EntityTemplates> }
├── import/
│   └── mod.rs              # ImportError, EntitySpawnYaml, load/spawn, tests
├── components/             # + Deserialize on each
└── lib.rs                  # mod import; pub use import::ImportError;
```

`EntityTemplates` is a private type alias in `import` (see Storage).

## YAML Contract (templates file)

Root has exactly one key, `entities`:

```yaml
entities:
  scout:
    position: { x: 0, y: 0 }
    velocity: { vx: 2, y: 0 }
    faction: 1

  base:
    faction: 2

  marker: {}
```

| Field | Type | Notes |
|-------|------|-------|
| `entities` | map string → component map | Template name → components |
| Per-template keys | component names only | `template` not supported in this increment |

Component map (value under each template name):

| Key | Value shape | Notes |
|-----|-------------|-------|
| `position` | `{ x: f32, y: f32 }` | Optional |
| `velocity` | `{ vx: f32, vy: f32 }` | Optional |
| `faction` | number (`u32`) | Transparent `Faction` |
| `move_target` | `{ x: f32, y: f32 }` | Optional |

Invalid (load returns `Err`, previous `templates` map unchanged if replacing — see reload):

```yaml
health: 1
```

```yaml
entities:
  bad:
    position: "not-an-object"
```

## Storage and types

Only one domain type is persisted after load:

```rust
// import/mod.rs (private)
type EntityTemplates = BTreeMap<String, EntitySpawnYaml>;

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct EntitySpawnYaml {
    position: Option<Position>,
    velocity: Option<Velocity>,
    faction: Option<Faction>,
    move_target: Option<MoveTarget>,
}
```

`Api`:

```rust
pub struct Api {
    core: Core,
    templates: Option<EntityTemplates>,  // None until first successful load
}
```

`EntitySpawnYaml` is cloned on each spawn (definitions are immutable; each spawn is a new entity).

The YAML root key `entities` exists only in the file format. `load_templates_yaml` deserializes that map and assigns it to `self.templates` — no separate file-shaped type in memory. A private one-off deserialize helper inside `load` (not part of this contract) may wrap the `entities:` key for `serde`.

## Algorithms

### `load_templates_yaml`

1. Parse `yaml` → `BTreeMap<String, EntitySpawnYaml>` (via `entities` root in YAML).
2. `self.templates = Some(map)`.
3. `Ok(())`.

On parse error: return `ImportError::Yaml`; if replacing an existing map, **keep the previous `templates`** (load is atomic).

### `spawn_yaml(template_name)`

1. `let templates = self.templates.as_ref().ok_or(ImportError::TemplatesNotLoaded)?`
2. `let doc = templates.get(template_name).ok_or(ImportError::UnknownTemplate(...))?`
3. `Ok(spawn_from_doc(self.core_mut().world_mut(), doc.clone()))`

`spawn_from_doc`:

1. `let mut entity = world.spawn_empty();`
2. For each `Some` field in `doc`, `entity.insert(component)`
3. `entity.id()`

Each successful spawn creates a **new** entity; templates are not consumed.

## Public API

```rust
impl Api {
    /// Parses a YAML templates file and stores it for later spawns.
    ///
    /// Root must be `entities: { <name>: <components>, ... }`.
    /// Replaces any previously loaded templates.
    pub fn load_templates_yaml(&mut self, yaml: &str) -> Result<(), ImportError>;

    /// Spawns one entity from a previously loaded template by name.
    pub fn spawn_yaml(&mut self, template_name: &str) -> Result<Entity, ImportError>;
}
```

`Api::new()` sets `templates: None`.

`lib.rs`: `pub mod import;` and `pub use import::ImportError;`.

## Dependencies

```toml
# workspace + open-entities-lib
serde_yaml = "0.9"
```

Components: add `Deserialize` alongside `Serialize`.

## Error Handling

```rust
pub enum ImportError {
    Yaml(serde_yaml::Error),
    TemplatesNotLoaded,
    UnknownTemplate(String),
}
```

| Case | Result |
|------|--------|
| YAML syntax / type / unknown field on load | `ImportError::Yaml` |
| `spawn_yaml` before load | `TemplatesNotLoaded` |
| Unknown template name | `UnknownTemplate` |
| Valid load + valid spawn | `Ok(())` / `Ok(Entity)` |

## Testing

| Test | Assertion |
|------|-----------|
| `spawn_yaml_without_load` | `TemplatesNotLoaded` |
| `load_templates_yaml_invalid` | `Err`; spawn still `TemplatesNotLoaded` if first load failed |
| `load_templates_yaml_unknown_root` | `foo: 1` → `Err` |
| `load_templates_yaml_invalid_nested` | bad component → `Err`; `templates` stays `None` on first load |
| `spawn_yaml_unknown_template` | after valid load, `spawn_yaml("nope")` → `UnknownTemplate` |
| `spawn_yaml_scout` | scout has position + velocity + faction |
| `spawn_yaml_base` | base has faction only |
| `spawn_yaml_marker` | empty template → entity without components |
| `spawn_yaml_twice_same_name` | two calls → two entities (same components) |
| `load_templates_yaml_replaces` | load A, load B, spawn only B's names work |

```bash
cargo test -p open_entities
cargo clippy -p open_entities -- -D warnings
```

## Symmetry with Export v2

| Export (entity row) | Template entry under `entities.<name>` |
|---------------------|----------------------------------------|
| `position` | `position` |
| `velocity` | `velocity` |
| `faction` | `faction` |
| `move_target` | `move_target` |
| `id` | not used |

## Alternatives Considered

| Approach | Why not |
|----------|---------|
| `EntityTemplatesFile` + `EntityTemplateRegistry` types | Same as `BTreeMap`; file wrapper is parse-only |
| `spawn_yaml(yaml)` inline + batch spawn API | Redundant; load + spawn by name |
| `spawn_yaml(yaml, name)` every call | Re-parses file; no in-memory map |
| Keep inline spawn | Out of scope; templates file only |

## Out of Scope

- `template` inheritance ([ideas](../ideas/2026-05-16-yaml-template-inheritance.md)).
- Batch spawn all templates.
- World `version` document.

## Future Extensions

- [Template inheritance](../ideas/2026-05-16-yaml-template-inheritance.md) (`template:` key, resolve before store or at spawn).
- `fn template_names(&self) -> impl Iterator<Item = &str>`.
- `assets/entities.yaml` example.
- `wasm-bindings`.
- Optional `spawn_yaml` count / override hooks for placement.

## License Note

`serde_yaml` is MIT OR Apache-2.0; compatible with GPL-3.0-or-later.
