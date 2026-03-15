use bevy::prelude::*;

mod ambiance;

pub struct SonPlugin;

impl Plugin for SonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                ambiance::charger_piste_ambiance,
                ambiance::jouer_piste_ambiance,
            )
                .chain(),
        );
    }
}
