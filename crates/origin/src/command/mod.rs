use bevy::app::Plugin;

use crate::command::{
    auth::CommmandLoginPlugin, fetch::CommandFetchPlugin, list::CommandListPlugin,
    status::CommandStatusPlugin,
};

pub mod auth;
pub mod clone;
pub mod fetch;
pub mod list;
pub mod status;

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(CommandListPlugin)
            .add_plugins(CommmandLoginPlugin)
            .add_plugins(CommandFetchPlugin)
            .add_plugins(CommandStatusPlugin);
    }
}
