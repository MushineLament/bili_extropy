use bevy::{
    app::{Plugin, Update},
    ecs::{message::MessageReader, system::Res},
};

use tracing::error;

use crate::{console::ConsoleMessage, db::Db};

pub struct CommandFetchPlugin;

impl Plugin for CommandFetchPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(Update, spawn_fetch_task);
    }
}

pub fn spawn_fetch_task(
    mut console_message: MessageReader<ConsoleMessage>,
    // db: Res<Db>,
    // mut commands: Commands,
    // mut runtimer: ResMut<TokioTasksRuntime>,
) {
    for message in console_message.read() {
        // let db = db.clone();
        let (args, _argv) = argmap::parse(message.0.iter());

        if !args.get(1).is_some_and(|list| list.eq("fetch")) {
            continue;
        }

        match args.get(2).map(String::as_str) {
            Some(unkown) => {
                error!("not has this command: {:?}", unkown);
            }
            None => {
                // 输出help
            }
        }
    }
}

// pub async fn fetch(prune: bool) -> Result<()> {
//     let db = db(false).await;
//     let accounts = db
//         .get_accounts_filtered(account::Column::State.eq(AccountState::Active))
//         .await?;
//     for account in accounts.iter() {
//         add_cookie_jar(parse_cookies(&account.cookies));

//         let account_id = account.account_id;

//         info!("Fetching sets with account<{}>", account.name);

//         let ListSetResp {
//             data: ListSetData { list },
//         } = BiliApi::request(ListSetPayload { up_mid: account_id }).await?;

//         if !list.is_empty() {
//             db.upsert_collections(list.iter().map(|set| {
//                 debug!("Updating set<{}>", set.title);
//                 collection::CollectionModel {
//                     collection_id: set.id,
//                     name: set.title.to_owned(),
//                     count: set.media_count,
//                     state: SetState::Inactive.to_string(), // conflic skip
//                 }
//             }))
//             .await?;
//             db.upsert_set_accounts(list.iter().map(|set| {
//                 debug!("Linking account<{}> and set<{}>", account.name, set.title,);
//                 account_collection::AccountCollectionModel {
//                     collection_id: set.id,
//                     account_id,
//                 }
//             }))
//             .await?;
//         }

//         let mut old_set_ids: HashSet<i64> =
//             HashSet::from_iter(db.get_set_ids_of_account(account_id).await?);

//         for set in list {
//             old_set_ids.remove(&set.id);
//         }

//         for set_id in old_set_ids {
//             db.delete_set_account(account_collection::AccountCollectionModel {
//                 collection_id: set_id,
//                 account_id,
//             })
//             .await?;
//             warn!("Unlinked account<{}> and set<{}>", account.name, set_id,);
//         }

