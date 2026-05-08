use bevy::{
    ecs::component::Component,
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use migration::OnConflict;
use sea_orm::{DbErr, EntityTrait};
use tracing::info;

use crate::{
    components::handle::ECSHandleResult,
    db::Db,
    entity::downloadtask_medias::{
            self, DownloadtaskMediasActiveModel, DownloadtaskMediasEntity,
        },
};

/// insert data into sql.
#[derive(Debug, Component, Default, Clone, PartialEq, Eq, Hash)]
pub struct InsertTask;

#[derive(Debug, Component, Deref, DerefMut)]
pub struct InsertDownloadtaskRelatedMedias(pub ECSHandleResult<u64, DbErr>);

impl InsertDownloadtaskRelatedMedias {
    pub fn new(
        db: Db,
        runtimer: &mut TokioTasksRuntime,
        iter: impl Iterator<Item = DownloadtaskMediasActiveModel> + Send + 'static,
    ) -> Self {
        let handle = runtimer.spawn_background_task(|_ctx| async move {
            let on_conflict = OnConflict::columns([
                downloadtask_medias::Column::TaskId,
                downloadtask_medias::Column::MediaId,
            ])
            .update_columns([
                downloadtask_medias::Column::TaskId,
                downloadtask_medias::Column::MediaId,
            ])
            .to_owned();

            let mut count = 0;
            for model in iter {
                let media_id = model.media_id.clone();
                let effect_count = DownloadtaskMediasEntity::insert(model)
                    .on_conflict(on_conflict.clone())
                    .exec_without_returning(&db.db)
                    .await;

                if let Err(effect_count) = effect_count.as_ref() {
                    info!("db error:{:?},media<{:?}>", effect_count, media_id);
                }

                count += effect_count.unwrap_or_default();
            }

            info!(
                "update downloadtask related medias finished,count: {:?}",
                count
            );

            Ok(count)
        });

        Self(ECSHandleResult::new(handle))
    }
}
