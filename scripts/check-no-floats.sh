#!/usr/bin/env sh
# Fails if `f32` or `f64` appears in a simulation module. Simulation state is integer milli-units;
# only boundary modules (import/, export/, units.rs, map.rs, entity_components.rs) may use floats.
# See docs/design/lockstep-roadmap.md, "Definitions".
set -eu
cd "$(dirname "$0")/../open-entities-lib/src"
if grep -rnwE 'f32|f64' \
    components systems simulation.rs orders.rs groups.rs missions.rs boarding.rs core.rs \
    commands.rs replay.rs state_hash.rs; then
    echo "error: f32/f64 in a simulation module (listed above)" >&2
    exit 1
fi
echo "float check: no f32/f64 in simulation modules"
