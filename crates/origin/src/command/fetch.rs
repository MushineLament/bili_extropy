use clap::{Arg, ArgMatches, Command};

use crate::{
    RUNTIMER,
    command::account::ACTIVE_ACCOUNT,
    components::fetch::{FetchMedia, MediaInputId},
    db::DB,
};

pub fn command_fetch() -> Command {
    Command::new("fetch")
        .about("Fetch metadata of following ups, sets, medias, ups [alias: f]")
        .aliases(["f"])
        .subcommands([Command::new("media")
            .about("Pull media infomation")
            .args([Arg::new("media")
                .help("Media BvId or Cid")
                .value_name("BVID")
                .required(true)])])
}

pub fn execute_fetch(arg: &ArgMatches) {
    match arg.subcommand() {
        Some(("media", subarg)) => fetch_media(subarg),

        _ => unreachable!(),
    }
}

pub fn fetch_media(subarg: &ArgMatches) {
    let Some(id) = subarg.get_one::<String>("media") else {
        println!("Not a BvId or Cid");
        return;
    };

    let media = RUNTIMER.block_on(FetchMedia::async_fetch(
        &DB,
        MediaInputId::from_bvid_or_aid(id),
        &ACTIVE_ACCOUNT.0.cookies,
    ));

    println!("media:{:?}", media);
}

// pub struct CommandFetchPlugin;

// impl Plugin for CommandFetchPlugin {
//     fn build(&self, app: &mut bevy::app::App) {
//         app.add_systems(PreUpdate, spawn_fetch_task).add_systems(
//             Update,
//             (
//                 fetch_account_following_task,
//                 fetch_account_collection_id_task,
//                 fetch_upper_following_task,
//                 fetch_upper_collection_task,
//                 fetch_upper_medias_task,
//                 fetch_collect_medias_task,
//                 // fetch_media_task,
//                 fetch_pending_media,
//                 fetch_downloadtask_medias,
//                 fetch_downloadtask_related_medias,
//                 insert_downloadtask_related_medias,
//                 downloadtask_related_upper_medias,
//             ),
//         );
//     }
// }

// pub fn spawn_fetch_task(
//     mut console_message: MessageReader<ConsoleTrims>,
//     db: Res<Db>,
//     mut commands: Commands,
//     mut runtimer: ResMut<TokioTasksRuntime>,
//     active_account: Query<&ActiveAccount>,
// ) {
//     if console_message.is_empty() {
//         return;
//     }

//     let account_ids = active_account
//         .iter()
//         .map(ActiveAccount::id)
//         .flatten()
//         .collect::<Vec<_>>();

//     let Some(account_model) = active_account
//         .iter()
//         .map(ActiveAccount::model)
//         .flatten()
//         .next()
//     else {
//         error!("not any active account");
//         return;
//     };

//     let cookies = account_model.cookies.as_str();

//     for message in console_message.read() {
//         // let db = db.clone();
//         let ConsoleTrims { args, argv: _ } = message;

//         if !args.get(1).is_some_and(|list| list.eq("fetch")) {
//             continue;
//         }

//         let point_id = message
//             .ids()
//             .into_iter()
//             .filter_map(|str| str.parse::<i64>().ok())
//             .collect::<Vec<_>>();

//         let ids = if point_id.is_empty() {
//             account_ids.clone()
//         } else {
//             point_id
//         };

//         if ids.is_empty() {
//             error!("not any point id or active account");
//             continue;
//         }

