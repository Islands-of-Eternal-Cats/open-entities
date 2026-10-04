use open_entities::components::MoveTarget;
use open_entities::{Api, EntityComponents, EntityId, ImportError, hello};
use open_entities::{BoardError, GroupError, MissionError, units};
use open_entities::{Command, CommandOutcome, CommandResult, StepReport};
use serde::Serialize;

/// Converts a JS point in map units to a [`MoveTarget`] in milli-units.
fn move_target(x: f64, y: f64, call: &str) -> Result<MoveTarget, JsValue> {
    let convert =
        |v: f64| units::to_milli(v).map_err(|e| JsValue::from_str(&format!("{call}: {e}")));
    Ok(MoveTarget {
        x: convert(x)?,
        y: convert(y)?,
    })
}
use wasm_bindgen::prelude::*;

/// A [`StepReport`] as JS sees it: `{ tick, outcomes }`.
#[derive(Serialize)]
struct JsStepReport {
    tick: u64,
    outcomes: Vec<JsOutcome>,
}

/// One [`CommandOutcome`], flattened for JS: `ok` plus whichever of the other fields apply.
///
/// `{ seq, ok: true, spawned | group | mission: {index, generation} }` for a creation,
/// `{ seq, ok: true, applied, skipped }` for anything else, `{ seq, ok: false, error }` when the
/// command was refused.
#[derive(Serialize, Default)]
struct JsOutcome {
    seq: u64,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    spawned: Option<EntityId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<EntityId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mission: Option<EntityId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    applied: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl From<CommandOutcome> for JsOutcome {
    fn from(outcome: CommandOutcome) -> Self {
        let seq = outcome.seq.0;
        match outcome.result {
            Ok(CommandResult::Spawned(id)) => Self {
                seq,
                ok: true,
                spawned: Some(id),
                ..Self::default()
            },
            Ok(CommandResult::GroupCreated(id)) => Self {
                seq,
                ok: true,
                group: Some(id),
                ..Self::default()
            },
            Ok(CommandResult::MissionCreated(id)) => Self {
                seq,
                ok: true,
                mission: Some(id),
                ..Self::default()
            },
            Ok(CommandResult::Applied { applied, skipped }) => Self {
                seq,
                ok: true,
                applied: Some(applied),
                skipped: Some(skipped),
                ..Self::default()
            },
            Err(err) => Self {
                seq,
                ok: false,
                error: Some(err.to_string()),
                ..Self::default()
            },
        }
    }
}

impl From<StepReport> for JsStepReport {
    fn from(report: StepReport) -> Self {
        Self {
            tick: report.tick,
            outcomes: report.outcomes.into_iter().map(JsOutcome::from).collect(),
        }
    }
}

/// Reads a command object: `{ type: "move_to", ids: [...], target: { x, y } }` and so on.
fn command_from_js(command: JsValue, call: &str) -> Result<Command, JsValue> {
    serde_wasm_bindgen::from_value(command)
        .map_err(|e| JsValue::from_str(&format!("{call}: invalid command: {e}")))
}

/// Sequence numbers count calls; they stay far below 2^53, where a JS number is still exact.
#[allow(clippy::cast_precision_loss)]
fn seq_to_js(seq: open_entities::CommandSeq) -> f64 {
    seq.0 as f64
}

#[wasm_bindgen]
pub struct Simulation {
    api: Api,
    /// Length of the last frame, so the next one allocates once.
    frame_len: usize,
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl Simulation {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            api: Api::new(),
            frame_len: 0,
        }
    }

    /// Returns the canonical greeting from `open_entities::hello()`.
    pub fn hello(&self) -> String {
        hello().to_owned()
    }

    /// JS: `loadTemplatesYaml(yaml)`
    #[wasm_bindgen(js_name = loadTemplatesYaml)]
    pub fn load_templates_yaml(&mut self, yaml: &str) -> Result<(), JsValue> {
        self.api
            .load_templates_yaml(yaml)
            .map_err(|e: ImportError| JsValue::from_str(&e.to_string()))
    }

    /// JS: `spawnEntity(templateName, overrides)` — returns the new entity's id.
    ///
    /// The id is an `{index, generation}` object, the same shape `getWorldAsJson()` reports and
    /// `orderMoveTo`, `orderStop`, `despawn` and `isAlive` accept.
    #[wasm_bindgen(js_name = spawnEntity)]
    pub fn spawn_entity(
        &mut self,
        template_name: &str,
        overrides: JsValue,
    ) -> Result<JsValue, JsValue> {
        let overrides: EntityComponents = serde_wasm_bindgen::from_value(overrides)
            .map_err(|e| JsValue::from_str(&format!("invalid overrides: {e}")))?;
        let id = self
            .api
            .spawn_entity(template_name, overrides)
            .map_err(|e: ImportError| JsValue::from_str(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&id)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize id: {e}")))
    }

    /// JS: `getWorldAsJson()` — every entity with every component, for debugging and saves.
    ///
    /// Not for every frame: a renderer reads `writeFrame()` and `metaDelta()`.
    #[wasm_bindgen(js_name = getWorldAsJson)]
    pub fn world_json(&mut self) -> Result<String, JsValue> {
        serde_json::to_string(&self.api.world_snapshot())
            .map_err(|e| JsValue::from_str(&format!("JSON export failed: {e}")))
    }

    /// JS: `orderMoveTo(ids, x, y)` — move order for a group of entities.
    ///
    /// `ids` is an array of `{index, generation}` objects, exactly as `getWorldAsJson()` reports
    /// each entity's `id`. Returns how many entities took the order; immobile and unknown ids are
    /// skipped. Destinations are spread over a grid so a group does not pile onto one point.
    #[wasm_bindgen(js_name = orderMoveTo)]
    pub fn order_move_to(&mut self, ids: JsValue, x: f64, y: f64) -> Result<u32, JsValue> {
        let ids: Vec<EntityId> = serde_wasm_bindgen::from_value(ids)
            .map_err(|e| JsValue::from_str(&format!("invalid entity ids: {e}")))?;
        if ids.is_empty() {
            return Err(JsValue::from_str(
                "orderMoveTo(ids, x, y) requires at least one id",
            ));
        }
        if !x.is_finite() || !y.is_finite() {
            return Err(JsValue::from_str(
                "orderMoveTo(ids, x, y) requires finite coordinates",
            ));
        }
        let target = move_target(x, y, "orderMoveTo")?;
        let report = self.api.order_move_to(&ids, target);
        u32::try_from(report.ordered)
            .map_err(|_| JsValue::from_str("orderMoveTo ordered more entities than u32 can hold"))
    }

    /// JS: `loadMapYaml(yaml)` — spawns a starting layout, returns the new entities' ids.
    ///
    /// Ids come back as `{index, generation}` objects, ready to pass to `orderMoveTo`. A bad
    /// template name aborts the whole map and spawns nothing.
    #[wasm_bindgen(js_name = loadMapYaml)]
    pub fn load_map_yaml(&mut self, yaml: &str) -> Result<JsValue, JsValue> {
        let spawned = self
            .api
            .load_map_yaml(yaml)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&spawned)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize ids: {e}")))
    }

