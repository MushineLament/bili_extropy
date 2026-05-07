use bevy::{
    ecs::component::Component,
    platform::collections::{HashMap, hash_map::Entry},
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseBackend, DbErr, QueryFilter, Statement};

use crate::{
    components::{
        downloadtask::{handle::DownloadRelatedTaskId, load::LoadDownloadtaskTask},
        fetch::handle::Loadable,
        handle::ECSHandleResult,
    },
    db::Db,
    entity::{
        MediaAid,
        downloadtask::{self},
        media::MEDIA,
    },
};

#[derive(Debug, Component, Deref, DerefMut)]
pub struct RelatedDownloadtaskMediasTask(
    pub ECSHandleResult<HashMap<MediaAid, DownloadRelatedTaskId>, DbErr>,
);

impl RelatedDownloadtaskMediasTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let handle = runtimer
            .spawn_background_task(|_ctx| async move { Self::related_all_medias(&db).await });

        Self(ECSHandleResult::new(handle))
    }

    pub async fn related_medias(db: &Db) -> Result<Vec<DownloadRelatedTaskId>, DbErr> {
        let taskids = LoadDownloadtaskTask::load_with(db, |select| {
            select.filter(downloadtask::Column::TypeId.eq(MEDIA))
        })
        .await?;

        let relateds = taskids
            .iter()
            .map(|model| DownloadRelatedTaskId {
                id: model.generic_id,
                taskid: vec![model.id],
            })
            .collect();

        Ok(relateds)
    }

    pub async fn related_upper_medias(
        db: &Db,
    ) -> Result<HashMap<MediaAid, DownloadRelatedTaskId>, DbErr> {
        let querys = db
            .db
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
SELECT dt.id AS task_id, um.media_id
FROM downloadtask dt
JOIN upper_media um ON dt.generic_id = um.upper_id
WHERE dt.type_id = 'Upper'
AND dt.state IN ('Pending', 'Downloading')
        "#,
            ))
            .await?;

        let mut hash: HashMap<i64, DownloadRelatedTaskId> = HashMap::new();

        for (task_id, media_id) in querys.into_iter().filter_map(|query| {
            let task_id: i64 = query.try_get("", "task_id").ok()?;
            let media_id: i64 = query.try_get("", "media_id").ok()?;

            Some((task_id, media_id))
        }) {
            match hash.entry(media_id) {
                Entry::Occupied(occupied) => {
                    occupied.into_mut().taskid.push(task_id);
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(DownloadRelatedTaskId {
                        id: media_id,
                        taskid: vec![task_id],
                    });
                }
            }
        }

        Ok(hash)
    }

    pub async fn related_collection_medias(
        db: &Db,
    ) -> Result<HashMap<MediaAid, DownloadRelatedTaskId>, DbErr> {
        let querys = db
            .db
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
SELECT dt.id AS task_id, cm.media_cid
FROM downloadtask dt
JOIN collection_media cm ON dt.generic_id = cm.collection_id
WHERE dt.type_id = 'Collection'
AND dt.state IN ('Pending', 'Downloading')
        "#,
            ))
            .await?;

        let mut hash: HashMap<i64, DownloadRelatedTaskId> = HashMap::new();

        for (task_id, media_id) in querys.into_iter().filter_map(|query| {
            let task_id: i64 = query.try_get("", "task_id").ok()?;
            let media_id: i64 = query.try_get("", "media_id").ok()?;

            Some((task_id, media_id))
        }) {
            match hash.entry(media_id) {
                Entry::Occupied(occupied) => {
                    occupied.into_mut().taskid.push(task_id);
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(DownloadRelatedTaskId {
                        id: media_id,
                        taskid: vec![task_id],
                    });
                }
            }
        }

        Ok(hash)
    }

    pub async fn related_all_medias(
        db: &Db,
    ) -> Result<HashMap<MediaAid, DownloadRelatedTaskId>, DbErr> {
        let (medias, upper_medias, collection_medias) = tokio::join!(
            Self::related_medias(db),
            Self::related_upper_medias(db),
            Self::related_collection_medias(db)
        );

        let (medias, upper_medias, collection_medias) =
            (medias?, upper_medias?, collection_medias?);

        let mut relateds = upper_medias;

        for media in medias {
            match relateds.entry(media.id) {
                Entry::Occupied(occupied) => {
                    occupied.into_mut().taskid.extend(media.taskid);
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(media);
                }
            }
        }

        for (_, media) in collection_medias {
            match relateds.entry(media.id) {
                Entry::Occupied(occupied) => {
                    occupied.into_mut().taskid.extend(media.taskid);
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(media);
                }
            }
        }

        Ok(relateds)
    }
}
