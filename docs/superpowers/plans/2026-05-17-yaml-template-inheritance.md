# YAML Template Inheritance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `load_templates_yaml` so `entities.<name>` may declare `template` (string or list) to inherit component fields from other entries in the same file; resolve and flatten at load time; keep `spawn_yaml` unchanged.

**Architecture:** Split parse shape (`EntityTemplateRaw` + `TemplateParents`) from stored shape (`EntityComponents`, aliased as `EntitySpawnYaml`). Add `merge_components` and `resolve_template` (DFS + memo + cycle stack). `load_templates_yaml` parses raw map, resolves every name into a flattened `BTreeMap`, then stores it. Component merge is top-level only: `child.field.or(parent.field)` per `position`, `velocity`, `faction`, `move_target`.

**Tech Stack:** Rust 2024, `bevy_ecs 0.18`, `serde`, `serde_yaml 0.9` (existing).

**Spec:** `docs/superpowers/specs/2026-05-17-yaml-template-inheritance-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `open-entities-lib/src/import/mod.rs` | New types, `merge_components`, `resolve_template`, `resolve_all_templates`, extended `ImportError`, updated `load_templates_yaml`, all new tests |
| `open-entities-lib/src/api.rs` | No changes (`EntityTemplates` alias unchanged) |

**Out of scope (no tasks):** `wasm-bindings`, examples, Makefile, cross-file templates, ECS `Parent`, deep field merge inside components.

---

### Task 1: Extend `ImportError` with template resolution variants

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs`

- [ ] **Step 1: Write failing tests for new error variants**

Add inside `mod tests` in `import/mod.rs`:

```rust
#[test]
fn import_error_unknown_template_parent_display() {
    let err = ImportError::UnknownTemplateParent {
        child: "scout".to_owned(),
        parent: "ghost".to_owned(),
    };
    assert_eq!(
        err.to_string(),
        r#"template "ghost" not found (referenced from "scout")"#
    );
}

#[test]
fn import_error_template_cycle_display() {
    let err = ImportError::TemplateCycle {
        chain: vec![
            "scout".to_owned(),
            "unit".to_owned(),
            "scout".to_owned(),
        ],
    };
    assert_eq!(
        err.to_string(),
        "template inheritance cycle: scout -> unit -> scout"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p open_entities import_error_unknown_template_parent_display import_error_template_cycle_display -- --nocapture
```

Expected: compile errors — variants `UnknownTemplateParent` and `TemplateCycle` missing on `ImportError`.

- [ ] **Step 3: Add variants and `Display` arms**

In `ImportError` enum, after `UnknownTemplate(String)`:

```rust
/// Referenced parent name missing from `entities` map.
UnknownTemplateParent { child: String, parent: String },
/// Circular `template` chain detected during load.
TemplateCycle { chain: Vec<String> },
```

In `impl Display for ImportError`, add match arms:

```rust
Self::UnknownTemplateParent { child, parent } => {
    write!(
        f,
        r#"template "{parent}" not found (referenced from "{child}")"#
    )
}
Self::TemplateCycle { chain } => {
    write!(f, "template inheritance cycle: {}", chain.join(" -> "))
}
```

Update `impl Error for ImportError` `source` arm to return `None` for the two new variants.

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test -p open_entities import_error_unknown_template_parent_display import_error_template_cycle_display -- --nocapture
```

Expected: both tests `ok`.

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "feat: add template inheritance ImportError variants"
```

---

### Task 2: Introduce `EntityComponents`, `TemplateParents`, and `EntityTemplateRaw`

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs`

- [ ] **Step 1: Rename `EntitySpawnYaml` struct to `EntityComponents`**

Replace the existing struct (keep fields private, same four components):

```rust
/// Shared component bundle — single place to add new importable components.
#[derive(Deserialize, Clone, Default, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
struct EntityComponents {
    position: Option<Position>,
    velocity: Option<Velocity>,
    faction: Option<Faction>,
    move_target: Option<MoveTarget>,
}

/// Flattened template stored on Api and used at spawn.
pub(crate) type EntitySpawnYaml = EntityComponents;

