use bevy::{
    app::{Plugin, Startup, Update},
    ecs::{
        component::Component,
        entity::Entity,
        message::MessageReader,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use migration::OnConflict;
use sea_orm::EntityTrait;
use tracing::{error, info};

use crate::{
    components::{downloadrule::load::LoadDownloadrule, handle::ECSHandleResult},
    console::ConsoleTrims,
    db::Db,
    entity::downloadrule::{self, DownloadruleActiveModel, DownloadruleModel},
};

pub const HELP_DOWNLOAD_RULE: &str = r#"
Back up your favorite bilibili online resources with RESP.

Usage: downloadrule <COMMAND> [SUB_COMMAND] [OPTIONS]

Commands:
    insert                      Insert a download rule.
        <name> [--AddRule]          Insert a <name> rule.

    remove                      Remove rule.
        <id>                    remove by rule id.

    help                        Print this message or the help of the given subcommand(s)

AddRule:
    -d,         --data          Show debug messages
    

Options:
    -v,         --verbose       Show debug messages
    -h,         --help          Print help
    -V,         --version       Print version
    -id [ID],   --id [ID]       Point ID

Example:
    List medias
    List account --id 114514 
    List upper followings
"#;

pub const DOWNLOAD_RULE_COMMAND_INDEX: usize = 2;

pub struct CommandDownloadrulePlugin;

impl Plugin for CommandDownloadrulePlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(Startup, load_downloadrule).add_systems(
            Update,
            (
                spawn_list_task,
                (download_rule_insert_task,).after(spawn_list_task),
            ),
        );
    }
}

pub fn load_downloadrule(
    mut commands: Commands,
    db: Res<Db>,
    mut runtimer: ResMut<TokioTasksRuntime>,
) {
    let mut loading = LoadDownloadrule::new(db.clone(), runtimer.as_mut());
    let _ = loading.block_on();

    let Ok(models) = loading
        .take_result()
        .map_err(|err| error!("load sql active status error: {:?}", err))
    else {
        return;
    };

    commands.spawn_batch(models.into_iter().map(|e| ActiveDownloadrule(e)));
}

pub fn spawn_list_task(
    mut commands: Commands,
    db: Res<Db>,
    mut runtimer: ResMut<TokioTasksRuntime>,
    mut console_message: MessageReader<ConsoleTrims>,
) {
    for message in console_message.read() {
        let _db = db.clone();
        let ConsoleTrims { args, argv } = message;

        if !args.get(1).is_some_and(|list| list.eq("downloadrule")) {
            continue;
        }

        match args.get(DOWNLOAD_RULE_COMMAND_INDEX).map(String::as_str) {
            Some("insert") => match args.get(3).map(String::as_str) {
                Some(rule_name) => {
                    commands.spawn(DownloadRuleInsertTask::new(
                        db.clone(),
                        runtimer.as_mut(),
                        downloadrule::DownloadruleActiveModel::from_argv_name(
                            &argv,
                            rule_name.to_string(),
                        ),
                    ));
                }
                None => {
                    error!("not a rule name");
                }
            },

            Some(unkown) => {
                error!("not has this command: {:?}", unkown);
            }

            None => {
                // 输出help
                commands.spawn(LoadDownloadrule::new(db.clone(), runtimer.as_mut()));
            }
        }
    }
}

pub fn download_rule_insert_task(
    mut commands: Commands,
    query: Query<(&mut DownloadRuleInsertTask, Entity)>,
) {
    for (mut task, entity) in query {
        let Ok(result) = task.try_result() else {
            continue;
        };
        commands.entity(entity).despawn();

        info!("insert a rule id<{}>", result);
    }
}

pub type RuleId = i64;

#[derive(Debug, Component, Deref, DerefMut)]
pub struct DownloadRuleInsertTask(pub ECSHandleResult<RuleId, anyhow::Error>);

impl DownloadRuleInsertTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime, model: DownloadruleActiveModel) -> Self {
        let task = async move {
            let pri = downloadrule::DownloadruleEntity::insert(model)
                .on_conflict(
                    OnConflict::columns([downloadrule::Column::Id])
                        .update_columns([
                            downloadrule::Column::Name,
                            downloadrule::Column::Size,
                            downloadrule::Column::RelationSize,
                            downloadrule::Column::Date,
                            downloadrule::Column::RelationDate,
                            downloadrule::Column::Repeat,
                            downloadrule::Column::State,
                        ])
                        .to_owned(),
                )
                .exec_with_returning(&db.db)
                .await?;

            Ok(pri.id)
        };
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(ECSHandleResult::new(handle))
    }
}

#[derive(Debug, Component, Deref, DerefMut, Clone)]
pub struct ActiveDownloadrule(pub DownloadruleModel);
