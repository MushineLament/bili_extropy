use bevy::{
    ecs::component::Component,
    platform::collections::{HashMap, hash_map::Entry},
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DbErr, QueryFilter, Select, Statement,
};

use crate::{
    components::{
        downloadtask::handle::DownloadtaskRelatedMediaPending, fetch::handle::Loadable,
        handle::ECSHandleResult,
    },
    db::Db,
    entity::{
        MediaAid,
        downloadtask::{self, DownloadtaskModel},
        downloadtask_medias::{DownloadtaskMediasEntity, DownloadtaskMediasModel},
        media::MEDIA,
    },
};

#[derive(Debug, Component, Deref, DerefMut)]
pub struct LoadDownloadtask(pub ECSHandleResult<Vec<DownloadtaskModel>, anyhow::Error>);

impl LoadDownloadtask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            Self::load(&db)
                .await
                .map_err(|err| anyhow::anyhow!("load download task error :{:?}", err))
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }

    pub fn new_with<F>(db: Db, runtimer: &mut TokioTasksRuntime, func: F) -> Self
    where
        F: FnOnce(
                Select<<LoadDownloadtask as Loadable>::Entity>,
            ) -> Select<<LoadDownloadtask as Loadable>::Entity>
            + Send
            + 'static,
    {
        let task = async move {
            Self::load_with(&db, func)
                .await
                .map_err(|err| anyhow::anyhow!("load download task error :{:?}", err))
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }

    pub async fn related_medias(db: &Db) -> Result<Vec<DownloadtaskRelatedMediaPending>, DbErr> {
        let taskids = Self::load_with(db, |select| {
            select.filter(downloadtask::Column::TypeId.eq(MEDIA))
        })
        .await?;

        let relateds = taskids
            .iter()
            .map(|model| DownloadtaskRelatedMediaPending {
                media_id: model.generic_id,
                taskid: vec![model.id],
            })
            .collect();

        Ok(relateds)
    }

    pub async fn related_upper_medias(
        db: &Db,
    ) -> Result<HashMap<MediaAid, DownloadtaskRelatedMediaPending>, DbErr> {
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

        let mut hash: HashMap<i64, DownloadtaskRelatedMediaPending> = HashMap::new();

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
                    vacant.insert(DownloadtaskRelatedMediaPending {
                        media_id,
                        taskid: vec![task_id],
                    });
                }
            }
        }

        Ok(hash)
    }

    pub async fn related_collection_medias(
        db: &Db,
    ) -> Result<HashMap<MediaAid, DownloadtaskRelatedMediaPending>, DbErr> {
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

        let mut hash: HashMap<i64, DownloadtaskRelatedMediaPending> = HashMap::new();

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
                    vacant.insert(DownloadtaskRelatedMediaPending {
                        media_id,
                        taskid: vec![task_id],
                    });
                }
            }
        }

        Ok(hash)
    }

    pub async fn related_all_medias(
        db: &Db,
    ) -> Result<HashMap<MediaAid, DownloadtaskRelatedMediaPending>, DbErr> {
        let (medias, upper_medias, collection_medias) = tokio::join!(
            Self::related_medias(db),
            Self::related_upper_medias(db),
            Self::related_collection_medias(db)
        );

        let (medias, upper_medias, collection_medias) =
            (medias?, upper_medias?, collection_medias?);

        let mut relateds = upper_medias;

        for media in medias {
            match relateds.entry(media.media_id) {
                Entry::Occupied(occupied) => {
                    occupied.into_mut().taskid.extend(media.taskid);
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(media);
                }
            }
        }

        for (_, media) in collection_medias {
            match relateds.entry(media.media_id) {
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

impl Loadable for LoadDownloadtask {
    type Entity = downloadtask::DownloadtaskEntity;
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct LoadDownloadtaskMedias(pub ECSHandleResult<Vec<DownloadtaskMediasModel>, anyhow::Error>);

impl LoadDownloadtaskMedias {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move { Self::load(&db).await.map_err(Into::into) };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

impl Loadable for LoadDownloadtaskMedias {
    type Entity = DownloadtaskMediasEntity;
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct LoadDownloadtaskRelatedMedias(
    pub ECSHandleResult<HashMap<MediaAid, DownloadtaskRelatedMediaPending>, DbErr>,
);

impl LoadDownloadtaskRelatedMedias {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let handle = runtimer
            .spawn_background_task(|_ctx| async move { Self::load_related_all_medias(&db).await });

        Self(ECSHandleResult::new(handle))
    }

    pub async fn load_related_medias(
        db: &Db,
    ) -> Result<Vec<DownloadtaskRelatedMediaPending>, DbErr> {
        let taskids = LoadDownloadtask::load_with(db, |select| {
            select.filter(downloadtask::Column::TypeId.eq(MEDIA))
        })
        .await?;

        let relateds = taskids
            .iter()
            .map(|model| DownloadtaskRelatedMediaPending {
                media_id: model.generic_id,
                taskid: vec![model.id],
            })
            .collect();

        Ok(relateds)
    }

    pub async fn load_related_upper_medias(
        db: &Db,
    ) -> Result<HashMap<MediaAid, DownloadtaskRelatedMediaPending>, DbErr> {
        let querys = db
            .db
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
SELECT dt.id AS related_id, um.media_id
FROM downloadtask dt
JOIN upper_media um ON dt.generic_id = um.upper_id
WHERE dt.type_id = 'Upper'
        "#,
            ))
            .await?;

        let mut hash: HashMap<i64, DownloadtaskRelatedMediaPending> = HashMap::new();

        for (related_id, media_id) in
            querys
                .into_iter()
                .filter_map(|query: sea_orm::QueryResult| {
                    let task_id: i64 = query.try_get("", "related_id").ok()?;
                    let media_id: i64 = query.try_get("", "media_id").ok()?;

                    Some((task_id, media_id))
                })
        {
            match hash.entry(media_id) {
                Entry::Occupied(occupied) => {
                    occupied.into_mut().taskid.push(related_id);
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(DownloadtaskRelatedMediaPending {
                        media_id,
                        taskid: vec![related_id],
                    });
                }
            }
        }

        Ok(hash)
    }

    pub async fn load_related_collection_medias(
        db: &Db,
    ) -> Result<HashMap<MediaAid, DownloadtaskRelatedMediaPending>, DbErr> {
        let querys = db
            .db
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
SELECT dt.id AS related_id, cm.media_cid
FROM downloadtask dt
JOIN collection_media cm ON dt.generic_id = cm.collection_id
WHERE dt.type_id = 'Collection'
        "#,
            ))
            .await?;

        let mut hash: HashMap<i64, DownloadtaskRelatedMediaPending> = HashMap::new();

        for (related_id, media_id) in querys.into_iter().filter_map(|query| {
            let task_id: i64 = query.try_get("", "related_id").ok()?;
            let media_id: i64 = query.try_get("", "media_id").ok()?;

            Some((task_id, media_id))
        }) {
            match hash.entry(media_id) {
                Entry::Occupied(occupied) => {
                    occupied.into_mut().taskid.push(related_id);
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(DownloadtaskRelatedMediaPending {
                        media_id,
                        taskid: vec![related_id],
                    });
                }
            }
        }

        Ok(hash)
    }

    pub async fn load_related_all_medias(
        db: &Db,
    ) -> Result<HashMap<MediaAid, DownloadtaskRelatedMediaPending>, DbErr> {
        let (medias, upper_medias, collection_medias) = tokio::join!(
            Self::load_related_medias(db),
            Self::load_related_upper_medias(db),
            Self::load_related_collection_medias(db)
        );

        let (medias, upper_medias, collection_medias) =
            (medias?, upper_medias?, collection_medias?);

        let mut relateds = upper_medias;

        for media in medias {
            match relateds.entry(media.media_id) {
                Entry::Occupied(occupied) => {
                    occupied.into_mut().taskid.extend(media.taskid);
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(media);
                }
            }
        }

        for (_, media) in collection_medias {
            match relateds.entry(media.media_id) {
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

/// load downloadtask_medias pending download medias from sql.
#[derive(Debug, Component, Deref, DerefMut)]
pub struct LoadDownloadtaskMediasPendingDownloadTask(
    pub ECSHandleResult<Vec<MediaAid>, anyhow::Error>,
);

impl LoadDownloadtaskMediasPendingDownloadTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let handle = runtimer.spawn_background_task(move |_ctx| async move {
            let querys = db
                .db
                .query_all(Statement::from_string(
                    DatabaseBackend::Sqlite,
                    r#"
SELECT dt_m.media_id AS media_id
FROM downloadtask_medias dt_m
JOIN media m ON m.aid = dt_m.media_id
        "#,
                ))
                .await?;

            let pending_medias = querys
                .into_iter()
                .filter_map(|query| query.try_get::<i64>("", "media_id").ok())
                .collect::<Vec<_>>();

            Ok(pending_medias)
        });

        Self(ECSHandleResult::new(handle))
    }
}
