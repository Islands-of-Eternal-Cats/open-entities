//! WASM bindings for open-entities library
//!
//! This crate provides WebAssembly bindings to use open-entities
//! from JavaScript projects.

use js_sys::Array;
use open_entities::{
    create_empty_world, get_entities_position_velocity, run_tick, Position, Schedule, Velocity,
    World,
};
use wasm_bindgen::prelude::*;

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
    /// Create an empty world. Call `spawn` to add entities, then `tick(dt)` each frame.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let (world, schedule) = create_empty_world();
        Self { world, schedule }
    }

    /// Spawn an entity with position and velocity.
    #[wasm_bindgen]
    pub fn spawn(&mut self, x: f32, y: f32, vx: f32, vy: f32) {
        self.world
            .spawn((Position { x, y }, Velocity { vx, vy }));
    }

    /// Run one simulation tick with the given delta time in seconds.
    #[wasm_bindgen]
    pub fn tick(&mut self, dt: f32) {
        run_tick(&mut self.world, &mut self.schedule, dt);
    }

    /// Snapshot of all entities as an array of `{ id, pos: { x, y }, velocity: { vx, vy } }` for rendering.
    /// `id` is a stable entity identifier (Entity::to_bits) so the same entity keeps the same id across frames.
    #[wasm_bindgen]
    pub fn get_entities(&mut self) -> Array {
        let snapshot = get_entities_position_velocity(&mut self.world);
        let arr = Array::new();
        for (id_bits, pos, vel) in snapshot {
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
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &JsValue::from_str("id"), &JsValue::from_f64(id_bits as f64)).unwrap();
            js_sys::Reflect::set(&obj, &JsValue::from_str("pos"), &pos_obj).unwrap();
            js_sys::Reflect::set(&obj, &JsValue::from_str("velocity"), &vel_obj).unwrap();
            arr.push(&obj);
        }
        arr
    }
}
