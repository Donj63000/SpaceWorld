mod placement;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::colony::{
    StructureIdAllocator, StructureKind, StructureState, ZoneRechargeBase, structure_cells,
};
use crate::core::{GameState, WorldOrigin};
use crate::world::{
    HoveredCell, PlanetProfile, WorldCache, WorldSeed, structure_anchor_translation,
};

pub use placement::{
    EtatConnexionPlacement, PlacementValidation, cellules_interaction_structure,
    cout_trajet_base_vers, meilleure_cellule_interaction, validate_structure_placement,
};

pub struct ConstructionPlugin;

fn etat_interactif(state: Res<State<GameState>>) -> bool {
    matches!(state.get(), GameState::InGame | GameState::Paused)
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct BlocageConstructionParUi(pub bool);

#[derive(Resource, Clone, Copy)]
pub struct SelectedBuild(pub StructureKind);

#[derive(Resource, Clone, Debug, Default)]
pub struct PlacementFeedback {
    pub valid: bool,
    pub reason: String,
    pub hovered_anchor: Option<IVec2>,
    pub cellule_interaction: Option<IVec2>,
    pub etat_connexion: EtatConnexionPlacement,
}

#[derive(Component)]
struct PlacementGhost;

#[derive(Resource)]
struct ConstructionVisualAssets {
    mesh: Handle<Mesh>,
    valid_material: Handle<StandardMaterial>,
    warning_material: Handle<StandardMaterial>,
    invalid_material: Handle<StandardMaterial>,
}

#[derive(SystemParam)]
struct ContexteApercuPlacement<'w, 's> {
    hovered: Res<'w, HoveredCell>,
    selected: Res<'w, SelectedBuild>,
    blocage_ui: Res<'w, BlocageConstructionParUi>,
    zone_recharge: Res<'w, ZoneRechargeBase>,
    origin: Res<'w, WorldOrigin>,
    feedback: ResMut<'w, PlacementFeedback>,
    cache: ResMut<'w, WorldCache>,
    profile: Res<'w, PlanetProfile>,
    seed: Res<'w, WorldSeed>,
    structures: Query<'w, 's, &'static StructureState>,
    visuals: Res<'w, ConstructionVisualAssets>,
}

impl Plugin for ConstructionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SelectedBuild(StructureKind::Habitat))
            .insert_resource(PlacementFeedback::default())
            .insert_resource(BlocageConstructionParUi::default())
            .add_systems(
                Startup,
                (setup_construction_visuals, spawn_construction_ghost).chain(),
            )
            .add_systems(
                Update,
                (
                    select_building_hotkeys,
                    update_ghost_preview,
                    place_structure,
                )
                    .run_if(etat_interactif),
            );
    }
}

fn setup_construction_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(ConstructionVisualAssets {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        valid_material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.42, 0.88, 0.59, 0.55),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
        warning_material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.98, 0.72, 0.24, 0.58),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
        invalid_material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.96, 0.34, 0.31, 0.55),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
    });
}

fn spawn_construction_ghost(mut commands: Commands, visuals: Res<ConstructionVisualAssets>) {
    commands.spawn((
        Mesh3d(visuals.mesh.clone()),
        MeshMaterial3d(visuals.valid_material.clone()),
        Transform::from_scale(Vec3::splat(0.1)),
        Visibility::Hidden,
        PlacementGhost,
    ));
}

fn select_building_hotkeys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedBuild>,
) {
    if keyboard.just_pressed(KeyCode::Digit1) {
        selected.0 = StructureKind::Habitat;
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        selected.0 = StructureKind::SolarArray;
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        selected.0 = StructureKind::OxygenExtractor;
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        selected.0 = StructureKind::Storage;
    }
    if keyboard.just_pressed(KeyCode::Digit5) {
        selected.0 = StructureKind::Tube;
    }
    if keyboard.just_pressed(KeyCode::Digit6) {
        selected.0 = StructureKind::Lander;
    }
}

fn update_ghost_preview(
    mut contexte: ContexteApercuPlacement,
    ghost: Single<
        (
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<PlacementGhost>,
    >,
) {
    let selected_kind = contexte.selected.0;

    if contexte.blocage_ui.0 {
        contexte.feedback.valid = false;
        contexte.feedback.reason = "Curseur sur l'interface.".into();
        contexte.feedback.hovered_anchor = None;
        contexte.feedback.cellule_interaction = None;
        contexte.feedback.etat_connexion = EtatConnexionPlacement::Connecte;
        let (_, mut visibility, _) = ghost.into_inner();
        *visibility = Visibility::Hidden;
        return;
    }

    let Some(anchor) = contexte.hovered.0 else {
        contexte.feedback.valid = false;
        contexte.feedback.reason = "Deplace le curseur sur le terrain.".into();
        contexte.feedback.hovered_anchor = None;
        contexte.feedback.cellule_interaction = None;
        contexte.feedback.etat_connexion = EtatConnexionPlacement::Connecte;
        let (_, mut visibility, _) = ghost.into_inner();
        *visibility = Visibility::Hidden;
        return;
    };

    let structures_snapshot: Vec<_> = contexte.structures.iter().cloned().collect();
    let validation = validate_structure_placement(
        selected_kind,
        anchor,
        &structures_snapshot,
        &contexte.zone_recharge,
        |cellule| {
            contexte
                .cache
                .terrain_at(cellule, &contexte.profile, *contexte.seed)
        },
    );

    contexte.feedback.valid = validation.valid;
    contexte.feedback.reason = validation.reason.clone();
    contexte.feedback.hovered_anchor = Some(anchor);
    contexte.feedback.cellule_interaction = validation.cellule_interaction;
    contexte.feedback.etat_connexion = validation.etat_connexion;

    let (mut transform, mut visibility, mut material) = ghost.into_inner();
    *visibility = Visibility::Visible;
    *material = match (validation.valid, validation.etat_connexion) {
        (true, EtatConnexionPlacement::Connecte) => {
            MeshMaterial3d(contexte.visuals.valid_material.clone())
        }
        (true, EtatConnexionPlacement::NonConnecte) => {
            MeshMaterial3d(contexte.visuals.warning_material.clone())
        }
        (false, _) => MeshMaterial3d(contexte.visuals.invalid_material.clone()),
    };

    let occupied = structure_cells(selected_kind, anchor);
    transform.translation = structure_anchor_translation(
        &occupied,
        &mut contexte.cache,
        &contexte.profile,
        *contexte.seed,
        &contexte.origin,
    );
    transform.scale = Vec3::new(
        selected_kind.footprint().x as f32 * contexte.profile.cell_size_meters * 0.98,
        0.35,
        selected_kind.footprint().y as f32 * contexte.profile.cell_size_meters * 0.98,
    );
    if !validation.valid {
        transform.translation.y -= 0.08;
    }
}

fn place_structure(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    selected: Res<SelectedBuild>,
    feedback: Res<PlacementFeedback>,
    blocage_ui: Res<BlocageConstructionParUi>,
    mut ids: ResMut<StructureIdAllocator>,
) {
    if blocage_ui.0 || !buttons.just_pressed(MouseButton::Left) || !feedback.valid {
        return;
    }

    let Some(anchor) = feedback.hovered_anchor else {
        return;
    };

    commands.spawn((
        StructureState {
            id: ids.allocate(),
            kind: selected.0,
            anchor,
            built: false,
            build_progress: 0.0,
            network_id: None,
            internal_ice: 0.0,
        },
        Name::new(format!("{} Site", selected.0.label())),
    ));
}
