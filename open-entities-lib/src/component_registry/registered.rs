use crate::components::{Faction, Health, MoveTarget, Position, Velocity};

define_registered_components! {
    register_component!(position, Position);
    register_component!(velocity, Velocity);
    register_component!(faction, Faction);
    register_component!(move_target, MoveTarget);
    register_component!(health, Health);
}
