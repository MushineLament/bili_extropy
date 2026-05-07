use std::sync::Arc;

use bevy::{
    app::{Plugin, PostStartup, PreUpdate, Update},
    ecs::{
        message::MessageReader,
        system::{Commands, Query, Res, ResMut},
    },
    platform::collections::HashMap,
};
use bevy_tokio_tasks::TokioTasksRuntime;
use sea_orm::ActiveValue;
use tracing::{error, info};

use crate::{
    command::HELP,
    components::{
        initialize::DbInitailizeComponent as _,
        list::handle::ListStatusTask,
        status::handle::{
            ActiveStatus, InsertStatusRelatedDownloadruleTask, LoadStatusRelatedDownloadruleTask,
            LoadStatusTask, StatusInsertTask, StatusRelatedDownloadrule, StatusState,
        },
    },
    console::ConsoleTrims,
    db::Db,
    entity::status,
};

pub const HELP_STATUS: &str = r#"
Back up your favorite bilibili online resources with RESP.

Usage: status [COMMAND] [OPTIONS]

Commands:
    insert <FOLDER_NAME> [PATH] [OPTIONS]   Insert a download media path.
    help                                    Print this help message.

If no command is given, all status entries will be listed.

Options (for insert):
    --state <Active|Inactive>       Set the status state. If omitted, state is not set.
    --id <ID>                       Specify a status entry ID to update an existing record.
    --downloadrule <RULE_ID>        Associate one or more download rules by their IDs
                                    (e.g. --downloadrule 1).

Common Options:
    -v, --verbose                   Show debug messages
    -h, --help                      Print help (alias: status help)
    -V, --version                   Print version

Examples:
    status insert my_folder
    status insert my_folder ./downloads --state Active
    status insert my_folder ./downloads --state Active --id 42 --downloadrule 1
    status --help
"#;

const STATUS_COMMAND_INDEX: usize = 2;
// const STATUS_SUBCOMMAND_INDEX: usize = 3;

pub struct CommandStatusPlugin;

impl Plugin for CommandStatusPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<ActiveStatus>()
            .init_resource::<StatusRelatedDownloadrule>()
            .add_systems(
                PostStartup,
                (
                    LoadStatusTask::new.to_system(),
                    LoadStatusRelatedDownloadruleTask::new.to_system(),
                ),
            )
            .add_systems(PreUpdate, spawn_status_task)
            .add_systems(
                Update,
                (active_status, load_status_related_downloadrule_task),
            );
    }
}

pub fn spawn_status_task(
    mut commands: Commands,
    db: Res<Db>,
    mut console_message: MessageReader<ConsoleTrims>,
    mut runtimer: ResMut<TokioTasksRuntime>,
) {
    for message in console_message.read() {
        let db = db.clone();

        let ConsoleTrims { args, argv } = message;

        if !args.get(1).is_some_and(|list| list.eq("status")) {
            continue;
        }

        match args.get(STATUS_COMMAND_INDEX).map(String::as_str) {
            Some("insert") => match args.get(3).map(String::as_str) {
                Some(folder_name) => {
                    let path = args
                        .get(4)
                        .map(|path| ActiveValue::set(path.clone()))
                        .unwrap_or(ActiveValue::NotSet);

                    let state = match message.get_first_state() {
                        Some("Active") => ActiveValue::Set(StatusState::Active.to_string()),
                        Some("Inactive") => ActiveValue::Set(StatusState::Inactive.to_string()),
                        Some(unkonw) => ActiveValue::Set(unkonw.to_string()),
                        None => ActiveValue::NotSet,
                    };

                    info!(
                        "spawn a insert status task, name<{:?}>, path<{:?}> ,state<{:?}>",
                        folder_name, path, state
                    );

                    let id = message
                        .ids()
                        .into_iter()
                        .filter_map(|id| id.parse::<i64>().ok())
                        .next()
                        .map(|id| ActiveValue::Set(id))
                        .unwrap_or(ActiveValue::NotSet);

                    commands.spawn(StatusInsertTask::new(
                        db.clone(),
                        runtimer.as_mut(),
                        status::StatusActiveModel {
                            id: id.clone(),
                            name: ActiveValue::Set(folder_name.to_owned()),
                            path,
                            state,
                        },
                    ));

                    commands.spawn(LoadStatusTask::new(db.clone(), runtimer.as_mut()));

                    if let Some(relation) = argv.get("downloadrule") {
                        let related = relation
                            .iter()
                            .filter_map(|rule_id| rule_id.parse::<i64>().ok())
                            .map(|rule_id| {
                                InsertStatusRelatedDownloadruleTask::new(
                                    db.clone(),
                                    runtimer.as_mut(),
                                    id.clone(),
                                    ActiveValue::Set(rule_id),
                                )
                            })
                            .collect::<Vec<_>>();

                        commands.spawn_batch(related);
                    }
                }
                None => {
                    error!("not a vaild folder <name>");
                    continue;
                }
            },
            Some(help) if help.to_lowercase().eq(HELP) => {
                info!("\n{}", HELP_STATUS);
            }
            Some(unkown) => {
                error!("not has this command: {:?}", unkown);
            }
            None => {
                commands.spawn(ListStatusTask::new(db.clone(), runtimer.as_mut()));
            }
        }
    }
}

pub fn active_status(mut res: ResMut<ActiveStatus>, query: Query<&mut LoadStatusTask>) {
    for mut task in query {
        let Ok(result) = task.try_result() else {
            continue;
        };

        res.0 = Arc::new(
            result
                .iter()
                .filter(|status| status.state == StatusState::Active.to_string())
                .map(|model| (model.id, model.clone()))
                .collect::<HashMap<_, _>>(),
        );
    }
}

pub fn load_status_related_downloadrule_task(
    mut res: ResMut<StatusRelatedDownloadrule>,
    query: Query<&mut LoadStatusRelatedDownloadruleTask>,
) {
    for mut task in query {
        let Ok(result) = task.try_result() else {
            continue;
        };

        res.0 = Arc::new(
            result
                .iter()
                .map(|model| (model.status_id, model.rule_id))
                .collect(),
        );
    }
}
