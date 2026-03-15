use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::render::{
    RenderPlugin,
    settings::{InstanceFlags, RenderCreation, WgpuSettings},
};
use bevy::window::WindowPlugin;

use crate::colony::ColonyPlugin;
use crate::construction::ConstructionPlugin;
use crate::core::{CorePlugin, GameState, SIMULATION_HZ};
use crate::son::SonPlugin;
use crate::ui::UiPlugin;
use crate::world::WorldPlugin;

pub fn run() {
    let racine_assets = format!("{}/assets", env!("CARGO_MANIFEST_DIR"));
    let default_plugins = DefaultPlugins
        .set(AssetPlugin {
            file_path: racine_assets,
            ..default()
        })
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: "SpaceWorld".into(),
                resolution: (1600, 900).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        })
        .set(RenderPlugin {
            render_creation: RenderCreation::Automatic(WgpuSettings {
                // Keep startup clean on machines without validation layers installed.
                instance_flags: InstanceFlags::empty(),
                ..default()
            }),
            ..default()
        })
        .build()
        .disable::<bevy::gilrs::GilrsPlugin>();

    App::new()
        .add_plugins(default_plugins)
        .init_state::<GameState>()
        .insert_resource(Time::<Fixed>::from_hz(SIMULATION_HZ))
        .insert_resource(ClearColor(Color::srgb(0.74, 0.53, 0.43)))
        .add_plugins((
            CorePlugin,
            WorldPlugin,
            ColonyPlugin,
            ConstructionPlugin,
            SonPlugin,
            UiPlugin,
        ))
        .run();
}