//         info!("Fetching following ups with account<{}>", account.name);
//         let FollowingNumResp {
//             data: FollowingNumData { following },
//         } = BiliApi::request(FollowingNumPayload { vmid: account_id })
//             .await
//             .context("Failed to fetch following ups number")?;
//         if following == 0 {
//             continue;
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
//         let mut old_following_ids: HashSet<i64> =
//             HashSet::from_iter(db.get_up_ids_of_account(account_id).await?);
//         if !ups.is_empty() {
//             db.upsert_ups(ups.iter().map(|up| {
//                 debug!("Updating following up<{}>", up.name);
//                 up::Model {
//                     up_id: up.mid,
//                     name: up.name.to_owned(),
//                     state: UpState::Inactive.to_string(),
//                 }
//             }))
//             .await?;
//             db.upsert_up_accounts(ups.iter().map(|up| {
//                 debug!("Linking account<{}> and up<{}>", account.name, up.name);
//                 up_account::Model {
//                     up_id: up.mid,
//                     account_id,
//                 }
//             }))
//             .await?;
//             for up in ups {
//                 old_following_ids.remove(&up.mid);
//             }
//             for up_id in old_following_ids {
//                 db.delete_up_account(up_account::Model { up_id, account_id })
//                     .await?;
//                 warn!("Unlinked account<{}> and up<{}>", account.name, up_id,);
//             }
//         }
//     }
//     let fetched_sets = DashSet::<i64>::new();
//     for account in accounts.iter() {
//         info!("Fetching set medias with account<{}>", account.name);
//         add_cookie_jar(parse_cookies(&account.cookies));
//         let account_id = account.account_id;
//         let set_ids_of_account = db.get_set_ids_of_account(account_id).await?;
//         for set_id in set_ids_of_account {
//             if fetched_sets.contains(&set_id) {
//                 continue;
//             }
//             let set = db.get_set(set_id).await?;
//             if set.state != SetState::Active.to_string() || set.count == 0 {
//                 continue;
//             }
//             info!("Fetching medias in set<{}>", set.name);
//             let page = (set.count - 1) / 20 + 1;
//             let mut tasks = futures::stream::iter(1..=page)
//                 .map(|pn| async move {
//                     // 通过收藏夹id，获取视频的id
//                     let InSetResp {
//                         data: InSetData { medias },
//                     } = BiliApi::request(InSetPayload {
//                         media_id: set.collection_id,
//                         pn,
//                         ps: 20,
//                     })
//                     .await
//                     // .context(format!("Failed to fetch sets' page {pn}"))
//                     ?;
//                     Ok::<_, anyhow::Error>(medias)
//                 })
//                 .buffer_unordered(8);
//             let mut medias = vec![];
//             while let Some(res) = tasks.next().await {
//                 match res {
//                     Ok(list) => medias.extend(list),
//                     Err(e) => error!("caller: {:?},{}", (file!(), line!()), e),
//                 }
//             }
//             if !medias.is_empty() {
//                 db.upsert_medias(medias.iter().map(|m| {
//                     debug!("Updating media<{}>", m.title);
//                     media::MediaModel {
//                         aid: m.id,
//                         bv_id: m.bv_id.to_owned(),
//                         title: m.title.to_owned(),
//                         r#type: m.r#type.to_string(),
//                         state: MediaState::Pending.to_string(),
//                         cid: m.upper.mid,
//                     }
//                 }))
//                 .await?;
//                 db.upsert_media_sets(medias.into_iter().map(|m| {
//                     debug!("Linking media<{}> and set<{}>", m.title, set.name);
//                     collection_media::CollectionMediaModel {
//                         id: m.id,
//                         collection_id: set_id,
//                     }
//                 }))
//                 .await?;
//             }
//             fetched_sets.insert(set_id);
//         }
//     }
//     let fetched_ups = DashSet::<i64>::new();
//     for account in accounts.iter() {
//         info!(
//             "Fetching published contents of ups with account<{}>",
//             account.name
//         );
//         add_cookie_jar(parse_cookies(&account.cookies));
//         let account_id = account.account_id;
//         let up_ids_of_account = db.get_up_ids_of_account(account_id).await?;
//         for up_id in up_ids_of_account {
//             if fetched_ups.contains(&up_id) {
//                 continue;
//             }
//             let up = db.get_up(up_id).await?;
//             if up.state != SetState::Active.to_string() {
//                 continue;
//             }
//             let PublishNumResp {
//                 data: PublishNumData { video },
//             } = BiliApi::request(PublishNumPayload { mid: up_id }).await?;
//             if video == 0 {
//                 continue;
//             }
//             info!("Fetching published videos of up<{}>", up.name);
//             let page = (video - 1) / 30 + 1;
//             let mut tasks = futures::stream::iter(1..=page)
//                 .map(|pn| async move {
//                     let InUpResp {
//                         data:
//                             InUpData {
//                                 list: InUpList { vlist },
//                             },
//                     } = BiliApi::request(InUpPayload::new(up_id, pn, 30).await?)
//                         .await
//                         .map_err(|err| {
//                             anyhow::anyhow!("Failed to fetch up space page {pn}, error: {:?}", err)
//                         })?;
//                     Ok::<_, anyhow::Error>(vlist)
//                 })
//                 .buffer_unordered(8);
//             let mut medias = vec![];
//             while let Some(res) = tasks.next().await {
//                 match res {
//                     Ok(list) => medias.extend(list),
//                     Err(e) => error!("{}", e),
//                 }
//             }
//             if !medias.is_empty() {
//                 db.upsert_medias(medias.iter().map(|m| {
//                     debug!("Updating media<{}>", m.title);
//                     media::MediaModel {
//                         aid: m.id,
//                         bv_id: m.bv_id.to_owned(),
//                         title: m.title.to_owned(),
//                         r#type: m.r#type.to_string(),
//                         state: MediaState::Pending.to_string(),
//                         cid: m.mid,
//                     }
//                 }))
//                 .await?;
//                 db.upsert_media_ups(medias.into_iter().map(|m| {
//                     debug!("Linking media<{}> and up<{}>", m.title, up.name);
//                     up_media::UpMediaModel { id: m.id, up_id }
//                 }))
//                 .await?;
//             }
//             fetched_ups.insert(up_id);
//         }
//     }
//     let fetched_medias = Arc::new(DashSet::<i64>::new());
//     for account in accounts.iter() {
//         info!("Fetching media metadatas with account<{}>", account.name);
//         add_cookie_jar(parse_cookies(&account.cookies));
//         let medias = db.all_active_medias().await?;
//         let mut tasks = futures::stream::iter(
//             medias
//                 .into_iter()
//                 .filter(|media| !fetched_medias.contains(&media.aid)),
//         )
//         .map(|media| async move {
//             match BiliApi::request(MediaInfoAidPayload { aid: media.aid }).await? {
//                 MediaInfoResp {
//                     data: Some(MediaInfoData { owner, staff, .. }),
//                     code: 0,
//                     ..
//                 } => Ok((owner, staff, media)),
//                 MediaInfoResp {
//                     message: option_msg,
//                     ..
//                 } => Err(anyhow!(
//                     "Info unreachable media<{} {}>: {}",
//                     media.title,
//                     media.aid,
//                     option_msg.unwrap_or_default()
//                 )),
//             }
//         })
//         .buffer_unordered(128);
//         let mut media_ups = vec![];
//         let mut ups = HashMap::new();
//         while let Some(res) = tasks.next().await {
//             match res {
//                 Ok((owner, staff, media)) => {
//                     ups.insert(owner.mid, owner.clone());
//                     media_ups.push((media.clone(), owner));
//                     if let Some(staff) = staff {
//                         staff.into_iter().for_each(|staff| {
//                             ups.insert(staff.mid, staff.clone());
//                             media_ups.push((media.clone(), staff));
//                         });
//                     }
//                 }
//                 Err(e) => error!("{}", e),
//             }
//         }
//         if !ups.is_empty() {
//             db.upsert_ups(ups.into_values().map(|up| {
//                 debug!("Updating up<{}>", up.name);
//                 up::Model {
//                     up_id: up.mid,
//                     name: up.name,
//                     state: UpState::Inactive.to_string(),
//                 }
//             }))
//             .await?;
//         }
//         if !media_ups.is_empty() {
//             db.upsert_media_ups(media_ups.iter().map(|(media, up)| {
//                 debug!("Linking media<{}> and up<{}>", media.title, up.name);
//                 up_media::UpMediaModel {
//                     id: media.aid,
//                     up_id: up.mid,
//                 }
//             }))
//             .await?;
//         }
//         for (media, _) in media_ups.into_iter() {
//             fetched_medias.insert(media.aid);
//         }
//     }
//     if prune {
//         info!("Pruning unfaved sets");
//         db.prune_sets().await?;
//         info!("Pruning unfollowed ups");
//         db.prune_ups().await?;
//         info!("Pruning unfollowed medias");
//         db.prune_medias().await?;
//     }
//     info!("Finished fetching");
//     Ok(())
// }
