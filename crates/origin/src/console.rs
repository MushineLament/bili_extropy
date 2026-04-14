use anyhow::Result;
use bevy::{
    app::{AppExit, Plugin, PreUpdate, Startup},
    ecs::{
        change_detection::MaybeLocation,
        message::Message,
        resource::Resource,
        system::{Commands, ResMut},
    },
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use rustyline::DefaultEditor;
use tokio::task::JoinHandle;
use tracing::{error, info};

pub const APP_NAME: &str = "bili_extropy_ecs";

#[derive(Debug, Deref, DerefMut)]
pub struct Console(pub DefaultEditor);

impl Console {
    #[track_caller]
    fn execute_line(&mut self, line: &str) -> Result<Vec<String>> {
        if line.to_lowercase() == "mushinelament" {
            info!("!The world is Unworld!");
        }

        let split = shell_words::split(line).map_err(|err| {
            anyhow::anyhow!(
                "caller: {:?},shell_words::split error: {:?}",
                MaybeLocation::caller(),
                err
            )
        })?;

        let mut trim = vec![APP_NAME.to_string()];

        trim.extend(split);

        if trim.len() <= 1 {
            return Err(anyhow::anyhow!("plase input commands"));
        }

        Ok(trim)
    }
}

impl Default for Console {
    #[track_caller]
    fn default() -> Self {
        let console = DefaultEditor::new().expect("error initialize resp");
        Self(console)
    }
}

#[derive(Debug, Resource, Deref, DerefMut, Default)]
pub struct ConsoleReadTask(pub Option<JoinHandle<ConsoleCommand>>);

impl ConsoleReadTask {
    pub fn as_task(console: Console, runtimer: &mut TokioTasksRuntime) -> Self {
        let console_task = async move {
            let mut console = console;

            loop {
                let Ok(line) = console.readline("bili_extropy_ecs> ") else {
                    continue;
                };

                let line = line.trim();

                // quit process command
                if matches!(line, "exit" | "quit" | "q") {
                    return ConsoleCommand::Exit;
                }

                info!("读到命令:{:?}", line);

                match console.execute_line(line) {
                    Ok(trims) if trims.len() <= 1 => {
                        error!("trims is empty");
                        continue;
                    }
                    Ok(trims) => {
                        return ConsoleCommand::Trims((console, trims));
                    }
                    Err(err) => {
                        error!("trims error:{:?}", err);
                        continue;
                    }
                };
            }
        };

        let task = runtimer.spawn_background_task(|_ctx| console_task);

        Self(Some(task))
    }
}

#[derive(Debug, Message, Deref, DerefMut, Clone)]
pub struct ConsoleMessage(pub Vec<String>);

#[derive(Debug)]
pub enum ConsoleCommand {
    Trims((Console, Vec<String>)),
    Exit,
}

pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<ConsoleReadTask>()
            .add_message::<ConsoleMessage>()
            .add_systems(Startup, console_initalize)
            .add_systems(PreUpdate, console_task);
    }
}

fn console_initalize(mut input: ResMut<ConsoleReadTask>, mut runtimer: ResMut<TokioTasksRuntime>) {
    if input.is_some() {
        return;
    }

    *input = ConsoleReadTask::as_task(Console::default(), &mut runtimer);
}

fn console_task(
    mut commands: Commands,
    mut input: ResMut<ConsoleReadTask>,
    mut runtimer: ResMut<TokioTasksRuntime>,
) {
    let Some(handle) = input.take() else {
        error!("console lose, respawn a default");
        *input = ConsoleReadTask::as_task(Console::default(), &mut runtimer);
        return;
    };

    if !handle.is_finished() {
        let _handle = input.insert(handle);
        return;
    }

    let Ok(result) = bevy::tasks::block_on(handle) else {
        return;
    };

    match result {
        ConsoleCommand::Trims((console, trims)) => {
            commands.write_message(ConsoleMessage(trims));
            *input = ConsoleReadTask::as_task(console, &mut runtimer);
        }
        ConsoleCommand::Exit => {
            info!("custom input app exit command");
            commands.write_message(AppExit::Success);
        }
    }
}
