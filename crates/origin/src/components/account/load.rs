use bevy::{
    ecs::component::Component,
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use sea_orm::EntityTrait;

use crate::{
    components::handle::ECSHandleResult,
    db::Db,
    entity::account_collection::{AccountCollectionEntity, AccountCollectionModel},
};

#[derive(Debug, Component, Deref, DerefMut)]
pub struct LoadAccountCollectionsTask(
    pub ECSHandleResult<Vec<AccountCollectionModel>, anyhow::Error>,
);

impl LoadAccountCollectionsTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let medias = AccountCollectionEntity::find().all(&db.db).await?;
            Ok(medias)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}