//         match args.get(FETCH_COMMAND_INDEX).map(String::as_str) {
//             // 上述有问题
//             Some("account") => match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
//                 Some("followings") => {
//                     commands.spawn_batch(
//                         active_account
//                             .iter()
//                             .map(ActiveAccount::model)
//                             .flatten()
//                             .map(|account| {
//                                 (
//                                     FetchAccountFollowing::new(
//                                         db.clone(),
//                                         account.clone(),
//                                         runtimer.as_mut(),
//                                     ),
//                                     FetchTask,
//                                 )
//                             })
//                             .collect::<Vec<_>>(),
//                     );
//                 }
//                 Some("collections") => {
//                     commands.spawn_batch(
//                         active_account
//                             .iter()
//                             .map(ActiveAccount::model)
//                             .flatten()
//                             .map(|account| {
//                                 (
//                                     FetchAccountCollectionId::new(
//                                         db.clone(),
//                                         account.clone(),
//                                         runtimer.as_mut(),
//                                     ),
//                                     FetchTask,
//                                 )
//                             })
//                             .collect::<Vec<_>>(),
//                     );
//                 }
//                 Some(unkown) => {
//                     error!("not has this command: {:?}", unkown);
//                 }
//                 None => {}
//             },
//             Some("upper") => match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
//                 Some("followings") => {
//                     commands.spawn_batch(
//                         ids.into_iter()
//                             .map(|id| {
//                                 info!("Fetching upper following with id<{}>", id);
//                                 (
//                                     FetchTask,
//                                     FetchUpperFollowing::new(
//                                         db.clone(),
//                                         id,
//                                         runtimer.as_mut(),
//                                         cookies.to_string(),
//                                     ),
//                                 )
//                             })
//                             .collect::<Vec<_>>(),
//                     );
//                 }
//                 Some("collections") => {
//                     commands.spawn_batch(
//                         ids.into_iter()
//                             .map(|id| {
//                                 info!("spawn fetch upper<{}>'s collection task", id);
//                                 (
//                                     FetchUpperCollection::new(
//                                         db.clone(),
//                                         runtimer.as_mut(),
//                                         id,
//                                         cookies.to_string(),
//                                     ),
//                                     FetchTask,
//                                 )
//                             })
//                             .collect::<Vec<_>>(),
//                     );
//                 }

//                 Some("medias") => {
//                     commands.spawn_batch(
//                         ids.into_iter()
//                             .map(|id| {
//                                 info!("spawn fetch upper<{}>'s medias task", id);
//                                 (
//                                     FetchUpperMedias::new(
//                                         db.clone(),
//                                         id,
//                                         runtimer.as_mut(),
//                                         cookies.to_string(),
//                                     ),
//                                     FetchTask,
//                                 )
//                             })
//                             .collect::<Vec<_>>(),
//                     );
//                 }

//                 Some(unkown) => {
//                     error!("not has this command: {:?}", unkown);
//                 }
//                 None => {}
//             },
//             Some("collection") => match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
//                 Some("medias") => {
//                     commands.spawn_batch(
//                         ids.into_iter()
//                             .map(|id| {
//                                 (
//                                     FetchCollectMedias::new(
//                                         db.clone(),
//                                         id,
//                                         runtimer.as_mut(),
//                                         cookies.to_string(),
//                                     ),
//                                     FetchTask,
//                                 )
//                             })
//                             .collect::<Vec<_>>(),
//                     );
//                 }
//                 Some(unkown) => {
//                     error!("not has this command: {:?}", unkown);
//                 }
//                 None => {}
//             },

//             Some("task") => match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
//                 Some("upper") => {
//                     // fetch upper's medias into downloadtask_medias
//                     info!("spawn fetch downloadtask related upper's medias task");
//                     commands.spawn((
//                         LoadDownloadtask::new_with(db.clone(), runtimer.as_mut(), |select| {
//                             select.filter(downloadtask::Column::TypeId.eq(UPPER))
//                         }),
//                         DownloadtaskRelatedUpperMedias,
//                     ));
//                 }

//                 Some("medias") => {
//                     // fetch downloadtask_medias
//                     info!("spawn fetch downloadtask's medias task");
//                     commands.spawn((
//                         LoadDownloadtaskMedias::new(db.clone(), runtimer.as_mut()),
//                         FetchTask,
//                     ));
//                 }
//                 Some("related") => {
//                     commands.spawn((
//                         LoadDownloadtaskRelatedMedias::new(db.clone(), runtimer.as_mut()),
//                         FetchTask,
//                     ));
//                     info!("related downloadtask and medias");
//                 }
//                 Some(unkown) => {
//                     error!("not has this subcommand: {:?}", unkown);
//                 }
//                 None => {
//                     error!("not any subcommand");
//                 }
//             },

//             Some("media") => match args.get(FETCH_SUBCOMMAND_INDEX).map(String::as_str) {
//                 Some(id) => {
//                     let Ok(fetch) = runtimer.runtime().block_on(async {
//                         tokio::time::timeout(
//                             Duration::from_secs(3),
//                             FetchMedia::fetch_async(
//                                 db.clone(),
//                                 MediaBvidOrAid(id.to_string()).parse(),
//                                 cookies,
//                             ),
//                         )
//                         .await
//                     }) else {
//                         error!("fetch media timeout");
//                         continue;
//                     };