    /// JS: `mapBounds()` — `{width, height}` of the last loaded map, or `null`.
    #[wasm_bindgen(js_name = mapBounds)]
    pub fn map_bounds(&self) -> Result<JsValue, JsValue> {
        match self.api.map_bounds() {
            Some(bounds) => serde_wasm_bindgen::to_value(&bounds)
                .map_err(|e| JsValue::from_str(&format!("failed to serialize map bounds: {e}"))),
            None => Ok(JsValue::NULL),
        }
    }

    /// JS: `orderStop(ids)` — zero velocity and drop any move target.
    ///
    /// Works on anything carrying a velocity, including entities that drift with no move speed.
    /// Returns how many entities were stopped.
    #[wasm_bindgen(js_name = orderStop)]
    pub fn order_stop(&mut self, ids: JsValue) -> Result<u32, JsValue> {
        let ids: Vec<EntityId> = serde_wasm_bindgen::from_value(ids)
            .map_err(|e| JsValue::from_str(&format!("invalid entity ids: {e}")))?;
        if ids.is_empty() {
            return Err(JsValue::from_str("orderStop(ids) requires at least one id"));
        }
        let report = self.api.order_stop(&ids);
        u32::try_from(report.ordered)
            .map_err(|_| JsValue::from_str("orderStop stopped more entities than u32 can hold"))
    }

    /// JS: `despawn(ids)` — removes entities, returns how many were actually removed.
    ///
    /// Unknown, stale and repeated ids are skipped, so the count is of entities removed, never of
    /// ids passed in.
    #[wasm_bindgen(js_name = despawn)]
    pub fn despawn(&mut self, ids: JsValue) -> Result<u32, JsValue> {
        let ids: Vec<EntityId> = serde_wasm_bindgen::from_value(ids)
            .map_err(|e| JsValue::from_str(&format!("invalid entity ids: {e}")))?;
        if ids.is_empty() {
            return Err(JsValue::from_str("despawn(ids) requires at least one id"));
        }
        let removed = self.api.despawn(&ids);
        u32::try_from(removed)
            .map_err(|_| JsValue::from_str("despawn removed more entities than u32 can hold"))
    }

    /// JS: `isAlive(id)` — `true` while the entity behind this `{index, generation}` is spawned.
    #[wasm_bindgen(js_name = isAlive)]
    pub fn is_alive(&self, id: JsValue) -> Result<bool, JsValue> {
        let id: EntityId = serde_wasm_bindgen::from_value(id)
            .map_err(|e| JsValue::from_str(&format!("invalid entity id: {e}")))?;
        Ok(self.api.is_alive(id))
    }

    /// JS: `createGroup(faction)` — a new empty group for that faction.
    #[wasm_bindgen(js_name = createGroup)]
    pub fn create_group(&mut self, faction: u32) -> Result<JsValue, JsValue> {
        let id = self.api.create_group(faction);
        serde_wasm_bindgen::to_value(&id)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize id: {e}")))
    }

    /// JS: `addToGroup(groupId, unitId)` — join, leaving whatever group the unit was in.
    #[wasm_bindgen(js_name = addToGroup)]
    pub fn add_to_group(&mut self, group: JsValue, unit: JsValue) -> Result<(), JsValue> {
        let group: EntityId = serde_wasm_bindgen::from_value(group)
            .map_err(|e| JsValue::from_str(&format!("invalid group id: {e}")))?;
        let unit: EntityId = serde_wasm_bindgen::from_value(unit)
            .map_err(|e| JsValue::from_str(&format!("invalid entity id: {e}")))?;
        self.api
            .add_to_group(group, unit)
            .map_err(|e: GroupError| JsValue::from_str(&e.to_string()))
    }

    /// JS: `removeFromGroup(unitId)` — `true` when it was in a group.
    #[wasm_bindgen(js_name = removeFromGroup)]
    pub fn remove_from_group(&mut self, unit: JsValue) -> Result<bool, JsValue> {
        let unit: EntityId = serde_wasm_bindgen::from_value(unit)
            .map_err(|e| JsValue::from_str(&format!("invalid entity id: {e}")))?;
        Ok(self.api.remove_from_group(unit))
    }

    /// JS: `groupOf(unitId)` — the unit's group, or `null`.
    #[wasm_bindgen(js_name = groupOf)]
    pub fn group_of(&self, unit: JsValue) -> Result<JsValue, JsValue> {
        let unit: EntityId = serde_wasm_bindgen::from_value(unit)
            .map_err(|e| JsValue::from_str(&format!("invalid entity id: {e}")))?;
        match self.api.group_of(unit) {
            Some(group) => serde_wasm_bindgen::to_value(&group)
                .map_err(|e| JsValue::from_str(&format!("failed to serialize id: {e}"))),
            None => Ok(JsValue::NULL),
        }
    }

