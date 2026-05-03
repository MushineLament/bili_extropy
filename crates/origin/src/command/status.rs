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

Usage: status <COMMAND> [OPTIONS] 

Commands:
    insert                     insert a download media path.
        <FOLDER_NAME> <PATH>       dowload media into PATH's folder.

Options:
    -v,         --verbose           Show debug messages
    -h,         --help              Print help
    -V,         --version           Print version
    -id [ID],   --id [ID]           Point ID

Example:
    fetch account followings                     # uses active account ID
    status add folder_name
    status add folder_name .temp
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

                    let state = if argv.contains_key("exclusive") {
                        ActiveValue::Set(StatusState::Exclusive.to_string())
                    } else if argv.contains_key("active") {
                        ActiveValue::Set(StatusState::Active.to_string())
                    } else {
                        ActiveValue::NotSet
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
