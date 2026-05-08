use bevy::{
    app::{Plugin, PreUpdate, Update},
    ecs::{
        entity::Entity,
        message::MessageReader,
        query::With,
        system::{Commands, Query, Res, ResMut},
    },
};
use bevy_tokio_tasks::TokioTasksRuntime;
use tracing::{error, info};

use crate::{
    components::{
        account::{handle::ActiveAccounts, load::LoadAccountCollectionsTask},
        download::DownloadHandle,
        downloadrule::load::LoadDownloadrule,
        downloadtask::load::{
            LoadDownloadtask, LoadDownloadtaskMedias, LoadDownloadtaskMediasPendingDownloadTask,
            LoadDownloadtaskRelatedMedias,
        },
        fetch::handle::LoadUpperMediasTask,
        list::{
            handle::{
                ListAccountFollwedTask, ListAccountTask, ListCollectionMediasTask,
                ListCollectionTask, ListTask,
            },
            load::LoadMediasTask,
        },
        status::handle::{LoadStatusRelatedDownloadruleTask, LoadStatusTask},
        upper::load::LoadUppersTask,
    },
    console::ConsoleTrims,
    db::Db,
    table::ToTable,
};

pub const HELP_LIST: &str = r#"
Back up your favorite bilibili online resources with RESP.

Usage: list <COMMAND> [SUB_COMMAND] [OPTIONS]

Commands:
    account                     List data related to a login account.
        followings                  List account's followeds.
        collections                 List account's collections.

    upper                       List data related to an Upper.
        followings [--id]                 List upper's followeds. If not point id, default use account's id.
        collections [--id]                List upper's collections. If not point id, default use account's id.
        medias [--id]                     List upper's medias. If not point id, default use account's id.

    collection                  List data related to a Collection.
        medias [--id]                      List collection's medias. If not point id, default use account's id.
    
    media                       List data related to a single media.
    
    help                        Print this.

Options:
    -v,         --verbose       Show debug messages
    -h,         --help          Print help
    -V,         --version       Print version
    -id [ID],   --id [ID]       Point ID

Example:
    List medias                 #List all medias
    List account                #List all account
    List upper followings       #List upper followings, Cause not point [--id], list all account followings.
"#;

pub const LIST_COMMAND_INDEX: usize = 2;
pub const LIST_SUBCOMMAND_INDEX: usize = 3;
pub const LIST_OPTION_INDEX: usize = 4;

pub struct CommandListPlugin;

impl Plugin for CommandListPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(PreUpdate, spawn_list_task).add_systems(
            Update,
            (
                list_account_task,
                list_collection_task,
                list_account_collections_task,
                list_account_follwed_task,
                list_uppers_task,
                list_collection_medias_task,
                list_medias,
                list_download_rule_task,
                list_status_related_downloadrule_task,
                list_status_task,
                list_downloadtask_task,
                list_downloadtask_medias_task,
                list_upper_medias_task,
                list_downloadtask_related_medias_task,
                list_downloadtask_medias_pending_download_task,
            ),
        );
    }
}

pub fn spawn_list_task(
    mut commands: Commands,
    db: Res<Db>,
    mut runtimer: ResMut<TokioTasksRuntime>,
    mut console_message: MessageReader<ConsoleTrims>,
    query: Query<&mut DownloadHandle>,
    mut active_account: ResMut<ActiveAccounts>,
) {
    for message in console_message.read() {
        let _db = db.clone();
        let ConsoleTrims { args, argv: _ } = message;

        if !args.get(1).is_some_and(|list| list.eq("list")) {
            continue;
        }

        let _ids = if message.is_empty_ids() {
            active_account.ids_mut().into_iter().collect::<Vec<_>>()
        } else {
            message
                .ids()
                .into_iter()
                .filter_map(|str| str.parse::<i64>().ok())
                .collect::<Vec<_>>()
        };

        match args.get(LIST_COMMAND_INDEX).map(String::as_str) {
            Some("account") => match args.get(LIST_SUBCOMMAND_INDEX).map(String::as_str) {
                Some("followings") => {
                    commands.spawn(ListAccountFollwedTask::new(db.clone(), runtimer.as_mut()));
                }
                Some("collections") => {
                    commands.spawn((
                        LoadAccountCollectionsTask::new(db.clone(), runtimer.as_mut()),
                        ListTask,
                    ));
                }
                Some(unkown) => {
                    error!("not has this command: {:?}", unkown);
                }
                None => {
                    commands.spawn(ListAccountTask::new(db.clone(), runtimer.as_mut()));
                }
            },
            Some("upper") => {
                // todo
                match args.get(LIST_SUBCOMMAND_INDEX).map(String::as_str) {
                    Some("medias") => {
                        commands.spawn((
                            LoadUpperMediasTask::new(db.clone(), runtimer.as_mut()),
                            ListTask,
                        ));
                    }
                    Some("collection") => {
                        // commands.spawn(ListUpperCollectionTask::new(db.clone(), runtimer.as_mut()));
                        error!("this subcommand has not finished");
                    }
                    Some(unkown) => {
                        error!("not has this command: {:?}", unkown);
                    }
                    None => {
                        commands
                            .spawn((LoadUppersTask::new(db.clone(), runtimer.as_mut()), ListTask));
                    }
                }
            }
            Some("collection") => {
                commands.spawn(ListCollectionTask::new(db.clone(), runtimer.as_mut()));
            }
            Some("medias") => match args.get(LIST_SUBCOMMAND_INDEX).map(String::as_str) {
                Some("collection") => {
                    commands.spawn(ListCollectionMediasTask::new(db.clone(), runtimer.as_mut()));
                }
                Some(unkown) => {
                    error!("not has this command: {:?}", unkown);
                }
                None => {
                    commands.spawn((LoadMediasTask::new(db.clone(), runtimer.as_mut()), ListTask));
                }
            },

            Some("status") => match args.get(LIST_SUBCOMMAND_INDEX).map(String::as_str) {
                Some("downloadrule") => {
                    commands.spawn((
                        LoadStatusRelatedDownloadruleTask::new(db.clone(), runtimer.as_mut()),
                        ListTask,
                    ));
                }
                Some(unkown) => {
                    error!("not has this command: {:?}", unkown);
                }
                None => {
                    commands.spawn((LoadStatusTask::new(db.clone(), runtimer.as_mut()), ListTask));
                }
            },
            Some("downloadrule") => match args.get(LIST_SUBCOMMAND_INDEX).map(String::as_str) {
                Some("status") => {
                    commands.spawn((
                        LoadStatusRelatedDownloadruleTask::new(db.clone(), runtimer.as_mut()),
                        ListTask,
                    ));
                }
                Some(unkown) => {
                    error!("not has this command: {:?}", unkown);
                }
                None => {
                    commands.spawn((
                        LoadDownloadrule::new(db.clone(), runtimer.as_mut()),
                        ListTask,
                    ));
                }
            },

            Some("task") => match args.get(LIST_SUBCOMMAND_INDEX).map(String::as_str) {
                Some("pendings") => {
                    commands.spawn((
                        LoadDownloadtaskMediasPendingDownloadTask::new(
                            db.clone(),
                            runtimer.as_mut(),
                        ),
                        ListTask,
                    ));
                }
                Some("related") => {
                    commands.spawn((
                        LoadDownloadtaskRelatedMedias::new(db.clone(), runtimer.as_mut()),
                        ListTask,
                    ));
                }
                Some("medias") => {
                    commands.spawn((
                        LoadDownloadtaskMedias::new(db.clone(), runtimer.as_mut()),
                        ListTask,
                    ));
                }
                Some(unkown) => {
                    error!("not has this command: {:?}", unkown);
                }
                None => {
                    commands.spawn((
                        LoadDownloadtask::new(db.clone(), runtimer.as_mut()),
                        ListTask,
                    ));
                }
            },

            Some("download") => {
                let count = query.count();
                info!("download count: {}", count);
            }

            Some(unkown) => {
                error!("not has this command: {:?}", unkown);
            }

            None => {
                // 输出help
            }
        }
    }
}

