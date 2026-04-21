use bevy::{
    app::{Plugin, PostUpdate, Update},
    ecs::{
        entity::Entity,
        message::MessageReader,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res, ResMut},
    },
};
use bevy_tokio_tasks::TokioTasksRuntime;
use indicatif::{MultiProgress, ProgressDrawTarget};
use tracing::{error, info};

use crate::{
    components::{
        auth::handle::ActiveAccounts,
        download::{DownloadHandle, DownloadList, DownloadWay},
        status::handle::ActiveStatus,
    },
    console::ConsoleTrims,
    db::Db,
};
pub struct DownloadPlugin;

impl Plugin for DownloadPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<DownloadList>()
            .add_systems(
                Update,
                (
                    upsert_download_list,
                    spawn_download_task.after(upsert_download_list),
                ),
            )
            .add_systems(PostUpdate, download_task_finished);
    }
}

pub fn upsert_download_list(
    mut console_message: MessageReader<ConsoleTrims>,
    mut lists: ResMut<DownloadList>,
) {
    for message in console_message.read() {
        let (args, _argv) = argmap::parse(message.0.iter());

        if !args.get(1).is_some_and(|list| list.eq("clone")) {
            continue;
        }

        match args.get(2).map(String::as_str) {
            Some(str) => {
                let way = DownloadWay::new(str);

                info!("add a download media<{:?}> task", way.0);
                lists.push(way);
            }
            None => {
                // 输出help
            }
        }
    }
}

pub fn spawn_download_task(
    mut commands: Commands,
    db: Res<Db>,
    mut runtimer: ResMut<TokioTasksRuntime>,
    mut lists: ResMut<DownloadList>,
    mut accounts: ResMut<ActiveAccounts>,
    query_handle: Query<&DownloadHandle>,
    mut active_status: ResMut<ActiveStatus>,
) {
    if lists.is_empty() {
        return;
    }

    let take_count = 4 - query_handle.count() as i8;

    if take_count <= 0 {
        return;
    }

    let bars = MultiProgress::with_draw_target(ProgressDrawTarget::stderr());

    let Some(account) = accounts
        .try_result()
        .ok()
        .and_then(|accounts| accounts.first())
    else {
        error!("No active account found. Please make sure login first.");
        return;
    };

    let take_count = if take_count as usize > lists.len() {
        lists.len()
    } else {
        take_count as usize
    };

    let status = match active_status.try_result() {
        Ok(result) => result,
        Err(err) => {
            error!("get active status error:{:?}", err);
            return;
        }
    };

    for list in lists.drain(0..take_count) {
        info!("spawn a download handle");
        commands.spawn(DownloadHandle::new(
            db.clone(),
            bars.clone(),
            &account.cookies,
            list,
            runtimer.as_mut(),
            status.clone(),
        ));
    }
}

pub fn download_task_finished(mut commands: Commands, query: Query<(&mut DownloadHandle, Entity)>) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        // whatever is fnished or error
        commands.entity(entity).try_despawn();

        match result {
            Ok(bvid) => {
                info!("download task handle finishied media<{:?}>", bvid);
            }
            Err(err) => {
                info!("download task handle error:{:?}", err);
            }
        }

        info!("download finishied");
    }
}
