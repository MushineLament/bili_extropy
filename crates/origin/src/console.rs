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
    platform::collections::HashMap,
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use rustyline::{DefaultEditor, error::ReadlineError};
use tracing::{error, info};

use crate::components::handle::{ECSHandle, ECSHandleError};

pub const APP_NAME: &str = "bili_extropy_ecs";

pub const HISTROY: &str = ".temp/.histroy.txt";

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

#[derive(Debug, Resource)]
pub struct ConsoleReadTask {
    pub handle: ECSHandle<ConsoleResult>,
    // last error
    pub error: Option<ConsoleResultError>,
    // last result
    pub result: Option<ConsoleCommand>,
}

impl ConsoleReadTask {
    pub fn new(mut console: Console, runtimer: &mut TokioTasksRuntime) -> Self {
        if let Err(e) = console.load_history(HISTROY) {
            error!("read history txt error: {}", e);
        }
        Self {
            handle: ECSHandle::new(Self::spawn_task(console, runtimer)),
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

                if !line.is_empty() {
                    match console.add_history_entry(line) {
                        Ok(_bool) => (),
                        Err(err) => {
                            error!("readline add histroy error:{:?}", err);
                        }
                    };
                }

                info!("读到命令:{:?}", line);

                match console.execute_line(line) {
                    Ok(trims) if trims.len() <= 1 => {
                        error!("trims is empty");
                        continue;
                    }
                    Ok(trims) => {
                        let (args, argv) = argmap::parse(trims.into_iter());
                        return ConsoleResult {
                            console,
                            command: ConsoleCommand::Trims(ConsoleTrims {
                                args: Cow::Owned(args),
                                argv: Cow::Owned(HashMap::from_iter(argv)),
                            }),
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

    #[track_caller]
    pub fn try_repeat(
        &mut self,
        runtimer: &mut TokioTasksRuntime,
    ) -> Result<&ConsoleCommand, &ConsoleResultError> {
        if !self.is_finished_mut() {
            return Err(&ConsoleResultError::NotFinished);
        }

        match self.handle.take_result() {
            Ok(ConsoleResult {
                console: _,
                command: ConsoleCommand::Exit,
            }) => Ok(self.result.insert(ConsoleCommand::Exit)),
            Ok(ConsoleResult { console, command }) => {
                self.handle.repeat(Self::spawn_task(console, runtimer));

                Ok(self.result.insert(command))
            }
            Err(err) => {
                let error = self
                    .error
                    .get_or_insert(ConsoleResultError::ECSHandleError(err));

                Err(error)
            }
        }
    }

    pub fn try_result(&mut self) -> Result<&ConsoleCommand, &ECSHandleError<()>> {
        match self.handle.try_result() {
            Ok(ConsoleResult {
                console: _,
                command: ConsoleCommand::Exit,
            }) => Ok(self.result.insert(ConsoleCommand::Exit)),
            Ok(ConsoleResult {
                console: _,
                command,
            }) => Ok(command),
            Err(error) => Err(error),
        }
    }

    pub fn error(&self) -> Option<&ConsoleResultError> {
        self.error.as_ref()
    }

    pub fn get_last_command(&self) -> Option<&ConsoleCommand> {
        self.result.as_ref()
    }

    pub fn is_finished_mut(&mut self) -> bool {
        match self.try_result() {
            Ok(_) => true,
            Err(err) => err.is_finished(),
        }
    }
}

#[derive(Debug, Message, Clone)]
pub struct ConsoleTrims {
    pub args: Cow<'static, Vec<String>>,
    pub argv: Cow<'static, HashMap<String, Vec<String>>>,
}

#[derive(Debug)]
pub struct ConsoleResult {
    pub console: Console,
    pub command: ConsoleCommand,
}

#[derive(Debug)]
pub enum ConsoleResultError {
    ECSHandleError(ECSHandleError<()>),
    ConsoleEmpty,
    NotFinished,
}

#[derive(Debug, Clone)]
pub enum ConsoleCommand {
    Trims(ConsoleTrims),
    Exit,
}

pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_message::<ConsoleTrims>()
            .add_systems(Startup, console_initalize)
            .add_systems(PreUpdate, console_task_repeat);
    }
}

fn console_initalize(mut commands: Commands, mut runtimer: ResMut<TokioTasksRuntime>) {
    commands.insert_resource(ConsoleReadTask::new(Console::default(), &mut runtimer));
}

fn console_task_repeat(
    mut commands: Commands,
    mut input: ResMut<ConsoleReadTask>,
    mut runtimer: ResMut<TokioTasksRuntime>,
) {
    let Ok(result) = input.try_result() else {
        return;
    };

    match result {
        ConsoleCommand::Trims(trims) => {
            commands.write_message(trims.clone());
        }
        ConsoleCommand::Exit => {
            info!("custom input app exit command");

            commands.write_message(AppExit::Success);

            let Ok(mut input) = input.handle.take_result() else {
                error!("lost Console");
                return;
            };

            if let Err(err) = input.console.save_history(HISTROY) {
                error!("Console save history error:{:?}", err);
            };
            return;
        }
    }

    let _result = input.try_repeat(runtimer.as_mut());
}
