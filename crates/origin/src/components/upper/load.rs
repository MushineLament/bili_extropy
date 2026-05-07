use bevy::{
    ecs::component::Component,
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use sea_orm::EntityTrait;

use crate::{
    components::handle::ECSHandleResult,
    db::Db,
    entity::upper::{UpperEntity, UpperModel},
};

// #[derive(Debug, Component, Deref, DerefMut)]
// pub struct LoadUpperCollectionsTask(pub ECSHandleResult<Vec<UpperCollectionModel>, anyhow::Error>);

// impl LoadUpperCollectionsTask {
//     pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
//         let task = async move {
//             let medias = UpperCollectionEntity::find().all(&db.db).await?;
//             Ok(medias)
//         };
//         let handle = runtimer.spawn_background_task(|_ctx| task);
//         Self(ECSHandleResult::new(handle))
//     }
// }

#[derive(Debug, Component, Deref, DerefMut)]
pub struct LoadUppersTask(pub ECSHandleResult<Vec<UpperModel>, anyhow::Error>);

impl LoadUppersTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let medias = UpperEntity::find().all(&db.db).await?;
            Ok(medias)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}
