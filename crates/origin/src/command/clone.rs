use std::{mem, time::Duration};

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
    command::{HELP, downloadrule::ActiveDownloadrule},
    components::{
        auth::handle::ActiveAccounts,
        download::{
            DownloadFileError, DownloadFileErrorKind, DownloadHandle, DownloadPendding, DownloadWay,
        },
        downloadtask::handle::DownloadRelatedTaskId,
        status::handle::{ActiveStatus, StatusRelatedDownloadrule},
    },
    console::ConsoleTrims,
    db::Db,
};

pub const HELP_CLONE: &str = r#"
Back up your favorite bilibili online resources with RESP.

Usage: clone <COMMAND> [SUB_COMMAND] [OPTIONS]

Commands:
    media                       Download media.
        <BvId>                  Download Single media.

Options:
    -v,         --verbose       Show debug messages
    -h,         --help          Print help
    -V,         --version       Print version

Example:
    clone media BV1dWcoe2EdF
    clone media 113844248642487
"#;

pub const CLONE_COMMAND_INDEX: usize = 2;
pub const CLONE_SUBCOMMAND_INDEX: usize = 3;
pub const CLONE_OPTION_INDEX: usize = 4;

pub struct DownloadPlugin;

impl Plugin for DownloadPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
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

        match args.get(CLONE_COMMAND_INDEX).map(String::as_str) {
            Some("media") => {
                let Some(media_id) = args.get(CLONE_SUBCOMMAND_INDEX).map(String::as_str) else {
                    error!("not is a media's bvid or aid");
                    continue;
                };

                let way = DownloadWay::new(media_id.to_string());
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

                                    world.world.spawn(DownloadRelatedTaskId {
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
            Some(help) if help.to_lowercase().eq(HELP) => {
                info!("/n{}", HELP_CLONE);
            }
            Some(unkown) => {
                error!("not has this command: {:?}", unkown);
            }
            None => {
                info!("/n{}", HELP_CLONE);
            }
        }
    }
}

pub fn spawn_download_task(
    mut commands: Commands,
    db: Res<Db>,
    mut runtimer: ResMut<TokioTasksRuntime>,
    mut accounts: ResMut<ActiveAccounts>,
    query_handle: Query<&DownloadHandle>,
    active_status: ResMut<ActiveStatus>,
    active_downloadrule: ResMut<ActiveDownloadrule>,
    status_related_downloadrule: ResMut<StatusRelatedDownloadrule>,
    lists: Query<(&mut DownloadRelatedTaskId, Entity)>,
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

    for (mut list, entity) in lists {
        info!("spawn a download handle");

        let list = mem::take(list.as_mut());

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

        commands.entity(entity).try_despawn();
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
