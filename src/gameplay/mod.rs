mod actions;
mod audio;
mod collision;
mod finish_areas;
mod game_object;
mod player;

use bevy::prelude::*;

use actions::ActionsPlugin;
use audio::InternalAudioPlugin;
use finish_areas::FinishAreaPlugin;
use game_object::GameObjectPlugin;
use player::PlayerPlugin;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ActionsPlugin,
            InternalAudioPlugin,
            FinishAreaPlugin,
            GameObjectPlugin,
            PlayerPlugin,
        ));
    }
}
