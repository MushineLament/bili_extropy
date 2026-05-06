use std::time::Duration;

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
    command::downloadrule::ActiveDownloadrule,
    components::{
        auth::handle::ActiveAccounts,
        download::{
            DownloadFileError, DownloadFileErrorKind, DownloadHandle, DownloadPendding, DownloadWay,
        },
        downloadtask::handle::{DownloadList, DownloadRelatedTaskId},
        status::handle::{ActiveStatus, StatusRelatedDownloadrule},
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
    runtimer: ResMut<TokioTasksRuntime>,
) {
    for message in console_message.read() {
        let ConsoleTrims { args, argv: _ } = message;

        if !args.get(1).is_some_and(|list| list.eq("clone")) {
            continue;
        }

        match args.get(2).map(String::as_str) {
            Some(str) => {
                let way = DownloadWay::new(str.to_string());
                runtimer.spawn_background_task(move |mut ctx| async move {
                    let timer = tokio::time::timeout(Duration::from_secs(3), async {
                        let infomation = way.to_response().await?;

                        infomation
                            .data
                            .ok_or(DownloadFileError::new(DownloadFileErrorKind::MediaPage))
                    });

                    let Ok(result) = timer.await else {
                        error!(
                            "add a download media<{:?}> task error, time response overflow",
                            way.0
                        );
                        return;
                    };

                    match result {
                        Ok(media) => {
                            info!("add a download media<{:?}> task", media.aid);
                            let _ = ctx
                                .run_on_main_thread(move |world| {
                                    let mut building = world.world.resource_mut::<ActiveAccounts>();
                                    if building.try_result().is_ok_and(|result| result.is_empty()) {
                                        error!(
                                            "No active account found. Please make sure login first."
                                        );
                                    }

                                    let mut downloadlist =
                                        world.world.resource_mut::<DownloadList>();

                                    downloadlist.push(DownloadRelatedTaskId {
                                        id: media.aid,
                                        taskid: vec![],
                                    });
                                })
                                .await;
                        }
                        Err(err) => {
                            error!("add a download media<{:?}> task error: {:?}", way.0, err);
                        }
                    }
                });
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
    active_status: ResMut<ActiveStatus>,
    active_downloadrule: ResMut<ActiveDownloadrule>,
    status_related_downloadrule: ResMut<StatusRelatedDownloadrule>,
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

    for list in lists.drain(0..take_count) {
        info!("spawn a download handle");
        commands.spawn(DownloadHandle::new(
            db.clone(),
            bars.clone(),
            &account.cookies,
            list,
            runtimer.as_mut(),
            active_status.0.clone(),
            active_downloadrule.0.clone(),
            status_related_downloadrule.0.clone(),
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
                error!("download task handle error:{:?}", err);
            }
        }

        info!("download finishied");
    }
}
