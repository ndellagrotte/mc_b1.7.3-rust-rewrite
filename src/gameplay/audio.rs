use crate::GameState;
use crate::gameplay::actions::{Actions, set_movement_actions};
use crate::loading::AudioAssets;
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

pub struct InternalAudioPlugin;

// This plugin is responsible to control the game audio
impl Plugin for InternalAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AudioPlugin)
            .add_systems(OnEnter(GameState::Playing), start_audio)
            .add_systems(
                Update,
                control_flying_sound
                    .after(set_movement_actions)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnExit(GameState::Playing), stop_audio);
    }
}

#[derive(Resource)]
struct FlyingAudio(Handle<AudioInstance>);

fn start_audio(mut commands: Commands, audio_assets: Res<AudioAssets>, audio: Res<Audio>) {
    audio.pause();
    let handle = audio
        .play(audio_assets.flying.clone())
        .looped()
        .with_volume(0.3)
        .handle();
    commands.insert_resource(FlyingAudio(handle));
}

fn stop_audio(
    mut commands: Commands,
    audio: Res<FlyingAudio>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
) {
    if let Some(mut instance) = audio_instances.get_mut(&audio.0) {
        instance.stop(AudioTween::default());
    }
    commands.remove_resource::<FlyingAudio>();
}

fn control_flying_sound(
    actions: Res<Actions>,
    audio: Res<FlyingAudio>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
) {
    if let Some(mut instance) = audio_instances.get_mut(&audio.0) {
        match instance.state() {
            PlaybackState::Paused { .. } if actions.player_movement.is_some() => {
                instance.resume(AudioTween::default());
            }
            PlaybackState::Playing { .. } if actions.player_movement.is_none() => {
                instance.pause(AudioTween::default());
            }
            _ => {}
        }
    }
}