pub fn list_account_task(mut commands: Commands, query: Query<(&mut ListAccountTask, Entity)>) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result.iter().table_head(["account_id", "name", "state"]);
        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_account_collections_task(
    mut commands: Commands,
    query: Query<(&mut LoadAccountCollectionsTask, Entity), With<ListTask>>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result.iter().table_head(["collection_id", "account_id"]);
        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_collection_task(
    mut commands: Commands,
    query: Query<(&mut ListCollectionTask, Entity)>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result
            .iter()
            .table_head(["collection_id", "name", "count", "state"]);
        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_account_follwed_task(
    mut commands: Commands,
    query: Query<(&mut ListAccountFollwedTask, Entity)>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result.iter().table_head(["up_id", "account_id"]);
        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_uppers_task(
    mut commands: Commands,
    query: Query<(&mut LoadUppersTask, Entity), With<ListTask>>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result.iter().table_head(["up_id", "account_id"]);
        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_collection_medias_task(
    mut commands: Commands,
    query: Query<(&mut ListCollectionMediasTask, Entity)>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result.iter().table_head(["media_cid", "collection_id"]);
        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_medias(
    mut commands: Commands,
    query: Query<(&mut LoadMediasTask, Entity), With<ListTask>>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result
            .iter()
            .table_head(["id", "bvid", "title", "type", "state"]);
        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_download_rule_task(
    mut commands: Commands,
    query: Query<(&mut LoadDownloadrule, Entity), With<ListTask>>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result.iter().table_head([
            "id",
            "name",
            "size",
            "relation size",
            "date",
            "relation date",
            "repeat",
            "state",
        ]);
        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_status_related_downloadrule_task(
    mut commands: Commands,
    query: Query<(&mut LoadStatusRelatedDownloadruleTask, Entity), With<ListTask>>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result.iter().table_head(["status id", "rule id"]);
        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_status_task(
    mut commands: Commands,
    query: Query<(&mut LoadStatusTask, Entity), With<ListTask>>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result
            .iter()
            .table_head(["id", "folder name", "path", "state"]);
        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_downloadtask_task(
    mut commands: Commands,
    query: Query<(&mut LoadDownloadtask, Entity), With<ListTask>>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result
            .iter()
            .table_head(["id", "type", "related id", "state"]);
        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_downloadtask_medias_task(
    mut commands: Commands,
    query: Query<(&mut LoadDownloadtaskMedias, Entity), With<ListTask>>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result.iter().table_head(["task id", "media id", "state"]);
        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_upper_medias_task(
    mut commands: Commands,
    query: Query<(&mut LoadUpperMediasTask, Entity), With<ListTask>>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result.iter().table_head(["media id", "upper id"]);
        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_downloadtask_related_medias_task(
    mut commands: Commands,
    query: Query<(&mut LoadDownloadtaskRelatedMedias, Entity), With<ListTask>>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result.values().table_head(["related id", "task id"]);

        info!("\n{}\nrows: {}", table, table.count_rows() - 1);
    }
}

pub fn list_downloadtask_medias_pending_download_task(
    mut commands: Commands,
    query: Query<(&mut LoadDownloadtaskMediasPendingDownloadTask, Entity), With<ListTask>>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        for id in result {
            info!("downloadtask's pending state media id<{}>", id);
        }
    }
}
