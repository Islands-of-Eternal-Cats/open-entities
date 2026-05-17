# Design: YAML Template Inheritance (`template`)

**Date:** 2026-05-17  
**Status:** Approved (brainstorming)  
**Depends on:** [YAML Component Spawn Import](2026-05-16-yaml-spawn-import-design.md)  
**Idea:** [YAML template inheritance](../ideas/2026-05-16-yaml-template-inheritance.md)  
**Scope:** Extend `load_templates_yaml` so entries under `entities` may declare `template: <name>` to inherit component fields from another entry in the same file. Resolve and flatten at load time; `spawn_yaml` unchanged. Component-level merge only. No cross-file refs, multiple inheritance, or ECS hierarchy.

## Summary

RTS unit YAML often shares a base (`unit` → `scout`, `tank`). Authors set a reserved key `template` on a child entry to inherit from another named entry in the same `entities` map. Inheritance is **not** Bevy `Parent` / scene graph.

After a successful load, `Api.templates` holds **flattened** component maps with no `template` key. Resolution errors (cycle, missing parent) fail load atomically. Base templates remain spawnable.

## Goals

- Reduce duplication of shared fields (`faction`, default stats) within one templates file.
- Keep YAML shape flat at authoring time (`template` beside component keys via `serde(flatten)`).
- Fail fast at load with typed `ImportError` variants.
- Avoid duplicating component field lists across raw and stored types (`EntityComponents` shared block).

## Non-Goals

