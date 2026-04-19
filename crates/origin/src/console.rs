use std::borrow::Cow;

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
use rustyline::{DefaultEditor, error::ReadlineError};
use tracing::{error, info};

use crate::components::handle::{DbHandle, DbHandleError};

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

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct ConsoleReadTask {
    #[deref]
    pub handle: Option<DbHandle<ConsoleResult>>,
    pub error: Option<ConsoleResultError>,
    pub result: Option<ConsoleCommand>,
}

impl ConsoleReadTask {
    pub fn new(console: Console, runtimer: &mut TokioTasksRuntime) -> Self {
        Self {
            handle: Some(DbHandle::new(Self::spawn_task(console, runtimer))),
            error: None,
            result: None,
        }
    }

    pub fn spawn_task(
        console: Console,
        runtimer: &mut TokioTasksRuntime,
    ) -> tokio::task::JoinHandle<ConsoleResult> {
        let console_task = async move {
            let mut console = console;

            loop {
                let readline = console.readline("bili_extropy_ecs> ");

                if let Err(ReadlineError::Interrupted) = readline {
                    info!("收到中断信号 (Ctrl+C)，正在退出...");
                    return ConsoleResult {
                        console,
                        command: ConsoleCommand::Exit,
                    };
                }

                let Ok(line) = readline else {
                    continue;
                };

                let line = line.trim();

                // quit process command
                if matches!(line, "exit" | "quit" | "q") {
                    return ConsoleResult {
                        console,
                        command: ConsoleCommand::Exit,
                    };
                }

                info!("读到命令:{:?}", line);

                match console.execute_line(line) {
                    Ok(trims) if trims.len() <= 1 => {
                        error!("trims is empty");
                        continue;
                    }
                    Ok(trims) => {
                        return ConsoleResult {
                            console,
                            command: ConsoleCommand::Trims(Cow::Owned(trims)),
                        };
                    }
                    Err(err) => {
                        error!("trims error:{:?}", err);
                        continue;
                    }
                };
            }
        };

        runtimer.spawn_background_task(|_ctx| console_task)
    }

    pub fn try_repeat(
        &mut self,
        runtimer: &mut TokioTasksRuntime,
    ) -> Result<&ConsoleCommand, &ConsoleResultError> {
        let Some(mut task) = self.handle.take() else {
            if self.error.is_none() {
                let _ = self.error.insert(ConsoleResultError::ConsoleEmpty);
            }
            return Err(&ConsoleResultError::ConsoleEmpty);
        };

        if task.try_result().is_err_and(|err| !err.is_finished()) {
            let _ = self.handle.insert(task);
            return Err(&ConsoleResultError::NotFinished);
        };

        match task.take_result() {
            Ok(ConsoleResult {
                console: _,
                command: ConsoleCommand::Exit,
            }) => Ok(self.result.insert(ConsoleCommand::Exit)),
            Ok(ConsoleResult { console, command }) => {
                self.handle = Some(DbHandle::new(Self::spawn_task(console, runtimer)));

                Ok(self.result.insert(command))
            }
            Err(err) => {
                let error = self
                    .error
                    .get_or_insert(ConsoleResultError::DbHandleError(err));

                error!("console read task error:{:?}", error);
                Err(error)
            }
        }
    }

    pub fn error(&self) -> Option<&ConsoleResultError> {
        self.error.as_ref()
    }

    pub fn get_last_command(&self) -> Option<ConsoleCommand> {
        self.result.clone()
    }
}

#[derive(Debug, Message, Deref, DerefMut, Clone)]
pub struct ConsoleTrims(pub Cow<'static, Vec<String>>);

#[derive(Debug)]
pub struct ConsoleResult {
    pub console: Console,
    pub command: ConsoleCommand,
}

#[derive(Debug)]
pub enum ConsoleResultError {
    DbHandleError(DbHandleError<()>),
    ConsoleEmpty,
    NotFinished,
}

#[derive(Debug, Clone)]
pub enum ConsoleCommand {
    Trims(Cow<'static, Vec<String>>),
    Exit,
}

pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_message::<ConsoleTrims>()
            .add_systems(Startup, console_initalize)
            .add_systems(PreUpdate, console_task);
    }
}

fn console_initalize(mut commands: Commands, mut runtimer: ResMut<TokioTasksRuntime>) {
    commands.insert_resource(ConsoleReadTask::new(Console::default(), &mut runtimer));
}

fn console_task(
    mut commands: Commands,
    mut input: ResMut<ConsoleReadTask>,
    mut runtimer: ResMut<TokioTasksRuntime>,
) {
    let Ok(result) = input.try_repeat(runtimer.as_mut()) else {
        return;
    };

    match result {
        ConsoleCommand::Trims(trims) => {
            commands.write_message(ConsoleTrims(trims.clone()));
        }
        ConsoleCommand::Exit => {
            info!("custom input app exit command");
            commands.write_message(AppExit::Success);
        }
    }
}
