use open_entities::{hello, Api, EntityComponents, ExportError, ImportError};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct SpawnedEntity {
    index: u32,
    generation: u32,
}

#[wasm_bindgen]
impl SpawnedEntity {
    #[wasm_bindgen(getter)]
    pub fn index(&self) -> u32 {
        self.index
    }

    #[wasm_bindgen(getter)]
    pub fn generation(&self) -> u32 {
        self.generation
    }
}

#[wasm_bindgen]
pub struct Simulation {
    api: Api,
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

    /// JS: `spawnEntity(templateName, overrides)`
    #[wasm_bindgen(js_name = spawnEntity)]
    pub fn spawn_entity(
        &mut self,
        template_name: &str,
        overrides: JsValue,
    ) -> Result<SpawnedEntity, JsValue> {
        let overrides: EntityComponents = serde_wasm_bindgen::from_value(overrides)
            .map_err(|e| JsValue::from_str(&format!("invalid overrides: {e}")))?;
        let entity = self
            .api
            .spawn_entity(template_name, overrides)
            .map_err(|e: ImportError| JsValue::from_str(&e.to_string()))?;
        Ok(SpawnedEntity {
            index: entity.index_u32(),
            generation: entity.generation().to_bits(),
        })
    }

    /// JS: `getWorldAsJson()`
    #[wasm_bindgen(js_name = getWorldAsJson)]
    pub fn world_json(&mut self) -> Result<String, JsValue> {
        self.api
            .world_json()
            .map_err(|e: ExportError| JsValue::from_str(&e.to_string()))
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
    use open_entities::components::{Health, Position};
    use open_entities::EntityComponents;
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

    fn err_string(result: Result<SpawnedEntity, JsValue>) -> String {
        match result {
            Err(e) => e.as_string().expect("JsValue error should be a string"),
            Ok(_) => panic!("expected error"),
        }
    }

    #[wasm_bindgen_test]
    fn load_and_spawn_from_fixture() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML)
            .expect("load fixture");
        sim.spawn_entity("marker", empty_overrides())
            .expect("spawn marker");
        let json = sim.world_json().expect("export world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["version"], 3);
        let entities = value["entities"]
            .as_array()
            .expect("entities array");
        assert!(!entities.is_empty());
    }

    #[wasm_bindgen_test]
    fn spawn_scout_with_overrides() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML)
            .expect("load fixture");
        sim.spawn_entity("scout", scout_overrides())
            .expect("spawn scout");
        let json = sim.world_json().expect("export world");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("valid JSON");
        let entities = value["entities"]
            .as_array()
            .expect("entities array");
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
        sim.load_templates_yaml(FIXTURE_YAML)
            .expect("load fixture");
        sim.spawn_entity("scout", scout_overrides())
            .expect("spawn scout");

        let before = sim.world_json().expect("export before");
        let before_val: serde_json::Value =
            serde_json::from_str(&before).expect("parse JSON");
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
        let after_val: serde_json::Value =
            serde_json::from_str(&after).expect("parse JSON");
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
    fn unknown_template_fails() {
        let mut sim = Simulation::new();
        sim.load_templates_yaml(FIXTURE_YAML)
            .expect("load fixture");
        let msg = err_string(sim.spawn_entity("nope", empty_overrides()));
        assert!(
            msg.contains("unknown template name: nope"),
            "expected UnknownTemplate message, got: {msg}"
        );
    }
}
