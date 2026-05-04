use bevy::app::Plugin;

use crate::command::{
    auth::CommmandLoginPlugin, downloadrule::CommandDownloadrulePlugin, fetch::CommandFetchPlugin,
    help::CommandHelpPlugin, list::CommandListPlugin, status::CommandStatusPlugin,
};

pub mod auth;
pub mod clone;
pub mod downloadrule;
pub mod downloadtask;
pub mod fetch;
pub mod help;
pub mod list;
pub mod status;

pub mod initialize;

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(CommandListPlugin)
            .add_plugins(CommmandLoginPlugin)
            .add_plugins(CommandFetchPlugin)
            .add_plugins(CommandStatusPlugin)
            .add_plugins(CommandHelpPlugin)
            .add_plugins(CommandDownloadrulePlugin);
    }
}
