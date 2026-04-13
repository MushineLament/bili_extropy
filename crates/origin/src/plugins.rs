use bevy::{
    MinimalPlugins,
    app::{App, Plugin},
    log::LogPlugin,
};

use crate::console::ConsolePlugin;

pub struct MainPlugin;

impl Plugin for MainPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(MinimalPlugins)
            .add_plugins(LogPlugin::default())
            .add_plugins(ConsolePlugin);
    }
}

pub fn app() {
    App::new().add_plugins(MainPlugin).run();
}
