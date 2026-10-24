use std::{borrow::Cow, panic::Location};

use api_req::{ApiCaller as _, Payload};

use migration::OnConflict;
use sea_orm::{DbErr, EntityTrait, IntoActiveModel};
use serde::Serialize;

use crate::{
    api::BiliApi,
    cookies::{add_cookie_jar, parse_cookies},
    db::Db,
    entity::media::{self, Media, MediaInfoSingle},
};

// /// get upper's all media ids.return is current online upper's media ids.
// #[derive(Debug)]
// pub struct FetchUpperMedias {
//     /// Is alse fetch media into sql database;
//     pub upper_cid: UpperCid,
//     pub handle: Vec<MediaAid>,
// }

// impl FetchUpperMedias {
//     pub async fn new(db: Db, cid: UpperCid, cookies: String) -> Result<Self> {
//         add_cookie_jar(parse_cookies(&cookies));

//         let PublishNumResp {
//             data: PublishNumData { video },
//         } = BiliApi::request(PublishNumPayload { mid: cid })
//             .await
//             .map_err(|err| {
//                 error!("this happend error:{:?} ,caller:{:?}", err, line!());
//                 err
//             })?;

//         if video == 0 {
//             return Ok(Self {
//                 upper_cid: cid,
//                 handle: vec![],
//             });
//         }

//         // info!("Fetching published videos of up<{}>", up.name);
//         let page = (video - 1) / 30 + 1;

//         let mut medias = vec![];

//         for pn in 1..=page {
//             let Ok(up_payload) = InUpPayload::new(cid, pn, 30)
//                 .await
//                 .map_err(|err| info!("Failed to fetch up, cuase wbi get error:{:?}", err))
//             else {
//                 continue;
//             };

//             let Ok(InUpResp {
//                 data: InUpData {
//                     list: InUpList { vlist },
//                 },
//             }) = BiliApi::request(up_payload)
//                 .await
//                 .map_err(|err| error!("Failed to fetch up space page {pn}, error: {:?}", err))
//             else {
//                 continue;
//             };

//             medias.extend(vlist.into_iter().map(|m| upper_media::UpperMediaModel {
//                 media_id: m.id,
//                 upper_id: cid,
//             }));
//         }

//         let upper_medias = medias
//             .iter()
//             .map(|model| model.media_id)
//             .collect::<Vec<MediaAid>>();

//         let state = upper_media::UpperMediaEntity::insert_many(
//             medias.into_iter().map(|m| m.into_active_model()),
//         )
//         .on_conflict(
//             OnConflict::columns([upper_media::Column::MediaId, upper_media::Column::UpperId])
//                 .update_columns([upper_media::Column::MediaId, upper_media::Column::UpperId])
//                 .to_owned(),
//         )
//         .exec_without_returning(&db.db)
//         .await
//         .map_err(|err| {
//             error!(
//                 "insert many upper medias error: {:?}, upperid<{:?}>, caller:{:?}",
//                 err,
//                 cid,
//                 line!()
//             )
//         });

//         state.map(|state| (upper_medias, state))
//     }
// }

#[derive(Debug, Payload, Serialize, Clone, PartialEq, Eq, Hash)]
#[api_req(path = "/x/web-interface/wbi/view")]
pub struct MediaInfoAidPayload {
    pub aid: i64,
}

#[derive(Debug, Payload, Serialize, Clone, PartialEq, Eq, Hash)]
#[api_req(path = "/x/web-interface/wbi/view")]
pub struct MediaInfoBvidPayload {
    pub bvid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MediaInputId {
    Aid(i64),
    BvId(String),
}

impl MediaInputId {
    pub fn from_bvid_or_aid(data: &str) -> Self {
        match data.parse::<i64>() {
            Ok(aid) => MediaInputId::Aid(aid),
            Err(_) => MediaInputId::BvId(data.to_owned()),
        }
    }

    pub fn as_str(&self) -> Cow<'_, str> {
        match self {
            MediaInputId::Aid(aid) => Cow::Owned(aid.to_string()),
            MediaInputId::BvId(bvid) => Cow::Borrowed(&bvid),
        }
    }

    pub async fn to_response(&self) -> Result<MediaInfoSingle, FetchError> {
        match self {
            MediaInputId::Aid(aid) => BiliApi::request(MediaInfoAidPayload { aid: *aid })
                .await
                .map_err(|err| FetchError::new(FetchErrorKind::ApiReq(err))),
            MediaInputId::BvId(bvid) => {
                BiliApi::request(MediaInfoBvidPayload { bvid: bvid.clone() })
                    .await
                    .map_err(|err| FetchError::new(FetchErrorKind::ApiReq(err)))
            }
        }
    }

    // pub fn media_aid(&self) -> impl Future<Output = Result<MediaAid, FetchError>> {
    //     async {
    //         let response = self.to_response().await?;

