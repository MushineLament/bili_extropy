use bevy::{
    ecs::component::Component,
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use sea_orm::EntityTrait as _;

use crate::{
    components::{
        handle::ECSHandleResult,
        list::load::LoadMediasTask,
    },
    db::Db,
    entity::{
        account::{self, AccountModel},
        collection::{self, CollectionModel},
        collection_media, upper_account,
    },
};

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListMediasTask(pub LoadMediasTask);

impl ListMediasTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        Self(LoadMediasTask::new(db, runtimer))
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

// #[derive(Debug, Component, Deref, DerefMut)]
// pub struct ListUpperCollectionsTask(pub LoadUpperCollectionsTask);

// impl ListUpperCollectionsTask {
//     pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
//         Self(LoadUpperCollectionsTask::new(db, runtimer))
//     }
// }

/// if has this mark,will println data about load.
#[derive(Debug, Component, Default, Clone, PartialEq, Eq, Hash)]
pub struct ListTask;
