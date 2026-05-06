use bevy::{
    app::{Plugin, Update},
    ecs::message::MessageReader,
};
use tracing::info;

use crate::console::ConsoleTrims;

pub const HELP: &str = r#"
Back up your favorite bilibili online resources with RESP.

Usage: [COMMAND]

Commands:
    account     User account
    list        List infomation
    status      Download media into folder
    fetch       Fetch metadata of following collection, medias, uppers
    pull        Download medias by downloadtask tables
    clone       Download single media
    help        Print this.

Options:
    -v, --verbose  Show debug messages
    -h, --help     Print help
    -V, --version  Print version
    
Example:
    list        
    status      #Print download medias into path.
"#;

pub struct CommandHelpPlugin;

impl Plugin for CommandHelpPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(Update, spawn_fetch_task);
    }
}

pub fn spawn_fetch_task(mut console_message: MessageReader<ConsoleTrims>) {
    for message in console_message.read() {
        let ConsoleTrims { args, argv: _ } = message;

        if !args.get(1).is_some_and(|list| list.eq("help")) {
            continue;
        }

        info!("{}", HELP);
    }
}
