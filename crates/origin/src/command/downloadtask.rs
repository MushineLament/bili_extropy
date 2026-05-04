use bevy::{
    app::{Plugin, PostStartup, PreUpdate, Update},
    ecs::{
        component::Component,
        entity::Entity,
        message::MessageReader,
        system::{Commands, Query, Res, ResMut},
    },
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use migration::OnConflict;
use sea_orm::{ActiveValue, EntityTrait};
use tracing::{error, info};

use crate::{
    components::{
        handle::ECSHandleResult, initialize::DbInitailizeComponent,
        list::handle::ListDownloadruleTask, status::handle::StatusState,
    },
    console::ConsoleTrims,
    db::Db,
    entity::downloadtask::{self, DownloadtaskActiveModel},
};

pub const HELP_DOWNLOAD_TASK: &str = r#"
Back up your favorite bilibili online resources with RESP.

Usage: downloadrule <COMMAND> [SUB_COMMAND] [OPTIONS]

Commands:
    insert                      Insert a download rule.
        <name> [--AddRule]          Insert a <name> rule.

    remove                      Remove rule.
        <id>                    remove by rule id.

    help                        Print this message or the help of the given subcommand(s)

AddRule:
    -d,         --data          Show debug messages
    

Options:
    -v,         --verbose       Show debug messages
    -h,         --help          Print help
    -V,         --version       Print version
    -id [ID],   --id [ID]       Point ID

Example:
    List medias
    List account --id 114514 
    List upper followings
"#;

pub const DOWNLOAD_TASK_COMMAND_INDEX: usize = 2;

pub struct CommandDownloadtaskPlugin;

impl Plugin for CommandDownloadtaskPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(PostStartup, LoadDownloadtaskTask::new.to_system())
            .add_systems(PreUpdate, spawn_list_task)
            .add_systems(Update, (download_rule_insert_task,));
    }
}

pub fn spawn_list_task(
    mut commands: Commands,
    db: Res<Db>,
    mut runtimer: ResMut<TokioTasksRuntime>,
    mut console_message: MessageReader<ConsoleTrims>,
) {
    for message in console_message.read() {
        let _db = db.clone();
        let ConsoleTrims { args, argv } = message;

        if !args.get(1).is_some_and(|list| list.eq("downloadtask")) {
            continue;
        }

        let id = argv
            .get("id")
            .map(|id| id.iter())
            .into_iter()
            .flatten()
            .find_map(|id| id.parse::<i64>().ok())
            .into_iter()
            .next()
            .map(|id| ActiveValue::Set(id))
            .unwrap_or(ActiveValue::NotSet);

        match args.get(DOWNLOAD_TASK_COMMAND_INDEX).map(String::as_str) {
            Some("insert") => {
                let Some(r#type) = args.get(3).map(String::as_str) else {
                    error!("not a type name");
                    continue;
                };

                let Some(Ok(generic_id)) =
                    args.get(3).map(String::as_str).map(|id| id.parse::<i64>())
                else {
                    error!("not a number id");
                    continue;
                };

                commands.spawn(DownloadtaskInsertTask::new(
                    db.clone(),
                    runtimer.as_mut(),
                    downloadtask::DownloadtaskActiveModel {
                        id,
                        type_id: ActiveValue::Set(r#type.to_string()),
                        generic_id: ActiveValue::Set(generic_id),
                        state: ActiveValue::Set(StatusState::Active.to_string()),
                    },
                ));
            }

            Some(unkown) => {
                error!("not has this command: {:?}", unkown);
            }

            None => {
                // 输出help
                commands.spawn(ListDownloadruleTask::new(db.clone(), runtimer.as_mut()));
            }
        }
    }
}

pub fn download_rule_insert_task(
    mut commands: Commands,
    query: Query<(&mut DownloadtaskInsertTask, Entity)>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        info!("insert a task id<{}>", result);
    }
}

pub type TaskId = i64;

#[derive(Debug, Component, Deref, DerefMut)]
pub struct DownloadtaskInsertTask(pub ECSHandleResult<TaskId, anyhow::Error>);

impl DownloadtaskInsertTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime, model: DownloadtaskActiveModel) -> Self {
        let task = async move {
            let pri = downloadtask::DownloadtaskEntity::insert(model)
                .on_conflict(
                    OnConflict::columns([downloadtask::Column::Id])
                        .update_columns([
                            downloadtask::Column::TypeId,
                            downloadtask::Column::GenericId,
                        ])
                        .to_owned(),
                )
                .exec_with_returning(&db.db)
                .await?;

            Ok(pri.id)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct LoadDownloadtaskTask(
    pub ECSHandleResult<Vec<downloadtask::DownloadtaskModel>, anyhow::Error>,
);

impl LoadDownloadtaskTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let medias = downloadtask::DownloadtaskEntity::find().all(&db.db).await?;
            Ok(medias)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}
