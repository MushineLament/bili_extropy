use bevy::{
    app::{Plugin, Update},
    ecs::message::MessageReader,
};
use tracing::info;

use crate::console::ConsoleTrims;

pub const HELP: &str = r#"
Back up your favorite bilibili online resources with RESP.

Usage: [OPTIONS] [COMMAND]

Commands:
    auth        Auth account
    list        List accounts/sets/ups/medias [alias: ls, l]
    activate    Activate obj [alias: active, a]
    deactivate  Deactivate obj [alias: d]
    status      download media into folder [alias: s]
    fetch       Fetch metadata of following ups, fav sets, medias, ups [alias: f]
    pull        Pull fetched medias [alias: p]
    clone       download single medias [alias: c]
    like        Like medias
    completion  Generate completion script
    help        Print this message or the help of the given subcommand(s)

Options:
    -v, --verbose  Show debug messages
    -h, --help     Print help
    -V, --version  Print version
    
Example:
    list account
    clone 
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