pub(crate) type EntityTemplates = BTreeMap<String, EntitySpawnYaml>;
```

Update `spawn_from_doc` signature parameter type to `&EntityComponents` (body unchanged).

- [ ] **Step 2: Add `TemplateParents` and `EntityTemplateRaw`**

Insert after `EntityComponents`:

```rust
/// One parent name or an ordered list (serde untagged).
#[derive(Deserialize, Clone, Default, PartialEq, Debug)]
#[serde(untagged)]
enum TemplateParents {
    #[default]
    None,
    One(String),
    Many(Vec<String>),
}

impl TemplateParents {
    fn into_vec(self) -> Vec<String> {
        match self {
            Self::None => Vec::new(),
            Self::One(name) => vec![name],
            Self::Many(names) => names,
        }
    }
}

/// Parsed template entry (load only); `template` is stripped after resolve.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct EntityTemplateRaw {
    #[serde(default)]
    template: TemplateParents,
    #[serde(flatten)]
    components: EntityComponents,
}
```

- [ ] **Step 3: Point `TemplatesFileRoot` at raw parse type**

Change:

```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TemplatesFileRoot {
    entities: BTreeMap<String, EntityTemplateRaw>,
}
```

- [ ] **Step 4: Run full import test suite (expect compile failure on load)**

Run:

```bash
cargo test -p open_entities --lib import::tests -- --nocapture
```

Expected: compile error in `load_templates_yaml` — cannot assign `BTreeMap<String, EntityTemplateRaw>` to `Option<EntityTemplates>` until Task 6.

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "refactor: split EntityTemplateRaw from EntityComponents"
```

---

### Task 3: `merge_components`

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs`

- [ ] **Step 1: Write failing unit tests**

Add before `impl Api` (or after helper fns section), still in `mod.rs` but outside `tests`:

```rust
#[cfg(test)]
mod merge_tests {
    use super::*;

    #[test]
    fn merge_child_wins_over_parent() {
        let parent = EntityComponents {
            faction: Some(Faction(1)),
            velocity: Some(Velocity { vx: 1.0, vy: 0.0 }),
            ..Default::default()
        };
        let child = EntityComponents {
            faction: Some(Faction(2)),
            ..Default::default()
        };
        let merged = merge_components(&parent, &child);
        assert_eq!(merged.faction, Some(Faction(2)));
        assert_eq!(merged.velocity, Some(Velocity { vx: 1.0, vy: 0.0 }));
    }

