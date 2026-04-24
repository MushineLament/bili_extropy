use bevy::{
    ecs::{component::Component, resource::Resource},
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use sea_orm::EntityTrait as _;

use crate::{
    components::handle::ECSHandleResult,
    db::Db,
    entity::{
        account::{self, AccountModel},
        media::{self, MediaModel},
    },
};

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct ListMedias(pub ECSHandleResult<Vec<MediaModel>, anyhow::Error>);

impl ListMedias {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let medias = media::MediaEntity::find().all(&db.db).await?;
            Ok(medias)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListAccountTask(pub ECSHandleResult<Vec<AccountModel>, anyhow::Error>);

impl ListAccountTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let medias = account::AccountEntity::find().all(&db.db).await?;
            Ok(medias)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}
