use crate::gameplay::collision::CircleCollider;
use crate::gameplay::game_object::GameObject;
use crate::gameplay::player::Player;
use crate::{GameResult, GameState};
use bevy::prelude::*;

pub struct FinishAreaPlugin;

#[derive(Component, Debug, PartialEq, Eq)]
pub enum FinishArea {
    Win,
    Lose,
}

/// This plugin handles finish_areas - when the player touches a finish area the game is over.
impl Plugin for FinishAreaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_finish_areas)
            .add_systems(
                Update,
                end_game_on_player_touch.run_if(in_state(GameState::Playing)),
            );
    }
}

fn spawn_finish_areas(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    windows: Query<&Window>,
) -> Result {
    let window = windows.single()?;
    let (win_area_translation, loose_area_translation) =
        determine_area_placements(window.resolution.width(), window.resolution.height());

    const RADIUS: f32 = 50.;
    let circle = meshes.add(Circle::new(RADIUS));
    commands.spawn((
        Mesh2d(circle.clone()),
        MeshMaterial2d(materials.add(Color::hsl(120., 0.95, 0.7))),
        Transform::from_translation(win_area_translation),
        FinishArea::Win,
        CircleCollider { radius: RADIUS },
        GameObject,
    ));
    commands.spawn((
        Mesh2d(circle.clone()),
        MeshMaterial2d(materials.add(Color::hsl(0., 0.95, 0.7))),
        Transform::from_translation(loose_area_translation),
        FinishArea::Lose,
        CircleCollider { radius: RADIUS },
        GameObject,
    ));

    Ok(())
}

// determine the placements of the win- and loosearea depending on the window dimensions
fn determine_area_placements(window_width: f32, window_height: f32) -> (Vec3, Vec3) {
    let distance_from_player: f32;

    if window_width < window_height {
        distance_from_player = window_height / 3.;
        // portrait
        (
            Vec3::new(0., distance_from_player, 1.),
            Vec3::new(0., -distance_from_player, 1.),
        )
    } else {
        distance_from_player = window_width / 3.;
        // landscape
        (
            Vec3::new(distance_from_player, 0., 1.),
            Vec3::new(-distance_from_player, 0., 1.),
        )
    }
}

fn end_game_on_player_touch(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    player_query: Query<(&Transform, &CircleCollider), With<Player>>,
    finish_query: Query<(&Transform, &CircleCollider, &FinishArea)>,
) {
    // Normally there is a single player, but the loop works for any count.
    for (player_tf, player_circ) in player_query.iter() {
        let player_pos = player_tf.translation.truncate(); // Vec2 (x, y)

        for (finish_tf, finish_circ, finish_area) in finish_query.iter() {
            let finish_pos = finish_tf.translation.truncate();

            // Vector between the two centres.
            let delta = finish_pos - player_pos;
            // Squared distance (cheaper than sqrt).
            let dist_sq = delta.length_squared();
            // Sum of radii.
            let radius_sum = player_circ.radius + finish_circ.radius;

            // Collision test.
            if dist_sq <= radius_sum * radius_sum {
                match finish_area {
                    FinishArea::Win => {
                        info!("Player won that game!");
                        commands.insert_resource(GameResult::Won);
                    }
                    FinishArea::Lose => {
                        info!("Player lost that game!");
                        commands.insert_resource(GameResult::Lost);
                    }
                }
                next_state.set(GameState::Menu);
                return;
            }
        }
    }
}
