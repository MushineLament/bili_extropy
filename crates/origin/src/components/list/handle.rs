use bevy::{
    ecs::component::Component,
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use sea_orm::EntityTrait as _;

use crate::{
    command::downloadrule::LoadDownloadruleTask,
    components::{handle::ECSHandleResult, status::handle::LoadStatusTask},
    db::Db,
    entity::{
        account::{self, AccountModel},
        account_collection::{self, AccountCollectionModel},
        collection::{self, CollectionModel},
        collection_media, downloadrule,
        media::{self, MediaModel}, upper, upper_account,
    },
};

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListMediasTask(pub ECSHandleResult<Vec<MediaModel>, anyhow::Error>);

impl ListMediasTask {
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

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListAccountCollectionsTask(
    pub ECSHandleResult<Vec<AccountCollectionModel>, anyhow::Error>,
);

impl ListAccountCollectionsTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let medias = account_collection::AccountCollectionEntity::find()
                .all(&db.db)
                .await?;
            Ok(medias)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListAccountFollwedTask(pub ECSHandleResult<Vec<upper_account::Model>, anyhow::Error>);

impl ListAccountFollwedTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let medias = upper_account::Entity::find().all(&db.db).await?;
            Ok(medias)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListCollectionTask(pub ECSHandleResult<Vec<CollectionModel>, anyhow::Error>);

impl ListCollectionTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let medias = collection::CollectionEntity::find().all(&db.db).await?;
            Ok(medias)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListUppersTask(pub ECSHandleResult<Vec<upper::UpperModel>, anyhow::Error>);

impl ListUppersTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let medias = upper::UpperEntity::find().all(&db.db).await?;
            Ok(medias)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListCollectionMediasTask(
    pub ECSHandleResult<Vec<collection_media::CollectionMediaModel>, anyhow::Error>,
);

impl ListCollectionMediasTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let medias = collection_media::CollectionMediaEntity::find()
                .all(&db.db)
                .await?;
            Ok(medias)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListDownloadruleTask(
    pub ECSHandleResult<Vec<downloadrule::DownloadruleModel>, anyhow::Error>,
);

impl ListDownloadruleTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let medias = downloadrule::DownloadruleEntity::find().all(&db.db).await?;
            Ok(medias)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListStatusDownloadRuleTask(pub LoadDownloadruleTask);

impl ListStatusDownloadRuleTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        Self(LoadDownloadruleTask::new(db, runtimer))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListStatusTask(pub LoadStatusTask);

impl ListStatusTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        Self(LoadStatusTask::new(db, runtimer))
    }
}
