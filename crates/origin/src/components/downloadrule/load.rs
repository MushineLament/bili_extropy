use bevy::{
    ecs::component::Component,
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use sea_orm::EntityTrait as _;

use crate::{
    components::handle::ECSHandleResult,
    db::Db,
    entity::downloadrule::{DownloadruleEntity, DownloadruleModel},
};

#[derive(Debug, Component, Deref, DerefMut)]
pub struct LoadDownloadrule(pub ECSHandleResult<Vec<DownloadruleModel>, anyhow::Error>);

impl LoadDownloadrule {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let medias = DownloadruleEntity::find().all(&db.db).await?;
            Ok(medias)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}
