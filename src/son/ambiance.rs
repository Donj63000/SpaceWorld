use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
use bevy::prelude::*;

const CHEMIN_PISTE_AMBIANCE: &str = "sons/ambiance_spatiale.wav";

#[derive(Resource, Clone)]
pub(super) struct PisteAmbiance(pub Handle<AudioSource>);

#[derive(Component)]
pub(super) struct MusiqueAmbiance;

pub(super) fn charger_piste_ambiance(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(PisteAmbiance(asset_server.load(CHEMIN_PISTE_AMBIANCE)));
}

pub(super) fn jouer_piste_ambiance(mut commands: Commands, piste: Res<PisteAmbiance>) {
    commands.spawn((
        Name::new("Musique Ambiance"),
        MusiqueAmbiance,
        AudioPlayer::new(piste.0.clone()),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.20)),
    ));
}
