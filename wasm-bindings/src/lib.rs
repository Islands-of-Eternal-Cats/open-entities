//! WASM bindings for open-entities library
//!
//! This crate provides WebAssembly bindings to use open-entities
//! from JavaScript projects.

use open_entities::{Position, Velocity};
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

/// Move an entity's position based on its velocity
///
/// This function takes a position and velocity, and returns a new position
/// after moving for one tick (assuming 1 second delta time).
#[wasm_bindgen]
pub fn move_position(pos: &JsPosition, vel: &JsVelocity) -> JsPosition {
    let new_x = pos.position.x + vel.velocity.vx;
    let new_y = pos.position.y + vel.velocity.vy;

    JsPosition {
        position: Position { x: new_x, y: new_y },
    }
}
