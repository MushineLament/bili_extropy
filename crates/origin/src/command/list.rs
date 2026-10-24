use clap::{ArgMatches, Command};
use sea_orm::EntityTrait;

use crate::{
    RUNTIMER, command::status::execute_status, console::APP_NAME, db::DB, entity::media,
    table::ToTable,
};

pub fn command_list() -> Command {
    Command::new("list")
        .about("List accounts/sets/ups/medias [alias: ls, l]")
        .arg_required_else_help(true)
        .aliases(["ls", "l"])
        .subcommands([
            Command::new("status").about("List Download Floders"),
            Command::new("media").about("List DB media"),
        ])
}

pub fn execute_list(arg: &ArgMatches) {
    match arg.subcommand() {
        Some(("status", _)) => {
            let arg = super::status::command_status()
                .try_get_matches_from([APP_NAME, "list"])
                .unwrap();
            execute_status(&arg);
        }

        Some(("media", subarg)) => list_media(subarg),

        _ => unreachable!(),
    }
}

pub fn list_media(_subarg: &ArgMatches) {
    match RUNTIMER.block_on(media::MediaEntity::find().all(&DB.0)) {
        Ok(data) => {
            let table = data.iter().table_head(["id", "bvid", "title", "type"]);
            println!("\n{}\nrows: {}", table, table.count_rows() - 1);
        }
        Err(err) => {
            println!("read DB data error: {:?}", err);
        }
    }
}

// pub fn spawn_list_task(
//     mut commands: Commands,
//     db: Res<Db>,
//     mut runtimer: ResMut<TokioTasksRuntime>,
//     mut console_message: MessageReader<ConsoleTrims>,
//     query: Query<&mut RelatedDownload>,
//     active_account: Query<&ActiveAccount>,
// ) {
//     for message in console_message.read() {
//         let _db = db.clone();
//         let ConsoleTrims { args, argv: _ } = message;

//         if !args.get(1).is_some_and(|list| list.eq("list")) {
//             continue;
//         }

//         let _ids = if message.is_empty_ids() {
//             active_account
//                 .iter()
//                 .map(|e| e.0.as_ref())
//                 .flatten()
//                 .map(|e| e.account_id)
//                 .collect::<Vec<_>>()
//         } else {
//             message
//                 .ids()
//                 .into_iter()
//                 .filter_map(|str| str.parse::<i64>().ok())
//                 .collect::<Vec<_>>()
//         };

//         match args.get(LIST_COMMAND_INDEX).map(String::as_str) {
//             Some("account") => match args.get(LIST_SUBCOMMAND_INDEX).map(String::as_str) {
//                 Some("followings") => {
//                     commands.spawn(ListAccountFollwedTask::new(db.clone(), runtimer.as_mut()));
//                 }
//                 Some("collections") => {
//                     commands.spawn((
//                         LoadAccountCollectionsTask::new(db.clone(), runtimer.as_mut()),
//                         ListTask,
//                     ));
//                 }
//                 Some(unkown) => {
//                     error!("not has this command: {:?}", unkown);
//                 }
//                 None => {
//                     commands.spawn(ListAccountTask::new(db.clone(), runtimer.as_mut()));
//                 }
//             },
//             Some("upper") => {
//                 // todo
//                 match args.get(LIST_SUBCOMMAND_INDEX).map(String::as_str) {
//                     Some("medias") => {
//                         commands.spawn((
//                             LoadUpperMediasTask::new(db.clone(), runtimer.as_mut()),
//                             ListTask,
//                         ));
//                     }
//                     Some("collection") => {
//                         // commands.spawn(ListUpperCollectionTask::new(db.clone(), runtimer.as_mut()));
//                         error!("this subcommand has not finished");
//                     }
//                     Some(unkown) => {
//                         error!("not has this command: {:?}", unkown);
//                     }
//                     None => {
//                         commands
//                             .spawn((LoadUppersTask::new(db.clone(), runtimer.as_mut()), ListTask));
//                     }
//                 }
//             }
//             Some("collection") => {
//                 commands.spawn(ListCollectionTask::new(db.clone(), runtimer.as_mut()));
//             }
//             Some("medias") => match args.get(LIST_SUBCOMMAND_INDEX).map(String::as_str) {
//                 Some("collection") => {
//                     commands.spawn(ListCollectionMediasTask::new(db.clone(), runtimer.as_mut()));
//                 }
//                 Some(unkown) => {
//                     error!("not has this command: {:?}", unkown);
//                 }
//                 None => {
//                     commands.spawn((LoadMediasTask::new(db.clone(), runtimer.as_mut()), ListTask));
//                 }
//             },

