use open_entities::components::MoveTarget;
use open_entities::{Api, EntityComponents, EntityId, ExportError, ImportError, hello};
use open_entities::{BoardError, GroupError, MissionError};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Simulation {
    api: Api,
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
        Self { api: Api::new() }
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

    /// JS: `getWorldAsJson()`
    #[wasm_bindgen(js_name = getWorldAsJson)]
    pub fn world_json(&mut self) -> Result<String, JsValue> {
        self.api
            .world_json()
            .map_err(|e: ExportError| JsValue::from_str(&e.to_string()))
    }

    /// JS: `orderMoveTo(ids, x, y)` — move order for a group of entities.
    ///
    /// `ids` is an array of `{index, generation}` objects, exactly as `getWorldAsJson()` reports
    /// each entity's `id`. Returns how many entities took the order; immobile and unknown ids are
    /// skipped. Destinations are spread over a grid so a group does not pile onto one point.
    #[wasm_bindgen(js_name = orderMoveTo)]
    pub fn order_move_to(&mut self, ids: JsValue, x: f32, y: f32) -> Result<u32, JsValue> {
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
        let report = self.api.order_move_to(&ids, MoveTarget { x, y });
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
    pub fn order_group_move_to(&mut self, group: JsValue, x: f32, y: f32) -> Result<u32, JsValue> {
        let group: EntityId = serde_wasm_bindgen::from_value(group)
            .map_err(|e| JsValue::from_str(&format!("invalid group id: {e}")))?;
        if !x.is_finite() || !y.is_finite() {
            return Err(JsValue::from_str(
                "orderGroupMoveTo(groupId, x, y) requires finite coordinates",
            ));
        }
        let report = self
            .api
            .order_group_move_to(group, MoveTarget { x, y })
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
    pub fn create_mission(&mut self, x: f32, y: f32, radius: f32) -> Result<JsValue, JsValue> {
        if !x.is_finite() || !y.is_finite() || !radius.is_finite() {
            return Err(JsValue::from_str(
                "createMission(x, y, radius) requires finite numbers",
            ));
        }
        let id = self.api.create_mission(MoveTarget { x, y }, radius);
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

    /// JS: `tick(dtMs)` — positive integer milliseconds only.
    #[wasm_bindgen(js_name = tick)]
    pub fn tick(&mut self, dt_ms: f64) -> Result<(), JsValue> {
        if !dt_ms.is_finite() || dt_ms <= 0.0 || dt_ms.fract() != 0.0 {
            return Err(JsValue::from_str(
                "tick(dtMs) requires a positive finite integer",
            ));
        }
        if dt_ms > f64::from(u32::MAX) {
            return Err(JsValue::from_str("tick(dtMs) exceeds u32::MAX"));
        }
        let dt_ms = dt_ms as u32;
        self.api
            .tick(dt_ms)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
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
            position: Some(Position { x: 50.0, y: 25.0 }),
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
        assert_eq!(value["version"], 4);
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
    fn tick_advances_scout() {
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

        for _ in 0..60 {
            sim.tick(16.0).expect("tick");
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
    fn tick_zero_rejected() {
        let mut sim = Simulation::new();
        let err = sim.tick(0.0).unwrap_err();
        let msg = err.as_string().expect("string error");
        assert!(
            msg.contains("positive finite integer"),
            "expected JS validation error for tick(0), got: {msg}"
        );
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

        for _ in 0..200 {
            sim.tick(16.0).expect("tick");
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
                    position: Some(Position { x: 10.5, y: 5.0 }),
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

        for _ in 0..60 {
            sim.tick(16.0).expect("tick");
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
                    position: Some(Position { x: 0.0, y: 0.0 }),
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
                    position: Some(Position { x: 30.0, y: 0.0 }),
                    base_move_speed: Some(BaseMoveSpeed(10.0)),
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
            sim.tick(50.0).expect("tick");
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
                    position: Some(Position { x: 0.0, y: 0.0 }),
                    ..Default::default()
                })
                .expect("rock overrides"),
            )
            .expect("spawn rock");
        let unit = sim
            .spawn_entity(
                "marker",
                serde_wasm_bindgen::to_value(&EntityComponents {
                    position: Some(Position { x: 0.5, y: 0.0 }),
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
