use bevy_ecs::prelude::Component;
use serde::{Deserialize, Serialize};

/// How many passengers a vehicle can carry.
///
/// In YAML a template writes `boardable: 4`. Without this component an entity cannot be boarded
/// at all, which is the difference between a truck and a rock.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Boardable(pub u8);
