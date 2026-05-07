use bevy::{
    ecs::component::Component,
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use sea_orm::EntityTrait as _;

use crate::{
    components::{
        account::load::LoadAccountCollectionsTask,
        downloadtask::load::{LoadDownloadtaskMediasTask, LoadDownloadtaskTask},
        fetch::handle::LoadUpperMediasTask,
        handle::ECSHandleResult,
        list::load::LoadMediasTask,
        status::handle::{LoadStatusRelatedDownloadruleTask, LoadStatusTask},
        upper::load::LoadUppersTask,
    },
    db::Db,
    entity::{
        account::{self, AccountModel},
        collection::{self, CollectionModel},
        collection_media, downloadrule, upper_account,
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
pub struct ListDownloadtaskTask(pub LoadDownloadtaskTask);

impl ListDownloadtaskTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        Self(LoadDownloadtaskTask::new(db, runtimer))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListStatusRelatedDownloadruleTask(pub LoadStatusRelatedDownloadruleTask);

impl ListStatusRelatedDownloadruleTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        Self(LoadStatusRelatedDownloadruleTask::new(db, runtimer))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListStatusTask(pub LoadStatusTask);

impl ListStatusTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        Self(LoadStatusTask::new(db, runtimer))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListDownloadtaskMediasTask(pub LoadDownloadtaskMediasTask);

impl ListDownloadtaskMediasTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        Self(LoadDownloadtaskMediasTask::new(db, runtimer))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListUpperMediasTask(pub LoadUpperMediasTask);

impl ListUpperMediasTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        Self(LoadUpperMediasTask::new(db, runtimer))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListAccountCollectionsTask(pub LoadAccountCollectionsTask);

impl ListAccountCollectionsTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        Self(LoadAccountCollectionsTask::new(db, runtimer))
    }
}

// #[derive(Debug, Component, Deref, DerefMut)]
// pub struct ListUpperCollectionsTask(pub LoadUpperCollectionsTask);

// impl ListUpperCollectionsTask {
//     pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
//         Self(LoadUpperCollectionsTask::new(db, runtimer))
//     }
// }

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ListUppersTask(pub LoadUppersTask);

impl ListUppersTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        Self(LoadUppersTask::new(db, runtimer))
    }
}
