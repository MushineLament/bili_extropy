use std::borrow::Cow;

use anyhow::Result;
use bevy::{
    ecs::{component::Component, resource::Resource},
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use migration::Expr;
use sea_orm::{ColumnTrait, EntityTrait as _, QueryFilter};
use strum::{Display, EnumString};

use crate::{
    components::handle::ECSHandleResult,
    db::Db,
    entity::status::{self, StatusActiveModel, StatusEntity, StatusModel},
};

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display)]
pub enum StatusState {
    Active,
    Inactive,
    Switch,
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct ActiveStatus(pub ECSHandleResult<Cow<'static, Vec<StatusModel>>, anyhow::Error>);

impl ActiveStatus {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let task = StatusEntity::find()
                .filter(status::Column::State.eq(StatusState::Active.to_string()))
                .all(&db.db)
                .await?;
            Ok(Cow::Owned(task))
        };

        let handle = runtimer.spawn_background_task(|_ctx| task);

        Self(ECSHandleResult::new(handle))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct AddStatusTask(pub ECSHandleResult<StatusModel, anyhow::Error>);

impl AddStatusTask {
    pub fn new(
        db: Db,
        runtimer: &mut TokioTasksRuntime,
        name: String,
        path: String,
        state: StatusState,
    ) -> Self {
        let task = async move {
            let db = db;
            match db.get_status_by_folder(name.as_str(), path.as_str()).await {
                Result::Ok(model) => {
                    db.activate_status_by_id(model.id).await?;
                    Ok(model)
                }
                Err(_) => {
                    let inserted = StatusEntity::insert(StatusActiveModel {
                        id: sea_orm::ActiveValue::NotSet,
                        name: sea_orm::ActiveValue::Set(name),
                        path: sea_orm::ActiveValue::Set(path),
                        state: sea_orm::ActiveValue::Set(state.to_string()),
                    })
                    .exec(&db.db)
                    .await?;

                    let generated_id = inserted.last_insert_id;

                    let status = db.get_status_by_id(generated_id).await?;

                    if state == StatusState::Switch {
                        StatusEntity::update_many()
                            .filter(status::Column::Id.eq(generated_id))
                            .col_expr(
                                status::Column::State,
                                Expr::value(StatusState::Inactive.to_string()),
                            )
                            .exec(&db.db)
                            .await?;
                    }

                    Ok(status)
                }
            }
        };

        let handle = runtimer.spawn_background_task(|_ctx| task);

        Self(ECSHandleResult::new(handle))
    }
}