//                     info!("fetch media: {:?}", fetch.0);
//                 }
//                 None => {}
//             },

//             Some(help) if help.to_lowercase().eq(HELP) => {
//                 error!("\n{}", help);
//             }
//             Some(unkown) => {
//                 error!("not has this command: {:?}", unkown);
//             }
//             None => {
//                 // 输出help
//             }
//         }
//     }
// }

// pub fn fetch_upper_collection_task(
//     mut commands: Commands,
//     query: Query<(&mut FetchUpperCollection, Entity)>,
// ) {
//     for (mut handle, entity) in query {
//         if !handle.is_finished() {
//             continue;
//         }

//         commands.entity(entity).try_despawn();

//         match handle.try_result() {
//             Ok((cid, state)) => {
//                 info!(
//                     "fetch upper<{:?}> collection is finished,state:{:?}",
//                     cid, state
//                 );
//             }
//             Err(err) => {
//                 error!("fetch upper collection error:{:?}", err);
//             }
//         }
//     }
// }

// pub fn fetch_account_collection_id_task(
//     mut commands: Commands,
//     query: Query<(&mut FetchAccountCollectionId, Entity)>,
// ) {
//     for (mut handle, entity) in query {
//         if !handle.is_finished() {
//             continue;
//         }

//         commands.entity(entity).try_despawn();

//         match handle.try_result() {
//             Ok((cid, state)) => {
//                 info!(
//                     "fetch upper<{:?}> collection is finished,state:{:?}",
//                     cid, state
//                 );
//             }
//             Err(err) => {
//                 error!("fetch upper collection error:{:?}", err);
//             }
//         }
//     }
// }

// pub fn fetch_upper_following_task(
//     mut commands: Commands,
//     query: Query<(&mut FetchUpperFollowing, Entity)>,
// ) {
//     for (mut handle, entity) in query {
//         if !handle.is_finished() {
//             continue;
//         }

//         commands.entity(entity).try_despawn();

//         match handle.try_result() {
//             Ok((cid, state)) => {
//                 info!(
//                     "fetch upper<{:?}> collection is finished,state:{:?}",
//                     cid, state
//                 );
//             }
//             Err(err) => {
//                 error!("fetch upper collection error:{:?}", err);
//             }
//         }
//     }
// }

// pub fn fetch_account_following_task(
//     mut commands: Commands,
//     query: Query<(&mut FetchAccountFollowing, Entity)>,
// ) {
//     for (mut handle, entity) in query {
//         if !handle.is_finished() {
//             continue;
//         }

//         commands.entity(entity).try_despawn();

//         match handle.try_result() {
//             Ok((cid, state)) => {
//                 info!(
//                     "fetch upper<{:?}> collection is finished,state:{:?}",
//                     cid, state
//                 );
//             }
//             Err(err) => {
//                 error!("fetch upper collection error:{:?}", err);
//             }
//         }
//     }
// }

// pub fn fetch_collect_medias_task(
//     mut commands: Commands,
//     query: Query<(&mut FetchCollectMedias, Entity)>,
// ) {
//     for (mut handle, entity) in query {
//         if !handle.is_finished() {
//             continue;
//         }

//         commands.entity(entity).try_despawn();

//         match handle.try_result() {
//             Ok((cid, state)) => {
//                 info!(
//                     "fetch upper<{:?}> collection is finished,state:{:?}",
//                     cid, state
//                 );
//             }
//             Err(err) => {
//                 error!("fetch upper collection error:{:?}", err);
//             }
//         }
//     }
// }

// pub fn fetch_upper_medias_task(
//     mut commands: Commands,
//     query: Query<(&mut FetchUpperMedias, Entity)>,
// ) {
//     for (mut handle, entity) in query {
//         if !handle.is_finished() {
//             continue;
//         }

//         let upper_id = handle.upper_cid;
//         let fetch_medias = handle.fetch_medias;

//         commands.entity(entity).try_despawn();

//         match handle.try_result() {
//             Ok((cid, state)) => {
//                 info!(
//                     "fetch upper<{}>'s medias is finished,state:{}",
//                     upper_id, state
//                 );

