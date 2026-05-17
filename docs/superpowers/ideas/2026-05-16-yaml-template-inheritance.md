# Idea: YAML template inheritance (`template`)

**Date:** 2026-05-16  
**Status:** Deferred (not in yaml-spawn-import PR)  
**Related spec:** [YAML Component Spawn Import](../specs/2026-05-16-yaml-spawn-import-design.md)

## Summary

Extend named entity templates so a template can inherit component fields from another template in the same file via a reserved key `template` (not ECS parent/child).

## Motivation

RTS unit definitions share bases (`unit` → `scout`, `tank`). Without inheritance, authors duplicate `faction`, default stats, etc. Named templates (current PR) reduce file sprawl but not duplication within the file.

## Proposed format

```yaml
entities:
  unit:
    faction: 1

  scout:
    template: unit
    position: { x: 0, y: 0 }
    velocity: { vx: 2, vy: 0 }
```

- `template` — name of another entry in the same `entities` map (same file).
- **Not** an ECS hierarchy component; name chosen to avoid confusion with Bevy `Parent`.

## Resolution rules (draft)

| Rule | Choice |
|------|--------|
| Merge granularity | **Component-level replace**: child `position` fully overrides parent `position`; no deep merge of `x`/`y` |
| Chain | Allow `a → b → c`; resolve after full parse |
| Cycles | Error (`scout` → `unit` → `scout`) |
| Missing `template` target | Error |
| Order in YAML | Irrelevant; resolve by name after parse |
| `template` on root | Only inside `entities.<name>` entries, never at file root |
| Atomicity | Resolve all templates before any `World` spawn; any error → no entities created |

## API sketch

Resolve inheritance inside `load_templates_yaml` (store flattened templates) or lazily on `spawn_yaml(name)`. Reserved key `template` only inside `entities.<name>` entries. Current PR: load + `spawn_yaml(name)` only; no `template` key yet.

## Alternatives

| Approach | Notes |
|----------|-------|
| YAML anchors `<<: *unit` | No custom code; opaque errors; harder for non-YAML authors |
| `extends` instead of `template` | Common in other systems; user preferred `template` |
| Deep field merge | More surprising; defer unless needed |

## Implementation estimate

~80–150 LOC + tests on top of `spawn_templates_yaml`: resolve graph, merge `EntitySpawnYaml`, cycle detection.

## Out of scope for this idea

- Cross-file `template` references
- Multiple inheritance
- ECS `Parent` / scene graph from YAML
