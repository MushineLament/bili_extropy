use anyhow::Context as _;
use api_req::ApiCaller as _;
use bevy::{
    app::{Plugin, Update},
    ecs::{
        change_detection::MaybeLocation,
        component::Component,
        entity::Entity,
        message::MessageReader,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    prelude::{Deref, DerefMut},
};

use bevy_tokio_tasks::TokioTasksRuntime;
use futures::StreamExt;
use migration::OnConflict;
use sea_orm::{EntityTrait, IntoActiveModel};
use tracing::{debug, error, info};

use crate::{
    api::BiliApi,
    components::{
        auth::handle::ActiveAccounts,
        download::{DownloadFileError, DownloadFileErrorKind, DownloadWay},
        handle::ECSHandleResult,
    },
    console::ConsoleTrims,
    db::Db,
    entity::{
        CollectionId, UpperCid,
        account::AccountModel,
        account_collection,
        collection::{
            self, InSetData, InSetResp, InUpData, InUpList, InUpResp, ListUpperCollectData,
            ListUpperCollectResp,
        },
        collection_media,
        media::{self, MediaInfoSingle},
        upper::{
            self, FollowingNumData, FollowingNumResp, FollowingUpData, FollowingUpResp,
            PublishNumData, PublishNumResp,
        },
        upper_account, upper_media,
    },
    payload::{
        FollowingNumPayload, FollowingUpPayload, InSetPayload, InUpPayload,
        ListUpperCollectPayload, PublishNumPayload,
    },
    state::{CollectionState, MediaState, UpState},
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
        let ConsoleTrims { args, argv } = message;

        if !args.get(1).is_some_and(|list| list.eq("fetch")) {
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

        match args.get(FETCH_COMMAND_INDEX).map(String::as_str) {
            // 上述有问题
            Some("account") => {
                let Ok(accounts) = accounts else {
                    error!("not any active account");
                    continue;
                };

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
                let cookies = accounts
                    .ok()
                    .and_then(|accounts| accounts.first())
                    .map(|model| model.cookies.clone());

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
                        commands.spawn_batch(
                            ids.into_iter()
                                .map(|id| {
                                    FetchUpperMediasData::new(db.clone(), id, runtimer.as_mut())
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
            Some("collection") => {
                match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
                    Some("medias") => {
                        let _cookies = accounts
                            .ok()
                            .iter()
                            .map(|accounts| accounts.iter())
                            .flatten()
                            // .find_or_first(|model| model.account_id == id)
                            .next()
                            .map(|model| {
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
                }
            }

            Some("media") => {
                let bvids = argv
                    .get("id")
                    .iter()
                    .map(|str| str.iter().map(String::as_str))
                    .flatten()
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

/// 更新数据库中的uppercid用户下的所有收藏夹信息
#[derive(Debug, Component, Deref, DerefMut)]
pub struct FetchUpperCollectionData(pub ECSHandleResult<(UpperCid, u64), anyhow::Error>);

impl FetchUpperCollectionData {
    pub fn new(
        db: Db,
        runtimer: &mut TokioTasksRuntime,
        cid: UpperCid,
        cookies: Option<String>,
    ) -> Self {
        let task = runtimer.spawn_background_task(move |_ctx| Self::task(db, cid, cookies));

        Self(ECSHandleResult::new(task))
    }

    pub async fn task(
        db: Db,
        cid: UpperCid,
        cookies: Option<String>,
    ) -> Result<(UpperCid, u64), anyhow::Error> {
        debug!("cookies:{:?}", cookies);
        if let Some(cookies) = cookies {
            crate::cookies::add_cookie_jar(crate::cookies::parse_cookies(&cookies));
        }

        let Ok(ListUpperCollectResp {
            data: ListUpperCollectData { list },
        }) = BiliApi::request(ListUpperCollectPayload { up_mid: cid }).await
        else {
            return Err(anyhow::anyhow!(
                "bilibili api request error,caller: {:?}",
                MaybeLocation::caller()
            ));
        };

        let state = collection::CollectionEntity::insert_many(list.into_iter().map(|info| {
            collection::CollectionModel {
                collection_id: info.id,
                name: info.title.to_owned(),
                count: info.media_count,
                state: CollectionState::Inactive.to_string(), // conflic skip
            }
            .into_active_model()
        }))
        .on_conflict(
            OnConflict::column(collection::Column::CollectionId)
                .update_columns([collection::Column::Name, collection::Column::Count])
                .to_owned(),
        )
        .exec_without_returning(&db.db)
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "insert many collection error: {:?}, account_id<{:?}>, caller:{:?}",
                err,
                cid,
                MaybeLocation::caller()
            )
        });

        state.map(|state| (cid, state))
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

/// 更新数据库中的登录账户与收藏夹id的对应关系
#[derive(Debug, Component, Deref, DerefMut)]
pub struct FetchAccountCollectionIdData(pub ECSHandleResult<(UpperCid, u64), anyhow::Error>);

impl FetchAccountCollectionIdData {
    #[track_caller]
    pub fn new(db: Db, model: AccountModel, runtimer: &mut TokioTasksRuntime) -> Self {
        let task: tokio::task::JoinHandle<Result<(i64, u64), anyhow::Error>> =
            runtimer.spawn_background_task(|_ctx| Self::task(db, model, MaybeLocation::caller()));

        Self(ECSHandleResult::new(task))
    }

    pub async fn task(
        db: Db,
        model: AccountModel,
        caller: MaybeLocation,
    ) -> Result<(UpperCid, u64), anyhow::Error> {
        crate::cookies::add_cookie_jar(crate::cookies::parse_cookies(&model.cookies));

        let account_id = model.account_id;

        info!("Fetching sets with account<{}>", model.name);

        let ListUpperCollectResp {
            data: ListUpperCollectData { list },
        } = BiliApi::request(ListUpperCollectPayload { up_mid: account_id }).await?;

        let state = account_collection::AccountCollectionEntity::insert_many(list.into_iter().map(
            |info| {
                account_collection::AccountCollectionModel {
                    collection_id: info.id,
                    account_id,
                }
                .into_active_model()
            },
        ))
        .on_conflict(
            OnConflict::columns([
                account_collection::Column::CollectionId,
                account_collection::Column::AccountId,
            ])
            .update_columns([
                account_collection::Column::CollectionId,
                account_collection::Column::AccountId,
            ])
            .to_owned(),
        )
        .exec_without_returning(&db.db)
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "insert many accountcollectid error: {:?}, account_id<{:?}>, caller:{:?}",
                err,
                account_id,
                caller
            )
        });

        state.map(|state| (account_id, state))
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

/// 获取uppercid用户关注的up列表
#[derive(Debug, Component, Deref, DerefMut)]
pub struct FetchUpperFollowingData(pub ECSHandleResult<(UpperCid, u64), anyhow::Error>);

impl FetchUpperFollowingData {
    #[track_caller]
    pub fn new(
        db: Db,
        cid: UpperCid,
        runtimer: &mut TokioTasksRuntime,
        cookies: Option<String>,
    ) -> Self {
        let task = runtimer.spawn_background_task(move |_ctx| {
            Self::task(db, cid, cookies, MaybeLocation::caller())
        });

        Self(ECSHandleResult::new(task))
    }

    pub async fn task(
        db: Db,
        cid: UpperCid,
        cookies: Option<String>,
        caller: MaybeLocation,
    ) -> Result<(UpperCid, u64), anyhow::Error> {
        if let Some(cookies) = cookies {
            crate::cookies::add_cookie_jar(crate::cookies::parse_cookies(&cookies));
        }

        info!("Fetching following with upper<{}>", cid);

        let FollowingNumResp {
            data: FollowingNumData { following },
        } = BiliApi::request(FollowingNumPayload { vmid: cid })
            .await
            .context("Failed to fetch following ups number")?;

        if following == 0 {
            return Ok((cid, 0));
        }

        let page = (following - 1) / 50 + 1;

        let mut tasks = futures::stream::iter(1..=page)
            .map(|pn| async move {
                let FollowingUpResp {
                    data: FollowingUpData { list },
                } = BiliApi::request(FollowingUpPayload {
                    vmid: cid,
                    pn,
                    ps: 50,
                })
                .await
                .context(format!("Failed to fetch following ups' page {pn}"))?;
                Ok::<_, anyhow::Error>(list)
            })
            .buffer_unordered(8);

        let mut ups = vec![];
        while let Some(res) = tasks.next().await {
            match res {
                Ok(list) => ups.extend(list),
                Err(e) => error!("{}", e),
            }
        }

        let state = upper::UpperEntity::insert_many(ups.iter().map(|up| {
            upper::UpperModel {
                upper_id: up.mid,
                name: up.name.to_owned(),
                state: UpState::Inactive.to_string(),
            }
            .into_active_model()
        }))
        .on_conflict(
            OnConflict::column(upper::Column::UpperId)
                .update_columns([upper::Column::UpperId, upper::Column::Name])
                .to_owned(),
        )
        .exec_without_returning(&db.db)
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "insert many accountcollectid error: {:?}, account_id<{:?}>, caller:{:?}",
                err,
                cid,
                caller
            )
        });

        state.map(|state| (cid, state))
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

/// 获取登录账户与关注uppercid的关系
#[derive(Debug, Component, Deref, DerefMut)]
pub struct FetchAccountFollowingData(pub ECSHandleResult<(UpperCid, u64), anyhow::Error>);

impl FetchAccountFollowingData {
    #[track_caller]
    pub fn new(db: Db, model: AccountModel, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = runtimer.spawn_background_task(|_ctx| Self::task(db, model));

        Self(ECSHandleResult::new(task))
    }
    pub async fn task(db: Db, model: AccountModel) -> Result<(UpperCid, u64), anyhow::Error> {
        crate::cookies::add_cookie_jar(crate::cookies::parse_cookies(&model.cookies));

        let account_id = model.account_id;

        info!("Fetching sets with account<{}>", model.name);

        let FollowingNumResp {
            data: FollowingNumData { following },
        } = BiliApi::request(FollowingNumPayload { vmid: account_id })
            .await
            .map_err(|err| {
                anyhow::anyhow!("Failed to fetch following ups number, error:{:?}", err)
            })?;

        if following == 0 {
            return Ok((account_id, 0));
        }

        let page = (following - 1) / 50 + 1;

        let mut tasks = futures::stream::iter(1..=page)
            .map(|pn| async move {
                let FollowingUpResp {
                    data: FollowingUpData { list },
                } = BiliApi::request(FollowingUpPayload {
                    vmid: account_id,
                    pn,
                    ps: 50,
                })
                .await
                .map_err(|err| {
                    anyhow::anyhow!("Failed to fetch following ups' page {pn},api:{:?}", err)
                })?;
                Ok::<_, anyhow::Error>(list)
            })
            .buffer_unordered(8);

        let mut ups = vec![];
        while let Some(res) = tasks.next().await {
            match res {
                Ok(list) => ups.extend(list),
                Err(e) => error!("{}", e),
            }
        }

        let state = upper_account::Entity::insert_many(ups.into_iter().map(|up| {
            upper_account::Model {
                upper_id: up.mid,
                account_id,
            }
            .into_active_model()
        }))
        .on_conflict(
            OnConflict::columns([
                upper_account::Column::UpperId,
                upper_account::Column::AccountId,
            ])
            .update_columns([
                upper_account::Column::UpperId,
                upper_account::Column::AccountId,
            ])
            .to_owned(),
        )
        .exec_without_returning(&db.db)
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "insert many accountcollectid error: {:?}, account_id<{:?}>, caller:{:?}",
                err,
                account_id,
                MaybeLocation::caller()
            )
        });

        state.map(|state| (account_id, state))
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

/// 获取收藏夹id下的所有mediacid
#[derive(Debug, Component, Deref, DerefMut)]
pub struct FetchCollectMediasData(pub ECSHandleResult<(CollectionId, u64), anyhow::Error>);

impl FetchCollectMediasData {
    #[track_caller]
    pub fn new(db: Db, id: CollectionId, runtimer: &mut TokioTasksRuntime) -> Self {
        let task =
            runtimer.spawn_background_task(move |_ctx| Self::task(db, id, MaybeLocation::caller()));

        Self(ECSHandleResult::new(task))
    }

    pub async fn task(
        db: Db,
        id: CollectionId,
        caller: MaybeLocation,
    ) -> Result<(CollectionId, u64), anyhow::Error> {
        let Some(model) = collection::CollectionEntity::find_by_id(id)
            .one(&db.db)
            .await?
        else {
            return Err(anyhow::anyhow!(
                "sql not has collection<{:?}> infomation",
                id
            ));
        };

        let page = (model.count - 1) / 20 + 1;

        let mut tasks = futures::stream::iter(1..=page)
            .map(|pn| async move {
                // 通过收藏夹id，获取视频的id
                let InSetResp {
                    data: InSetData { medias },
                } = BiliApi::request(InSetPayload {
                    media_id: model.collection_id,
                    pn,
                    ps: 20,
                })
                .await
                .context(format!("Failed to fetch sets' page {pn}"))?;
                Ok::<_, anyhow::Error>(medias)
            })
            .buffer_unordered(8);

        let mut medias = vec![];
        while let Some(res) = tasks.next().await {
            match res {
                Ok(list) => medias.extend(list),
                Err(e) => error!("caller: {:?},{}", (file!(), line!()), e),
            }
        }

        let state =
            collection_media::CollectionMediaEntity::insert_many(medias.into_iter().map(|m| {
                debug!("Linking media<{}> and set<{}>", m.title, model.name);
                collection_media::CollectionMediaModel {
                    media_cid: m.id,
                    collection_id: model.collection_id,
                }
                .into_active_model()
            }))
            .exec_without_returning(&db.db)
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "insert many accountcollectid error: {:?}, collectonid<{:?}>, caller:{:?}",
                    err,
                    model.collection_id,
                    caller
                )
            });

        state.map(|state| (model.collection_id, state))
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

/// 获取收藏夹id下的所有mediacid
#[derive(Debug, Component, Deref, DerefMut)]
pub struct FetchMediaData(pub ECSHandleResult<(DownloadWay, u64), anyhow::Error>);

impl FetchMediaData {
    #[track_caller]
    pub fn new(db: Db, id: DownloadWay, runtimer: &mut TokioTasksRuntime) -> Self {
        let task =
            runtimer.spawn_background_task(move |_ctx| Self::task(db, id, MaybeLocation::caller()));

        Self(ECSHandleResult::new(task))
    }

    pub async fn task(
        db: Db,
        id: DownloadWay,
        caller: MaybeLocation,
    ) -> Result<(DownloadWay, u64), anyhow::Error> {
        let Ok(MediaInfoSingle {
            code: _,
            data: Some(media),
            message: _,
        }) = id
            .clone()
            .response()
            .await
            .map_err(|err| DownloadFileError::new(DownloadFileErrorKind::ApiReq(err)))
        else {
            return Err(anyhow::anyhow!(
                "fetch single media error,id<{:?}>,caller{:?}",
                id.0,
                caller
            ));
        };

        let state = media::MediaEntity::insert(
            crate::entity::media::MediaModel {
                aid: media.aid,
                bv_id: media.bvid.to_owned(),
                title: media.title.to_owned(),
                r#type: media.r#type.to_string(),
                state: MediaState::Pending.to_string(),
                cid: media.cid,
                pic: None,
            }
            .into_active_model(),
        )
        .on_conflict(
            OnConflict::column(media::Column::Aid)
                .update_columns([
                    media::Column::BvId,
                    media::Column::Cid,
                    media::Column::Title,
                    media::Column::Type,
                ])
                .to_owned(),
        )
        .exec_without_returning(&db.db)
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "can't not upsert media<{:?}> in to table, error:{:?}",
                id,
                err
            )
        });

        state.map(|state| (id, state))
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

/// 获取upperid下的所有mediacid
#[derive(Debug, Component, Deref, DerefMut)]
pub struct FetchUpperMediasData(pub ECSHandleResult<(UpperCid, u64), anyhow::Error>);

impl FetchUpperMediasData {
    #[track_caller]
    pub fn new(db: Db, id: UpperCid, runtimer: &mut TokioTasksRuntime) -> Self {
        let task =
            runtimer.spawn_background_task(move |_ctx| Self::task(db, id, MaybeLocation::caller()));

        Self(ECSHandleResult::new(task))
    }

    pub async fn task(
        db: Db,
        cid: UpperCid,
        caller: MaybeLocation,
    ) -> Result<(UpperCid, u64), anyhow::Error> {
        let PublishNumResp {
            data: PublishNumData { video },
        } = BiliApi::request(PublishNumPayload { mid: cid }).await?;
        if video == 0 {
            return Ok((cid, 0));
        }

        // info!("Fetching published videos of up<{}>", up.name);
        let page = (video - 1) / 30 + 1;
        let mut tasks = futures::stream::iter(1..=page)
            .map(|pn| async move {
                let InUpResp {
                    data:
                        InUpData {
                            list: InUpList { vlist },
                        },
                } = BiliApi::request(InUpPayload::new(cid, pn, 30).await?)
                    .await
                    .map_err(|err| {
                        anyhow::anyhow!("Failed to fetch up space page {pn}, error: {:?}", err)
                    })?;
                Ok::<_, anyhow::Error>(vlist)
            })
            .buffer_unordered(8);

        let mut medias = vec![];

        while let Some(res) = tasks.next().await {
            match res {
                Ok(list) => medias.extend(list),
                Err(e) => error!("{}", e),
            }
        }

        let state = upper_media::UpMediaEntity::insert_many(medias.into_iter().map(|m| {
            // debug!("Linking media<{}> and up<{}>", m.title, up.name);
            upper_media::UpMediaModel {
                id: m.id,
                upper_id: cid,
            }
            .into_active_model()
        }))
        .exec_without_returning(&db.db)
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "insert many upper medias error: {:?}, upperid<{:?}>, caller:{:?}",
                err,
                cid,
                caller
            )
        });

        state.map(|state| (cid, state))
    }
}
