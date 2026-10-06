use crate::GameState;
use bevy::prelude::*;

pub struct GameObjectPlugin;

impl Plugin for GameObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(GameState::Playing), cleanup_game_objects);
    }
}

#[derive(Component)]
pub struct GameObject;

fn cleanup_game_objects(mut commands: Commands, menu: Query<Entity, With<GameObject>>) {
    for entity in menu.iter() {
        commands.entity(entity).despawn();
    }
}