    //         let media = response
    //             .data
    //             .ok_or(FetchError::new(DownloadFileErrorKind::MediaPage))?;

    //         Ok(media.aid)
    //     }
    // }

    // pub fn related_task_id(
    //     &self,
    //     db: &Db,
    // ) -> impl Future<Output = Result<Vec<TaskId>, FetchError>> {
    //     async {
    //         let media_id = self.media_aid().await?;

    //         let realted_taskids = LoadDownloadtask::load_with(db, |select| {
    //             select.filter(downloadtask::Column::Id.eq(media_id))
    //         })
    //         .await
    //         .map_err(|err| FetchError::new(DownloadFileErrorKind::Db(err)))?;

    //         Ok(realted_taskids.into_iter().map(|model| model.id).collect())
    //     }
    // }

    // pub fn into_unique_id(self) -> MediaInputId {
    //     self
    // }

    // pub fn massage(&self) -> impl Display {
    //     self.as_str()
    // }
}

/// load in memory, and fetch media infomation from bilibili api, and insert into sql database.
#[derive(Debug, Clone)]
pub struct FetchMedia(pub Media);

impl FetchMedia {
    pub async fn fetch(id: MediaInputId, cookies: &str) -> Result<Self, FetchError> {
        add_cookie_jar(parse_cookies(cookies));

        let MediaInfoSingle {
            data: Some(media), ..
        } = id.to_response().await?
        else {
            return Err(FetchError::new(FetchErrorKind::MediaPage));
        };

        Ok(Self(media))
    }

    pub async fn async_fetch(db: &Db, id: MediaInputId, cookies: &str) -> Result<Self, FetchError> {
        let media = Self::fetch(id, cookies).await?;
        media.clone().insert_db(db).await?;
        Ok(media)
    }

    pub async fn insert_db(self, db: &Db) -> Result<u64, DbErr> {
        let state = media::MediaEntity::insert(
            crate::entity::media::MediaModel {
                aid: self.0.aid,
                bv_id: self.0.bvid.to_owned(),
                title: self.0.title.to_owned(),
                r#type: self.0.r#type.to_string(),
                cid: self.0.cid,
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
        .exec_without_returning(&db.0)
        .await?;

        Ok(state)
    }
}

#[derive(Debug)]
pub struct FetchError {
    pub caller: &'static Location<'static>,
    pub error: FetchErrorKind,
}

impl FetchError {
    #[track_caller]
    pub fn new(error: FetchErrorKind) -> Self {
        Self {
            caller: Location::caller(),
            error,
        }
    }
}

impl std::error::Error for FetchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.error {
            FetchErrorKind::Reqwest(e) => Some(e),
            FetchErrorKind::ApiReq(e) => Some(e),
            FetchErrorKind::Db(e) => Some(e),
            FetchErrorKind::MediaPage => None,
        }
    }
}

#[derive(Debug)]
pub enum FetchErrorKind {
    Reqwest(reqwest::Error),
    ApiReq(api_req::error::ApiErr),
    Db(sea_orm::DbErr),
    MediaPage,
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.error {
            FetchErrorKind::Reqwest(error) => {
                write!(f, "Reqwest error:{}, caller:{:?}", error, self.caller)
            }
            FetchErrorKind::ApiReq(error) => {
                write!(f, "ApiErr error:{}, caller:{:?}", error, self.caller)
            }
            FetchErrorKind::Db(error) => {
                write!(f, "Db error:{}, caller:{:?}", error, self.caller)
            }
            FetchErrorKind::MediaPage => {
                write!(
                    f,
                    "media page error, maybe media is not exist, caller:{:?}",
                    self.caller
                )
            }
        }
    }
}

impl From<reqwest::Error> for FetchError {
    #[track_caller]
    fn from(error: reqwest::Error) -> Self {
        Self {
            caller: Location::caller(),
            error: FetchErrorKind::Reqwest(error),
        }
    }
}

impl From<api_req::error::ApiErr> for FetchError {
    #[track_caller]
    fn from(error: api_req::error::ApiErr) -> Self {
        Self {
            caller: Location::caller(),
            error: FetchErrorKind::ApiReq(error),
        }
    }
}

impl From<sea_orm::DbErr> for FetchError {
    #[track_caller]
    fn from(error: sea_orm::DbErr) -> Self {
        Self {
            caller: Location::caller(),
            error: FetchErrorKind::Db(error),
        }
    }
}

// /// 获取收藏夹id下的所有mediacid
// #[derive(Debug)]
// pub struct FetchCollectMedias;

// impl FetchCollectMedias {
//     pub async fn task(db: Db, id: CollectionId, cookies: String) -> Result<(CollectionId, u64)> {
//         add_cookie_jar(parse_cookies(&cookies));

