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

use sea_orm::{ColumnTrait, QueryFilter};
use tracing::{error, info};

use crate::{
    command::HELP,
    components::{
        account::handle::ActiveAccounts,
        download::{MediaBvidOrAid, MediaUniqueId},
        downloadtask::load::LoadDownloadtaskRelatedMediasTask,
        fetch::{
            handle::FetchPendingMediaId,
            task::{
                FetchAccountCollectionIdTask, FetchAccountFollowingTask, FetchCollectMediasTask,
                FetchMediaTask, FetchUpperCollectionTask, FetchUpperFollowingTask,
                FetchUpperMediasTask,
            },
        },
        list::load::LoadMediasTask,
    },
    console::ConsoleTrims,
    db::Db,
    entity::media::{self},
};

pub const HELP_FETCH: &str = r#"
Back up your favorite bilibili online resources with RESP.

Usage: fetch <COMMAND> [SUB_COMMAND] [OPTIONS] 

Commands:
    account                     Fetch data related to a login account.
        followings                  Fetch account's followed list.
        collections                 Fetch account's collections list.

    upper                       Fetch data related to an Upper.
        followings [--id]           Fetch upper's followed list. If not point id, default use account's id. 
        collections [--id]          Fetch upper's collection list. If not point id, default use account's id. 
        medias [--id]               Fetch upper's media list. If not point id, default use account's id. 

    collection                  Fetch data related to a Collection.
        medias                      Fetch collection's media list.
    
    media                       Fetch data related media.
        <BvId>/<Aid>                Fetch data related to a single media.

    help                        Print this.

Options:
    -v,         --verbose           Show debug messages
    -h,         --help              Print help
    -V,         --version           Print version
    -id [ID],   --id [ID]           Point ID

Example:
    fetch account followings                     # uses active account ID
    fetch account followings --id 328853714         # override account ID
    fetch upper followings
    fetch media BV1dWcoe2EdF
"#;

const FETCH_COMMAND_INDEX: usize = 2;
const FETCH_SUBCOMMAND_INDEX: usize = 3;

pub struct CommandFetchPlugin;

impl Plugin for CommandFetchPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(PreUpdate, spawn_fetch_task).add_systems(
            Update,
            (
                fetch_account_following_task,
                fetch_account_collection_id_task,
                fetch_upper_following_task,
                fetch_upper_collection_task,
                fetch_upper_medias_task,
                fetch_collect_medias_task,
                fetch_media_task,
                fetch_pending_media_id,
            ),
        );
    }
}

