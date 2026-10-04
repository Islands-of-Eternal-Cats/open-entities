//! [`Api::state_hash`]: one number that two runs agree on exactly when their simulation state does.
//!
//! The hash is FNV-1a 64 over a byte stream written in a fixed order:
//!
//! 1. [`Api::current_tick`], `u64`;
//! 2. every entity, in ascending `(index, generation)`: its id as two `u32`, then each simulation
//!    component it carries as a tag byte ([`StateHash::TAG`]) plus the component's fields, then a
//!    `0` byte closing the entity.
//!
//! Every integer is little-endian, so the bytes — and the hash — are the same on every platform.
//! Components are visited in a fixed order: the registered ones in registry order, then the
//! template name and the internal ones (relations, markers, order claims) in a fixed order of
//! their own. Per-tick scratch such as
//! [`ArrivedThisTick`](crate::simulation::ArrivedThisTick) is not state and is left out, as are
//! resources in general.
//!
//! Peers in lockstep compare this hash to detect a desync; a replay pins it in a golden file.

#![deny(clippy::float_arithmetic)]

use bevy_ecs::prelude::{Component, Entity, World};
use bevy_ecs::query::Without;
use bevy_ecs::resource::IsResource;

use crate::api::Api;
use crate::component_registry::hash_registered_components;
use crate::components::{
    AssignedTo, BaseMoveSpeed, Boardable, BoardingTarget, EntityType, Faction, Group, Health,
    ManualActive, MemberOf, Mission, MissionCompleted, MoveTarget, NeedsMission, OrderSource,
    PassengerOf, Position, Velocity,
};
use crate::orders::EntityId;
use crate::simulation::SimTick;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Byte written after an entity's last component. No component uses it as a tag.
const END_OF_ENTITY: u8 = 0;

/// FNV-1a 64 over the bytes written to it. Integers go in little-endian.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateHasher(u64);

impl StateHasher {
    /// A hasher that has seen nothing yet.
    #[must_use]
    pub const fn new() -> Self {
        Self(FNV_OFFSET_BASIS)
    }

    /// Feeds raw bytes.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(FNV_PRIME);
        }
    }

    /// Feeds one byte.
    pub fn write_u8(&mut self, value: u8) {
        self.write_bytes(&[value]);
    }

    /// Feeds a `u32`, little-endian.
    pub fn write_u32(&mut self, value: u32) {
        self.write_bytes(&value.to_le_bytes());
    }

    /// Feeds an `i32`, little-endian two's complement.
    pub fn write_i32(&mut self, value: i32) {
        self.write_bytes(&value.to_le_bytes());
    }

    /// Feeds a `u64`, little-endian.
    pub fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    /// Feeds an entity as its [`EntityId`]: `index`, then `generation`.
    pub fn write_entity(&mut self, entity: Entity) {
        let id = EntityId::of(entity);
        self.write_u32(id.index);
        self.write_u32(id.generation);
    }

    /// Feeds a string as its byte length (`u32`) followed by its UTF-8 bytes.
    ///
    /// # Panics
    ///
    /// When the string is longer than `u32::MAX` bytes.
    pub fn write_str(&mut self, value: &str) {
        self.write_u32(u32::try_from(value.len()).expect("hashed string fits u32 length"));
        self.write_bytes(value.as_bytes());
    }

    /// The hash of everything written so far.
    #[must_use]
    pub const fn finish(&self) -> u64 {
        self.0
    }
}

impl Default for StateHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// A component that is part of the simulation state, and how it goes into the hash.
pub trait StateHash {
    /// Byte that marks this component in the stream. Unique per component type, never `0`.
    const TAG: u8;

    /// Writes every field, in declaration order. A field left out is a desync nobody sees.
    fn hash_fields(&self, hasher: &mut StateHasher);
}

/// Writes `T`'s tag and fields when `entity` carries a `T`.
pub(crate) fn hash_component<T: Component + StateHash>(
    world: &World,
    entity: Entity,
    hasher: &mut StateHasher,
) {
    if let Some(component) = world.get::<T>(entity) {
        hasher.write_u8(T::TAG);
        component.hash_fields(hasher);
    }
}

/// The components outside the registry: the template name, and the relations, markers and order
/// claims the simulation keeps for itself.
fn hash_internal_components(world: &World, entity: Entity, hasher: &mut StateHasher) {
    hash_component::<EntityType>(world, entity, hasher);
    hash_component::<PassengerOf>(world, entity, hasher);
    hash_component::<BoardingTarget>(world, entity, hasher);
    hash_component::<MemberOf>(world, entity, hasher);
    hash_component::<Group>(world, entity, hasher);
    hash_component::<ManualActive>(world, entity, hasher);
    hash_component::<AssignedTo>(world, entity, hasher);
    hash_component::<Mission>(world, entity, hasher);
    hash_component::<MissionCompleted>(world, entity, hasher);
    hash_component::<NeedsMission>(world, entity, hasher);
    hash_component::<OrderSource>(world, entity, hasher);
}