//         let Some(model) = collection::CollectionEntity::find_by_id(id)
//             .one(&db.db)
//             .await?
//         else {
//             error!("sql not has collection<{:?}> infomation", id);
//             return Err(());
//         };

//         let page = (model.count - 1) / 20 + 1;

//         let mut tasks = futures::stream::iter(1..=page)
//             .map(|pn| async move {
//                 // 通过收藏夹id，获取视频的id
//                 let InSetResp {
//                     data: InSetData { medias },
//                 } = BiliApi::request(InSetPayload {
//                     media_id: model.collection_id,
//                     pn,
//                     ps: 20,
//                 })
//                 .await
//                 .context(format!("Failed to fetch sets' page {pn}"))?;
//                 Ok::<_, anyhow::Error>(medias)
//             })
//             .buffer_unordered(8);

//         let mut medias = vec![];
//         while let Some(res) = tasks.next().await {
//             match res {
//                 Ok(list) => medias.extend(list),
//                 Err(e) => error!("caller: {:?},{}", (file!(), line!()), e),
//             }
//         }

//         let state =
//             collection_media::CollectionMediaEntity::insert_many(medias.into_iter().map(|m| {
//                 debug!("Linking media<{}> and set<{}>", m.title, model.name);
//                 collection_media::CollectionMediaModel {
//                     media_id: m.id,
//                     collection_id: model.collection_id,
//                 }
//                 .into_active_model()
//             }))
//             .exec_without_returning(&db.db)
//             .await
//             .map_err(|err| {
//                 error!(
//                     "insert many accountcollectid error: {:?}, collectonid<{:?}>, caller:{:?}",
//                     err,
//                     model.collection_id,
//                     MaybeLocation::caller()
//                 )
//             });

//         state.map(|state| (model.collection_id, state))
//     }
// }

// /// 获取登录账户与关注uppercid的关系
// #[derive(Debug)]
// pub struct FetchAccountFollowing;

// impl FetchAccountFollowing {
//     pub async fn task(db: Db, model: AccountModel) -> Result<(UpperCid, u64)> {
//         crate::cookies::add_cookie_jar(crate::cookies::parse_cookies(&model.cookies));

//         let account_id = model.account_id;

//         info!("Fetching sets with account<{}>", model.name);

//         let FollowingNumResp {
//             data: FollowingNumData { following },
//         } = BiliApi::request(FollowingNumPayload { vmid: account_id })
//             .await
//             .map_err(|err| error!("Failed to fetch following ups number, error:{:?}", err))?;

//         if following == 0 {
//             return Ok((account_id, 0));
//         }

//         let page = (following - 1) / 50 + 1;

//         let mut tasks = futures::stream::iter(1..=page)
//             .map(|pn| async move {
//                 let FollowingUpResp {
//                     data: FollowingUpData { list },
//                 } = BiliApi::request(FollowingUpPayload {
//                     vmid: account_id,
//                     pn,
//                     ps: 50,
//                 })
//                 .await
//                 .map_err(|err| error!("Failed to fetch following ups' page {pn},api:{:?}", err))?;
//                 Ok::<_, ()>(list)
//             })
//             .buffer_unordered(8);

//         let mut ups = vec![];
//         while let Some(res) = tasks.next().await {
//             match res {
//                 Ok(list) => ups.extend(list),
//                 Err(e) => error!("{}", e),
//             }
//         }

//         let state = upper_account::Entity::insert_many(ups.into_iter().map(|up| {
//             upper_account::Model {
//                 upper_id: up.mid,
//                 account_id,
//             }
//             .into_active_model()
//         }))
//         .on_conflict(
//             OnConflict::columns([
//                 upper_account::Column::UpperId,
//                 upper_account::Column::AccountId,
//             ])
//             .update_columns([
//                 upper_account::Column::UpperId,
//                 upper_account::Column::AccountId,
//             ])
//             .to_owned(),
//         )
//         .exec_without_returning(&db.db)
//         .await
//         .map_err(|err| {
//             error!(
//                 "insert many accountcollectid error: {:?}, account_id<{:?}>, caller:{:?}",
//                 err,
//                 account_id,
//                 MaybeLocation::caller()
//             )
//         });

//         state.map(|state| (account_id, state))
//     }
// }

// /// 获取uppercid用户关注的up列表
// #[derive(Debug)]
// pub struct FetchUpperFollowing;

// impl FetchUpperFollowing {
//     pub async fn task(db: Db, cid: UpperCid, cookies: String) -> Result<(UpperCid, u64)> {
//         add_cookie_jar(parse_cookies(&cookies));

//         info!("Fetching following with upper<{}>", cid);

//         let FollowingNumResp {
//             data: FollowingNumData { following },
//         } = BiliApi::request(FollowingNumPayload { vmid: cid })
//             .await
//             .context("Failed to fetch following ups number")?;

