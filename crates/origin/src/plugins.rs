use bevy::{
    MinimalPlugins,
    app::{App, Plugin},
};
use bevy_tokio_tasks::TokioTasksPlugin;

use crate::{
    command::{CommandPlugin, clone::DownloadPlugin},
    console::ConsolePlugin,
    db::DbPlugin,
    log::ExtropyLogPlugin,
};

pub struct MainPlugin;

impl Plugin for MainPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(MinimalPlugins)
            // .add_plugins(LogPlugin {
            //     filter: "sqlx=off".to_string(),
            //     ..Default::default()
            // })
            .add_plugins(ExtropyLogPlugin)
            .add_plugins(TokioTasksPlugin::default())
            .add_plugins(ConsolePlugin)
            .add_plugins(CommandPlugin)
            .add_plugins(DownloadPlugin)
            .add_plugins(DbPlugin);
    }
}

pub fn app() {
    App::new().add_plugins(MainPlugin).run();
}
