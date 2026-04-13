use anyhow::Result;
use bevy::{
    app::{Plugin, PreUpdate},
    ecs::{
        change_detection::MaybeLocation,
        event::Event,
        resource::Resource,
        system::{Commands, ResMut},
    },
    prelude::{Deref, DerefMut},
    tasks::{IoTaskPool, Task},
};
use rustyline::DefaultEditor;
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

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct ConsoleReadTask(pub Task<(Console, Vec<String>)>);

impl ConsoleReadTask {
    pub fn as_task(console: Console) -> Self {
        let async_tasks = IoTaskPool::get();

        let console_task = async move {
            let mut console = console;

            loop {
                let Ok(line) = console.readline("bili_extropy_ecs> ") else {
                    continue;
                };

                let line = line.trim();

                // quit process command
                if matches!(line, "exit" | "quit" | "q") {
                    todo!("PS:quit process command");
                }

                info!("读到命令:{:?}", line);

                match console.execute_line(line) {
                    Ok(trims) if trims.len() <= 1 => {
                        error!("trims is empty");
                        continue;
                    }
                    Ok(trims) => {
                        return (console, trims);
                    }
                    Err(err) => {
                        error!("trims error:{:?}", err);
                        continue;
                    }
                };
            }
        };
        ConsoleReadTask(async_tasks.spawn(console_task))
    }
}

impl Default for ConsoleReadTask {
    fn default() -> Self {
        let console = Console::default();
        Self::as_task(console)
    }
}

#[derive(Debug, Event, Deref, DerefMut)]
pub struct ConsoleTrigger(pub Vec<String>);

pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<ConsoleReadTask>()
            .add_systems(PreUpdate, run_resp);
    }
}

fn run_resp(mut commands: Commands, mut task: ResMut<ConsoleReadTask>) {
    if !task.0.is_finished() {
        return;
    }

    let (console, trims) = bevy::tasks::block_on(&mut task.0);

    commands.trigger(ConsoleTrigger(trims));

    *task = ConsoleReadTask::as_task(console);
}
