use bevy::{
    ecs::resource::Resource,
    prelude::{Deref, DerefMut},
};

use crate::components::download::DownloadWay;

#[derive(Debug, Resource, Default, Clone, Deref, DerefMut)]
pub struct DownloadList(pub Vec<DownloadWay>);
