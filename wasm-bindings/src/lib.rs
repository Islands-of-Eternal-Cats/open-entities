//! WASM bindings for open-entities library
//!
//! This crate provides WebAssembly bindings to use open-entities
//! from JavaScript projects.

use js_sys::Array;
use open_entities::{
    LoadError, Position, Schedule, SpawnError, Velocity, World, create_world_with_definitions,
    get_entities, run_tick, spawn_entity_by_type_in_world,
};
use wasm_bindgen::prelude::*;

fn format_init_error(err: &LoadError) -> String {
    format!("init_world failed: {}", err)
}

fn format_spawn_error(err: &SpawnError) -> String {
    format!("spawn failed: {}", err)
}

/// Initialize the WASM environment
#[wasm_bindgen(start)]
pub fn wasm_init() {
    console_error_panic_hook::set_once();
}

/// A JavaScript-compatible wrapper for Position
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct JsPosition {
    position: Position,
}

#[wasm_bindgen]
impl JsPosition {
    #[wasm_bindgen(constructor)]
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            position: Position { x, y },
        }
    }

    pub fn x(&self) -> f32 {
        self.position.x
    }

    pub fn set_x(&mut self, x: f32) {
        self.position.x = x;
    }

    pub fn y(&self) -> f32 {
        self.position.y
    }

    pub fn set_y(&mut self, y: f32) {
        self.position.y = y;
    }
}

/// A JavaScript-compatible wrapper for Velocity
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct JsVelocity {
    velocity: Velocity,
}

#[wasm_bindgen]
impl JsVelocity {
    #[wasm_bindgen(constructor)]
    pub fn new(vx: f32, vy: f32) -> Self {
        Self {
            velocity: Velocity { vx, vy },
        }
    }

    pub fn vx(&self) -> f32 {
        self.velocity.vx
    }

    pub fn set_vx(&mut self, vx: f32) {
        self.velocity.vx = vx;
    }

    pub fn vy(&self) -> f32 {
        self.velocity.vy
    }

    pub fn set_vy(&mut self, vy: f32) {
        self.velocity.vy = vy;
    }
}

/// Move an entity's position based on its velocity for one tick (legacy helper).
/// Prefer using `JsWorld` and `tick(dt)` for time-based simulation.
#[wasm_bindgen]
pub fn move_position(pos: &JsPosition, vel: &JsVelocity) -> JsPosition {
    let new_x = pos.position.x + vel.velocity.vx;
    let new_y = pos.position.y + vel.velocity.vy;

    JsPosition {
        position: Position { x: new_x, y: new_y },
    }
}

/// ECS world for use from JavaScript. Holds entities and runs simulation ticks with delta time.
#[wasm_bindgen]
pub struct JsWorld {
    world: World,
    schedule: Schedule,
}

#[wasm_bindgen]
impl JsWorld {
    /// Create a world from required entity definitions YAML
    /// (e.g. content of assets/entities.yaml).
    #[wasm_bindgen(constructor)]
    pub fn new(entities_yaml: String) -> Result<JsWorld, JsValue> {
        let (world, schedule) = create_world_with_definitions(&entities_yaml)
            .map_err(|e: LoadError| JsValue::from_str(&format_init_error(&e)))?;
        Ok(JsWorld { world, schedule })
    }

    /// Spawn an entity by type name from the loaded definitions (from assets/entities.yaml).
    /// Returns error if definitions were not loaded or the type name is unknown.
    #[wasm_bindgen]
    pub fn spawn(&mut self, type_name: &str) -> Result<(), JsValue> {
        spawn_entity_by_type_in_world(&mut self.world, type_name)
            .map(|_| ())
            .map_err(|e| JsValue::from_str(&format_spawn_error(&e)))
    }

    /// Run one simulation tick with the given delta time in seconds.
    #[wasm_bindgen]
    pub fn tick(&mut self, dt: f32) {
        run_tick(&mut self.world, &mut self.schedule, dt);
    }

    /// Snapshot of all entities as an array of `{ id, pos: { x, y }, velocity: { vx, vy } | null }` for rendering.
    /// Entities with only Position (static) have `velocity: null`; moving entities have velocity.
    /// `id` is a stable entity identifier (Entity::to_bits) so the same entity keeps the same id across frames.
    #[wasm_bindgen]
    pub fn get_entities(&mut self) -> Array {
        let snapshot = get_entities(&mut self.world);
        let arr = Array::new();
        for (id_bits, pos, vel_opt) in snapshot {
            let pos_obj = js_sys::Object::new();
            js_sys::Reflect::set(
                &pos_obj,
                &JsValue::from_str("x"),
                &JsValue::from_f64(pos.x as f64),
            )
            .unwrap();
            js_sys::Reflect::set(
                &pos_obj,
                &JsValue::from_str("y"),
                &JsValue::from_f64(pos.y as f64),
            )
            .unwrap();
            let vel_js = match vel_opt {
                Some(vel) => {
                    let vel_obj = js_sys::Object::new();
                    js_sys::Reflect::set(
                        &vel_obj,
                        &JsValue::from_str("vx"),
                        &JsValue::from_f64(vel.vx as f64),
                    )
                    .unwrap();
                    js_sys::Reflect::set(
                        &vel_obj,
                        &JsValue::from_str("vy"),
                        &JsValue::from_f64(vel.vy as f64),
                    )
                    .unwrap();
                    JsValue::from(vel_obj)
                }
                None => JsValue::NULL,
            };
            let obj = js_sys::Object::new();
            // Entity id as string to avoid JS Number precision loss (u64 > 2^53-1)
            js_sys::Reflect::set(
                &obj,
                &JsValue::from_str("id"),
                &JsValue::from(id_bits.to_string()),
            )
            .unwrap();
            js_sys::Reflect::set(&obj, &JsValue::from_str("pos"), &pos_obj).unwrap();
            js_sys::Reflect::set(&obj, &JsValue::from_str("velocity"), &vel_js).unwrap();
            arr.push(&obj);
        }
        arr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_error_message_is_stable_and_readable() {
        let message = format_init_error(&LoadError::Yaml {
            op: "load_from_str",
            source: "expected key".to_string(),
        });
        assert!(message.contains("init_world failed"));
        assert!(message.contains("YAML parse error"));
    }

    #[test]
    fn spawn_error_message_is_stable_and_readable() {
        let message = format_spawn_error(&SpawnError::UnknownEntityType {
            type_name: "ghost".to_string(),
        });
        assert!(message.contains("spawn failed"));
        assert!(message.contains("ghost"));
    }
}
