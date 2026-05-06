use api_req::ApiCaller;
use bevy::{
    ecs::{component::Component, resource::Resource},
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use futures::TryFutureExt;
use migration::OnConflict;
use sea_orm::EntityTrait;

use crate::{
    api::BiliApi,
    components::{
        download::{DownloadFileError, DownloadPendding, MediaInfoAidPayload},
        downloadtask::load::LoadDownloadtaskTask,
        handle::ECSHandleResult,
    },
    db::Db,
    entity::{
        MediaAid,
        downloadtask::{self, DownloadtaskActiveModel},
        media::MediaInfoSingle,
    },
};

#[derive(Debug, Resource, Default, Clone, Deref, DerefMut)]
pub struct DownloadList(pub Vec<DownloadRelatedTaskId>);

pub type TaskId = i64;

#[derive(Debug, Component, Deref, DerefMut)]
pub struct InsertDownloadtaskTask(pub ECSHandleResult<TaskId, anyhow::Error>);

impl InsertDownloadtaskTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime, model: DownloadtaskActiveModel) -> Self {
        let active_id = model.id.try_as_ref().cloned();

        let mut on_conflict = OnConflict::columns([downloadtask::Column::Id])
            .update_columns([
                downloadtask::Column::TypeId,
                downloadtask::Column::GenericId,
            ])
            .to_owned();

        if model.state.is_set() {
            on_conflict.update_column(downloadtask::Column::State);
        }

        let task = async move {
            let opr = downloadtask::DownloadtaskEntity::insert(model).on_conflict(on_conflict);

            let pri = if let Some(id) = active_id {
                opr.exec_without_returning(&db.db).await?;
                id
            } else {
                opr.exec_with_returning(&db.db).await?.id
            };

            Ok(pri)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

/// 经过处理的，等待下载的最小视频单位media
#[derive(Debug, Component, Clone)]
pub struct DownloadRelatedTaskId {
    /// 唯一性去重，bvid必需经过转化为aid
    pub id: MediaAid,

    /// 关联的taskid，下载完成后，会将taskid下对应的mediaaid更新
    pub taskid: Vec<TaskId>,
}

impl DownloadRelatedTaskId {
    pub async fn task(db: &Db) -> Result<Vec<Self>, DownloadFileError> {
        let result = LoadDownloadtaskTask::related_all_medias(db).await?;

        Ok(result.into_values().collect())
    }
}

impl DownloadPendding for DownloadRelatedTaskId {
    fn to_response(
        &self,
    ) -> impl Future<Output = anyhow::Result<MediaInfoSingle, DownloadFileError>> + Send {
        BiliApi::request(MediaInfoAidPayload { aid: self.id }).map_err(Into::into)
    }

    fn media_aid(&self) -> impl Future<Output = Result<MediaAid, DownloadFileError>> {
        async { Ok(self.id) }
    }

    fn related_task_id(
        &self,
        _db: &Db,
    ) -> impl Future<Output = Result<Vec<TaskId>, DownloadFileError>> {
        async { Ok(self.taskid.clone()) }
    }
}

/// 经过处理视频待下载集合，拥有对应的taskid
#[derive(Debug, Resource, Deref, DerefMut)]
pub struct MediaDownloadList(pub ECSHandleResult<Vec<DownloadRelatedTaskId>, DownloadFileError>);

impl MediaDownloadList {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move { DownloadRelatedTaskId::task(&db).await };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}