//             Some("status") => match args.get(LIST_SUBCOMMAND_INDEX).map(String::as_str) {
//                 Some("downloadrule") => {
//                     commands.spawn((
//                         LoadStatusRelatedDownloadruleTask::new(db.clone(), runtimer.as_mut()),
//                         ListTask,
//                     ));
//                 }
//                 Some(unkown) => {
//                     error!("not has this command: {:?}", unkown);
//                 }
//                 None => {
//                     commands.spawn((LoadStatusTask::new(db.clone(), runtimer.as_mut()), ListTask));
//                 }
//             },
//             Some("downloadrule") => match args.get(LIST_SUBCOMMAND_INDEX).map(String::as_str) {
//                 Some("status") => {
//                     commands.spawn((
//                         LoadStatusRelatedDownloadruleTask::new(db.clone(), runtimer.as_mut()),
//                         ListTask,
//                     ));
//                 }
//                 Some(unkown) => {
//                     error!("not has this command: {:?}", unkown);
//                 }
//                 None => {
//                     commands.spawn((
//                         LoadDownloadrule::new(db.clone(), runtimer.as_mut()),
//                         ListTask,
//                     ));
//                 }
//             },

//             Some("task") => match args.get(LIST_SUBCOMMAND_INDEX).map(String::as_str) {
//                 Some("pendings") => {
//                     commands.spawn((
//                         LoadDownloadtaskMediasPendingDownloadTask::new(
//                             db.clone(),
//                             runtimer.as_mut(),
//                         ),
//                         ListTask,
//                     ));
//                 }
//                 Some("related") => {
//                     commands.spawn((
//                         LoadDownloadtaskRelatedMedias::new(db.clone(), runtimer.as_mut()),
//                         ListTask,
//                     ));
//                 }
//                 Some("medias") => {
//                     commands.spawn((
//                         LoadDownloadtaskMedias::new(db.clone(), runtimer.as_mut()),
//                         ListTask,
//                     ));
//                 }
//                 Some(unkown) => {
//                     error!("not has this command: {:?}", unkown);
//                 }
//                 None => {
//                     commands.spawn((
//                         LoadDownloadtask::new(db.clone(), runtimer.as_mut()),
//                         ListTask,
//                     ));
//                 }
//             },

//             Some("download") => {
//                 let count = query.count();
//                 info!("download count: {}", count);
//             }

//             Some("fetch") => {
//                 commands.run_system_cached(list_fetch_media);
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

// pub fn list_account_task(mut commands: Commands, query: Query<(&mut ListAccountTask, Entity)>) {
//     for (mut task, entity) in query {
//         let Ok(result) = task.try_result() else {
//             continue;
//         };
//         commands.entity(entity).despawn();

//         let table = result.iter().table_head(["account_id", "name", "state"]);
//         info!("\n{}\nrows: {}", table, table.count_rows() - 1);
//     }
// }

// pub fn list_account_collections_task(
//     mut commands: Commands,
//     query: Query<(&mut LoadAccountCollectionsTask, Entity), With<ListTask>>,
// ) {
//     for (mut task, entity) in query {
//         let Ok(result) = task.try_result() else {
//             continue;
//         };
//         commands.entity(entity).despawn();

//         let table = result.iter().table_head(["collection_id", "account_id"]);
//         info!("\n{}\nrows: {}", table, table.count_rows() - 1);
//     }
// }

// pub fn list_collection_task(
//     mut commands: Commands,
//     query: Query<(&mut ListCollectionTask, Entity)>,
// ) {
//     for (mut task, entity) in query {
//         let Ok(result) = task.try_result() else {
//             continue;
//         };
//         commands.entity(entity).despawn();

//         let table = result
//             .iter()
//             .table_head(["collection_id", "name", "count", "state"]);
//         info!("\n{}\nrows: {}", table, table.count_rows() - 1);
//     }
// }

// pub fn list_account_follwed_task(
//     mut commands: Commands,
//     query: Query<(&mut ListAccountFollwedTask, Entity)>,
// ) {
//     for (mut task, entity) in query {
//         let Ok(result) = task.try_result() else {
//             continue;
//         };
//         commands.entity(entity).despawn();

//         let table = result.iter().table_head(["up_id", "account_id"]);
//         info!("\n{}\nrows: {}", table, table.count_rows() - 1);
//     }
// }

// pub fn list_uppers_task(
//     mut commands: Commands,
//     query: Query<(&mut LoadUppersTask, Entity), With<ListTask>>,
// ) {
//     for (mut task, entity) in query {
//         let Ok(result) = task.try_result() else {
//             continue;
//         };
//         commands.entity(entity).despawn();

