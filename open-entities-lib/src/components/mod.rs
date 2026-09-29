//! Gameplay ECS components. Most hosts only meet these as fields of
//! [`EntityComponents`](crate::EntityComponents); querying them directly goes through
//! [`Api::core`](crate::Api::core).

mod assigned_to;
mod base_move_speed;
mod boardable;
mod boarding_target;
mod entity_type;
mod faction;
mod group;
mod health;
mod manual_active;
mod member_of;
mod mission;
mod move_target;
mod needs_mission;
mod order_source;
mod passenger_of;
mod position;
mod velocity;

pub use assigned_to::AssignedTo;
pub use base_move_speed::BaseMoveSpeed;
pub use boardable::Boardable;
pub use boarding_target::BoardingTarget;
pub use entity_type::EntityType;
pub use faction::Faction;
pub use group::Group;
pub use health::Health;
pub use manual_active::ManualActive;
pub use member_of::MemberOf;
pub use mission::{Mission, MissionCompleted};
pub use move_target::MoveTarget;
pub use needs_mission::NeedsMission;
pub use order_source::OrderSource;
pub use passenger_of::PassengerOf;
pub use position::Position;
pub use velocity::Velocity;
