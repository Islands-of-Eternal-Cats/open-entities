use open_entities::{hello, Api, ExportError};
use wasm_bindgen::prelude::*;

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

    /// Serializes the ECS world to JSON (schema version 3).
    pub fn world_json(&mut self) -> Result<String, JsValue> {
        self.api
            .world_json()
            .map_err(|e: ExportError| JsValue::from_str(&e.to_string()))
    }
}
