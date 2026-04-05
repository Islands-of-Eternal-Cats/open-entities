use bevy_ecs::prelude::Component;

/// Идентификатор фракции (команды). В YAML задаётся полем `faction` (неотрицательное целое).
/// Без этого компонента сущность не отнесена ни к одной фракции.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Faction(pub u32);
