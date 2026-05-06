use bevy::app::Plugin;

use crate::command::{
    account::CommmandAccountPlugin, downloadrule::CommandDownloadrulePlugin,
    downloadtask::CommandDownloadtaskPlugin, fetch::CommandFetchPlugin, help::CommandHelpPlugin,
    list::CommandListPlugin, status::CommandStatusPlugin,
};

pub mod account;
pub mod clone;
pub mod downloadrule;
pub mod downloadtask;
pub mod fetch;
pub mod help;
pub mod list;
pub mod status;

pub const HELP: &str = "help";

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(CommandListPlugin)
            .add_plugins(CommmandAccountPlugin)
            .add_plugins(CommandFetchPlugin)
            .add_plugins(CommandStatusPlugin)
            .add_plugins(CommandHelpPlugin)
            .add_plugins(CommandDownloadrulePlugin)
            .add_plugins(CommandDownloadtaskPlugin);
    }
}
