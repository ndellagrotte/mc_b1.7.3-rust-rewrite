use crate::GameState;
use crate::gameplay::actions::Actions;
use crate::gameplay::collision::CircleCollider;
use crate::gameplay::game_object::GameObject;
use crate::loading::TextureAssets;
use bevy::prelude::*;

pub struct PlayerPlugin;

#[derive(Component)]
pub struct Player;

/// This plugin handles player related stuff like movement
/// Player logic is only active during the State `GameState::Playing`
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing),
            (spawn_player, spawn_game_camera),
        )
        .add_systems(Update, move_player.run_if(in_state(GameState::Playing)));
    }
}

fn spawn_game_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Msaa::Off, GameObject));
}

fn spawn_player(mut commands: Commands, textures: Res<TextureAssets>) {
    commands.spawn((
        Sprite::from_image(textures.bevy.clone()),
        Transform::from_translation(Vec3::new(0., 0., 2.)),
        Player,
        GameObject,
        CircleCollider { radius: 128. },
    ));
}

fn move_player(
    time: Res<Time>,
    actions: Res<Actions>,
    mut player_query: Query<&mut Transform, With<Player>>,
) {
    let Some(movement) = actions.player_movement else {
        return;
    };
    let speed = 150.;
    let movement = Vec3::new(
        movement.x * speed * time.delta_secs(),
        movement.y * speed * time.delta_secs(),
        0.,
    );
    for mut player_transform in &mut player_query {
        player_transform.translation += movement;
    }
}
