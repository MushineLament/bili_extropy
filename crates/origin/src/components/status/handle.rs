use std::borrow::Cow;

use bevy::{
    ecs::{component::Component, resource::Resource},
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use migration::OnConflict;
use sea_orm::{ActiveValue, EntityTrait as _};
use strum::{Display, EnumString};

use crate::{
    components::handle::ECSHandleResult,
    db::Db,
    entity::{
        status::{self, StatusModel},
        status_downloadrule::{self, StatusDownloadruleModel},
    },
};

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display)]
pub enum StatusState {
    Active,
    Inactive,
    Exclusive,
}

#[derive(Debug, Resource, Deref, DerefMut, Default, Clone)]
pub struct ActiveStatus(pub Cow<'static, Vec<StatusModel>>);

#[derive(Debug, Component, Deref, DerefMut)]
pub struct LoadStatusTask(pub ECSHandleResult<Vec<status::StatusModel>, anyhow::Error>);

impl LoadStatusTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let medias = status::StatusEntity::find().all(&db.db).await?;
            Ok(medias)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct StatusInsertTask(pub ECSHandleResult<StatusModel, anyhow::Error>);

impl StatusInsertTask {
    pub fn new(
        db: Db,
        runtimer: &mut TokioTasksRuntime,
        activemodel: status::StatusActiveModel,
    ) -> Self {
        let task = async move {
            let db = db;

            let model = status::StatusEntity::insert(activemodel)
                .on_conflict(
                    OnConflict::column(status::Column::Id)
                        .update_columns([
                            status::Column::Name,
                            status::Column::Path,
                            status::Column::State,
                        ])
                        .to_owned(),
                )
                .exec_with_returning(&db.db)
                .await?;

            Ok(model)
        };

        let handle = runtimer.spawn_background_task(|_ctx| task);

        Self(ECSHandleResult::new(handle))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct StatusRelatedDownloadruleTask(
    pub ECSHandleResult<StatusDownloadruleModel, anyhow::Error>,
);

impl StatusRelatedDownloadruleTask {
    pub fn new(
        db: Db,
        runtimer: &mut TokioTasksRuntime,
        status_id: ActiveValue<i64>,
        rule_id: ActiveValue<i64>,
    ) -> Self {
        let task = async move {
            let db = db;

            let model = status_downloadrule::StatusDownloadruleEntity::insert(
                status_downloadrule::ActiveModel { status_id, rule_id },
            )
            .on_conflict(
                OnConflict::columns([
                    status_downloadrule::Column::StatusId,
                    status_downloadrule::Column::RuleId,
                ])
                .update_columns([
                    status_downloadrule::Column::StatusId,
                    status_downloadrule::Column::RuleId,
                ])
                .to_owned(),
            )
            .exec_with_returning(&db.db)
            .await?;

            Ok(model)
        };

        let handle = runtimer.spawn_background_task(|_ctx| task);

        Self(ECSHandleResult::new(handle))
    }
}