    /// JS: `groupMembers(groupId)` — the group's live members.
    #[wasm_bindgen(js_name = groupMembers)]
    pub fn group_members(&mut self, group: JsValue) -> Result<JsValue, JsValue> {
        let group: EntityId = serde_wasm_bindgen::from_value(group)
            .map_err(|e| JsValue::from_str(&format!("invalid group id: {e}")))?;
        let members = self.api.group_members(group);
        serde_wasm_bindgen::to_value(&members)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize ids: {e}")))
    }

    /// JS: `orderGroupMoveTo(groupId, x, y)` — a manual order to the whole group.
    ///
    /// Returns how many members took it. The group is left under manual control, and members
    /// following a personal order keep it.
    #[wasm_bindgen(js_name = orderGroupMoveTo)]
    pub fn order_group_move_to(&mut self, group: JsValue, x: f64, y: f64) -> Result<u32, JsValue> {
        let group: EntityId = serde_wasm_bindgen::from_value(group)
            .map_err(|e| JsValue::from_str(&format!("invalid group id: {e}")))?;
        if !x.is_finite() || !y.is_finite() {
            return Err(JsValue::from_str(
                "orderGroupMoveTo(groupId, x, y) requires finite coordinates",
            ));
        }
        let target = move_target(x, y, "orderGroupMoveTo")?;
        let report = self
            .api
            .order_group_move_to(group, target)
            .map_err(|e: GroupError| JsValue::from_str(&e.to_string()))?;
        u32::try_from(report.ordered)
            .map_err(|_| JsValue::from_str("orderGroupMoveTo ordered more than u32 can hold"))
    }

    /// JS: `isGroupManual(groupId)`.
    #[wasm_bindgen(js_name = isGroupManual)]
    pub fn is_group_manual(&self, group: JsValue) -> Result<bool, JsValue> {
        let group: EntityId = serde_wasm_bindgen::from_value(group)
            .map_err(|e| JsValue::from_str(&format!("invalid group id: {e}")))?;
        Ok(self.api.is_group_manual(group))
    }

    /// JS: `clearGroupManual(groupId)` — hand the group back to automation.
    #[wasm_bindgen(js_name = clearGroupManual)]
    pub fn clear_group_manual(&mut self, group: JsValue) -> Result<bool, JsValue> {
        let group: EntityId = serde_wasm_bindgen::from_value(group)
            .map_err(|e| JsValue::from_str(&format!("invalid group id: {e}")))?;
        self.api
            .clear_group_manual(group)
            .map_err(|e: GroupError| JsValue::from_str(&e.to_string()))
    }

    /// JS: `createMission(x, y, radius)`.
    #[wasm_bindgen(js_name = createMission)]
    pub fn create_mission(&mut self, x: f64, y: f64, radius: f64) -> Result<JsValue, JsValue> {
        if !x.is_finite() || !y.is_finite() || !radius.is_finite() {
            return Err(JsValue::from_str(
                "createMission(x, y, radius) requires finite numbers",
            ));
        }
        let target = move_target(x, y, "createMission")?;
        let radius = units::to_milli(radius)
            .map_err(|e| JsValue::from_str(&format!("createMission: radius {e}")))?;
        let id = self.api.create_mission(target, radius);
        serde_wasm_bindgen::to_value(&id)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize id: {e}")))
    }

    /// JS: `assignGroup(missionId, groupId)` — send a group to a mission.
    #[wasm_bindgen(js_name = assignGroup)]
    pub fn assign_group(&mut self, mission: JsValue, group: JsValue) -> Result<(), JsValue> {
        let mission: EntityId = serde_wasm_bindgen::from_value(mission)
            .map_err(|e| JsValue::from_str(&format!("invalid mission id: {e}")))?;
        let group: EntityId = serde_wasm_bindgen::from_value(group)
            .map_err(|e| JsValue::from_str(&format!("invalid group id: {e}")))?;
        self.api
            .assign_group(mission, group)
            .map_err(|e: MissionError| JsValue::from_str(&e.to_string()))
    }

    /// JS: `missionOf(groupId)` — the mission the group is working, or `null`.
    #[wasm_bindgen(js_name = missionOf)]
    pub fn mission_of(&self, group: JsValue) -> Result<JsValue, JsValue> {
        let group: EntityId = serde_wasm_bindgen::from_value(group)
            .map_err(|e| JsValue::from_str(&format!("invalid group id: {e}")))?;
        match self.api.mission_of(group) {
            Some(mission) => serde_wasm_bindgen::to_value(&mission)
                .map_err(|e| JsValue::from_str(&format!("failed to serialize id: {e}"))),
            None => Ok(JsValue::NULL),
        }
    }

    /// JS: `isMissionCompleted(missionId)`.
    #[wasm_bindgen(js_name = isMissionCompleted)]
    pub fn is_mission_completed(&self, mission: JsValue) -> Result<bool, JsValue> {
        let mission: EntityId = serde_wasm_bindgen::from_value(mission)
            .map_err(|e| JsValue::from_str(&format!("invalid mission id: {e}")))?;
        Ok(self.api.is_mission_completed(mission))
    }

    /// JS: `board(unitId, vehicleId)` — put a unit inside a vehicle, from any distance.
    ///
    /// Fails when the vehicle has no seats or none are left. A passenger's move target is
    /// dropped: while aboard it goes where the vehicle goes.
    #[wasm_bindgen(js_name = board)]
    pub fn board(&mut self, unit: JsValue, vehicle: JsValue) -> Result<(), JsValue> {
        let unit: EntityId = serde_wasm_bindgen::from_value(unit)
            .map_err(|e| JsValue::from_str(&format!("invalid entity id: {e}")))?;
        let vehicle: EntityId = serde_wasm_bindgen::from_value(vehicle)
            .map_err(|e| JsValue::from_str(&format!("invalid vehicle id: {e}")))?;
        self.api
            .board(unit, vehicle)
            .map_err(|e: BoardError| JsValue::from_str(&e.to_string()))
    }

