use bevy::{
    app::{Plugin, Update},
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
        auth::handle::ActiveAccounts,
        download::DownloadWay,
        fetch::task::{
            FetchAccountCollectionIdData, FetchAccountFollowingData, FetchCollectMediasData,
            FetchMediaData, FetchUpperCollectionData, FetchUpperFollowingData,
            FetchUpperMediasData,
        },
    },
    console::ConsoleTrims,
    db::Db,
};

pub const HELP_FETCH: &str = r#"
Back up your favorite bilibili online resources with RESP.

Usage: fetch <COMMAND> [SUB_COMMAND] [OPTIONS] 

Commands:
    account                     Fetch data related to a login account.
        followings                  Fetch account's followed list.
        collections                 Fetch account's collections list.

    upper                       Fetch data related to an Upper.
        followings                  Fetch upper's followed list.
        collections                 Fetch upper's collection list.
        medias                      Fetch upper's media list.

    collection                  Fetch data related to a Collection.
        medias                      Fetch collection's media list.
    
    media                       Fetch data related to a single media.
    
    help                        Print this message or the help of the given subcommand(s)

Options:
    -v,         --verbose           Show debug messages
    -h,         --help              Print help
    -V,         --version           Print version
    -id [ID],   --id [ID]           Point ID

Example:
    fetch account followings                     # uses active account ID
    fetch account followings --id 123456         # override account ID
    fetch upper followings
"#;

const FETCH_COMMAND_INDEX: usize = 2;
const FETCH_SUBCOMMAND_INDEX: usize = 3;

pub struct CommandFetchPlugin;

impl Plugin for CommandFetchPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            Update,
            (
                spawn_fetch_task,
                (
                    fetch_account_following_data,
                    fetch_account_collection_id_data,
                    fetch_upper_following_data,
                    fetch_upper_collection_data,
                    fetch_upper_medias_data,
                    fetch_collect_medias_data,
                    fetch_media_data,
                )
                    .after(spawn_fetch_task),
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
            active_account.ids_mut().into_iter().collect::<Vec<_>>()
        } else {
            point_id
        };

        if ids.is_empty() {
            error!("not any point id or active account");
            continue;
        }

        let Ok(accounts) = active_account.try_result() else {
            error!("not any active account");
            continue;
        };

        match args.get(FETCH_COMMAND_INDEX).map(String::as_str) {
            // 上述有问题
            Some("account") => {
                let accounts = accounts
                    .iter()
                    .filter(|model| ids.contains(&model.account_id));

                match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
                    Some("followings") => {
                        commands.spawn_batch(
                            accounts
                                .map(|account| {
                                    FetchAccountFollowingData::new(
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
                            accounts
                                .map(|account| {
                                    FetchAccountCollectionIdData::new(
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
                }
            }
            Some("upper") => {
                let cookies = accounts.first().map(|model| model.cookies.clone());

                match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
                    Some("followings") => {
                        commands.spawn_batch(
                            ids.into_iter()
                                .map(|id| {
                                    info!("Fetching upper following with id<{}>", id);

                                    FetchUpperFollowingData::new(
                                        db.clone(),
                                        id,
                                        runtimer.as_mut(),
                                        cookies.clone(),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        );
                    }
                    Some("collections") => {
                        commands.spawn_batch(
                            ids.into_iter()
                                .map(|id| {
                                    FetchUpperCollectionData::new(
                                        db.clone(),
                                        runtimer.as_mut(),
                                        id,
                                        cookies.clone(),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        );
                    }

                    Some("medias") => {
                        let Ok(result) = active_account.try_result() else {
                            continue;
                        };

                        let Some(cookies) = result.first().map(|first| first.cookies.clone())
                        else {
                            error!("not any active account");
                            continue;
                        };

                        commands.spawn_batch(
                            ids.into_iter()
                                .map(|id| {
                                    FetchUpperMediasData::new(
                                        db.clone(),
                                        id,
                                        runtimer.as_mut(),
                                        cookies.clone(),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        );
                    }

                    Some(unkown) => {
                        error!("not has this command: {:?}", unkown);
                    }
                    None => {}
                }
            }
            Some("collection") => match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
                Some("medias") => {
                    let _cookies = accounts.iter().map(|model| {
                        info!(
                            "Fetching collection ids<{:?}> with account<{:?}>",
                            ids, model.account_id
                        );
                        model.cookies.clone()
                    });

                    commands.spawn_batch(
                        ids.into_iter()
                            .map(|id| {
                                FetchCollectMediasData::new(db.clone(), id, runtimer.as_mut())
                            })
                            .collect::<Vec<_>>(),
                    );
                }
                Some(unkown) => {
                    error!("not has this command: {:?}", unkown);
                }
                None => {}
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
                                    FetchMediaData::new(
                                        db.clone(),
                                        DownloadWay(bvid),
                                        runtimer.as_mut(),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        );
                    }
                }
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

pub fn fetch_upper_collection_data(
    mut commands: Commands,
    query: Query<(&mut FetchUpperCollectionData, Entity)>,
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

pub fn fetch_account_collection_id_data(
    mut commands: Commands,
    query: Query<(&mut FetchAccountCollectionIdData, Entity)>,
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

pub fn fetch_upper_following_data(
    mut commands: Commands,
    query: Query<(&mut FetchUpperFollowingData, Entity)>,
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

pub fn fetch_account_following_data(
    mut commands: Commands,
    query: Query<(&mut FetchAccountFollowingData, Entity)>,
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

pub fn fetch_collect_medias_data(
    mut commands: Commands,
    query: Query<(&mut FetchCollectMediasData, Entity)>,
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

pub fn fetch_media_data(mut commands: Commands, query: Query<(&mut FetchMediaData, Entity)>) {
    for (mut handle, entity) in query {
        if !handle.is_finished() {
            continue;
        }

        commands.entity(entity).try_despawn();

        match handle.try_result() {
            Ok((cid, state)) => {
                info!("fetch media<{}> is finished,state:{}", cid.0, state);
            }
            Err(err) => {
                error!("fetch upper collection error:{:?}", err);
            }
        }
    }
}

pub fn fetch_upper_medias_data(
    mut commands: Commands,
    query: Query<(&mut FetchUpperMediasData, Entity)>,
) {
    for (mut handle, entity) in query {
        if !handle.is_finished() {
            continue;
        }

        commands.entity(entity).try_despawn();

        match handle.try_result() {
            Ok((cid, state)) => {
                info!("fetch upper<{}>'s medias is finished,state:{}", cid, state);
            }
            Err(err) => {
                error!("fetch upper medias error:{:?}", err);
            }
        }
    }
}