- Cross-file `template` references.
- Multiple inheritance (`template: [a, b]`).
- Deep merge of fields inside a component (e.g. child `position.x` + parent `position.y`).
- ECS `Parent` or scene graph from YAML.
- `entity_type` in YAML (still injected at spawn from template name).
- Component import registry / inventory / equipment (future; see [Evolution](#evolution)).

## Decisions (Brainstorming)

| Topic | Choice | Rationale |
|-------|--------|-----------|
| Resolution timing | At `load_templates_yaml` (flatten before store) | Errors before spawn; simple `spawn_yaml` |
| Stored shape | `BTreeMap<String, EntityComponents>` (alias `EntitySpawnYaml`) | No `template` in memory |
| Parse shape | `EntityTemplateRaw { template, #[serde(flatten)] components }` | Single component field list |
| Merge | Component-level: `child.field.or(parent.field)` per component | Predictable; matches idea doc |
| Chains | Allowed (`c → b → a`) | DFS/memo resolve |
| Cycles | `ImportError::TemplateCycle { chain }` | Testable |
| Missing parent | `ImportError::UnknownTemplateParent { child, parent }` | Distinct from spawn `UnknownTemplate` |
| Base templates | Spawnable (`spawn_yaml("unit")` works) | No `abstract` flag |
| Load failure | Previous `templates` unchanged | Same as spawn-import spec |
| Macros / registry | Neither for this increment | `EntityComponents` + manual merge/spawn until scale demands registry |

## YAML Contract

```yaml
entities:
  unit:
    faction: 1

  scout:
    template: unit
    position: { x: 0, y: 0 }
    velocity: { vx: 2, vy: 0 }

  tank:
    template: unit
    faction: 2
    velocity: { vx: 0.5, vy: 0 }
```

| Key | Location | Notes |
|-----|----------|-------|
| `template` | Inside `entities.<name>` only | Value: name of another entry in the same `entities` map |
| Component keys | Same entry (flattened with `template` in file) | `position`, `velocity`, `faction`, `move_target` — same shapes as spawn-import spec |

**Merge example:** `unit { faction: 1 }` + `scout { template: unit, velocity: … }` → flattened scout `{ faction: 1, velocity: … }` (no `position` unless scout sets it).

**Override:** Child `position: { x: 1, y: 1 }` fully replaces parent `position`; no merge of `x`/`y`.

Order of keys/entries in the file is irrelevant; resolution is by name after parse.

## Types (`import/mod.rs`)

```rust
/// Shared component bundle — single place to add new importable components.
#[derive(Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
struct EntityComponents {
    position: Option<Position>,
    velocity: Option<Velocity>,
    faction: Option<Faction>,
    move_target: Option<MoveTarget>,
}

/// Parsed template entry (load only); `template` is stripped after resolve.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct EntityTemplateRaw {
    #[serde(default)]
    template: Option<String>,
    #[serde(flatten)]
    components: EntityComponents,
}

/// Flattened template stored on Api and used at spawn.
pub(crate) type EntitySpawnYaml = EntityComponents;

pub(crate) type EntityTemplates = BTreeMap<String, EntitySpawnYaml>;
```

`TemplatesFileRoot` deserializes `entities: BTreeMap<String, EntityTemplateRaw>`.

`#[serde(flatten)]` keeps authoring YAML flat:

```yaml
scout:
  template: unit
  faction: 1
```

without a nested `components:` wrapper.

## Algorithms

### `merge_components(parent: &EntityComponents, child: &EntityComponents) -> EntityComponents`

For each of `position`, `velocity`, `faction`, `move_target`:

`merged.field = child.field.or(parent.field)` (child wins when `Some`).

### `resolve_template(name, raw, stack, memo) -> Result<EntityComponents, ImportError>`

1. If `memo` contains `name`, return clone.
2. If `stack` contains `name`, return `TemplateCycle { chain: stack + [name] }`.
3. Let `entry = raw.get(name)` or `UnknownTemplateParent { child: current_child, parent: name }` when resolving a referenced parent that does not exist.
4. Let `base = if let Some(parent_name) = &entry.template { resolve(parent_name, …)? } else { EntityComponents::default() }`.
5. `merged = merge_components(&base, &entry.components)`.
6. Insert `merged` into `memo`, return `merged`.

When resolving the top-level map, call resolve for each key in `raw` with `current_child = name` and empty initial stack.

### `load_templates_yaml`

1. Parse YAML → `BTreeMap<String, EntityTemplateRaw>` (`ImportError::Yaml` on failure).
2. Resolve every entry → `BTreeMap<String, EntityComponents>`.
3. `self.templates = Some(flattened)`.
4. On any resolve error: return `Err`, leave existing `templates` unchanged.

### `spawn_yaml`

Unchanged: lookup flattened map, `spawn_from_doc` inserts each `Some` component and `EntityType(template_name)`.

## Error Handling

```rust
pub enum ImportError {
    Yaml(serde_yaml::Error),
    TemplatesNotLoaded,
    UnknownTemplate(String),
    UnknownTemplateParent { child: String, parent: String },
    TemplateCycle { chain: Vec<String> },
}
```

| Case | Variant |
|------|---------|
| `template: ghost` on `scout` | `UnknownTemplateParent { child: "scout", parent: "ghost" }` |
| `scout → unit → scout` | `TemplateCycle { chain: ["scout", "unit", "scout"] }` |
| Invalid YAML / unknown component key | `Yaml` |
| `spawn_yaml("nope")` after successful load | `UnknownTemplate` |

**Display (suggested):**

- Cycle: `template inheritance cycle: scout -> unit -> scout`
- Missing parent: `template "ghost" not found (referenced from "scout")`

## Testing

| Test | Assertion |
|------|-----------|
| `inherit_single_level` | scout inherits `faction` from unit + own `velocity` |
| `inherit_chain` | `c → b → a`; components accumulate along chain |
| `inherit_override_component` | child `position` replaces parent entirely |
| `inherit_child_only_template` | entry with only `template: unit` matches unit components |
| `spawn_base_template` | `spawn_yaml("unit")` succeeds |
| `load_unknown_template_parent` | `UnknownTemplateParent` |
| `load_template_cycle` | `TemplateCycle` with full chain |
| `load_failed_resolve_keeps_previous` | bad second load leaves first map |
| Existing spawn-import tests without `template` | unchanged behavior |

```bash
cargo test -p open_entities
cargo clippy -p open_entities -- -D warnings
```

## Alternatives Considered

| Approach | Why not (this increment) |
|----------|---------------------------|
| Lazy resolve at `spawn_yaml` | Errors only at spawn; repeated work |
| Topological sort batch | Same result, more code for small files |
| Duplicate `EntitySpawnYaml` + `EntityTemplateRaw` fields | Drift when adding components |
| `EntitySpawnYaml` with `template: Option` kept in memory | Mixes parse and stored phases |
| YAML anchors `<<: *unit` | Opaque errors; no typed cycle/parent errors |
| Declarative macro for all components | Premature; inventory/equipment need different merge rules later |

## Evolution

When importable data grows (inventory slots, items, clothing):

- Add fields to `EntityComponents` only while merge rule stays `child.or(parent)` per top-level block.
- If merge rules diverge (deep slot merge, nested maps), add separate YAML sections and `import/` submodules rather than expanding a macro.
- If many homogeneous components share one merge rule, consider a **component import registry** (name → deserialize / merge / spawn) instead of a large `macro_rules!`.

## Future Extensions

- Cross-file `template` via path or include.
- `template_names()` iterator on `Api`.
- Example `assets/entities.yaml` with inheritance for RTS units.

## License Note

No new dependencies beyond existing `serde_yaml`.
