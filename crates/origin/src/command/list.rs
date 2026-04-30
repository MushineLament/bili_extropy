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
        download::DownloadHandle,
        initialize::DbInitailizeComponent as _,
        list::handle::{
            ListAccountCollectionsTask, ListAccountFollwedTask, ListAccountTask,
            ListCollectionMediasTask, ListCollectionTask, ListMediasTask, ListUppersTask,
        },
    },
    console::ConsoleTrims,
    db::Db,
    table::ToTable,
};

pub const OPTIONS_INDEX: usize = 2;
pub const ID_INDEX: usize = 3;
pub const COMMAND_INDEX: usize = 4;

pub struct CommandListPlugin;

impl Plugin for CommandListPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(PostStartup, ListMediasTask::new.to_system())
            .add_systems(
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
) {
    for message in console_message.read() {
        let _db = db.clone();
        let ConsoleTrims { args, argv: _ } = message;

        if !args.get(1).is_some_and(|list| list.eq("list")) {
            continue;
        }

        match args.get(2).map(String::as_str) {
            Some("upper") => {
                commands.spawn(ListUppersTask::new(db.clone(), runtimer.as_mut()));
            }
            Some("collection") => {
                commands.spawn(ListCollectionTask::new(db.clone(), runtimer.as_mut()));
            }
            Some("account") => match args.get(3).map(String::as_str) {
                Some("collection") => {
                    commands.spawn(ListAccountCollectionsTask::new(
                        db.clone(),
                        runtimer.as_mut(),
                    ));
                }
                Some("following") => {
                    commands.spawn(ListAccountFollwedTask::new(db.clone(), runtimer.as_mut()));
                }
                Some(unkown) => {
                    error!("not has this command: {:?}", unkown);
                }
                None => {
                    commands.spawn(ListAccountTask::new(db.clone(), runtimer.as_mut()));
                }
            },
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
