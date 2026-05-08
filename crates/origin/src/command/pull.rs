use bevy::{
    app::{Plugin, PreUpdate, Update},
    ecs::{
        entity::Entity,
        message::MessageReader,
        query::With,
        system::{Commands, Query, Res, ResMut},
    },
    platform::collections::{HashMap, hash_map::Entry},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use tracing::error;

use crate::{
    components::{
        downloadtask::{handle::DownloadtaskRelatedMediaPending, load::LoadDownloadtaskMedias},
        pull::handle::PullTask,
    },
    console::ConsoleTrims,
    db::Db,
    entity::MediaAid,
};

pub const HELP_PULL: &str = r#"
Back up your favorite bilibili online resources with RESP.

Usage: pull <COMMAND> [SUB_COMMAND] [OPTIONS]

Commands:

Options:
    -v,         --verbose       Show debug messages
    -h,         --help          Print help
    -V,         --version       Print version
    -id [ID],   --id [ID]       Point ID

Example:
    pull 
"#;

pub const PULL_COMMAND_INDEX: usize = 2;

pub struct CommandPullPlugin;

impl Plugin for CommandPullPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(PreUpdate, spawn_list_task)
            .add_systems(Update, (pull_downloadtask_medias,));
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
        let ConsoleTrims { args, argv: _ } = message;

        if !args.get(1).is_some_and(|list| list.eq("pull")) {
            continue;
        }

        match args.get(PULL_COMMAND_INDEX).map(String::as_str) {
            Some(unkown) => {
                error!("not has this command: {:?}", unkown);
            }

            None => {
                // 输出help
                commands.spawn((
                    LoadDownloadtaskMedias::new(db.clone(), runtimer.as_mut()),
                    PullTask,
                ));
            }
        }
    }
}

pub fn pull_downloadtask_medias(
    mut commands: Commands,
    query: Query<(&mut LoadDownloadtaskMedias, Entity), With<PullTask>>,
) {
    for (mut result, entity) in query {
        if !result.is_finished() {
            continue;
        }

        match result.try_result() {
            Ok(result) => {
                let mut hash: HashMap<MediaAid, DownloadtaskRelatedMediaPending> = HashMap::new();

                for model in result {
                    match hash.entry(model.media_id) {
                        Entry::Occupied(occupied) => {
                            occupied.into_mut().taskid.push(model.task_id);
                        }
                        Entry::Vacant(vacant) => {
                            vacant.insert(DownloadtaskRelatedMediaPending {
                                media_id: model.media_id,
                                taskid: vec![model.task_id],
                            });
                        }
                    }
                }

                commands.spawn_batch(hash.into_values());
            }
            Err(error) => {
                error!("pull downloadtask's related medias error: {:?}", error);
            }
        }

        commands.entity(entity).try_despawn();
    }
}
