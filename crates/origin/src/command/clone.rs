use std::{mem, time::Duration};

use bevy::{
    app::{Plugin, PostUpdate, PreUpdate, Update},
    ecs::{
        entity::Entity,
        message::MessageReader,
        system::{Commands, Query, Res, ResMut},
    },
};
use bevy_tokio_tasks::TokioTasksRuntime;
use indicatif::{MultiProgress, ProgressDrawTarget};
use tracing::{error, info};

use crate::{
    command::{HELP, downloadrule::ActiveDownloadrule},
    components::{
        account::handle::ActiveAccounts,
        download::{
            DownloadFileError, DownloadFileErrorKind, DownloadHandle, DownloadPendding,
            MediaBvidOrAid,
        },
        downloadtask::handle::DownloadtaskRelatedMediaPending,
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
        app.add_systems(PreUpdate, upsert_download_list)
            .add_systems(Update, spawn_download_task)
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

                let way = MediaBvidOrAid::new(media_id.to_string()).parse();
                runtimer.spawn_background_task(move |mut ctx| async move {
                    let timer = tokio::time::timeout(Duration::from_secs(3), async {
                        let infomation = way.to_response().await?;

                        infomation
                            .data
                            .ok_or(DownloadFileError::new(DownloadFileErrorKind::MediaPage))
                    });

                    let Ok(result) = timer.await else {
                        error!(
                            "add a download media<{}> task error, time response overflow",
                            way.as_str()
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

                                    world.world.spawn(DownloadtaskRelatedMediaPending {
                                        media_id: media.aid,
                                        taskid: vec![],
                                    });
                                })
                                .await;
                        }
                        Err(err) => {
                            error!(
                                "add a download media<{}> task error: {:?}",
                                way.as_str(),
                                err
                            );
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
    mut active_account: ResMut<ActiveAccounts>,
    query_handle: Query<&DownloadHandle>,
    active_status: ResMut<ActiveStatus>,
    active_downloadrule: ResMut<ActiveDownloadrule>,
    status_related_downloadrule: ResMut<StatusRelatedDownloadrule>,

    mut pending_lists: Query<(&mut DownloadtaskRelatedMediaPending, Entity)>,
) {
    if pending_lists.count() <= 0 {
        return;
    }

    let count = query_handle.count();

    let Some(sub) = 4usize.checked_sub(count).filter(|sub| *sub != 0) else {
        return;
    };

    let Some(cookies) = active_account.get_first_cookies_mut() else {
        error!("not any active account, fetch media failed");
        return;
    };

    let bars = MultiProgress::with_draw_target(ProgressDrawTarget::stderr());

    let mut iter = pending_lists.iter_mut();

    let pendings = (0..sub).map(|_| iter.next()).filter_map(|pending| pending);

    for (mut list, entity) in pendings {
        info!("spawn a download handle");

        let list = mem::take(list.as_mut());

        commands.spawn(DownloadHandle::new(
            db.clone(),
            bars.clone(),
            cookies,
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
