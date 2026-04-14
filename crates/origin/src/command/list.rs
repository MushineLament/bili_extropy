use anyhow::Result;
use bevy::ecs::{
    message::MessageReader,
    system::{Local, Res, ResMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use tokio::task::JoinHandle;
use tracing::error;

use crate::{
    console::ConsoleMessage,
    db::Db,
    entity::{ToTableRecord, media::MediaModel},
};

#[derive(Debug)]
pub enum CommandHandleList {
    Medias(JoinHandle<Result<Vec<MediaModel>>>),
}

impl CommandHandleList {
    pub fn is_finished(&self) -> bool {
        match self {
            CommandHandleList::Medias(join_handle) => join_handle.is_finished(),
        }
    }
}

pub fn list_medias(
    db: Res<Db>,
    runtimer: ResMut<TokioTasksRuntime>,
    mut console_message: MessageReader<ConsoleMessage>,
    mut local: Local<Vec<CommandHandleList>>,
) {
    for message in console_message.read() {
        let db = db.clone();
        let message = message.clone();
        let (args, _argv) = argmap::parse(message.0.iter());

        if !args.get(1).is_some_and(|list| list.eq("list")) {
            continue;
        }

        match args.get(2).map(String::as_str) {
            Some("medias") => {
                let medias = runtimer
                    .spawn_background_task(move |_ctx| async move { db.all_medias().await });

                local.push(CommandHandleList::Medias(medias));
            }
            Some(unkown) => {
                error!("not has this command: {:?}", unkown);
            }
            None => {
                // 输出help
            }
        }
    }

    let mut tmp = vec![];

    while let Some(handle) = local.pop() {
        if !handle.is_finished() {
            tmp.push(handle);
            continue;
        }

        match handle {
            CommandHandleList::Medias(join_handle) => {
                let Ok(result) = bevy::tasks::block_on(join_handle).map_err(|err| {
                    error!("list command joinhandle error:{:?}", err);
                }) else {
                    continue;
                };

                let Ok(medias) = result.map_err(|err| {
                    error!("query sql db list medias error:{:?}", err);
                }) else {
                    continue;
                };

                let table = crate::table::table(
                    ["id", "bvid", "title", "type", "state"],
                    medias.into_iter().map(ToTableRecord::to_record),
                );
                println!("{}\nrows: {}", table, table.count_rows() - 1);
            }
        }
    }

    local.extend(tmp);
}