impl Api {
    /// FNV-1a 64 hash of the simulation state; see the [module docs](crate::state_hash).
    ///
    /// Equal hashes on two peers at the same tick mean they are in step. The value is the same on
    /// every platform, native and wasm32.
    ///
    /// # Panics
    ///
    /// Never in practice: the world always holds resources, so the resource marker the query
    /// filters on is registered.
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let world = self.core().world();
        let mut hasher = StateHasher::new();
        hasher.write_u64(world.resource::<SimTick>().0);

        let query = world
            .try_query_filtered::<Entity, Without<IsResource>>()
            .expect("the world holds resources, so IsResource is registered");
        let mut entities: Vec<Entity> = query.iter_manual(world).collect();
        entities.sort_unstable_by_key(|entity| EntityId::of(*entity));

        for entity in entities {
            hasher.write_entity(entity);
            hash_registered_components(world, entity, &mut hasher);
            hash_internal_components(world, entity, &mut hasher);
            hasher.write_u8(END_OF_ENTITY);
        }
        hasher.finish()
    }
}

impl StateHash for Position {
    const TAG: u8 = 1;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_i32(self.x);
        hasher.write_i32(self.y);
    }
}

impl StateHash for Velocity {
    const TAG: u8 = 2;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_i32(self.vx);
        hasher.write_i32(self.vy);
    }
}

impl StateHash for Faction {
    const TAG: u8 = 3;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_u32(self.0);
    }
}

impl StateHash for MoveTarget {
    const TAG: u8 = 4;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_i32(self.x);
        hasher.write_i32(self.y);
    }
}

impl StateHash for BaseMoveSpeed {
    const TAG: u8 = 5;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_i32(self.0);
    }
}

impl StateHash for Health {
    const TAG: u8 = 6;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_u32(self.current);
        hasher.write_u32(self.max);
    }
}

impl StateHash for Boardable {
    const TAG: u8 = 7;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_u8(self.0);
    }
}

impl StateHash for EntityType {
    const TAG: u8 = 16;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_str(&self.0);
    }
}

impl StateHash for PassengerOf {
    const TAG: u8 = 17;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_entity(self.0);
    }
}

impl StateHash for BoardingTarget {
    const TAG: u8 = 18;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_entity(self.0);
    }
}

impl StateHash for MemberOf {
    const TAG: u8 = 19;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_entity(self.0);
    }
}

impl StateHash for Group {
    const TAG: u8 = 20;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_u32(self.faction);
    }
}

impl StateHash for ManualActive {
    const TAG: u8 = 21;
    fn hash_fields(&self, _: &mut StateHasher) {}
}

impl StateHash for AssignedTo {
    const TAG: u8 = 22;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_entity(self.0);
    }
}

impl StateHash for Mission {
    const TAG: u8 = 23;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_i32(self.target.x);
        hasher.write_i32(self.target.y);
        hasher.write_i32(self.radius);
    }
}

impl StateHash for MissionCompleted {
    const TAG: u8 = 24;
    fn hash_fields(&self, _: &mut StateHasher) {}
}

impl StateHash for NeedsMission {
    const TAG: u8 = 25;
    fn hash_fields(&self, _: &mut StateHasher) {}
}

impl StateHash for OrderSource {
    const TAG: u8 = 26;
    fn hash_fields(&self, hasher: &mut StateHasher) {
        hasher.write_u8(match self {
            Self::MissionSteering => 0,
            Self::GroupSteering => 1,
            Self::PlayerUnit => 2,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_64_matches_the_reference_vectors() {
        // From the FNV reference test suite.
        let hash = |bytes: &[u8]| {
            let mut hasher = StateHasher::new();
            hasher.write_bytes(bytes);
            hasher.finish()
        };
        assert_eq!(hash(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(hash(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(hash(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn integers_go_in_little_endian() {
        let mut wide = StateHasher::new();
        wide.write_u32(0x0403_0201);
        let mut bytes = StateHasher::new();
        bytes.write_bytes(&[1, 2, 3, 4]);
        assert_eq!(wide, bytes);
    }

    #[test]
    fn tags_are_unique_and_never_the_entity_terminator() {
        let tags = [
            Position::TAG,
            Velocity::TAG,
            Faction::TAG,
            MoveTarget::TAG,
            BaseMoveSpeed::TAG,
            Health::TAG,
            Boardable::TAG,
            EntityType::TAG,
            PassengerOf::TAG,
            BoardingTarget::TAG,
            MemberOf::TAG,
            Group::TAG,
            ManualActive::TAG,
            AssignedTo::TAG,
            Mission::TAG,
            MissionCompleted::TAG,
            NeedsMission::TAG,
            OrderSource::TAG,
        ];
        for (i, tag) in tags.iter().enumerate() {
            assert_ne!(*tag, END_OF_ENTITY);
            assert!(!tags[i + 1..].contains(tag), "tag {tag} is used twice");
        }
    }
}
