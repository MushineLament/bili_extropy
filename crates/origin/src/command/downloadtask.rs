use bevy::{
    app::{Plugin, PreUpdate, Update},
    ecs::{
        entity::Entity,
        message::MessageReader,
        system::{Commands, Query, Res, ResMut},
    },
};
use bevy_tokio_tasks::TokioTasksRuntime;
use sea_orm::ActiveValue;
use tracing::{error, info};

use crate::{
    command::HELP,
    components::{
        account::handle::ActiveAccounts,
        downloadtask::{handle::InsertDownloadtaskTask, related::RelatedDownloadtaskMediasTask},
        list::handle::ListDownloadtaskTask,
    },
    console::ConsoleTrims,
    db::Db,
    entity::downloadtask::{self},
};

pub const HELP_DOWNLOADTASK: &str = r#"
Back up your favorite bilibili online resources with RESP.

Usage: task <COMMAND> [SUB_COMMAND] [OPTIONS]

Commands:
    insert                      Insert downloadtask.
        Media/Upper/Collection <Id> [--state Pending/Active/Inactive]         Insert downloadtask, Id type must is number.

    remove                      Remove task.#Not Finished
        <id>                    remove by task id.

    help                        Print this.

Options:
    -v,         --verbose       Show debug messages
    -h,         --help          Print help
    -V,         --version       Print version

Example:
    insert Media 113844248642487    #Insert a pending media download task.
"#;

pub const DOWNLOADTASK_COMMAND_INDEX: usize = 2;

pub struct CommandDownloadtaskPlugin;

impl Plugin for CommandDownloadtaskPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(PreUpdate, spawn_list_task).add_systems(
            Update,
            (download_rule_insert_task, related_downloadtask_medias_task),
        );
    }
}

pub fn spawn_list_task(
    mut commands: Commands,
    db: Res<Db>,
    mut runtimer: ResMut<TokioTasksRuntime>,
    mut console_message: MessageReader<ConsoleTrims>,
    _active_account: ResMut<ActiveAccounts>,
) {
    for message in console_message.read() {
        let _db = db.clone();
        let ConsoleTrims { args, argv: _ } = message;

        if !args.get(1).is_some_and(|list| list.eq("task")) {
            continue;
        }

        let id = message
            .ids()
            .into_iter()
            .filter_map(|id| id.parse::<i64>().ok())
            .next()
            .map(|id| ActiveValue::Set(id))
            .unwrap_or(ActiveValue::NotSet);

        match args.get(DOWNLOADTASK_COMMAND_INDEX).map(String::as_str) {
            Some("related") => {
                info!("related downloadtask and medias");
                commands.spawn(RelatedDownloadtaskMediasTask::new(
                    db.clone(),
                    runtimer.as_mut(),
                ));
            }
            Some("insert") => {
                let Some(r#type) = args.get(3).map(String::as_str) else {
                    error!("not a type name");
                    continue;
                };

                let Some(Ok(generic_id)) =
                    args.get(4).map(String::as_str).map(|id| id.parse::<i64>())
                else {
                    error!("not a number id");
                    continue;
                };

                info!("task id:{:?}", id);

                let state = message
                    .get_first_state()
                    .map(|first| ActiveValue::Set(first.to_string()))
                    .unwrap_or(ActiveValue::NotSet);

                info!("state: {:?}", state);

                commands.spawn(InsertDownloadtaskTask::new(
                    db.clone(),
                    runtimer.as_mut(),
                    downloadtask::DownloadtaskActiveModel {
                        id,
                        type_id: ActiveValue::Set(r#type.to_string()),
                        generic_id: ActiveValue::Set(generic_id),
                        state,
                    },
                ));
            }

            Some(help) if help.to_lowercase().eq(HELP) => {
                info!("\n{}", HELP_DOWNLOADTASK);
            }

            Some(unkown) => {
                error!("not has this command: {:?}", unkown);
            }

            None => {
                // 输出help
                commands.spawn(ListDownloadtaskTask::new(db.clone(), runtimer.as_mut()));
            }
        }
    }
}

pub fn download_rule_insert_task(
    mut commands: Commands,
    query: Query<(&mut InsertDownloadtaskTask, Entity)>,
) {
    for (mut task, entity) in query {
        if !task.is_finished() {
            continue;
        }

        match task.try_result() {
            Ok(result) => {
                info!("insert a task id<{}>", result);
            }
            Err(err) => {
                error!("insert a task error: {:?}", err);
            }
        }

        commands.entity(entity).despawn();
    }
}

pub fn related_downloadtask_medias_task(
    mut commands: Commands,
    query: Query<(&mut RelatedDownloadtaskMediasTask, Entity)>,
) {
    for (mut task, entity) in query {
        if !task.is_finished() {
            continue;
        }

        match task.try_result() {
            Ok(_result) => {
                info!("related downloadtask with medias");
            }
            Err(err) => {
                error!("insert a task error: {:?}", err);
            }
        }

        commands.entity(entity).despawn();
    }
}
