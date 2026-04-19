use bevy::{
    app::{Plugin, PostStartup, Update},
    ecs::{
        message::MessageReader,
        system::{Commands, Res, ResMut},
        world::World,
    },
};
use bevy_tokio_tasks::TokioTasksRuntime;
use tracing::{error, info};

use crate::{
    components::{
        initialize::DbInitailizeResource,
        status::handle::{ActiveStatus, AddStatusTask, StatusState},
    },
    console::ConsoleTrims,
    db::Db,
    table::ToTable,
};

pub struct CommandStatusPlugin;

impl Plugin for CommandStatusPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(PostStartup, ActiveStatus::new.to_system())
            .add_systems(Update, (spawn_status_task,));
    }
}

pub fn spawn_status_task(
    mut commads: Commands,
    db: Res<Db>,
    mut console_message: MessageReader<ConsoleTrims>,
    mut runtimer: ResMut<TokioTasksRuntime>,
    res: Res<ActiveStatus>,
) {
    for message in console_message.read() {
        let db = db.clone();

        let (args, argv) = argmap::parse(message.0.iter());

        if !args.get(1).is_some_and(|list| list.eq("status")) {
            continue;
        }

        match args.get(2).map(String::as_str) {
            Some("add") => {
                let Some(name) = args.get(3).cloned() else {
                    error!("not a vaild folder <name>");
                    return;
                };

                let path = args.get(4).cloned().unwrap_or(".".to_string());

                let state = if argv.contains_key("switch") {
                    StatusState::Switch
                } else if argv.contains_key("active") {
                    StatusState::Active
                } else {
                    StatusState::Inactive
                };

                info!(
                    "spawn a add status task, name<{:?}>, path<{:?}> ,state<{:?}>",
                    name, path, state
                );
                commads.spawn(AddStatusTask::new(db, runtimer.as_mut(), name, path, state));
            }
            Some(unkown) => {
                error!("not has this command: {:?}", unkown);
            }
            None => {
                // 输出help
                let result = match res.get_result() {
                    Ok(result) => result,
                    Err(err) => {
                        error!("get active status error:{:?}", err);
                        commads.queue(|world: &mut World| {
                            let _ = world.resource_mut::<ActiveStatus>().try_result();
                        });
                        continue;
                    }
                };

                let table = result.iter().table_head(["id", "name", "path", "state"]);
                println!("{}\nrows: {}", table, table.count_rows() - 1);
            }
        }
    }
}

// pub async fn status() -> Result<()> {
//     let db = db(false).await;

//     let folders = match db.get_active_status().await {
//         Result::Ok(model) => model,
//         Err(_) => {
//             let test = db.all_status().await?;
//             if !test.is_empty() {
//                 return Err(anyhow::anyhow!("Not anyone status is active"));
//             }

//             db.upsert_status([StatusModel {
//                 id: None,
//                 name: "".to_owned(),
//                 path: ".".to_owned(),
//                 state: SetState::Active.to_string(),
//             }])
//             .await
//             .context("add a default status folder path error")?;

//             db.get_active_status()
//                 .await
//                 .context("get a default status folder path error")?
//         }
//     };

//     let table = table(
//         ["Id", "State", "Name", "Path"],
//         folders.into_iter().map(ToTableRecord::to_record),
//     );

//     println!("{}\nrows: {}", table, table.count_rows() - 1);

//     Ok(())
// }
