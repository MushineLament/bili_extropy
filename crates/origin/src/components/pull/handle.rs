use bevy::ecs::component::Component;

/// if has this mark,will println data about load.
#[derive(Debug, Component, Default, Clone, PartialEq, Eq, Hash)]
pub struct PullTask;
