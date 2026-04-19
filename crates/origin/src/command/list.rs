use bevy::{
    app::{Plugin, PostStartup, Update},
    ecs::{
        entity::Entity,
        message::MessageReader,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res, ResMut},
        world::World,
    },
};
use bevy_tokio_tasks::TokioTasksRuntime;
use tracing::error;

use crate::{
    components::{
        download::DownloadHandle,
        initialize::DbInitailizeResource,
        list::handle::{ListAccountTask, ListMedias},
    },
    console::ConsoleTrims,
    db::Db,
    table::ToTable,
};

pub struct CommandListPlugin;

impl Plugin for CommandListPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(PostStartup, ListMedias::new.to_system())
            .add_systems(
                Update,
                (
                    spawn_list_task,
                    list_account_finished.after(spawn_list_task),
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
    listmedias: Res<ListMedias>,
) {
    for message in console_message.read() {
        let _db = db.clone();
        let (args, _argv) = argmap::parse(message.0.iter());

        if !args.get(1).is_some_and(|list| list.eq("list")) {
            continue;
        }

        match args.get(2).map(String::as_str) {
            Some("account") => {
                commands.spawn(ListAccountTask::new(db.clone(), runtimer.as_mut()));
            }
            Some("medias") => {
                let table = match listmedias.get_result() {
                    Ok(result) => result
                        .iter()
                        .table_head(["id", "bvid", "title", "type", "state"]),
                    Err(err) => {
                        error!("list medias error:{:?}", err);
                        commands.queue(|world: &mut World| {
                            let _ = world.resource_mut::<ListMedias>().try_result();
                        });
                        continue;
                    }
                };

                println!("{}\nrows: {}", table, table.count_rows() - 1);
            }
            Some("download") => {
                let count = query.count();
                println!("count: {}", count);
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

pub fn list_account_finished(mut commands: Commands, query: Query<(&mut ListAccountTask, Entity)>) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        let table = result.iter().table_head(["account_id", "name", "state"]);
        println!("{}\nrows: {}", table, table.count_rows() - 1);
    }
}
