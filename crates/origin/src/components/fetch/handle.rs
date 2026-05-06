use bevy::{
    ecs::component::Component,
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use sea_orm::{EntityTrait, Select};

use crate::{
    components::handle::ECSHandleResult,
    db::Db,
    entity::{collection_media, downloadtask_medias, upper_media},
};

#[derive(Debug, Component, Deref, DerefMut)]
pub struct LoadUpperMediasTask(
    pub ECSHandleResult<Vec<upper_media::UpperMediaModel>, anyhow::Error>,
);

impl LoadUpperMediasTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move { Self::load(&db).await.map_err(Into::into) };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

impl Loadable for LoadUpperMediasTask {
    type Entity = upper_media::UpperMediaEntity;
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct LoadCollectionMediasTask(
    pub ECSHandleResult<Vec<collection_media::CollectionMediaModel>, anyhow::Error>,
);

impl LoadCollectionMediasTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            Self::task(&db)
                .await
                .map_err(|err| anyhow::anyhow!("list collection medias error:{:?}", err))
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }

    pub async fn task(
        db: &Db,
    ) -> Result<Vec<collection_media::CollectionMediaModel>, sea_orm::error::DbErr> {
        let medias = collection_media::CollectionMediaEntity::find()
            .all(&db.db)
            .await?;
        Ok(medias)
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct LoadDownloadtaskMediasTask(
    pub ECSHandleResult<Vec<downloadtask_medias::DownloadtaskMediasModel>, anyhow::Error>,
);

impl LoadDownloadtaskMediasTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move { Self::load(&db).await.map_err(Into::into) };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

use anyhow::Result;
use sea_orm::DbErr;

impl Loadable for LoadDownloadtaskMediasTask {
    type Entity = downloadtask_medias::DownloadtaskMediasEntity;
}

/// 定义可加载的实体特性
pub trait Loadable {
    type Entity: EntityTrait;

    fn base_query() -> Select<Self::Entity> {
        Self::Entity::find()
    }

    fn load(
        db: &Db,
    ) -> impl Future<Output = Result<Vec<<Self::Entity as EntityTrait>::Model>, DbErr>> {
        Self::base_query().all(&db.db)
    }

    fn load_with<F>(
        db: &Db,
        func: F,
    ) -> impl Future<Output = Result<Vec<<Self::Entity as EntityTrait>::Model>, DbErr>>
    where
        F: FnOnce(Select<Self::Entity>) -> Select<Self::Entity>,
    {
        func(Self::base_query()).all(&db.db)
    }
}
