use bevy::app::{App, Plugin, Update};
use bevy::ecs::change_detection::MaybeLocation;
use bevy::ecs::message::MessageReader;
use bevy::ecs::resource::Resource;
use bevy::ecs::system::ResMut;
use bevy::input::InputPlugin;
use bevy::log::LogPlugin;
use bevy::log::tracing_subscriber::reload;
use bevy::log::tracing_subscriber::{self, EnvFilter, prelude::*};
use bevy::prelude::{Deref, DerefMut};
use tracing::{dispatcher, error, info};
use tracing_subscriber::Registry;

use crate::console::ConsoleTrims;

pub const DEFAULT_LOG: &str = "info,sqlx=off";

#[derive(Debug)]
pub struct LogExtropyError {
    caller: MaybeLocation,
    error: LogExtropyErrorKind,
}

impl std::error::Error for LogExtropyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.error {
            LogExtropyErrorKind::Reload(error) => Some(error),
            LogExtropyErrorKind::ParseError(error) => Some(error),
        }
    }
}

#[derive(Debug)]
pub enum LogExtropyErrorKind {
    Reload(reload::Error),
    ParseError(tracing_subscriber::filter::ParseError),
}

#[derive(Resource, Deref, DerefMut)]
pub struct LogExtropy(pub reload::Handle<EnvFilter, Registry>);

impl std::fmt::Display for LogExtropyError {
    #[track_caller]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.error {
            LogExtropyErrorKind::Reload(error) => {
                write!(f, "Reload error:{}, caller:{:?}", error, self.caller)
            }
            LogExtropyErrorKind::ParseError(error) => {
                write!(f, "Parse error:{}, caller:{:?}", error, self.caller)
            }
        }
    }
}

impl From<reload::Error> for LogExtropyError {
    #[track_caller]
    fn from(error: reload::Error) -> Self {
        Self {
            caller: MaybeLocation::caller(),
            error: LogExtropyErrorKind::Reload(error),
        }
    }
}

impl From<tracing_subscriber::filter::ParseError> for LogExtropyError {
    #[track_caller]
    fn from(error: tracing_subscriber::filter::ParseError) -> Self {
        Self {
            caller: MaybeLocation::caller(),
            error: LogExtropyErrorKind::ParseError(error),
        }
    }
}

impl LogExtropy {
    #[track_caller]
    pub fn get_current(&self) -> Result<String, reload::Error> {
        self.0.with_current(|test| test.to_string())
    }

    #[track_caller]
    pub fn set<S: AsRef<str>>(&mut self, dirs: S) -> Result<(), LogExtropyError> {
        let new_filter = EnvFilter::try_new(dirs)?;
        Ok(self.modify(|filter| *filter = new_filter)?)
    }

    #[track_caller]
    pub fn add<S: AsRef<str>>(&mut self, dirs: S) -> Result<(), LogExtropyError> {
        let add = format!("{:?},{:?}", self.get_current()?, dirs.as_ref());
        self.set(add)
    }

    #[track_caller]
    pub fn remove<S: AsRef<str>>(&mut self, dirs: S) -> Result<(), LogExtropyError> {
        let remove = self.get_current()?.replace(dirs.as_ref(), "");
        self.set(remove)
    }
}

fn change_log_filter(
    mut console_message: MessageReader<ConsoleTrims>,
    mut reload_handle: ResMut<LogExtropy>,
) {
    for message in console_message.read() {
        let (args, _argv) = argmap::parse(message.0.iter());

        if !args.get(1).is_some_and(|list| list.eq("log")) {
            continue;
        }

        match args.get(2).map(String::as_str) {
            Some("set") => {
                let args = args.get(3).map(String::as_str).unwrap_or_default();
                if let Err(error) = reload_handle.set(args) {
                    println!("log setting set error: {:?}", error);
                }
                info!("✅ log setting: {:?}", reload_handle.get_current());
            }
            Some("add") => {
                let args = args.get(3).map(String::as_str).unwrap_or_default();
                if let Err(error) = reload_handle.add(args) {
                    println!("log setting add error: {:?}", error);
                }
                info!("✅ log setting: {:?}", reload_handle.get_current());
            }
            Some("remove") => {
                let args = args.get(3).map(String::as_str).unwrap_or_default();
                if let Err(error) = reload_handle.remove(args) {
                    println!("log setting remove error: {:?}", error);
                }
                info!("✅ log setting: {:?}", reload_handle.get_current());
            }
            Some("reset") => {
                if let Err(error) = reload_handle.set(DEFAULT_LOG) {
                    error!("Failed to reset log: {:?}", error);
                } else {
                    info!("✅ reset log settring");
                }
            }
            None => {
                println!("log setting: {:?}", reload_handle.get_current());
            }
            _ => {}
        }
    }
}

pub struct ExtropyLogPlugin;

impl Plugin for ExtropyLogPlugin {
    fn build(&self, app: &mut App) {
        let _default = LogPlugin::default();

        // 1. 默认的过滤器（例如，从 RUST_LOG 环境变量读取）
        let default_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::builder().parse_lossy(DEFAULT_LOG)); // 默认 'info' 级别

        // 2. 使用 reload::Layer 包装过滤器，并获得操作句柄
        let (filtered_layer, reload_handle) = reload::Layer::new(default_filter);

        // 3. 组装订阅者（也可添加自定义的文件写入层等）
        let subscriber = tracing_subscriber::registry()
            .with(filtered_layer)
            .with(tracing_subscriber::fmt::Layer::default());

        // 4. 设为全局订阅者
        dispatcher::set_global_default(subscriber.into()).expect("Failed to set subscriber");

        // 5. 将句柄存入 ECS 资源
        app.insert_resource(LogExtropy(reload_handle));

        app.add_plugins(InputPlugin::default())
            .add_systems(Update, change_log_filter);
    }
}