//         if following == 0 {
//             return Ok((cid, 0));
//         }

//         let page = (following - 1) / 50 + 1;

//         let mut tasks = futures::stream::iter(1..=page)
//             .map(|pn| async move {
//                 let FollowingUpResp {
//                     data: FollowingUpData { list },
//                 } = BiliApi::request(FollowingUpPayload {
//                     vmid: cid,
//                     pn,
//                     ps: 50,
//                 })
//                 .await
//                 .context(format!("Failed to fetch following ups' page {pn}"))?;
//                 Ok::<_, anyhow::Error>(list)
//             })
//             .buffer_unordered(8);

//         let mut ups = vec![];
//         while let Some(res) = tasks.next().await {
//             match res {
//                 Ok(list) => ups.extend(list),
//                 Err(e) => error!("{}", e),
//             }
//         }

//         let state = upper::UpperEntity::insert_many(ups.iter().map(|up| {
//             upper::UpperModel {
//                 upper_id: up.mid,
//                 name: up.name.to_owned(),
//             }
//             .into_active_model()
//         }))
//         .on_conflict(
//             OnConflict::column(upper::Column::UpperId)
//                 .update_columns([upper::Column::UpperId, upper::Column::Name])
//                 .to_owned(),
//         )
//         .exec_without_returning(&db.db)
//         .await
//         .map_err(|err| {
//             error!(
//                 "insert many accountcollectid error: {:?}, account_id<{:?}>, caller:{:?}",
//                 err,
//                 cid,
//                 MaybeLocation::caller()
//             )
//         });

//         state.map(|state| (cid, state))
//     }
// }

// /// 更新数据库中的登录账户与收藏夹id的对应关系
// #[derive(Debug)]
// pub struct FetchAccountCollectionId;

// impl FetchAccountCollectionId {
//     pub async fn task(db: Db, model: AccountModel) -> Result<(CollectionId, u64), ()> {
//         crate::cookies::add_cookie_jar(crate::cookies::parse_cookies(&model.cookies));

//         let account_id = model.account_id;

//         info!("Fetching sets with account<{}>", model.name);

//         let ListUpperCollectResp {
//             data: ListUpperCollectData { list },
//         } = BiliApi::request(ListUpperCollectPayload { up_mid: account_id }).await?;

//         let state = account_collection::AccountCollectionEntity::insert_many(list.into_iter().map(
//             |info| {
//                 account_collection::AccountCollectionModel {
//                     collection_id: info.id,
//                     account_id,
//                 }
//                 .into_active_model()
//             },
//         ))
//         .on_conflict(
//             OnConflict::columns([
//                 account_collection::Column::CollectionId,
//                 account_collection::Column::AccountId,
//             ])
//             .update_columns([
//                 account_collection::Column::CollectionId,
//                 account_collection::Column::AccountId,
//             ])
//             .to_owned(),
//         )
//         .exec_without_returning(&db.db)
//         .await
//         .map_err(|err| {
//             error!(
//                 "insert many accountcollectid error: {:?}, account_id<{:?}>, caller:{:?}",
//                 err, account_id, caller
//             )
//         });

//         state.map(|state| (account_id, state))
//     }
// }

// /// 更新数据库中的uppercid用户下的所有收藏夹信息
// #[derive(Debug)]
// pub struct FetchUpperCollection;

// impl FetchUpperCollection {
//     pub async fn task(db: Db, cid: UpperCid, cookies: String) -> Result<(UpperCid, u64), ()> {
//         crate::cookies::add_cookie_jar(crate::cookies::parse_cookies(&cookies));

//         let Ok(ListUpperCollectResp {
//             data: ListUpperCollectData { list },
//         }) = BiliApi::request(ListUpperCollectPayload { up_mid: cid }).await
//         else {
//             return Err(error!(
//                 "bilibili api request error,caller: {:?}",
//                 MaybeLocation::caller()
//             ));
//         };

//         let state = collection::CollectionEntity::insert_many(list.into_iter().map(|info| {
//             collection::CollectionModel {
//                 collection_id: info.id,
//                 name: info.title.to_owned(),
//                 count: info.media_count,
//             }
//             .into_active_model()
//         }))
//         .on_conflict(
//             OnConflict::column(collection::Column::CollectionId)
//                 .update_columns([collection::Column::Name, collection::Column::Count])
//                 .to_owned(),
//         )
//         .exec_without_returning(&db.db)
//         .await
//         .map_err(|err| {
//             error!(
//                 "insert many collection error: {:?}, account_id<{:?}>, caller:{:?}",
//                 err,
//                 cid,
//                 MaybeLocation::caller()
//             )
//         });

//         state.map(|state| (cid, state))
//     }
// }
