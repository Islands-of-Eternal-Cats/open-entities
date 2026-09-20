use open_entities::components::MoveTarget;
use open_entities::{Api, EntityComponents, EntityId, ExportError, ImportError, hello};
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
    use open_entities::components::{Health, Position};
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
        assert_eq!(value["version"], 3);
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
