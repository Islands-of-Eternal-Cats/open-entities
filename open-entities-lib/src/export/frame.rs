//! The position frame: every positioned entity as `[index, generation, x, y]`, flat `i32`s.
//!
//! This is what crosses to a renderer each tick. It holds integers only, so the host can hand it
//! over as a typed array without serialising anything; everything that changes rarely travels
//! separately, in the [`MetaDelta`](super::MetaDelta).

use bevy_ecs::prelude::{Entity, World};

use crate::api::Api;
use crate::components::Position;
use crate::simulation::SimTick;

/// `i32`s before the first entity row: `[tick_lo, tick_hi, count]`.
pub const FRAME_HEADER_LEN: usize = 3;

/// `i32`s per entity row: `[index, generation, x, y]`, positions in milli-units.
pub const FRAME_STRIDE: usize = 4;

/// Bits of a `u32` as an `i32`: the frame is one `i32` array, and ids and tick words are unsigned.
const fn bits(value: u32) -> i32 {
    i32::from_ne_bytes(value.to_ne_bytes())
}

/// Clears `out` and writes the header `[tick_lo, tick_hi, count]`.
///
/// The tick is split into its low and high 32 bits; `count` is the number of rows that follow.
///
/// # Panics
///
/// When `count` exceeds `u32::MAX`.
pub fn write_frame_header(out: &mut Vec<i32>, tick: u64, count: usize) {
    out.clear();
    let [lo0, lo1, lo2, lo3, hi0, hi1, hi2, hi3] = tick.to_le_bytes();
    out.push(bits(u32::from_le_bytes([lo0, lo1, lo2, lo3])));
    out.push(bits(u32::from_le_bytes([hi0, hi1, hi2, hi3])));
    out.push(bits(u32::try_from(count).expect("entity count fits u32")));
}

impl Api {
    /// Writes the position frame of the current tick into `out`, replacing what it held.
    ///
    /// Layout: [`FRAME_HEADER_LEN`] header words `[tick_lo, tick_hi, count]`, then `count` rows of
    /// [`FRAME_STRIDE`] words `[index, generation, x, y]` sorted by `index`. Every entity with a
    /// [`Position`] has a row; ids are the [`EntityId`](crate::EntityId) parts, bit-cast to `i32`.
    ///
    /// Pass the same `Vec` every tick and its allocation is reused.
    pub fn write_frame(&mut self, out: &mut Vec<i32>) {
        write_frame_from_world(self.core_mut().world_mut(), out);
    }
}

fn write_frame_from_world(world: &mut World, out: &mut Vec<i32>) {
    let tick = world.resource::<SimTick>().0;
    let mut query = world.query::<(Entity, &Position)>();
    write_frame_header(out, tick, 0);
    for (entity, position) in query.iter(world) {
        out.extend_from_slice(&[
            bits(entity.index_u32()),
            bits(entity.generation().to_bits()),
            position.x,
            position.y,
        ]);
    }

    let (rows, _) = out[FRAME_HEADER_LEN..].as_chunks_mut::<FRAME_STRIDE>();
    let count = rows.len();
    // Storage order follows archetypes; the frame promises index order so a reader can merge two
    // frames in one pass. Within an archetype rows are mostly ascending already.
    rows.sort_unstable_by_key(|row| u32::from_ne_bytes(row[0].to_ne_bytes()));
    out[2] = bits(u32::try_from(count).expect("entity count fits u32"));
}
