use bevy::{
    ecs::component::Component,
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use sea_orm::DbErr;

use crate::{
    components::{fetch::handle::Loadable, handle::ECSHandleResult},
    db::Db,
    entity::media::{MediaEntity, MediaModel},
};

#[derive(Debug, Component, Deref, DerefMut)]
pub struct LoadMediasTask(pub ECSHandleResult<Vec<MediaModel>, DbErr>);

impl LoadMediasTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move { Self::load(&db).await };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

impl Loadable for LoadMediasTask {
    type Entity = MediaEntity;
}