    /// JS: `orderBoard(unitIds, vehicleId)` — send units to walk to a vehicle and get in.
    ///
    /// The order the player gives. Each unit walks toward the vehicle tick by tick, following it
    /// if it drives off, and boards once within `BOARDING_RANGE`. Returns how many units took the
    /// order; passengers, immobile and unknown ids are skipped. Fails only when the vehicle has
    /// no seats at all. A unit that arrives to find no seat stays outside and its order is
    /// dropped — read `boardingTargetOf` / `approaching` to see who is still on the way.
    #[wasm_bindgen(js_name = orderBoard)]
    pub fn order_board(&mut self, units: JsValue, vehicle: JsValue) -> Result<u32, JsValue> {
        let units: Vec<EntityId> = serde_wasm_bindgen::from_value(units)
            .map_err(|e| JsValue::from_str(&format!("invalid entity ids: {e}")))?;
        if units.is_empty() {
            return Err(JsValue::from_str(
                "orderBoard(units, vehicle) requires at least one id",
            ));
        }
        let vehicle: EntityId = serde_wasm_bindgen::from_value(vehicle)
            .map_err(|e| JsValue::from_str(&format!("invalid vehicle id: {e}")))?;
        let report = self
            .api
            .order_board(&units, vehicle)
            .map_err(|e: BoardError| JsValue::from_str(&e.to_string()))?;
        u32::try_from(report.ordered)
            .map_err(|_| JsValue::from_str("orderBoard ordered more entities than u32 can hold"))
    }

    /// JS: `boardingTargetOf(unitId)` — the vehicle the unit is walking to board, or `null`.
    #[wasm_bindgen(js_name = boardingTargetOf)]
    pub fn boarding_target_of(&self, unit: JsValue) -> Result<JsValue, JsValue> {
        let unit: EntityId = serde_wasm_bindgen::from_value(unit)
            .map_err(|e| JsValue::from_str(&format!("invalid entity id: {e}")))?;
        match self.api.boarding_target_of(unit) {
            Some(vehicle) => serde_wasm_bindgen::to_value(&vehicle)
                .map_err(|e| JsValue::from_str(&format!("failed to serialize id: {e}"))),
            None => Ok(JsValue::NULL),
        }
    }

    /// JS: `approaching(vehicleId)` — who is on the way to board it.
    #[wasm_bindgen(js_name = approaching)]
    pub fn approaching(&mut self, vehicle: JsValue) -> Result<JsValue, JsValue> {
        let vehicle: EntityId = serde_wasm_bindgen::from_value(vehicle)
            .map_err(|e| JsValue::from_str(&format!("invalid vehicle id: {e}")))?;
        let units = self.api.approaching(vehicle);
        serde_wasm_bindgen::to_value(&units)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize ids: {e}")))
    }

    /// JS: `unboard(unitId)` — step off, beside the vehicle.
    #[wasm_bindgen(js_name = unboard)]
    pub fn unboard(&mut self, unit: JsValue) -> Result<(), JsValue> {
        let unit: EntityId = serde_wasm_bindgen::from_value(unit)
            .map_err(|e| JsValue::from_str(&format!("invalid entity id: {e}")))?;
        self.api
            .unboard(unit)
            .map_err(|e: BoardError| JsValue::from_str(&e.to_string()))
    }

    /// JS: `vehicleOf(unitId)` — what the unit is riding, or `null`.
    #[wasm_bindgen(js_name = vehicleOf)]
    pub fn vehicle_of(&self, unit: JsValue) -> Result<JsValue, JsValue> {
        let unit: EntityId = serde_wasm_bindgen::from_value(unit)
            .map_err(|e| JsValue::from_str(&format!("invalid entity id: {e}")))?;
        match self.api.vehicle_of(unit) {
            Some(vehicle) => serde_wasm_bindgen::to_value(&vehicle)
                .map_err(|e| JsValue::from_str(&format!("failed to serialize id: {e}"))),
            None => Ok(JsValue::NULL),
        }
    }

    /// JS: `passengers(vehicleId)` — who is aboard right now.
    #[wasm_bindgen(js_name = passengers)]
    pub fn passengers(&mut self, vehicle: JsValue) -> Result<JsValue, JsValue> {
        let vehicle: EntityId = serde_wasm_bindgen::from_value(vehicle)
            .map_err(|e| JsValue::from_str(&format!("invalid vehicle id: {e}")))?;
        let passengers = self.api.passengers(vehicle);
        serde_wasm_bindgen::to_value(&passengers)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize ids: {e}")))
    }

    /// JS: `freeSeats(vehicleId)` — seats still empty, or `null` when the entity has no seats.
    #[wasm_bindgen(js_name = freeSeats)]
    pub fn free_seats(&mut self, vehicle: JsValue) -> Result<JsValue, JsValue> {
        let vehicle: EntityId = serde_wasm_bindgen::from_value(vehicle)
            .map_err(|e| JsValue::from_str(&format!("invalid vehicle id: {e}")))?;
        match self.api.free_seats(vehicle) {
            Some(seats) => Ok(JsValue::from_f64(f64::from(seats))),
            None => Ok(JsValue::NULL),
        }
    }

