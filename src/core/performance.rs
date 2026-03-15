use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

const CHEMIN_POLICE_DIAGNOSTIC: &str = "polices/noto_sans_regular.ttf";

pub struct PerformanceJeuPlugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetPerformance {
    Equilibre,
}

#[derive(Resource, Debug, Clone)]
pub struct ParametresPerformanceJeu {
    pub preset: PresetPerformance,
    pub rayon_chunks_actifs: i32,
    pub subdivisions_maillage_terrain: u32,
    pub densite_decors: f32,
    pub ombres_directionnelles: bool,
    pub frequence_survol_hz: f64,
    pub frequence_hud_equipage_hz: f64,
    pub budget_chunks_terrain_par_frame: usize,
    pub budget_batches_decors_par_frame: usize,
}

impl ParametresPerformanceJeu {
    pub fn equilibre() -> Self {
        Self {
            preset: PresetPerformance::Equilibre,
            rayon_chunks_actifs: 2,
            subdivisions_maillage_terrain: 1,
            densite_decors: 0.70,
            ombres_directionnelles: false,
            frequence_survol_hz: 12.0,
            frequence_hud_equipage_hz: 4.0,
            budget_chunks_terrain_par_frame: 1,
            budget_batches_decors_par_frame: 1,
        }
    }
}

impl Default for ParametresPerformanceJeu {
    fn default() -> Self {
        Self::equilibre()
    }
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct StatistiquesPerformance {
    pub chunks_generes_frame: u32,
    pub chunks_mailles_frame: u32,
    pub decors_spawnes_frame: u32,
    pub survols_recalcules_frame: u32,
}

#[cfg(debug_assertions)]
#[derive(Resource, Debug, Clone, Copy)]
struct OverlayDiagnosticPerformance {
    texte: Entity,
    visible: bool,
}

#[cfg(debug_assertions)]
#[derive(Component)]
struct RacineDiagnosticPerformance;

impl Plugin for PerformanceJeuPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ParametresPerformanceJeu::default())
            .insert_resource(StatistiquesPerformance::default())
            .add_systems(First, reinitialiser_statistiques_performance);

        #[cfg(debug_assertions)]
        {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default())
                .add_systems(Startup, installer_overlay_diagnostics)
                .add_systems(
                    Update,
                    (basculer_overlay_diagnostics, rafraichir_overlay_diagnostics),
                );
        }
    }
}

fn reinitialiser_statistiques_performance(mut stats: ResMut<StatistiquesPerformance>) {
    *stats = StatistiquesPerformance::default();
}

#[cfg(debug_assertions)]
fn installer_overlay_diagnostics(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(CHEMIN_POLICE_DIAGNOSTIC);
    let texte = commands
        .spawn((
            Text::new("Perf"),
            TextFont::from_font_size(13.0).with_font(font),
            TextColor(Color::WHITE),
        ))
        .id();

    let racine = commands
        .spawn((
            RacineDiagnosticPerformance,
            Node {
                position_type: PositionType::Absolute,
                left: px(14.0),
                top: px(14.0),
                padding: UiRect::axes(px(10.0), px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.07, 0.07, 0.82)),
            BorderColor::all(Color::srgba(0.72, 0.61, 0.54, 0.45)),
            GlobalZIndex(1000),
        ))
        .id();

    commands.entity(racine).add_child(texte);
    commands.insert_resource(OverlayDiagnosticPerformance {
        texte,
        visible: true,
    });
}

#[cfg(debug_assertions)]
fn basculer_overlay_diagnostics(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut overlay: ResMut<OverlayDiagnosticPerformance>,
    mut racine: Single<&mut Visibility, With<RacineDiagnosticPerformance>>,
) {
    if !keyboard.just_pressed(KeyCode::F3) {
        return;
    }

    overlay.visible = !overlay.visible;
    **racine = if overlay.visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

#[cfg(debug_assertions)]
fn rafraichir_overlay_diagnostics(
    diagnostics: Res<DiagnosticsStore>,
    stats: Res<StatistiquesPerformance>,
    overlay: Res<OverlayDiagnosticPerformance>,
    mut textes: Query<&mut Text>,
) {
    let Ok(mut texte) = textes.get_mut(overlay.texte) else {
        return;
    };
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|entry| entry.smoothed())
        .unwrap_or(0.0);
    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|entry| entry.smoothed())
        .unwrap_or(0.0);

    *texte = format!(
        "Perf [{}]\nFPS {:.0} | {:.2} ms\nChunks gen {} | mailles {}\nDecors {} | survols {}",
        match overlay.visible {
            true => "F3 pour masquer",
            false => "F3 pour afficher",
        },
        fps,
        frame_ms,
        stats.chunks_generes_frame,
        stats.chunks_mailles_frame,
        stats.decors_spawnes_frame,
        stats.survols_recalcules_frame,
    )
    .into();
}