pub fn spawn_fetch_task(
    mut console_message: MessageReader<ConsoleTrims>,
    db: Res<Db>,
    mut commands: Commands,
    mut runtimer: ResMut<TokioTasksRuntime>,
    mut active_account: ResMut<ActiveAccounts>,
) {
    if console_message.is_empty() {
        return;
    }

    let account_ids = active_account.ids_mut().into_iter().collect::<Vec<_>>();

    let Some(account_model) = active_account.get_first_models_mut().cloned() else {
        error!("not any active account");
        return;
    };

    let cookies = account_model.cookies.as_str();

    for message in console_message.read() {
        // let db = db.clone();
        let ConsoleTrims { args, argv: _ } = message;

        if !args.get(1).is_some_and(|list| list.eq("fetch")) {
            continue;
        }

        let point_id = message
            .ids()
            .into_iter()
            .filter_map(|str| str.parse::<i64>().ok())
            .collect::<Vec<_>>();

        let ids = if point_id.is_empty() {
            account_ids.clone()
        } else {
            point_id
        };

        if ids.is_empty() {
            error!("not any point id or active account");
            continue;
        }

        match args.get(FETCH_COMMAND_INDEX).map(String::as_str) {
            // 上述有问题
            Some("account") => match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
                Some("followings") => {
                    commands.spawn_batch(
                        active_account
                            .models_mut()
                            .into_iter()
                            .map(|account| {
                                FetchAccountFollowingTask::new(
                                    db.clone(),
                                    account.clone(),
                                    runtimer.as_mut(),
                                )
                            })
                            .collect::<Vec<_>>(),
                    );
                }
                Some("collections") => {
                    commands.spawn_batch(
                        active_account
                            .models_mut()
                            .into_iter()
                            .map(|account| {
                                FetchAccountCollectionIdTask::new(
                                    db.clone(),
                                    account.clone(),
                                    runtimer.as_mut(),
                                )
                            })
                            .collect::<Vec<_>>(),
                    );
                }
                Some(unkown) => {
                    error!("not has this command: {:?}", unkown);
                }
                None => {}
            },
            Some("upper") => match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
                Some("followings") => {
                    commands.spawn_batch(
                        ids.into_iter()
                            .map(|id| {
                                info!("Fetching upper following with id<{}>", id);

                                FetchUpperFollowingTask::new(
                                    db.clone(),
                                    id,
                                    runtimer.as_mut(),
                                    cookies.to_string(),
                                )
                            })
                            .collect::<Vec<_>>(),
                    );
                }
                Some("collections") => {
                    commands.spawn_batch(
                        ids.into_iter()
                            .map(|id| {
                                info!("spawn fetch upper<{}>'s collection task", id);
                                FetchUpperCollectionTask::new(
                                    db.clone(),
                                    runtimer.as_mut(),
                                    id,
                                    cookies.to_string(),
                                )
                            })
                            .collect::<Vec<_>>(),
                    );
                }

                Some("medias") => {
                    commands.spawn_batch(
                        ids.into_iter()
                            .map(|id| {
                                info!("spawn fetch upper<{}>'s medias task", id);
                                FetchUpperMediasTask::new(
                                    db.clone(),
                                    id,
                                    runtimer.as_mut(),
                                    cookies.to_string(),
                                )
                            })
                            .collect::<Vec<_>>(),
                    );
                }

                Some(unkown) => {
                    error!("not has this command: {:?}", unkown);
                }
                None => {}
            },
            Some("collection") => match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
                Some("medias") => {
                    commands.spawn_batch(
                        ids.into_iter()
                            .map(|id| {
                                FetchCollectMediasTask::new(
                                    db.clone(),
                                    id,
                                    runtimer.as_mut(),
                                    cookies.to_string(),
                                )
                            })
                            .collect::<Vec<_>>(),
                    );
                }
                Some(unkown) => {
                    error!("not has this command: {:?}", unkown);
                }
                None => {}
            },

            Some("task") => match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
                Some("medias") => {}
                Some("all") => {
                    info!("related downloadtask and medias");
                    commands.spawn(LoadDownloadtaskRelatedMediasTask::new(
                        db.clone(),
                        runtimer.as_mut(),
                    ));
                }
                Some(unkown) => {
                    error!("not has this subcommand: {:?}", unkown);
                }
                None => {
                    error!("not any subcommand");
                }
            },

            Some("media") => {
                let bvids = message
                    .ids()
                    .into_iter()
                    .map(|str| str.to_string())
                    .collect::<Vec<_>>();

                match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
                    // Some("collectionid") => {
                    //     commands.spawn(FetchCollectMediasData::new(
                    //         db.clone(),
                    //         id,
                    //         runtimer.as_mut(),
                    //     ));
                    // }
                    Some(unkown) => {
                        error!("not has this command: {:?}", unkown);
                    }
                    None => {
                        commands.spawn_batch(
                            bvids
                                .into_iter()
                                .map(|bvid| {
                                    FetchMediaTask::new(
                                        db.clone(),
                                        MediaBvidOrAid(bvid).parse(),
                                        runtimer.as_mut(),
                                        cookies.to_string(),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        );
                    }
                }
            }

            Some(help) if help.to_lowercase().eq(HELP) => {
                error!("\n{}", help);
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

pub fn fetch_upper_collection_task(
    mut commands: Commands,
    query: Query<(&mut FetchUpperCollectionTask, Entity)>,
) {
    for (mut handle, entity) in query {
        if !handle.is_finished() {
            continue;
        }

        commands.entity(entity).try_despawn();

        match handle.try_result() {
            Ok((cid, state)) => {
                info!(
                    "fetch upper<{:?}> collection is finished,state:{:?}",
                    cid, state
                );
            }
            Err(err) => {
                error!("fetch upper collection error:{:?}", err);
            }
        }
    }
}

pub fn fetch_account_collection_id_task(
    mut commands: Commands,
    query: Query<(&mut FetchAccountCollectionIdTask, Entity)>,
) {
    for (mut handle, entity) in query {
        if !handle.is_finished() {
            continue;
        }

        commands.entity(entity).try_despawn();

        match handle.try_result() {
            Ok((cid, state)) => {
                info!(
                    "fetch upper<{:?}> collection is finished,state:{:?}",
                    cid, state
                );
            }
            Err(err) => {
                error!("fetch upper collection error:{:?}", err);
            }
        }
    }
}

pub fn fetch_upper_following_task(
    mut commands: Commands,
    query: Query<(&mut FetchUpperFollowingTask, Entity)>,
) {
    for (mut handle, entity) in query {
        if !handle.is_finished() {
            continue;
        }

        commands.entity(entity).try_despawn();

        match handle.try_result() {
            Ok((cid, state)) => {
                info!(
                    "fetch upper<{:?}> collection is finished,state:{:?}",
                    cid, state
                );
            }
            Err(err) => {
                error!("fetch upper collection error:{:?}", err);
            }
        }
    }
}

pub fn fetch_account_following_task(
    mut commands: Commands,
    query: Query<(&mut FetchAccountFollowingTask, Entity)>,
) {
    for (mut handle, entity) in query {
        if !handle.is_finished() {
            continue;
        }

        commands.entity(entity).try_despawn();

        match handle.try_result() {
            Ok((cid, state)) => {
                info!(
                    "fetch upper<{:?}> collection is finished,state:{:?}",
                    cid, state
                );
            }
            Err(err) => {
                error!("fetch upper collection error:{:?}", err);
            }
        }
    }
}

pub fn fetch_collect_medias_task(
    mut commands: Commands,
    query: Query<(&mut FetchCollectMediasTask, Entity)>,
) {
    for (mut handle, entity) in query {
        if !handle.is_finished() {
            continue;
        }

        commands.entity(entity).try_despawn();

        match handle.try_result() {
            Ok((cid, state)) => {
                info!(
                    "fetch upper<{:?}> collection is finished,state:{:?}",
                    cid, state
                );
            }
            Err(err) => {
                error!("fetch upper collection error:{:?}", err);
            }
        }
    }
}

pub fn fetch_upper_medias_task(
    mut commands: Commands,
    query: Query<(&mut FetchUpperMediasTask, Entity)>,
    db: Res<Db>,
    mut runtimer: ResMut<TokioTasksRuntime>,
) {
    for (mut handle, entity) in query {
        if !handle.is_finished() {
            continue;
        }

        let upper_id = handle.upper_cid;
        let fetch_medias = handle.fetch_medias;

        commands.entity(entity).try_despawn();

        match handle.try_result() {
            Ok((cid, state)) => {
                info!(
                    "fetch upper<{}>'s medias is finished,state:{}",
                    upper_id, state
                );

                if fetch_medias {
                    let fetchs = cid
                        .iter()
                        .map(|media_id| {
                            let media_id = *media_id;
                            (
                                FetchPendingMediaId(media_id),
                                LoadMediasTask::new_with(
                                    db.clone(),
                                    runtimer.as_mut(),
                                    move |select| select.filter(media::Column::Aid.eq(media_id)),
                                ),
                            )
                        })
                        .collect::<Vec<_>>();

                    commands.spawn_batch(fetchs);
                }
            }
            Err(err) => {
                error!("fetch upper medias error:{:?}", err);
            }
        }
    }
}

pub fn fetch_media_task(mut commands: Commands, query: Query<(&mut FetchMediaTask, Entity)>) {
    for (mut handle, entity) in query {
        if !handle.is_finished() {
            continue;
        }

        commands.entity(entity).try_despawn();

        match handle.try_result() {
            Ok((cid, state)) => {
                info!("fetch media<{}> is finished,state:{}", *cid, state);
            }
            Err(err) => {
                error!("fetch upper collection error:{:?}", err);
            }
        }
    }
}

pub fn fetch_pending_media_id(
    mut commands: Commands,
    mut pending_fetchs: Query<(
        &mut FetchPendingMediaId,
        Option<&mut LoadMediasTask>, //check befor fetch
        Entity,
    )>,
    fetching: Query<(), With<FetchMediaTask>>,
    db: Res<Db>,
    mut runtimer: ResMut<TokioTasksRuntime>,
    mut active_account: ResMut<ActiveAccounts>,
) {
    if pending_fetchs.count() <= 0 {
        return;
    }

    let count = fetching.count();

    let Some(sub) = 4usize.checked_sub(count).filter(|sub| *sub != 0) else {
        return;
    };

    let Some(cookies) = active_account.get_first_cookies_mut() else {
        error!("not any active account, fetch media failed");
        return;
    };

    let mut iter = pending_fetchs.iter_mut();

    let pendings = (0..sub).map(|_| iter.next()).filter_map(|pending| pending);

    for (fetch, load, entity) in pendings {
        if let Some(mut result) = load {
            match result.try_result() {
                Ok(result) => {
                    if result.iter().find(|model| model.aid == fetch.0).is_some() {
                        commands.entity(entity).try_despawn();
                        continue;
                    }
                }
                Err(err) => {
                    if !err.is_finished() {
                        continue;
                    }
                }
            };
        }

        commands.spawn(FetchMediaTask::new(
            db.clone(),
            MediaUniqueId::Aid(fetch.0),
            runtimer.as_mut(),
            cookies.to_string(),
        ));
        commands.entity(entity).try_despawn();
    }

    if pending_fetchs.count() < 4 && fetching.count() < 4 {
        info!("fetch media taks all is finished");
    }
}