    /// JS: `submit(command)` — queue a command for the next tick; returns its sequence number.
    ///
    /// Nothing changes until the `step()` that produces the next tick; that step's report carries
    /// the outcome under the same `seq`. This is how a networked host gives orders: the immediate
    /// methods above change the world on the spot and have no place in a lockstep match.
    #[wasm_bindgen(js_name = submit)]
    pub fn submit(&mut self, command: JsValue) -> Result<f64, JsValue> {
        let command = command_from_js(command, "submit")?;
        Ok(seq_to_js(self.api.submit(command)))
    }

    /// JS: `schedule(tick, command)` — queue a command for a later tick; returns its sequence
    /// number. Throws when `tick` is not after `currentTick()`.
    #[wasm_bindgen(js_name = schedule)]
    pub fn schedule(&mut self, tick: f64, command: JsValue) -> Result<f64, JsValue> {
        if !(tick.is_finite() && tick >= 0.0 && tick.fract() == 0.0) {
            return Err(JsValue::from_str(
                "schedule(tick, command) requires a whole, non-negative tick",
            ));
        }
        let command = command_from_js(command, "schedule")?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // checked above
        let tick = tick as u64;
        self.api
            .schedule(tick, command)
            .map(seq_to_js)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// JS: `step()` — advances exactly one tick of [`tickMs`](tick_ms) milliseconds.
    ///
    /// Returns `{ tick, outcomes }`: one outcome per command applied at the start of the step,
    /// in `seq` order.
    #[wasm_bindgen(js_name = step)]
    ///
    /// # Panics
    ///
    /// Never in practice: the report holds ids and counts, and the tick fits a JS number for
    /// fourteen million years at 20 Hz.
    pub fn step(&mut self) -> JsValue {
        JsStepReport::from(self.api.step())
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .expect("a step report serializes")
    }

    /// JS: `stateHash()` — the simulation state hash as 16 lowercase hex digits.
    ///
    /// Equal on every platform for the same state; peers compare it to detect a desync.
    #[wasm_bindgen(js_name = stateHash)]
    #[must_use]
    pub fn state_hash(&self) -> String {
        format!("{:016x}", self.api.state_hash())
    }

    /// JS: `writeFrame()` — positions of the current tick as an `Int32Array`.
    ///
    /// Header `[tick_lo, tick_hi, count]`, then per entity `[index, generation, x, y]` in
    /// milli-units, sorted by `index`; every entity with a position has a row. The array is the
    /// caller's own copy, so a worker can post its buffer as a transferable. This is the per-frame
    /// path: no JSON, nothing to parse.
    #[wasm_bindgen(js_name = writeFrame)]
    pub fn write_frame(&mut self) -> Vec<i32> {
        let mut out = Vec::with_capacity(self.frame_len);
        self.api.write_frame(&mut out);
        self.frame_len = out.len();
        out
    }

    /// JS: `metaDelta()` — JSON of the metadata that changed since the previous call, or
    /// `undefined` when nothing did.
    ///
    /// `{ changed: [{ id, entity_type?, faction?, seats?, mobile, aboard?, boarding?, group?,
    /// move_target? }], removed: [id] }`, rows sorted by id, points in map units. The first call
    /// reports every positioned entity. Nothing is serialised when nothing changed, so calling it
    /// every frame costs no JSON.
    #[wasm_bindgen(js_name = metaDelta)]
    ///
    /// # Panics
    ///
    /// Never in practice: the delta holds ids, strings and numbers.
    pub fn meta_delta(&mut self) -> Option<String> {
        let delta = self.api.meta_delta();
        (!delta.is_empty())
            .then(|| serde_json::to_string(&delta).expect("a metadata delta serializes"))
    }

    /// JS: `currentTick()` — ticks advanced since the simulation was created.
    #[wasm_bindgen(js_name = currentTick)]
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // exact below 2^53 ticks, ~14 million years at 20 Hz
    pub fn current_tick(&self) -> f64 {
        self.api.current_tick() as f64
    }
}

/// JS: `tickMs()` — length of one simulation tick in milliseconds.
#[wasm_bindgen(js_name = tickMs)]
#[must_use]
pub fn tick_ms() -> u32 {
    open_entities::TICK_MS
}

#[cfg(test)]
mod wasm_tests {
    use super::*;
    use open_entities::EntityComponents;
    use open_entities::components::{BaseMoveSpeed, Boardable, Health, Position};
    use wasm_bindgen_test::*;