//                 if fetch_medias {
//                     let fetchs = cid
//                         .iter()
//                         .map(|media_id| {
//                             let media_id = *media_id;
//                             (FetchPendingMedia(media_id),)
//                         })
//                         .collect::<Vec<_>>();

//                     commands.spawn_batch(fetchs);
//                 }
//             }
//             Err(err) => {
//                 error!("fetch upper medias error:{:?}", err);
//             }
//         }
//     }
// }

// // pub fn fetch_media_task(mut commands: Commands, query: Query<(&mut FetchMedia, Entity)>) {
// //     for (mut handle, entity) in query {
// //         if !handle.is_finished() {
// //             continue;
// //         }

// //         commands.entity(entity).try_despawn();

// //         match handle.try_result() {
// //             Ok((cid, state)) => {
// //                 info!("fetch media<{}> is finished,state:{}", *cid, state);
// //             }
// //             Err(err) => {
// //                 error!("fetch upper collection error:{:?}", err);
// //             }
// //         }
// //     }
// // }

// pub fn fetch_pending_media(
//     mut commands: Commands,
//     mut pending_fetchs: Query<(
//         &mut FetchPendingMedia,
//         Has<FetchCheck>,
//         Option<&mut LoadMediasTask>, //check befor fetch
//         Entity,
//     )>,
//     fetching: Query<(), With<FetchMedia>>,
//     db: Res<Db>,
//     mut runtimer: ResMut<TokioTasksRuntime>,
//     active_account: Query<&ActiveAccount>,
// ) {
//     if pending_fetchs.count() <= 0 {
//         return;
//     }

//     let count = fetching.count();

//     let Some(sub) = 4usize.checked_sub(count).filter(|sub| *sub != 0) else {
//         return;
//     };

//     let Some(cookies) = active_account
//         .iter()
//         .map(ActiveAccount::cookies)
//         .flatten()
//         .next()
//     else {
//         error!("not any active account, fetch media failed");
//         return;
//     };

//     let mut iter = pending_fetchs.iter_mut();

//     let pendings = (0..sub).map(|_| iter.next()).filter_map(|pending| pending);

//     for (fetch, is_update, load, entity) in pendings {
//         if is_update {
//             let mdeia_id = fetch.0;

//             commands
//                 .entity(entity)
//                 .remove::<FetchCheck>()
//                 .insert(LoadMediasTask::new_with(
//                     db.clone(),
//                     runtimer.as_mut(),
//                     move |select| select.filter(media::Column::Aid.eq(mdeia_id)),
//                 ));
//             continue;
//         }

//         if let Some(mut result) = load {
//             match result.try_result() {
//                 Ok(result) => {
//                     if result.iter().find(|model| model.aid == fetch.0).is_some() {
//                         commands.entity(entity).try_despawn();
//                         continue;
//                     }
//                 }
//                 Err(err) => {
//                     if !err.is_finished() {
//                         continue;
//                     }
//                 }
//             };
//         }

//         if let Ok(fetch) = FetchMedia::fetch(
//             db.clone(),
//             MediaInputId::Aid(fetch.0),
//             cookies,
//             runtimer.as_ref(),
//         ) {
//             commands.spawn(fetch);
//         }

//         commands.entity(entity).try_despawn();
//     }

//     if pending_fetchs.count() < 4 && fetching.count() < 4 {
//         info!("fetch media taks all is finished");
//     }
// }

// pub fn fetch_downloadtask_medias(
//     mut commands: Commands,
//     query: Query<(&mut LoadDownloadtaskMedias, Has<FetchCheck>, Entity), With<FetchTask>>,
// ) {
//     for (mut handle, update, entity) in query {
//         if !handle.is_finished() {
//             continue;
//         }

//         commands.entity(entity).try_despawn();

//         match handle.try_result() {
//             Ok(medias) => {
//                 if update {
//                     let iter = medias
//                         .iter()
//                         .map(|media| (FetchPendingMedia(media.media_id), FetchCheck))
//                         .collect::<Vec<_>>();

//                     commands.spawn_batch(iter);
//                 } else {
//                     let iter = medias
//                         .iter()
//                         .map(|media| FetchPendingMedia(media.media_id))
//                         .collect::<Vec<_>>();

