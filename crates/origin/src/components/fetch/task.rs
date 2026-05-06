use anyhow::Context as _;
use api_req::ApiCaller as _;
use bevy::{
    ecs::{change_detection::MaybeLocation, component::Component},
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
        download::{DownloadPendding, DownloadWay},
        handle::ECSHandleResult,
    },
    cookies::{add_cookie_jar, parse_cookies},
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
};

/// 获取upperid下的所有mediacid
#[derive(Debug, Component, Deref, DerefMut)]
pub struct FetchUpperMediasData(pub ECSHandleResult<(UpperCid, u64), anyhow::Error>);

impl FetchUpperMediasData {
    #[track_caller]
    pub fn new(db: Db, id: UpperCid, runtimer: &mut TokioTasksRuntime, cookies: String) -> Self {
        let task = runtimer.spawn_background_task(move |_ctx| Self::task(db, id, cookies));

        Self(ECSHandleResult::new(task))
    }

    pub async fn task(
        db: Db,
        cid: UpperCid,
        cookies: String,
    ) -> Result<(UpperCid, u64), anyhow::Error> {
        let PublishNumResp {
            data: PublishNumData { video },
        } = BiliApi::request(PublishNumPayload { mid: cid })
            .await
            .map_err(|err| {
                error!(
                    "this happend error:{:?} ,caller:{:?}",
                    err,
                    MaybeLocation::caller()
                );
                err
            })?;
        if video == 0 {
            return Ok((cid, 0));
        }

        // info!("Fetching published videos of up<{}>", up.name);
        let page = (video - 1) / 30 + 1;

        let mut medias = vec![];

        for pn in 1..=page {
            add_cookie_jar(parse_cookies(&cookies));

            let Ok(up_payload) = InUpPayload::new(cid, pn, 30)
                .await
                .map_err(|err| info!("Failed to fetch up, cuase wbi get error:{:?}", err))
            else {
                continue;
            };

            let Ok(InUpResp {
                data: InUpData {
                    list: InUpList { vlist },
                },
            }) = BiliApi::request(up_payload).await.map_err(|err| {
                anyhow::anyhow!("Failed to fetch up space page {pn}, error: {:?}", err)
            })
            else {
                continue;
            };

            medias.extend(vlist);
        }

        let state = upper_media::UpperMediaEntity::insert_many(medias.into_iter().map(|m| {
            // debug!("Linking media<{}> and up<{}>", m.title, up.name);
            upper_media::UpperMediaModel {
                media_id: m.id,
                upper_id: cid,
            }
            .into_active_model()
        }))
        .on_conflict(
            OnConflict::columns([upper_media::Column::MediaId, upper_media::Column::UpperId])
                .update_columns([upper_media::Column::MediaId, upper_media::Column::UpperId])
                .to_owned(),
        )
        .exec_without_returning(&db.db)
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "insert many upper medias error: {:?}, upperid<{:?}>, caller:{:?}",
                err,
                cid,
                MaybeLocation::caller()
            )
        });

        state.map(|state| (cid, state))
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
            data: Some(media), ..
        }) = id.to_response().await
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
                    media_id: m.id,
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