    #[test]
    fn merge_fills_missing_from_parent() {
        let parent = EntityComponents {
            faction: Some(Faction(1)),
            ..Default::default()
        };
        let child = EntityComponents {
            velocity: Some(Velocity { vx: 2.0, vy: 0.0 }),
            ..Default::default()
        };
        let merged = merge_components(&parent, &child);
        assert_eq!(merged.faction, Some(Faction(1)));
        assert_eq!(merged.velocity, Some(Velocity { vx: 2.0, vy: 0.0 }));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p open_entities merge_child_wins_over_parent merge_fills_missing_from_parent -- --nocapture
```

Expected: `merge_components` not found.

- [ ] **Step 3: Implement `merge_components`**

Add above `spawn_from_doc`:

```rust
fn merge_components(parent: &EntityComponents, child: &EntityComponents) -> EntityComponents {
    EntityComponents {
        position: child.position.or(parent.position),
        velocity: child.velocity.or(parent.velocity),
        faction: child.faction.or(parent.faction),
        move_target: child.move_target.or(parent.move_target),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test -p open_entities merge_child_wins_over_parent merge_fills_missing_from_parent -- --nocapture
```

Expected: both `ok`.

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "feat: add component-level merge_components helper"
```

---

### Task 4: `resolve_template` and `resolve_all_templates`

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs`

- [ ] **Step 1: Write failing resolve unit tests**

Add module `resolve_tests` in `#[cfg(test)]`:

```rust
#[cfg(test)]
mod resolve_tests {
    use super::*;

    fn raw_map(pairs: &[(&str, EntityTemplateRaw)]) -> BTreeMap<String, EntityTemplateRaw> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn entry(
        template: TemplateParents,
        components: EntityComponents,
    ) -> EntityTemplateRaw {
        EntityTemplateRaw {
            template,
            components,
        }
    }

    #[test]
    fn resolve_single_parent() {
        let raw = raw_map(&[
            (
                "unit",
                entry(TemplateParents::None, EntityComponents {
                    faction: Some(Faction(1)),
                    ..Default::default()
                }),
            ),
            (
                "scout",
                entry(
                    TemplateParents::One("unit".to_owned()),
                    EntityComponents {
                        velocity: Some(Velocity { vx: 2.0, vy: 0.0 }),
                        ..Default::default()
                    },
                ),
            ),
        ]);
        let resolved = resolve_all_templates(&raw).expect("resolve");
        let scout = resolved.get("scout").expect("scout");
        assert_eq!(scout.faction, Some(Faction(1)));
        assert_eq!(scout.velocity, Some(Velocity { vx: 2.0, vy: 0.0 }));
    }

    #[test]
    fn resolve_unknown_parent() {
        let raw = raw_map(&[(
            "scout",
            entry(
                TemplateParents::One("ghost".to_owned()),
                EntityComponents::default(),
            ),
        )]);
        let err = resolve_all_templates(&raw).unwrap_err();
        assert!(matches!(
            err,
            ImportError::UnknownTemplateParent { child, parent }
            if child == "scout" && parent == "ghost"
        ));
    }

    #[test]
    fn resolve_cycle() {
        let raw = raw_map(&[
            (
                "scout",
                entry(
                    TemplateParents::One("unit".to_owned()),
                    EntityComponents::default(),
                ),
            ),
            (
                "unit",
                entry(
                    TemplateParents::One("scout".to_owned()),
                    EntityComponents::default(),
                ),
            ),
        ]);
        let err = resolve_all_templates(&raw).unwrap_err();
        assert!(matches!(err, ImportError::TemplateCycle { chain } if chain == ["scout", "unit", "scout"]));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p open_entities resolve_single_parent resolve_unknown_parent resolve_cycle -- --nocapture
```

Expected: `resolve_all_templates` / `resolve_template` not found.

- [ ] **Step 3: Implement resolution**

Add above `spawn_from_doc`:

```rust
fn resolve_template(
    name: &str,
    child: &str,
    raw: &BTreeMap<String, EntityTemplateRaw>,
    stack: &mut Vec<String>,
    memo: &mut BTreeMap<String, EntityComponents>,
) -> Result<EntityComponents, ImportError> {
    if let Some(resolved) = memo.get(name) {
        return Ok(resolved.clone());
    }
    if stack.iter().any(|s| s == name) {
        let mut chain = stack.clone();
        chain.push(name.to_owned());
        return Err(ImportError::TemplateCycle { chain });
    }

    let entry = raw.get(name).ok_or_else(|| ImportError::UnknownTemplateParent {
        child: child.to_owned(),
        parent: name.to_owned(),
    })?;

    stack.push(name.to_owned());

    let mut base = EntityComponents::default();
    for parent_name in entry.template.clone().into_vec() {
        let parent_doc = resolve_template(&parent_name, child, raw, stack, memo)?;
        base = merge_components(&base, &parent_doc);
    }

    let merged = merge_components(&base, &entry.components);
    memo.insert(name.to_owned(), merged.clone());
    stack.pop();

    Ok(merged)
}

fn resolve_all_templates(
    raw: &BTreeMap<String, EntityTemplateRaw>,
) -> Result<EntityTemplates, ImportError> {
    let mut memo = BTreeMap::new();
    for name in raw.keys() {
        resolve_template(name, name, raw, &mut Vec::new(), &mut memo)?;
    }
    Ok(memo)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test -p open_entities resolve_single_parent resolve_unknown_parent resolve_cycle -- --nocapture
```

Expected: all three `ok`.

- [ ] **Step 5: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "feat: resolve template inheritance graph at load time"
```

---

### Task 5: Wire `load_templates_yaml` to resolve

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs`

- [ ] **Step 1: Write failing integration test `inherit_single_level`**

Add to `mod tests`:

```rust
#[test]
fn inherit_single_level() {
    let yaml = r"
entities:
  unit:
    faction: 1
  scout:
    template: unit
    velocity: { vx: 2, vy: 0 }
";
    let mut api = Api::new();
    api.load_templates_yaml(yaml).expect("load");

    let entity = api.spawn_yaml("scout").expect("spawn scout");
    let world = api.core_mut().world_mut();
    assert_eq!(world.get::<Faction>(entity).map(|f| f.0), Some(1));
    let velocity = world.get::<Velocity>(entity).expect("velocity");
    assert_eq!(velocity.vx, 2.0);
    assert_eq!(velocity.vy, 0.0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p open_entities inherit_single_level -- --nocapture
```

Expected: FAIL — scout spawns without inherited `faction` (or compile error if load not wired).

- [ ] **Step 3: Update `load_templates_yaml`**

Replace body:

```rust
pub fn load_templates_yaml(&mut self, yaml: &str) -> Result<(), ImportError> {
    let parsed: TemplatesFileRoot =
        serde_yaml::from_str(yaml).map_err(ImportError::Yaml)?;
    let flattened = resolve_all_templates(&parsed.entities)?;
    self.templates = Some(flattened);
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p open_entities inherit_single_level -- --nocapture
```

Expected: `ok`.

- [ ] **Step 5: Run existing import tests (regression)**

Run:

```bash
cargo test -p open_entities --lib import::tests -- --nocapture
```

Expected: all existing spawn-import tests still `ok` (fixtures have no `template` key).

- [ ] **Step 6: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "feat: flatten template inheritance in load_templates_yaml"
```

---

### Task 6: Chain and override inheritance tests

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs` (`mod tests`)

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn inherit_chain() {
    let yaml = r"
entities:
  a:
    faction: 1
  b:
    template: a
    velocity: { vx: 1, vy: 0 }
  c:
    template: b
    position: { x: 0, y: 0 }
";
    let mut api = Api::new();
    api.load_templates_yaml(yaml).expect("load");
    let entity = api.spawn_yaml("c").expect("spawn c");
    let world = api.core_mut().world_mut();
    assert_eq!(world.get::<Faction>(entity).map(|f| f.0), Some(1));
    let velocity = world.get::<Velocity>(entity).expect("velocity");
    assert_eq!(velocity.vx, 1.0);
    let position = world.get::<Position>(entity).expect("position");
    assert_eq!(position.x, 0.0);
    assert_eq!(position.y, 0.0);
}

#[test]
fn inherit_override_component() {
    let yaml = r"
entities:
  unit:
    position: { x: 1, y: 1 }
  scout:
    template: unit
    position: { x: 9, y: 9 }
";
    let mut api = Api::new();
    api.load_templates_yaml(yaml).expect("load");
    let entity = api.spawn_yaml("scout").expect("spawn scout");
    let world = api.core_mut().world_mut();
    let position = world.get::<Position>(entity).expect("position");
    assert_eq!(position.x, 9.0);
    assert_eq!(position.y, 9.0);
}

#[test]
fn inherit_child_only_template() {
    let yaml = r"
entities:
  unit:
    faction: 1
    velocity: { vx: 0.5, vy: 0 }
  clone:
    template: unit
";
    let mut api = Api::new();
    api.load_templates_yaml(yaml).expect("load");

    let unit = api.spawn_yaml("unit").expect("spawn unit");
    let clone = api.spawn_yaml("clone").expect("spawn clone");
    let world = api.core_mut().world_mut();
    assert_eq!(world.get::<Faction>(unit).map(|f| f.0), Some(1));
    assert_eq!(world.get::<Faction>(clone).map(|f| f.0), Some(1));
    assert!(world.get::<Velocity>(clone).is_some());
}
```

- [ ] **Step 2: Run tests (should pass immediately)**

Run:

```bash
cargo test -p open_entities inherit_chain inherit_override_component inherit_child_only_template -- --nocapture
```

Expected: all three `ok` (implementation already complete; tests document behavior).

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "test: template inheritance chain and override"
```

---

### Task 7: Multiple parents and empty list tests

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs` (`mod tests`)

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn inherit_multiple_templates() {
    let yaml = r"
entities:
  unit:
    faction: 1
  tank:
    template: unit
    velocity: { vx: 0.5, vy: 0 }
  heavy_tank:
    template: [unit, tank]
    faction: 3
";
    let mut api = Api::new();
    api.load_templates_yaml(yaml).expect("load");
    let entity = api.spawn_yaml("heavy_tank").expect("spawn");
    let world = api.core_mut().world_mut();
    assert_eq!(world.get::<Faction>(entity).map(|f| f.0), Some(3));
    let velocity = world.get::<Velocity>(entity).expect("velocity from tank");
    assert_eq!(velocity.vx, 0.5);
    assert_eq!(velocity.vy, 0.0);
}

#[test]
fn inherit_multiple_string_equivalent() {
    let yaml_one = r"
entities:
  unit:
    faction: 1
  scout:
    template: unit
    velocity: { vx: 2, vy: 0 }
";
    let yaml_many = r"
entities:
  unit:
    faction: 1
  scout:
    template: [unit]
    velocity: { vx: 2, vy: 0 }
";
    let mut api_one = Api::new();
    api_one.load_templates_yaml(yaml_one).expect("load one");
    let mut api_many = Api::new();
    api_many.load_templates_yaml(yaml_many).expect("load many");

    let e1 = api_one.spawn_yaml("scout").expect("spawn one");
    let e2 = api_many.spawn_yaml("scout").expect("spawn many");
    let w1 = api_one.core_mut().world_mut();
    let w2 = api_many.core_mut().world_mut();
    assert_eq!(w1.get::<Faction>(e1).map(|f| f.0), w2.get::<Faction>(e2).map(|f| f.0));
    assert_eq!(
        w1.get::<Velocity>(e1).map(|v| (v.vx, v.vy)),
        w2.get::<Velocity>(e2).map(|v| (v.vx, v.vy))
    );
}

#[test]
fn inherit_multiple_then_child_override() {
    let yaml = r"
entities:
  unit:
    faction: 1
  tank:
    template: unit
    faction: 2
  hybrid:
    template: [unit, tank]
    faction: 9
";
    let mut api = Api::new();
    api.load_templates_yaml(yaml).expect("load");
    let entity = api.spawn_yaml("hybrid").expect("spawn");
    let world = api.core_mut().world_mut();
    assert_eq!(world.get::<Faction>(entity).map(|f| f.0), Some(9));
}

#[test]
fn inherit_empty_template_list() {
    let yaml_with = r"
entities:
  unit:
    faction: 1
  bare:
    template: []
";
    let yaml_without = r"
entities:
  unit:
    faction: 1
  bare: {}
";
    let mut api_with = Api::new();
    api_with.load_templates_yaml(yaml_with).expect("load with");
    let mut api_without = Api::new();
    api_without
        .load_templates_yaml(yaml_without)
        .expect("load without");

    let e1 = api_with.spawn_yaml("bare").expect("spawn with");
    let e2 = api_without.spawn_yaml("bare").expect("spawn without");
    let w1 = api_with.core_mut().world_mut();
    let w2 = api_without.core_mut().world_mut();
    assert!(w1.get::<Faction>(e1).is_none());
    assert!(w2.get::<Faction>(e2).is_none());
}
```

- [ ] **Step 2: Run tests**

Run:

```bash
cargo test -p open_entities inherit_multiple_templates inherit_multiple_string_equivalent inherit_multiple_then_child_override inherit_empty_template_list -- --nocapture
```

Expected: all four `ok`.

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "test: multiple template parents and empty list"
```

---

### Task 8: Load error paths and atomic failure

**Files:**
- Modify: `open-entities-lib/src/import/mod.rs` (`mod tests`)

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn load_unknown_template_parent() {
    let mut api = Api::new();
    let yaml = r"
entities:
  scout:
    template: ghost
";
    let err = api.load_templates_yaml(yaml).unwrap_err();
    assert!(matches!(
        err,
        ImportError::UnknownTemplateParent { child, parent }
        if child == "scout" && parent == "ghost"
    ));
    assert!(api.templates.is_none());
}

#[test]
fn load_template_cycle() {
    let mut api = Api::new();
    let yaml = r"
entities:
  scout:
    template: unit
  unit:
    template: scout
";
    let err = api.load_templates_yaml(yaml).unwrap_err();
    assert!(matches!(
        err,
        ImportError::TemplateCycle { chain }
        if chain == ["scout", "unit", "scout"]
    ));
    assert!(api.templates.is_none());
}

#[test]
fn load_failed_resolve_keeps_previous() {
    let mut api = Api::new();
    api.load_templates_yaml("entities:\n  a:\n    faction: 1\n")
        .expect("first load");
    let err = api
        .load_templates_yaml(
            "entities:\n  bad:\n    template: missing\n",
        )
        .unwrap_err();
    assert!(matches!(
        err,
        ImportError::UnknownTemplateParent { .. }
    ));
    let templates = api.templates.as_ref().expect("first map kept");
    assert!(templates.contains_key("a"));
    assert!(!templates.contains_key("bad"));
}

#[test]
fn spawn_base_template() {
    let yaml = r"
entities:
  unit:
    faction: 1
  scout:
    template: unit
    velocity: { vx: 2, vy: 0 }
";
    let mut api = Api::new();
    api.load_templates_yaml(yaml).expect("load");
    let entity = api.spawn_yaml("unit").expect("spawn base");
    let world = api.core_mut().world_mut();
    assert_eq!(world.get::<Faction>(entity).map(|f| f.0), Some(1));
    assert_eq!(
        world.get::<EntityType>(entity).map(|t| t.0.as_str()),
        Some("unit")
    );
}
```

- [ ] **Step 2: Run tests**

Run:

```bash
cargo test -p open_entities load_unknown_template_parent load_template_cycle load_failed_resolve_keeps_previous spawn_base_template -- --nocapture
```

Expected: all four `ok`.

- [ ] **Step 3: Commit**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "test: template load errors and base template spawn"
```

---

### Task 9: Final verification

**Files:**
- (none — commands only)

- [ ] **Step 1: Run full library tests**

Run:

```bash
cargo test -p open_entities -- --nocapture
```

Expected: all tests pass.

- [ ] **Step 2: Run clippy**

Run:

```bash
cargo clippy -p open_entities -- -D warnings
```

Expected: no warnings (fix any `clone` on `TemplateParents` if clippy suggests `into_vec` consuming `entry.template` without extra clone — e.g. destructure `EntityTemplateRaw` once: `let EntityTemplateRaw { template, components } = entry.clone();`).

- [ ] **Step 3: Commit (only if clippy fixes were needed)**

```bash
git add open-entities-lib/src/import/mod.rs
git commit -m "chore: clippy fixes for template inheritance"
```

Skip commit if Step 2 is clean.

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| `EntityComponents` shared block | Task 2 |
| `TemplateParents` string / list / empty | Task 2, 7 |
| `EntityTemplateRaw` + flatten serde | Task 2 |
| `merge_components` child.or(parent) | Task 3 |
| `resolve_template` DFS + memo + cycle | Task 4 |
| `load_templates_yaml` flatten before store | Task 5 |
| `spawn_yaml` unchanged | Task 5 regression |
| `UnknownTemplateParent` | Task 1, 4, 8 |
| `TemplateCycle` | Task 1, 4, 8 |
| Atomic load on resolve failure | Task 8 |
| Single / chain / override inheritance | Task 5–6 |
| Multiple parents order | Task 7 |
| Base templates spawnable | Task 8 |
| Existing tests without `template` | Task 5 |

**Gaps (explicitly out of scope per spec):** cross-file templates, `template_names()` iterator, example `assets/entities.yaml`.

---

## Self-review notes

- All test names match the spec’s testing table.
- `EntitySpawnYaml` remains the public(crate) alias — `api.rs` needs no edit.
- `spawn_from_doc` and `spawn_yaml` bodies stay the same aside from the renamed component struct type.