    const FIXTURE_YAML: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/spawn_entity_templates.yaml"
    ));

    fn empty_overrides() -> JsValue {
        serde_wasm_bindgen::to_value(&EntityComponents::default()).expect("empty overrides")
    }

    fn scout_overrides() -> JsValue {
        serde_wasm_bindgen::to_value(&EntityComponents {
            position: Some(Position::from_units(50.0, 25.0)),
            health: Some(Health {
                current: 40,
                max: 100,
            }),
            ..Default::default()
        })
        .expect("scout overrides")
    }

    fn err_string(result: Result<JsValue, JsValue>) -> String {
        match result {
            Err(e) => e.as_string().expect("JsValue error should be a string"),
            Ok(_) => panic!("expected error"),
        }
    }

    #[wasm_bindgen_test]
    fn load_and_spawn_from_fixture() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML).expect("load fixture");
        sim.spawn_entity("marker", empty_overrides())
            .expect("spawn marker");
        let json = sim.world_json().expect("export world");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["version"], 5);
        let entities = value["entities"].as_array().expect("entities array");
        assert!(!entities.is_empty());
    }

    #[wasm_bindgen_test]
    fn spawn_scout_with_overrides() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML).expect("load fixture");
        sim.spawn_entity("scout", scout_overrides())
            .expect("spawn scout");
        let json = sim.world_json().expect("export world");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        let entities = value["entities"].as_array().expect("entities array");
        let scout = entities
            .iter()
            .find(|e| e["entity_type"] == "scout")
            .expect("scout row in export");
        assert_eq!(scout["position"]["x"], 50.0);
        assert_eq!(scout["position"]["y"], 25.0);
        assert_eq!(scout["health"]["current"], 40);
        assert_eq!(scout["health"]["max"], 100);
    }

    #[wasm_bindgen_test]
    fn spawn_without_load_fails() {
        let mut sim = Simulation::new();
        let msg = err_string(sim.spawn_entity("marker", empty_overrides()));
        assert!(
            msg.contains("templates not loaded"),
            "expected TemplatesNotLoaded message, got: {msg}"
        );
    }

    #[wasm_bindgen_test]
    fn step_advances_scout() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML).expect("load fixture");
        sim.spawn_entity("scout", scout_overrides())
            .expect("spawn scout");

        let before = sim.world_json().expect("export before");
        let before_val: serde_json::Value = serde_json::from_str(&before).expect("parse JSON");
        let scout_before = before_val["entities"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["entity_type"] == "scout")
            .expect("scout row");
        let x0 = scout_before["position"]["x"].as_f64().unwrap();

        // One second of simulation.
        for _ in 0..20 {
            sim.step();
        }

        let after = sim.world_json().expect("export after");
        let after_val: serde_json::Value = serde_json::from_str(&after).expect("parse JSON");
        let scout_after = after_val["entities"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["entity_type"] == "scout")
            .expect("scout row");
        let x1 = scout_after["position"]["x"].as_f64().unwrap();

        assert_ne!(x0, x1, "position should change after ticks");
    }

    #[wasm_bindgen_test]
    fn step_counts_ticks() {
        let mut sim = Simulation::new();
        assert_eq!(sim.current_tick(), 0.0);
        sim.step();
        sim.step();
        assert_eq!(sim.current_tick(), 2.0);
        let value: serde_json::Value =
            serde_json::from_str(&sim.world_json().expect("export")).expect("parse JSON");
        assert_eq!(value["version"], 5);
        assert_eq!(value["tick"], 2);
    }

    #[wasm_bindgen_test]
    fn order_move_to_moves_a_spawned_entity() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML).expect("load fixture");
        let spawned = sim
            .spawn_entity("scout", empty_overrides())
            .expect("spawn scout");
        let id: EntityId = serde_wasm_bindgen::from_value(spawned).expect("id");

        let ids = serde_wasm_bindgen::to_value(&vec![id]).expect("ids");
        // Scout starts at (10, 5) with base_move_speed 2.0: two units take about one second.
        let ordered = sim.order_move_to(ids, 10.0, 7.0).expect("order accepted");
        assert_eq!(ordered, 1);

        // Just over three seconds.
        for _ in 0..64 {
            sim.step();
        }

        let json = sim.world_json().expect("export world");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        let scout = value["entities"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["entity_type"] == "scout")
            .expect("scout row");
        assert!((scout["position"]["x"].as_f64().unwrap() - 10.0).abs() < 0.1);
        assert!((scout["position"]["y"].as_f64().unwrap() - 7.0).abs() < 0.1);
    }

    const MAP_YAML: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/init_map.yaml"
    ));

    #[wasm_bindgen_test]
    fn load_map_returns_ids_and_bounds() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML).expect("load fixture");

        let ids = sim.load_map_yaml(MAP_YAML).expect("load map");
        let ids: Vec<serde_json::Value> =
            serde_wasm_bindgen::from_value(ids).expect("ids deserialize");
        assert_eq!(ids.len(), 3);
        assert!(ids[0]["index"].is_number());
        assert!(ids[0]["generation"].is_number());

        let bounds = sim.map_bounds().expect("bounds");
        let bounds: serde_json::Value =
            serde_wasm_bindgen::from_value(bounds).expect("bounds deserialize");
        assert_eq!(bounds["width"], 200.0);
        assert_eq!(bounds["height"], 200.0);
    }

    #[wasm_bindgen_test]
    fn despawned_entity_leaves_the_export_and_stops_resolving() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML).expect("load fixture");
        let spawned = sim
            .spawn_entity("marker", empty_overrides())
            .expect("spawn marker");
        let id: EntityId = serde_wasm_bindgen::from_value(spawned).expect("id");
        let id_js = serde_wasm_bindgen::to_value(&id).expect("id");
        let ids_js = serde_wasm_bindgen::to_value(&vec![id]).expect("ids");

        assert!(sim.is_alive(id_js.clone()).expect("alive check"));
        assert_eq!(sim.despawn(ids_js).expect("despawn"), 1);
        assert!(!sim.is_alive(id_js).expect("alive check"));

        let json = sim.world_json().expect("export world");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["entities"].as_array().map(Vec::len), Some(0));
    }

    #[wasm_bindgen_test]
    fn map_bounds_is_null_before_any_map() {
        let sim = Simulation::new();
        assert!(sim.map_bounds().expect("bounds call").is_null());
    }

    #[wasm_bindgen_test]
    fn order_move_to_rejects_empty_ids() {
        let mut sim = Simulation::new();
        let ids = serde_wasm_bindgen::to_value(&Vec::<EntityId>::new()).expect("ids");
        let err = sim.order_move_to(ids, 1.0, 1.0).unwrap_err();
        let msg = err.as_string().expect("string error");
        assert!(
            msg.contains("at least one id"),
            "expected empty-ids validation error, got: {msg}"
        );
    }

    #[wasm_bindgen_test]
    fn a_passenger_rides_along_and_steps_off_beside_the_vehicle() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML).expect("load fixture");

        // The scout already drives toward (20, 0); four seats make it a vehicle.
        let vehicle = sim
            .spawn_entity(
                "scout",
                serde_wasm_bindgen::to_value(&EntityComponents {
                    boardable: Some(Boardable(4)),
                    ..Default::default()
                })
                .expect("vehicle overrides"),
            )
            .expect("spawn vehicle");
        // Standing half a unit from the scout's (10, 5).
        let rider = sim
            .spawn_entity(
                "marker",
                serde_wasm_bindgen::to_value(&EntityComponents {
                    position: Some(Position::from_units(10.5, 5.0)),
                    ..Default::default()
                })
                .expect("rider overrides"),
            )
            .expect("spawn rider");

        assert_eq!(
            free_seats_of(&mut sim, vehicle.clone()),
            Some(4.0),
            "an empty vehicle has every seat free"
        );
        sim.board(rider.clone(), vehicle.clone())
            .expect("the rider is close enough to board");
        assert_eq!(free_seats_of(&mut sim, vehicle.clone()), Some(3.0));

        // One second of simulation.
        for _ in 0..20 {
            sim.step();
        }

        let vehicle_id: EntityId = serde_wasm_bindgen::from_value(vehicle.clone()).expect("id");
        let rider_id: EntityId = serde_wasm_bindgen::from_value(rider.clone()).expect("id");
        let moved = position_of(&mut sim, vehicle_id);
        assert_ne!(moved.0, 10.0, "the vehicle should have driven off");
        assert_eq!(
            position_of(&mut sim, rider_id),
            moved,
            "a passenger is wherever its vehicle is"
        );

        sim.unboard(rider.clone()).expect("step off");
        assert_eq!(free_seats_of(&mut sim, vehicle), Some(4.0));
        assert_ne!(
            position_of(&mut sim, rider_id),
            moved,
            "stepping off puts the unit beside the vehicle, not inside it"
        );
    }

    /// Seats left on `vehicle`, as a plain number, or `None` when it carries no seats.
    fn free_seats_of(sim: &mut Simulation, vehicle: JsValue) -> Option<f64> {
        sim.free_seats(vehicle).expect("free seats call").as_f64()
    }

    /// The entity's exported position, which is the only place the demo reads one from.
    fn position_of(sim: &mut Simulation, id: EntityId) -> (f64, f64) {
        let json = sim.world_json().expect("export world");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        let row = value["entities"]
            .as_array()
            .expect("entities array")
            .iter()
            .find(|row| row["id"]["index"] == id.index && row["id"]["generation"] == id.generation)
            .expect("row for the requested id");
        (
            row["position"]["x"].as_f64().expect("x"),
            row["position"]["y"].as_f64().expect("y"),
        )
    }

    #[wasm_bindgen_test]
    fn an_order_to_board_walks_the_unit_over_first() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML).expect("load fixture");

        // A parked truck: a marker with seats. It has no speed, so it stays put.
        let vehicle = sim
            .spawn_entity(
                "marker",
                serde_wasm_bindgen::to_value(&EntityComponents {
                    position: Some(Position { x: 0, y: 0 }),
                    boardable: Some(Boardable(4)),
                    ..Default::default()
                })
                .expect("vehicle overrides"),
            )
            .expect("spawn vehicle");
        // A unit that can walk, well outside boarding range.
        let rider = sim
            .spawn_entity(
                "marker",
                serde_wasm_bindgen::to_value(&EntityComponents {
                    position: Some(Position::from_units(30.0, 0.0)),
                    base_move_speed: Some(BaseMoveSpeed(500)), // 10 units/s
                    ..Default::default()
                })
                .expect("rider overrides"),
            )
            .expect("spawn rider");
        let units = serde_wasm_bindgen::to_value(&[serde_wasm_bindgen::from_value::<EntityId>(
            rider.clone(),
        )
        .expect("id")])
        .expect("ids");

        let ordered = sim
            .order_board(units, vehicle.clone())
            .expect("the order is accepted");
        assert_eq!(ordered, 1);
        assert!(
            !sim.boarding_target_of(rider.clone())
                .expect("call")
                .is_null(),
            "the unit is on its way"
        );
        assert_eq!(
            free_seats_of(&mut sim, vehicle.clone()),
            Some(4.0),
            "not aboard yet"
        );

        for _ in 0..200 {
            sim.step();
        }

        assert_eq!(
            free_seats_of(&mut sim, vehicle.clone()),
            Some(3.0),
            "boarded on arrival"
        );
        assert!(
            sim.boarding_target_of(rider).expect("call").is_null(),
            "the order is spent"
        );
        let approaching: Vec<EntityId> =
            serde_wasm_bindgen::from_value(sim.approaching(vehicle).expect("call")).expect("ids");
        assert!(approaching.is_empty());
    }

    #[wasm_bindgen_test]
    fn boarding_a_rock_fails() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML).expect("load fixture");
        let rock = sim
            .spawn_entity(
                "marker",
                serde_wasm_bindgen::to_value(&EntityComponents {
                    position: Some(Position { x: 0, y: 0 }),
                    ..Default::default()
                })
                .expect("rock overrides"),
            )
            .expect("spawn rock");
        let unit = sim
            .spawn_entity(
                "marker",
                serde_wasm_bindgen::to_value(&EntityComponents {
                    position: Some(Position::from_units(0.5, 0.0)),
                    ..Default::default()
                })
                .expect("unit overrides"),
            )
            .expect("spawn unit");

        assert!(
            sim.free_seats(rock.clone())
                .expect("free seats call")
                .is_null(),
            "something with no seats reports none, rather than zero"
        );
        let err = sim.board(unit, rock).unwrap_err();
        let msg = err.as_string().expect("string error");
        assert!(
            msg.contains("no seats to board"),
            "expected NotBoardable message, got: {msg}"
        );
    }

    const GOLDEN_REPLAY: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/replays/basic.json"
    ));
    const GOLDEN_HASH: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/replays/basic.hash"
    ));

    /// The golden replay under wasm32: the same hash the native runners pin. A mismatch here is
    /// a determinism bug in the core, not something to fix in this test.
    #[wasm_bindgen_test]
    fn golden_replay_hash_matches_under_wasm() {
        let replay = open_entities::Replay::from_json(GOLDEN_REPLAY).expect("golden replay parses");
        let api = replay.run(600).expect("replay runs");
        assert_eq!(format!("{:016x}", api.state_hash()), GOLDEN_HASH.trim());
    }

    fn report(value: JsValue) -> serde_json::Value {
        serde_wasm_bindgen::from_value(value).expect("step report")
    }

    /// A sequence number as JSON compares it: an integer, not the `f64` JS hands back.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn seq(value: f64) -> u64 {
        value as u64
    }

    fn command(json: serde_json::Value) -> JsValue {
        json.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .expect("command")
    }

    #[wasm_bindgen_test]
    fn a_submitted_command_comes_back_as_an_outcome_of_the_next_step() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML).expect("load fixture");

        let spawn = sim
            .submit(command(serde_json::json!({
                "type": "spawn",
                "template": "scout",
                "overrides": { "position": { "x": 3, "y": 4.5 } }
            })))
            .expect("submit");
        assert_eq!(
            report(sim.step()),
            serde_json::json!({
                "tick": 1,
                "outcomes": [{ "seq": seq(spawn), "ok": true, "spawned": { "index": 4, "generation": 0 } }]
            })
        );

        let moved = sim
            .submit(command(serde_json::json!({
                "type": "move_to",
                "ids": [{ "index": 4, "generation": 0 }, { "index": 99, "generation": 0 }],
                "target": { "x": 10, "y": 10 }
            })))
            .expect("submit");
        let refused = sim
            .submit(command(serde_json::json!({
                "type": "add_to_group",
                "group": { "index": 4, "generation": 0 },
                "unit": { "index": 4, "generation": 0 }
            })))
            .expect("submit");
        let outcomes = report(sim.step())["outcomes"].clone();
        assert_eq!(
            outcomes,
            serde_json::json!([
                { "seq": seq(moved), "ok": true, "applied": 1, "skipped": 1 },
                { "seq": seq(refused), "ok": false, "error": "no group with id 4:0" },
            ])
        );
    }

    #[wasm_bindgen_test]
    fn a_malformed_command_is_refused_at_submit() {
        let mut sim = Simulation::new();
        let err = sim
            .submit(command(serde_json::json!({ "type": "teleport" })))
            .unwrap_err();
        let msg = err.as_string().expect("string error");
        assert!(msg.contains("invalid command"), "got: {msg}");
    }

    #[wasm_bindgen_test]
    fn schedule_refuses_the_past_and_state_hash_is_hex() {
        let mut sim = Simulation::new();
        sim.step();
        let past = sim
            .schedule(
                1.0,
                command(serde_json::json!({ "type": "create_group", "faction": 1 })),
            )
            .unwrap_err();
        assert!(
            past.as_string()
                .expect("string")
                .contains("cannot schedule for tick 1")
        );
        assert!(
            sim.schedule(
                2.0,
                command(serde_json::json!({ "type": "create_group", "faction": 1 }))
            )
            .is_ok()
        );

        let hash = sim.state_hash();
        assert_eq!(hash.len(), 16);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[wasm_bindgen_test]
    fn write_frame_is_a_header_and_rows_sorted_by_index() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML).expect("load fixture");
        sim.spawn_entity("scout", scout_overrides())
            .expect("spawn scout");
        sim.spawn_entity("heavy_tank", empty_overrides())
            .expect("spawn tank");
        sim.step();

        let frame = sim.write_frame();
        assert_eq!(&frame[..3], &[1, 0, 2]);
        assert_eq!(frame.len(), 3 + 2 * 4);
        let export: serde_json::Value =
            serde_json::from_str(&sim.world_json().expect("export")).expect("JSON");
        let rows: Vec<&[i32]> = frame[3..].chunks(4).collect();
        assert!(rows[0][0] < rows[1][0], "rows sorted by index");
        for row in rows {
            let entity = export["entities"]
                .as_array()
                .expect("entities")
                .iter()
                .find(|e| e["id"]["index"] == row[0] && e["id"]["generation"] == row[1])
                .expect("every row is an exported entity");
            let milli = |v: &serde_json::Value| {
                let units = v.as_f64().expect("a number");
                #[allow(clippy::cast_possible_truncation)]
                let milli = (units * 1000.0).round() as i32;
                milli
            };
            assert_eq!(row[2], milli(&entity["position"]["x"]));
            assert_eq!(row[3], milli(&entity["position"]["y"]));
        }
    }

    #[wasm_bindgen_test]
    fn meta_delta_is_json_once_and_then_nothing_until_a_change() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML).expect("load fixture");
        let tank = sim
            .spawn_entity("heavy_tank", empty_overrides())
            .expect("spawn tank");

        let first: serde_json::Value =
            serde_json::from_str(&sim.meta_delta().expect("first delta")).expect("JSON");
        assert_eq!(first["changed"][0]["entity_type"], "heavy_tank");
        assert_eq!(first["changed"][0]["faction"], 3);
        assert_eq!(first["removed"], serde_json::json!([]));

        sim.step();
        assert_eq!(sim.meta_delta(), None, "no change, no JSON");

        sim.despawn(serde_wasm_bindgen::to_value(&[tank_id(&tank)]).expect("ids"))
            .expect("despawn");
        let gone: serde_json::Value =
            serde_json::from_str(&sim.meta_delta().expect("removal delta")).expect("JSON");
        assert_eq!(
            gone["removed"][0],
            serde_json::to_value(tank_id(&tank)).expect("id")
        );
    }

    fn tank_id(value: &JsValue) -> EntityId {
        serde_wasm_bindgen::from_value(value.clone()).expect("an id")
    }

    #[wasm_bindgen_test]
    fn unknown_template_fails() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML).expect("load fixture");
        let msg = err_string(sim.spawn_entity("nope", empty_overrides()));
        assert!(
            msg.contains("unknown template name: nope"),
            "expected UnknownTemplate message, got: {msg}"
        );
    }
}
