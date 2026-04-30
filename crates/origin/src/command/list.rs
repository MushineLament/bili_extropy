use std::num::IntErrorKind;

use bevy::{
    app::{Plugin, PostStartup, Update},
    ecs::{
        entity::Entity,
        message::MessageReader,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res, ResMut},
    },
};
use bevy_tokio_tasks::TokioTasksRuntime;
use tracing::{error, info};

use crate::{
    components::{
        auth::handle::ActiveAccounts, download::DownloadHandle, initialize::DbInitailizeComponent as _, list::handle::{
            ListAccountCollectionsTask, ListAccountFollwedTask, ListAccountTask,
            ListCollectionMediasTask, ListCollectionTask, ListMediasTask, ListUppersTask,
        }
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
        followings                  List upper's followeds.
        collections                 List upper's collections.
        medias                      List upper's medias.

    collection                  List data related to a Collection.
        medias                      List collection's medias.
    
    media                       List data related to a single media.
    
    help                        Print this message or the help of the given subcommand(s)

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

pub const LIST_COMMAND_INDEX: usize = 2;
pub const LIST_SUBCOMMAND_INDEX: usize = 3;
pub const LIST_OPTION_INDEX: usize = 4;

pub struct CommandListPlugin;

impl Plugin for CommandListPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            Update,
            (
                spawn_list_task,
                (
                    list_account_task,
                    list_collection_task,
                    list_account_collections_task,
                    list_account_follwed_task,
                    list_uppers_task,
                    list_collection_medias_task,
                    list_medias,
                )
                    .after(spawn_list_task),
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
        let ConsoleTrims { args, argv } = message;

        if !args.get(1).is_some_and(|list| list.eq("list")) {
            continue;
        }

        let point_id = argv
            .get("id")
            .iter()
            .map(|str| str.iter())
            .flatten()
            .filter_map(|str| str.parse::<i64>().ok())
            .collect::<Vec<_>>();

        let accounts = active_account.try_result();

        let ids = if point_id.is_empty() {
            accounts
                .as_ref()
                .ok()
                .iter()
                .map(|accounts| accounts.iter())
                .flatten()
                .map(|account| account.account_id)
                .collect::<Vec<_>>()
        } else {
            point_id
        };

        if ids.is_empty() {
            error!("not any point id or active account");
            continue;
        }

        match args.get(LIST_COMMAND_INDEX).map(String::as_str) {
            Some("account") => match args.get(LIST_SUBCOMMAND_INDEX).map(String::as_str) {
                Some("following") => {
                    commands.spawn(ListAccountFollwedTask::new(db.clone(), runtimer.as_mut()));
                }
                Some("collection") => {
                    commands.spawn(ListAccountCollectionsTask::new(
                        db.clone(),
                        runtimer.as_mut(),
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
                commands.spawn(ListUppersTask::new(db.clone(), runtimer.as_mut()));
            }
            Some("collection") => {
                commands.spawn(ListCollectionTask::new(db.clone(), runtimer.as_mut()));
            }
            Some("medias") => match args.get(3).map(String::as_str) {
                Some("collection") => {
                    commands.spawn(ListCollectionMediasTask::new(db.clone(), runtimer.as_mut()));
                }
                Some(unkown) => {
                    error!("not has this command: {:?}", unkown);
                }
                None => {
                    commands.spawn(ListMediasTask::new(db.clone(), runtimer.as_mut()));
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
    query: Query<(&mut ListAccountCollectionsTask, Entity)>,
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

pub fn list_uppers_task(mut commands: Commands, query: Query<(&mut ListUppersTask, Entity)>) {
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

pub fn list_medias(mut commands: Commands, query: Query<(&mut ListMediasTask, Entity)>) {
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