//         let table = result.iter().table_head(["up_id", "account_id"]);
//         info!("\n{}\nrows: {}", table, table.count_rows() - 1);
//     }
// }

// pub fn list_collection_medias_task(
//     mut commands: Commands,
//     query: Query<(&mut ListCollectionMediasTask, Entity)>,
// ) {
//     for (mut task, entity) in query {
//         let Ok(result) = task.try_result() else {
//             continue;
//         };
//         commands.entity(entity).despawn();

//         let table = result.iter().table_head(["media_cid", "collection_id"]);
//         info!("\n{}\nrows: {}", table, table.count_rows() - 1);
//     }
// }

// pub fn list_download_rule_task(
//     mut commands: Commands,
//     query: Query<(&mut LoadDownloadrule, Entity), With<ListTask>>,
// ) {
//     for (mut task, entity) in query {
//         let Ok(result) = task.try_result() else {
//             continue;
//         };
//         commands.entity(entity).despawn();

//         let table = result.iter().table_head([
//             "id",
//             "name",
//             "size",
//             "relation size",
//             "date",
//             "relation date",
//             "repeat",
//             "state",
//         ]);
//         info!("\n{}\nrows: {}", table, table.count_rows() - 1);
//     }
// }

// pub fn list_status_related_downloadrule_task(
//     mut commands: Commands,
//     query: Query<(&mut LoadStatusRelatedDownloadruleTask, Entity), With<ListTask>>,
// ) {
//     for (mut task, entity) in query {
//         let Ok(result) = task.try_result() else {
//             continue;
//         };
//         commands.entity(entity).despawn();

//         let table = result.iter().table_head(["status id", "rule id"]);
//         info!("\n{}\nrows: {}", table, table.count_rows() - 1);
//     }
// }

// pub fn list_downloadtask_task(
//     mut commands: Commands,
//     query: Query<(&mut LoadDownloadtask, Entity), With<ListTask>>,
// ) {
//     for (mut task, entity) in query {
//         let Ok(result) = task.try_result() else {
//             continue;
//         };
//         commands.entity(entity).despawn();

//         let table = result
//             .iter()
//             .table_head(["id", "type", "related id", "state"]);
//         info!("\n{}\nrows: {}", table, table.count_rows() - 1);
//     }
// }

// pub fn list_downloadtask_medias_task(
//     mut commands: Commands,
//     query: Query<(&mut LoadDownloadtaskMedias, Entity), With<ListTask>>,
// ) {
//     for (mut task, entity) in query {
//         let Ok(result) = task.try_result() else {
//             continue;
//         };
//         commands.entity(entity).despawn();

//         let table = result.iter().table_head(["task id", "media id", "state"]);
//         info!("\n{}\nrows: {}", table, table.count_rows() - 1);
//     }
// }

// pub fn list_upper_medias_task(
//     mut commands: Commands,
//     query: Query<(&mut LoadUpperMediasTask, Entity), With<ListTask>>,
// ) {
//     for (mut task, entity) in query {
//         let Ok(result) = task.try_result() else {
//             continue;
//         };
//         commands.entity(entity).despawn();

//         let table = result.iter().table_head(["media id", "upper id"]);
//         info!("\n{}\nrows: {}", table, table.count_rows() - 1);
//     }
// }

// pub fn list_downloadtask_related_medias_task(
//     mut commands: Commands,
//     query: Query<(&mut LoadDownloadtaskRelatedMedias, Entity), With<ListTask>>,
// ) {
//     for (mut task, entity) in query {
//         let Ok(result) = task.try_result() else {
//             continue;
//         };
//         commands.entity(entity).despawn();

//         let table = result.values().table_head(["related id", "task id"]);

//         info!("\n{}\nrows: {}", table, table.count_rows() - 1);
//     }
// }

// pub fn list_downloadtask_medias_pending_download_task(
//     mut commands: Commands,
//     query: Query<(&mut LoadDownloadtaskMediasPendingDownloadTask, Entity), With<ListTask>>,
// ) {
//     for (mut task, entity) in query {
//         let Ok(result) = task.try_result() else {
//             continue;
//         };
//         commands.entity(entity).despawn();

//         for id in result {
//             info!("downloadtask's pending state media id<{}>", id);
//         }
//     }
// }

// pub fn list_fetch_media(_commands: Commands, query: Query<(&FetchMedia, Entity)>) {
//     info!("list fetch media count:{:?}", query.count());
// }
