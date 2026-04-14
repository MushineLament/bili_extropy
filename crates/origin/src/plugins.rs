use bevy::{
    MinimalPlugins,
    app::{App, Plugin},
    log::LogPlugin,
};
use bevy_tokio_tasks::TokioTasksPlugin;

use crate::{console::ConsolePlugin, db::DbPlugin};

pub struct MainPlugin;

impl Plugin for MainPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(MinimalPlugins)
            .add_plugins(LogPlugin::default())
            .add_plugins(TokioTasksPlugin::default())
            .add_plugins(ConsolePlugin)
            .add_plugins(DbPlugin);
    }
}

pub fn app() {
    App::new().add_plugins(MainPlugin).run();
}