//                     commands.spawn_batch(iter);
//                 }

//                 info!("spawn fetch downloadtask medias is finished");
//             }
//             Err(err) => {
//                 error!("fetch upper collection error:{:?}", err);
//             }
//         }
//     }
// }

// pub fn fetch_downloadtask_related_medias(
//     mut commands: Commands,
//     query: Query<(&mut LoadDownloadtaskRelatedMedias, Has<FetchCheck>, Entity), With<FetchTask>>,
//     db: Res<Db>,
//     mut runtimer: ResMut<TokioTasksRuntime>,
// ) {
//     for (mut handle, update, entity) in query {
//         if !handle.is_finished() {
//             continue;
//         }

//         commands.entity(entity).try_despawn();

//         match handle.try_result() {
//             Ok(medias) => {
//                 if update {
//                     let iter = medias
//                         .iter()
//                         .map(|(mediaid, _)| (FetchPendingMedia(*mediaid), FetchCheck))
//                         .collect::<Vec<_>>();

//                     commands.spawn_batch(iter);
//                 } else {
//                     let iter = medias
//                         .iter()
//                         .map(|(mediaid, _)| FetchPendingMedia(*mediaid))
//                         .collect::<Vec<_>>();

//                     commands.spawn_batch(iter);
//                 }

//                 let models = medias
//                     .iter()
//                     .map(|(media_id, relateds)| {
//                         relateds
//                             .taskid
//                             .iter()
//                             .map(|task_id| DownloadtaskMediasActiveModel {
//                                 task_id: ActiveValue::Set(*task_id),
//                                 media_id: ActiveValue::Set(*media_id),
//                                 state: ActiveValue::Set("Pending".to_string()),
//                             })
//                     })
//                     .flatten()
//                     .collect::<Vec<_>>();

//                 info!("models count:{:?}", models.len());

//                 commands.spawn((
//                     InsertDownloadtaskRelatedMedias::new(
//                         db.clone(),
//                         runtimer.as_mut(),
//                         models.into_iter(),
//                     ),
//                     InsertTask,
//                 ));

//                 info!("spawn fetch downloadtask medias is finished");
//             }
//             Err(err) => {
//                 error!("fetch upper collection error:{:?}", err);
//             }
//         }
//     }
// }

// pub fn insert_downloadtask_related_medias(
//     mut commands: Commands,
//     query: Query<(&mut InsertDownloadtaskRelatedMedias, Entity), With<InsertTask>>,
// ) {
//     for (mut task, entity) in query {
//         if !task.is_finished() {
//             continue;
//         }
//         match task.try_result() {
//             Ok(result) => {
//                 info!("insert downloadtask related medias count<{}>", result);
//             }
//             Err(err) => {
//                 error!("insert downloadtask related medias error: {:?}", err);
//             }
//         };
//         commands.entity(entity).despawn();
//     }
// }

// pub fn downloadtask_related_upper_medias(
//     mut commands: Commands,
//     query: Query<(&mut LoadDownloadtask, Entity), With<DownloadtaskRelatedUpperMedias>>,
//     db: Res<Db>,
//     mut runtimer: ResMut<TokioTasksRuntime>,
//     active_account: Query<&ActiveAccount>,
// ) {
//     for (mut task, entity) in query {
//         if !task.is_finished() {
//             continue;
//         }
//         match task.try_result() {
//             Ok(result) => {
//                 let Some(cookies) = active_account
//                     .iter()
//                     .map(ActiveAccount::cookies)
//                     .flatten()
//                     .next()
//                 else {
//                     continue;
//                 };

//                 let iter = result
//                     .iter()
//                     .filter(|model| model.type_id.eq(UPPER))
//                     .map(|upper| {
//                         (
//                             FetchUpperMedias::new(
//                                 db.clone(),
//                                 upper.generic_id,
//                                 runtimer.as_mut(),
//                                 cookies.to_string(),
//                             ),
//                             FetchTask,
//                         )
//                     })
//                     .collect::<Vec<_>>();

//                 commands.spawn_batch(iter);

//                 info!("fetch downloadtask related upper's medias");
//             }
//             Err(err) => {
//                 error!("fetch downloadtask related upper's medias error: {:?}", err);
//             }
//         };
//         commands.entity(entity).despawn();
//     }
// }
