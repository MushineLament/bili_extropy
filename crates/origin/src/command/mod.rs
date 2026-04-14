use bevy::app::{Plugin, Update};

pub mod list;

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(Update, list::list_medias);
    }
}
