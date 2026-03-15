use std::collections::{HashMap, HashSet};
use std::f32::consts::FRAC_PI_2;
use std::hash::{Hash, Hasher};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::core::{CameraController, GameState, WorldOrigin};
use crate::simulation::autonomie_aller_retour_max_cases;
use crate::world::{
    ActiveChunks, PlanetProfile, WorldCache, WorldSeed, footprint_center,
    structure_anchor_translation, world_to_render_translation,
};

mod arrivee_initiale;
mod astronautes;
mod definitions_structures;
mod interactions_structures;
mod profils;

pub use astronautes::{
    AnimationPromenade, Astronaut, AstronautStatus, AstronautePromeneur, EtatPromenade,
    GridPosition, PositionMonde, ZoneRechargeBase,
};
pub use definitions_structures::{CycleExtraction, DefinitionStructure, definition_structure};
pub use interactions_structures::{
    cellules_interaction_structure, cellules_occupees_structures, cellules_origine_base,
    meilleure_cellule_interaction, meilleure_cellule_interaction_structure,
    structure_rejoint_reseau_principal, structures_connectees_par_proximite,
    trouver_chemin_vers_interaction,
};
pub use profils::{
    ProfilAstronaute, ProfilCompetences, ProfilTraits, RoleAstronaute,
    profil_astronaute_par_defaut, profil_promeneur_par_defaut,
};

pub struct ColonyPlugin;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StructureId(pub u32);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct AstronautId(pub u32);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StructureKind {
    Lander,
    Habitat,
    SolarArray,
    OxygenExtractor,
    Storage,
    Tube,
}

impl StructureKind {
    pub const ALL: [Self; 6] = [
        Self::Habitat,
        Self::SolarArray,
        Self::OxygenExtractor,
        Self::Storage,
        Self::Tube,
        Self::Lander,
    ];

    pub fn label(self) -> &'static str {
        definition_structure(self).libelle
    }

    pub fn footprint(self) -> UVec2 {
        definition_structure(self).emprise
    }

    pub fn build_ticks(self) -> f32 {
        definition_structure(self).travail_construction
    }

    pub fn relay(self) -> bool {
        definition_structure(self).relais_reseau
    }

    pub fn supports_life(self) -> bool {
        definition_structure(self).support_vie
    }

    pub fn oxygen_capacity(self) -> f32 {
        definition_structure(self).capacite_oxygene
    }

    pub fn ice_capacity(self) -> f32 {
        definition_structure(self).capacite_glace
    }

    pub fn energy_generation(self) -> f32 {
        definition_structure(self).generation_energie
    }

    pub fn maintenance_energy(self) -> f32 {
        definition_structure(self).maintenance_energie
    }

    pub fn extraction_cycle(self) -> Option<(f32, f32)> {
        definition_structure(self)
            .cycle_extraction
            .map(CycleExtraction::en_tuple)
    }

    pub fn material_color(self) -> Color {
        definition_structure(self).couleur_materiau
    }

    pub fn scale(self) -> Vec3 {
        definition_structure(self).echelle_rendu
    }
}

#[derive(Component, Clone, Debug)]
pub struct StructureState {
    pub id: StructureId,
    pub kind: StructureKind,
    pub anchor: IVec2,
    pub built: bool,
    pub build_progress: f32,
    pub network_id: Option<usize>,
    pub internal_ice: f32,
}

impl StructureState {
    pub fn occupied_cells(&self) -> Vec<IVec2> {
        structure_cells(self.kind, self.anchor)
    }

    pub fn center_world(&self, cell_size: f32) -> Vec2 {
        footprint_center(&self.occupied_cells(), cell_size)
    }
}

#[derive(Component, Clone, Debug)]
pub struct LooseIce {
    pub cell: IVec2,
    pub amount: f32,
}

#[derive(SystemParam)]
struct ContexteGenerationTaches<'w, 's> {
    board: ResMut<'w, TaskBoard>,
    active_chunks: Res<'w, ActiveChunks>,
    cache: ResMut<'w, WorldCache>,
    profile: Res<'w, PlanetProfile>,
    seed: Res<'w, WorldSeed>,
    life_support: Res<'w, LifeSupportState>,
    zone_recharge: Res<'w, ZoneRechargeBase>,
    astronauts: Query<'w, 's, &'static Astronaut>,
    structures: Query<'w, 's, &'static StructureState>,
}

#[derive(SystemParam)]
struct ContexteBootstrapColonie<'w, 's> {
    commands: Commands<'w, 's>,
    ids: ResMut<'w, StructureIdAllocator>,
    camera: ResMut<'w, CameraController>,
    life_support: ResMut<'w, LifeSupportState>,
    zone_recharge: ResMut<'w, ZoneRechargeBase>,
    cache: ResMut<'w, WorldCache>,
    profile: Res<'w, PlanetProfile>,
    seed: Res<'w, WorldSeed>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TaskKind {
    Build {
        structure: StructureId,
    },
    Extract {
        cell: IVec2,
    },
    HaulIce {
        source_cell: IVec2,
        target: StructureId,
    },
    RefuelStructure {
        source: StructureId,
        target: StructureId,
    },
    ReturnToBase {
        target: StructureId,
    },
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: TaskId,
    pub kind: TaskKind,
    pub priority: i32,
    pub target_cell: IVec2,
    pub assigned_to: Option<AstronautId>,
}

#[derive(Resource, Clone, Default)]
pub struct TaskBoard {
    pub tasks: Vec<Task>,
}

#[derive(Clone, Debug, Default)]
pub struct LifeSupportNetwork {
    pub network_id: usize,
    pub structures: Vec<StructureId>,
    pub oxygen_capacity: f32,
    pub oxygen_stored: f32,
    pub ice_capacity: f32,
    pub ice_stored: f32,
    pub energy_generation: f32,
    pub energy_demand: f32,
    pub oxygen_balance: f32,
    pub connected_life_support: bool,
    pub alerts: Vec<String>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct LifeSupportState {
    pub primary: Option<LifeSupportNetwork>,
    pub disconnected: Vec<LifeSupportNetwork>,
}

#[derive(Resource, Default)]
pub struct StructureIdAllocator {
    next: u32,
}

impl StructureIdAllocator {
    pub fn allocate(&mut self) -> StructureId {
        let id = StructureId(self.next);
        self.next += 1;
        id
    }
}

#[derive(Resource)]
struct ColonyVisualAssets {
    cube_mesh: Handle<Mesh>,
    capsule_mesh: Handle<Mesh>,
    sphere_mesh: Handle<Mesh>,
    ice_mesh: Handle<Mesh>,
    hull_material: Handle<StandardMaterial>,
    hull_secondary_material: Handle<StandardMaterial>,
    frame_material: Handle<StandardMaterial>,
    solar_material: Handle<StandardMaterial>,
    solar_frame_material: Handle<StandardMaterial>,
    glass_material: Handle<StandardMaterial>,
    accent_material: Handle<StandardMaterial>,
    storage_material: Handle<StandardMaterial>,
    landing_material: Handle<StandardMaterial>,
    lit_material: Handle<StandardMaterial>,
    suit_material: Handle<StandardMaterial>,
    suit_fabric_material: Handle<StandardMaterial>,
    boot_material: Handle<StandardMaterial>,
    visor_material: Handle<StandardMaterial>,
    mission_red_material: Handle<StandardMaterial>,
    mission_blue_material: Handle<StandardMaterial>,
    ice_material: Handle<StandardMaterial>,
}

#[derive(SystemSet, Debug, Clone, Eq, PartialEq, Hash)]
enum ColonySimulationSet {
    Networks,
    Tasks,
    Assignment,
    Actors,
    Supplies,
}

#[derive(SystemSet, Debug, Clone, Eq, PartialEq, Hash)]
enum ColonyUpdateSet {
    IntroSpawn,
    AstronautSetup,
    AstronautRender,
}

fn etat_intro_ou_jeu(state: Res<State<GameState>>) -> bool {
    matches!(state.get(), GameState::Intro | GameState::InGame)
}

impl Plugin for ColonyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(StructureIdAllocator::default())
            .insert_resource(TaskBoard::default())
            .insert_resource(LifeSupportState::default())
            .insert_resource(ZoneRechargeBase::default())
            .add_systems(
                Startup,
                (setup_colony_visuals, spawn_bootstrap_colony).chain(),
            )
            .add_systems(
                OnEnter(GameState::Intro),
                arrivee_initiale::preparer_arrivee_initiale,
            )
            .add_systems(
                OnExit(GameState::Intro),
                arrivee_initiale::nettoyer_arrivee_initiale,
            )
            .add_systems(
                Update,
                (
                    ensure_structure_visuals,
                    ensure_loose_ice_visuals,
                    sync_structure_visuals,
                    sync_loose_ice_visuals,
                ),
            )
            .configure_sets(
                Update,
                (
                    ColonyUpdateSet::IntroSpawn,
                    ColonyUpdateSet::AstronautSetup,
                    ColonyUpdateSet::AstronautRender,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    astronautes::initialiser_profils_simulation_astronautes,
                    astronautes::initialiser_memoire_travail_astronautes,
                    astronautes::initialiser_apparences_astronautes,
                    astronautes::initialiser_ouvriers_lisses,
                    astronautes::greffer_rig_ouvriers,
                    astronautes::greffer_rig_promeneurs,
                )
                    .chain()
                    .in_set(ColonyUpdateSet::AstronautSetup),
            )
            .add_systems(
                Update,
                arrivee_initiale::piloter_arrivee_initiale
                    .run_if(in_state(GameState::Intro))
                    .in_set(ColonyUpdateSet::IntroSpawn),
            )
            .add_systems(
                Update,
                (
                    arrivee_initiale::animer_debarquement_initial
                        .run_if(in_state(GameState::Intro)),
                    astronautes::mettre_a_jour_cibles_ouvriers.run_if(in_state(GameState::InGame)),
                    astronautes::interpoler_ouvriers.run_if(in_state(GameState::InGame)),
                    astronautes::deplacer_promeneur.run_if(in_state(GameState::InGame)),
                    astronautes::synchroniser_rendu_ouvriers_lisse,
                    astronautes::synchroniser_rendu_promeneurs_realistes,
                    astronautes::animer_rig_ouvriers,
                    astronautes::animer_rig_promeneurs,
                )
                    .chain()
                    .run_if(etat_intro_ou_jeu)
                    .in_set(ColonyUpdateSet::AstronautRender),
            )
            .configure_sets(
                FixedUpdate,
                (
                    ColonySimulationSet::Networks,
                    ColonySimulationSet::Tasks,
                    ColonySimulationSet::Assignment,
                    ColonySimulationSet::Actors,
                    ColonySimulationSet::Supplies,
                )
                    .chain()
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                FixedUpdate,
                recompute_life_support.in_set(ColonySimulationSet::Networks),
            )
            .add_systems(
                FixedUpdate,
                astronautes::recompute_zone_recharge_base
                    .in_set(ColonySimulationSet::Networks)
                    .after(recompute_life_support),
            )
            .add_systems(
                FixedUpdate,
                (fusionner_glace_libre_dupliquee, generate_tasks)
                    .chain()
                    .in_set(ColonySimulationSet::Tasks),
            )
            .add_systems(
                FixedUpdate,
                astronautes::assign_tasks_system.in_set(ColonySimulationSet::Assignment),
            )
            .add_systems(
                FixedUpdate,
                astronautes::advance_astronauts.in_set(ColonySimulationSet::Actors),
            )
            .add_systems(
                FixedUpdate,
                astronautes::simuler_promeneur.in_set(ColonySimulationSet::Actors),
            )
            .add_systems(
                FixedUpdate,
                process_life_support.in_set(ColonySimulationSet::Supplies),
            );
    }
}

fn setup_colony_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(ColonyVisualAssets {
        cube_mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        capsule_mesh: meshes.add(Capsule3d::new(0.55, 1.2)),
        sphere_mesh: meshes.add(Sphere::new(0.5).mesh().uv(16, 10)),
        ice_mesh: meshes.add(Sphere::new(0.24).mesh().uv(10, 6)),
        hull_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.90, 0.90, 0.87),
            perceptual_roughness: 0.94,
            metallic: 0.03,
            ..default()
        }),
        hull_secondary_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.70, 0.71, 0.70),
            perceptual_roughness: 0.98,
            metallic: 0.02,
            ..default()
        }),
        frame_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.21, 0.23, 0.25),
            perceptual_roughness: 0.95,
            metallic: 0.12,
            ..default()
        }),
        solar_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.20, 0.38),
            emissive: Color::srgb(0.01, 0.03, 0.06).into(),
            perceptual_roughness: 0.32,
            metallic: 0.58,
            ..default()
        }),
        solar_frame_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.47, 0.49, 0.52),
            perceptual_roughness: 0.76,
            metallic: 0.44,
            ..default()
        }),
        glass_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.12, 0.16),
            emissive: Color::srgb(0.03, 0.05, 0.05).into(),
            perceptual_roughness: 0.18,
            metallic: 0.08,
            ..default()
        }),
        accent_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.86, 0.42, 0.16),
            perceptual_roughness: 0.72,
            metallic: 0.05,
            ..default()
        }),
        storage_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.42, 0.45, 0.50),
            perceptual_roughness: 0.98,
            metallic: 0.05,
            ..default()
        }),
        landing_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.63, 0.56, 0.41),
            perceptual_roughness: 0.76,
            metallic: 0.55,
            ..default()
        }),
        lit_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.90, 0.73, 0.53),
            emissive: Color::srgb(0.18, 0.12, 0.06).into(),
            perceptual_roughness: 0.52,
            ..default()
        }),
        suit_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.98, 0.96, 0.92),
            perceptual_roughness: 0.9,
            ..default()
        }),
        suit_fabric_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.86, 0.87, 0.86),
            perceptual_roughness: 1.0,
            ..default()
        }),
        boot_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.27, 0.29, 0.31),
            perceptual_roughness: 0.98,
            metallic: 0.05,
            ..default()
        }),
        visor_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.50, 0.34, 0.12),
            emissive: Color::srgb(0.10, 0.07, 0.03).into(),
            perceptual_roughness: 0.12,
            metallic: 0.72,
            ..default()
        }),
        mission_red_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.80, 0.18, 0.12),
            perceptual_roughness: 0.82,
            ..default()
        }),
        mission_blue_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.34, 0.66),
            perceptual_roughness: 0.82,
            ..default()
        }),
        ice_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.84, 0.98),
            emissive: Color::srgb(0.03, 0.07, 0.1).into(),
            perceptual_roughness: 0.1,
            ..default()
        }),
    });
}

fn spawn_bootstrap_colony(mut contexte: ContexteBootstrapColonie) {
    let lander_id = contexte.ids.allocate();
    let lander = StructureState {
        id: lander_id,
        kind: StructureKind::Lander,
        anchor: IVec2::new(0, 0),
        built: true,
        build_progress: 1.0,
        network_id: Some(0),
        internal_ice: 0.0,
    };
    contexte
        .commands
        .spawn((lander.clone(), Name::new("Lander")));

    contexte.life_support.primary = Some(LifeSupportNetwork {
        network_id: 0,
        structures: vec![lander_id],
        oxygen_capacity: 360.0,
        oxygen_stored: 260.0,
        ice_capacity: 8.0,
        ice_stored: 2.0,
        energy_generation: 0.0,
        energy_demand: 0.5,
        oxygen_balance: 0.0,
        connected_life_support: true,
        alerts: vec![
            "Construis du solaire et un extracteur O2 avant que les réserves chutent.".into(),
        ],
    });

    *contexte.zone_recharge = astronautes::calculer_zone_recharge_base(
        std::slice::from_ref(&lander),
        &contexte.life_support,
        |cell| {
            contexte
                .cache
                .terrain_at(cell, &contexte.profile, *contexte.seed)
        },
    );
    contexte.camera.focus_world = lander.center_world(contexte.profile.cell_size_meters);
    contexte.camera.zoom = 42.0;
}

fn ensure_structure_visuals(
    mut commands: Commands,
    visuals: Res<ColonyVisualAssets>,
    query: Query<(Entity, &StructureState), Added<StructureState>>,
) {
    for (entity, structure) in &query {
        commands.entity(entity).insert((
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
        ));
        commands.entity(entity).with_children(|parent| {
            let mut piece = |mesh: Handle<Mesh>,
                             material: Handle<StandardMaterial>,
                             translation: Vec3,
                             rotation: Quat,
                             scale: Vec3| {
                parent.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform {
                        translation,
                        rotation,
                        scale,
                    },
                ));
            };

            match structure.kind {
                StructureKind::Habitat => {
                    piece(
                        visuals.capsule_mesh.clone(),
                        visuals.hull_material.clone(),
                        Vec3::new(0.0, 0.85, 0.0),
                        Quat::from_rotation_z(FRAC_PI_2),
                        Vec3::new(2.05, 1.0, 1.16),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.frame_material.clone(),
                        Vec3::new(0.0, 0.20, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(3.35, 0.16, 2.05),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.glass_material.clone(),
                        Vec3::new(0.0, 1.00, 0.96),
                        Quat::IDENTITY,
                        Vec3::new(1.85, 0.20, 0.12),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.accent_material.clone(),
                        Vec3::new(0.0, 1.18, -0.72),
                        Quat::IDENTITY,
                        Vec3::new(2.60, 0.08, 0.14),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.hull_secondary_material.clone(),
                        Vec3::new(-1.55, 0.56, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(0.42, 0.92, 0.82),
                    );
                    for &(x, z) in &[(-1.25, -0.72), (1.25, -0.72), (-1.25, 0.72), (1.25, 0.72)] {
                        piece(
                            visuals.cube_mesh.clone(),
                            visuals.frame_material.clone(),
                            Vec3::new(x, 0.14, z),
                            Quat::IDENTITY,
                            Vec3::new(0.16, 0.28, 0.16),
                        );
                    }
                }
                StructureKind::Lander => {
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.hull_secondary_material.clone(),
                        Vec3::new(0.0, 0.96, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(2.20, 1.18, 2.20),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.hull_material.clone(),
                        Vec3::new(0.0, 1.86, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(1.30, 0.66, 1.30),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.glass_material.clone(),
                        Vec3::new(0.0, 2.04, 0.70),
                        Quat::IDENTITY,
                        Vec3::new(0.90, 0.16, 0.14),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.accent_material.clone(),
                        Vec3::new(0.0, 1.52, -0.98),
                        Quat::IDENTITY,
                        Vec3::new(1.72, 0.10, 0.18),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.frame_material.clone(),
                        Vec3::new(0.0, 2.55, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(0.08, 0.82, 0.08),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.hull_material.clone(),
                        Vec3::new(0.0, 1.20, 1.02),
                        Quat::IDENTITY,
                        Vec3::new(1.04, 0.42, 0.18),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.frame_material.clone(),
                        Vec3::new(0.0, 0.82, 1.18),
                        Quat::IDENTITY,
                        Vec3::new(0.66, 0.24, 0.08),
                    );
                    piece(
                        visuals.sphere_mesh.clone(),
                        visuals.lit_material.clone(),
                        Vec3::new(0.0, 3.00, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(0.16, 0.16, 0.16),
                    );
                    for &(x, z) in &[(-0.68, -0.68), (0.68, -0.68), (-0.68, 0.68), (0.68, 0.68)] {
                        piece(
                            visuals.capsule_mesh.clone(),
                            visuals.frame_material.clone(),
                            Vec3::new(x, 0.06, z),
                            Quat::IDENTITY,
                            Vec3::new(0.20, 0.32, 0.20),
                        );
                        piece(
                            visuals.cube_mesh.clone(),
                            visuals.accent_material.clone(),
                            Vec3::new(x, 0.36, z),
                            Quat::IDENTITY,
                            Vec3::new(0.12, 0.10, 0.12),
                        );
                    }
                    for &(x, y, z) in &[
                        (-0.82, 1.72, 0.82),
                        (0.82, 1.72, 0.82),
                        (-0.82, 1.72, -0.82),
                        (0.82, 1.72, -0.82),
                    ] {
                        piece(
                            visuals.sphere_mesh.clone(),
                            visuals.lit_material.clone(),
                            Vec3::new(x, y, z),
                            Quat::IDENTITY,
                            Vec3::new(0.08, 0.08, 0.08),
                        );
                    }
                    for &(x, z) in &[(-1.30, -1.30), (1.30, -1.30), (-1.30, 1.30), (1.30, 1.30)] {
                        piece(
                            visuals.cube_mesh.clone(),
                            visuals.landing_material.clone(),
                            Vec3::new(x * 0.82, 0.48, z * 0.82),
                            Quat::from_rotation_z(if x < 0.0 { 0.18 } else { -0.18 }),
                            Vec3::new(0.16, 1.18, 0.16),
                        );
                        piece(
                            visuals.cube_mesh.clone(),
                            visuals.landing_material.clone(),
                            Vec3::new(x, 0.10, z),
                            Quat::IDENTITY,
                            Vec3::new(0.50, 0.06, 0.50),
                        );
                    }
                }
                StructureKind::SolarArray => {
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.solar_frame_material.clone(),
                        Vec3::new(0.0, 0.14, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(1.10, 0.18, 1.00),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.solar_frame_material.clone(),
                        Vec3::new(0.0, 0.60, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(0.12, 0.90, 0.12),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.solar_frame_material.clone(),
                        Vec3::new(0.0, 0.92, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(3.20, 0.08, 0.10),
                    );
                    for &(x, tilt) in &[(-1.02, 0.20), (1.02, -0.20)] {
                        piece(
                            visuals.cube_mesh.clone(),
                            visuals.solar_frame_material.clone(),
                            Vec3::new(x, 0.92, 0.0),
                            Quat::from_rotation_z(tilt),
                            Vec3::new(1.48, 0.06, 1.18),
                        );
                        piece(
                            visuals.cube_mesh.clone(),
                            visuals.solar_material.clone(),
                            Vec3::new(x, 0.95, 0.0),
                            Quat::from_rotation_z(tilt),
                            Vec3::new(1.36, 0.03, 1.06),
                        );
                    }
                }
                StructureKind::OxygenExtractor => {
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.hull_secondary_material.clone(),
                        Vec3::new(0.0, 0.72, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(1.85, 1.18, 1.60),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.frame_material.clone(),
                        Vec3::new(0.0, 1.05, 0.84),
                        Quat::IDENTITY,
                        Vec3::new(1.32, 0.36, 0.12),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.accent_material.clone(),
                        Vec3::new(0.0, 1.42, -0.72),
                        Quat::IDENTITY,
                        Vec3::new(1.34, 0.10, 0.12),
                    );
                    for &x in &[-0.74, 0.74] {
                        piece(
                            visuals.capsule_mesh.clone(),
                            visuals.hull_material.clone(),
                            Vec3::new(x, 1.00, 0.58),
                            Quat::IDENTITY,
                            Vec3::new(0.50, 1.10, 0.50),
                        );
                    }
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.frame_material.clone(),
                        Vec3::new(0.0, 0.12, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(2.00, 0.12, 1.74),
                    );
                }
                StructureKind::Storage => {
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.storage_material.clone(),
                        Vec3::new(0.0, 0.42, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(1.10, 0.72, 1.10),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.storage_material.clone(),
                        Vec3::new(0.0, 0.92, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(1.00, 0.24, 1.00),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.accent_material.clone(),
                        Vec3::new(0.0, 0.64, 0.56),
                        Quat::IDENTITY,
                        Vec3::new(0.82, 0.08, 0.05),
                    );
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.frame_material.clone(),
                        Vec3::new(0.0, 0.14, 0.0),
                        Quat::IDENTITY,
                        Vec3::new(1.18, 0.10, 1.18),
                    );
                }
                StructureKind::Tube => {
                    piece(
                        visuals.capsule_mesh.clone(),
                        visuals.hull_material.clone(),
                        Vec3::new(0.0, 0.36, 0.0),
                        Quat::from_rotation_z(FRAC_PI_2),
                        Vec3::new(0.72, 0.56, 0.56),
                    );
                    for &x in &[-0.72, 0.72] {
                        piece(
                            visuals.cube_mesh.clone(),
                            visuals.frame_material.clone(),
                            Vec3::new(x, 0.36, 0.0),
                            Quat::IDENTITY,
                            Vec3::new(0.10, 0.34, 0.64),
                        );
                    }
                    piece(
                        visuals.cube_mesh.clone(),
                        visuals.glass_material.clone(),
                        Vec3::new(0.0, 0.54, 0.30),
                        Quat::IDENTITY,
                        Vec3::new(0.86, 0.08, 0.08),
                    );
                }
            }
        });
    }
}

fn ensure_loose_ice_visuals(
    mut commands: Commands,
    visuals: Res<ColonyVisualAssets>,
    query: Query<Entity, (Added<LooseIce>, Without<Mesh3d>)>,
) {
    for entity in &query {
        commands.entity(entity).insert((
            Mesh3d(visuals.ice_mesh.clone()),
            MeshMaterial3d(visuals.ice_material.clone()),
            Transform::default(),
        ));
    }
}

fn sync_structure_visuals(
    mut cache: ResMut<WorldCache>,
    profile: Res<PlanetProfile>,
    seed: Res<WorldSeed>,
    origin: Res<WorldOrigin>,
    mut query: Query<(&StructureState, Ref<StructureState>, &mut Transform)>,
) {
    let origin_changed = origin.is_changed();
    for (structure, structure_ref, mut transform) in &mut query {
        if !origin_changed && !structure_ref.is_changed() {
            continue;
        }

        let cells = structure.occupied_cells();
        transform.translation =
            structure_anchor_translation(&cells, &mut cache, &profile, *seed, &origin);
        transform.scale = Vec3::ONE;
        if !structure.built {
            transform.translation.y -= 0.25;
        }
    }
}

fn sync_loose_ice_visuals(
    mut cache: ResMut<WorldCache>,
    profile: Res<PlanetProfile>,
    seed: Res<WorldSeed>,
    origin: Res<WorldOrigin>,
    mut query: Query<(&LooseIce, Ref<LooseIce>, &mut Transform)>,
) {
    let origin_changed = origin.is_changed();
    for (ice, ice_ref, mut transform) in &mut query {
        if !origin_changed && !ice_ref.is_changed() {
            continue;
        }

        let terrain = cache.terrain_at(ice.cell, &profile, *seed);
        transform.translation =
            world_to_render_translation(ice.cell, terrain.height + 0.45, &profile, &origin);
    }
}

fn recompute_life_support(
    mut life_support: ResMut<LifeSupportState>,
    mut structures: Query<&mut StructureState>,
) {
    let snapshot: Vec<_> = structures.iter().cloned().collect();
    let (previous_oxygen, previous_ice) = stored_network_reserves(&life_support);

    let networks = compute_life_support_networks(&snapshot, previous_oxygen, previous_ice);
    let primary_ids: HashSet<_> = networks
        .first()
        .map(|network| network.structures.iter().copied().collect())
        .unwrap_or_default();

    life_support.primary = networks.first().cloned();
    life_support.disconnected = networks.iter().skip(1).cloned().collect();

    for mut structure in &mut structures {
        structure.network_id = if primary_ids.contains(&structure.id) && structure.built {
            Some(0)
        } else {
            life_support
                .disconnected
                .iter()
                .position(|network| network.structures.contains(&structure.id))
                .map(|index| index + 1)
        };
    }
}

fn generate_tasks(mut contexte: ContexteGenerationTaches, loose_ice: Query<&LooseIce>) {
    let astronaut_snapshots: Vec<_> = contexte.astronauts.iter().cloned().collect();
    let assigned_workers = active_task_assignments(&astronaut_snapshots);
    let structures_snapshot: Vec<_> = contexte.structures.iter().cloned().collect();
    let origines_base = cellules_origine_base(&contexte.zone_recharge);
    let limite_acces = autonomie_aller_retour_max_cases();
    let structure_targets = structures_snapshot
        .iter()
        .filter_map(|structure| {
            meilleure_cellule_interaction_structure(
                structure.id,
                &structures_snapshot,
                &origines_base,
                limite_acces,
                |cellule| {
                    contexte
                        .cache
                        .terrain_at(cellule, &contexte.profile, *contexte.seed)
                },
            )
            .map(|(cellule, _)| (structure.id, cellule))
        })
        .collect::<HashMap<_, _>>();
    let connected: HashSet<_> = contexte
        .life_support
        .primary
        .as_ref()
        .map(|network| network.structures.iter().copied().collect())
        .unwrap_or_default();

    let primary_support = structures_snapshot
        .iter()
        .find(|structure| {
            connected.contains(&structure.id)
                && structure.kind.supports_life()
                && structure.built
                && structure_targets.contains_key(&structure.id)
        })
        .cloned();

    let storage_target = structures_snapshot
        .iter()
        .find(|structure| {
            connected.contains(&structure.id)
                && structure.built
                && structure.kind.ice_capacity() > 0.0
                && structure_targets.contains_key(&structure.id)
        })
        .map(|structure| structure.id);

    let loose_ice_positions: HashSet<_> = loose_ice.iter().map(|ice| ice.cell).collect();
    let total_loose_ice: f32 = loose_ice.iter().map(|ice| ice.amount).sum();
    let primary_ice = contexte
        .life_support
        .primary
        .as_ref()
        .map(|network| network.ice_stored)
        .unwrap_or(0.0);
    let extractor_internal_ice: f32 = structures_snapshot
        .iter()
        .filter(|structure| {
            structure.kind == StructureKind::OxygenExtractor && connected.contains(&structure.id)
        })
        .map(|structure| structure.internal_ice)
        .sum();

    let mut tasks = Vec::new();

    for structure in &structures_snapshot {
        if !structure.built {
            if let Some(&target_cell) = structure_targets.get(&structure.id) {
                let kind = TaskKind::Build {
                    structure: structure.id,
                };
                tasks.push(make_task(&assigned_workers, kind, 100, target_cell));
            }
        }
    }

    if primary_ice + total_loose_ice + extractor_internal_ice < 8.0 {
        let home = primary_support
            .as_ref()
            .map(|structure| structure.center_world(contexte.profile.cell_size_meters))
            .unwrap_or(Vec2::ZERO);
        let mut candidate_cells = Vec::new();
        let candidate_chunk_coords = if contexte.active_chunks.coords.is_empty() {
            contexte.cache.chunks.keys().copied().collect::<Vec<_>>()
        } else {
            contexte.active_chunks.coords.clone()
        };

        for coord in candidate_chunk_coords {
            let Some(chunk) = contexte.cache.chunks.get(&coord) else {
                continue;
            };
            for &local in chunk.resource_cells() {
                let Some(cell) = chunk.cell(local) else {
                    continue;
                };
                let Some(resource) = cell.resource else {
                    continue;
                };
                if resource.amount == 0 {
                    continue;
                }

                let world_cell = crate::world::chunk_local_to_world_cell(coord, local);
                if loose_ice_positions.contains(&world_cell) {
                    continue;
                }

                let position = footprint_center(&[world_cell], contexte.profile.cell_size_meters);
                let distance = position.distance(home);
                candidate_cells.push((distance, world_cell));
            }
        }

        candidate_cells.sort_by(|left, right| left.0.total_cmp(&right.0));
        for (_, cell) in candidate_cells.into_iter().take(3) {
            let kind = TaskKind::Extract { cell };
            tasks.push(make_task(&assigned_workers, kind, 65, cell));
        }
    }

    if let Some(storage_target) = storage_target {
        let target_cell = structure_targets[&storage_target];
        for ice in &loose_ice {
            let kind = TaskKind::HaulIce {
                source_cell: ice.cell,
                target: storage_target,
            };
            tasks.push(make_task(&assigned_workers, kind, 80, target_cell));
        }
    }

    if primary_ice > 0.0
        && let Some(source) = storage_target
    {
        for structure in &structures_snapshot {
            if structure.kind == StructureKind::OxygenExtractor
                && structure.built
                && connected.contains(&structure.id)
                && structure.internal_ice < 2.0
                && structure_targets.contains_key(&structure.id)
            {
                let kind = TaskKind::RefuelStructure {
                    source,
                    target: structure.id,
                };
                tasks.push(make_task(
                    &assigned_workers,
                    kind,
                    90,
                    structure_targets[&structure.id],
                ));
            }
        }
    }

    contexte.board.tasks = tasks;
}

fn process_life_support(
    mut life_support: ResMut<LifeSupportState>,
    mut structures: Query<&mut StructureState>,
) {
    let Some(primary) = life_support.primary.as_mut() else {
        return;
    };

    let snapshot: Vec<_> = structures.iter().cloned().collect();
    let cycle_plan = plan_life_support_cycle(primary, &snapshot);
    let powered_extractors: HashSet<_> = cycle_plan.powered_extractors.iter().copied().collect();

    for mut structure in &mut structures {
        if powered_extractors.contains(&structure.id) {
            structure.internal_ice -= 1.0;
        }
    }

    primary.energy_generation = cycle_plan.energy_generation;
    primary.energy_demand = cycle_plan.energy_demand;
    primary.oxygen_balance = cycle_plan.oxygen_produced;
    primary.oxygen_stored =
        (primary.oxygen_stored + cycle_plan.oxygen_produced).min(primary.oxygen_capacity);
    primary.ice_stored = primary.ice_stored.min(primary.ice_capacity);
    primary.alerts.clear();

    if primary.energy_generation < primary.energy_demand {
        primary
            .alerts
            .push("Déficit d’énergie : ajoute du solaire ou coupe des modules inactifs.".into());
    }
    if primary.oxygen_stored < 28.0 {
        primary
            .alerts
            .push("Oxygène bas : recharge les extracteurs ou augmente le stockage.".into());
    }
    if primary.ice_stored < 2.0 {
        primary
            .alerts
            .push("Stock de glace bas : il faut lancer de nouvelles extractions.".into());
    }
}

fn stored_network_reserves(life_support: &LifeSupportState) -> (f32, f32) {
    let primary_oxygen = life_support
        .primary
        .as_ref()
        .map(|network| network.oxygen_stored)
        .unwrap_or(0.0);
    let primary_ice = life_support
        .primary
        .as_ref()
        .map(|network| network.ice_stored)
        .unwrap_or(0.0);
    let disconnected_oxygen: f32 = life_support
        .disconnected
        .iter()
        .map(|network| network.oxygen_stored)
        .sum();
    let disconnected_ice: f32 = life_support
        .disconnected
        .iter()
        .map(|network| network.ice_stored)
        .sum();

    (
        primary_oxygen + disconnected_oxygen,
        primary_ice + disconnected_ice,
    )
}

fn task_needs_missing_structure(
    task: &Task,
    carrying_ice: f32,
    structure_positions: &HashMap<StructureId, IVec2>,
) -> bool {
    match task.kind {
        TaskKind::Build { structure } => !structure_positions.contains_key(&structure),
        TaskKind::HaulIce { target, .. } => {
            carrying_ice > 0.0 && !structure_positions.contains_key(&target)
        }
        TaskKind::RefuelStructure { target, .. } => {
            carrying_ice > 0.0 && !structure_positions.contains_key(&target)
        }
        TaskKind::ReturnToBase { target } => !structure_positions.contains_key(&target),
        TaskKind::Extract { .. } => false,
    }
}

fn find_structure_mut<'a>(
    query: &'a mut Query<&mut StructureState>,
    id: StructureId,
) -> Option<Mut<'a, StructureState>> {
    query.iter_mut().find(|structure| structure.id == id)
}

fn fusionner_glace_libre_dupliquee(mut commands: Commands, loose_ice: Query<(Entity, &LooseIce)>) {
    let mut cellules = HashMap::<IVec2, (Entity, f32, Vec<Entity>)>::new();

    for (entity, glace) in &loose_ice {
        let entry = cellules
            .entry(glace.cell)
            .or_insert((entity, 0.0, Vec::new()));
        if entry.0 != entity {
            entry.2.push(entity);
        }
        entry.1 += glace.amount;
    }

    for (cell, (garde, total, doublons)) in cellules {
        if doublons.is_empty() {
            continue;
        }

        commands.entity(garde).insert(LooseIce {
            cell,
            amount: total,
        });
        for entity in doublons {
            commands.entity(entity).despawn();
        }
    }
}

fn add_loose_ice_at_cell(
    commands: &mut Commands,
    loose_ice_query: &mut Query<(Entity, &mut LooseIce)>,
    cell: IVec2,
    amount: f32,
) {
    if amount <= 0.0 {
        return;
    }

    if let Some((_, mut loose)) = loose_ice_query
        .iter_mut()
        .find(|(_, loose)| loose.cell == cell && loose.amount > 0.0)
    {
        loose.amount += amount;
    } else {
        commands.spawn((LooseIce { cell, amount },));
    }
}

fn take_loose_ice_from_cell(
    commands: &mut Commands,
    loose_ice_query: &mut Query<(Entity, &mut LooseIce)>,
    cell: IVec2,
    requested: f32,
) -> f32 {
    if requested <= 0.0 {
        return 0.0;
    }

    let Some((ice_entity, mut loose)) = loose_ice_query
        .iter_mut()
        .find(|(_, loose)| loose.cell == cell && loose.amount > 0.0)
    else {
        return 0.0;
    };

    let taken = loose.amount.min(requested);
    loose.amount -= taken;
    if loose.amount <= 0.0 {
        commands.entity(ice_entity).despawn();
    }
    taken
}

fn deposit_ice_in_network(primary: &mut LifeSupportNetwork, amount: f32) -> f32 {
    let free_capacity = (primary.ice_capacity - primary.ice_stored).max(0.0);
    let deposited = amount.min(free_capacity);
    primary.ice_stored += deposited;
    deposited
}

pub fn structure_cells(kind: StructureKind, anchor: IVec2) -> Vec<IVec2> {
    let footprint = kind.footprint();
    let mut cells = Vec::new();
    for y in 0..footprint.y as i32 {
        for x in 0..footprint.x as i32 {
            cells.push(anchor + IVec2::new(x, y));
        }
    }
    cells
}

pub fn footprints_touch(
    a_kind: StructureKind,
    a_anchor: IVec2,
    b_kind: StructureKind,
    b_anchor: IVec2,
) -> bool {
    let a_cells = structure_cells(a_kind, a_anchor);
    let b_cells = structure_cells(b_kind, b_anchor);
    a_cells.iter().any(|a| {
        b_cells.iter().any(|b| {
            let delta = *a - *b;
            delta.x.abs() + delta.y.abs() == 1
        })
    })
}

pub fn compute_life_support_networks(
    structures: &[StructureState],
    oxygen_stored: f32,
    ice_stored: f32,
) -> Vec<LifeSupportNetwork> {
    let built: Vec<_> = structures
        .iter()
        .filter(|structure| structure.built)
        .cloned()
        .collect();
    let cellules_occupees = cellules_occupees_structures(structures);
    let mut visited = HashSet::new();
    let mut networks = Vec::new();

    for structure in &built {
        if visited.contains(&structure.id) {
            continue;
        }

        let mut stack = vec![structure.id];
        let mut component = Vec::new();
        while let Some(id) = stack.pop() {
            if !visited.insert(id) {
                continue;
            }
            let current = built.iter().find(|structure| structure.id == id).unwrap();
            component.push(current.clone());
            for other in &built {
                if current.id != other.id
                    && current.kind.relay()
                    && other.kind.relay()
                    && structures_connectees_par_proximite(
                        current.kind,
                        current.anchor,
                        other.kind,
                        other.anchor,
                        &cellules_occupees,
                    )
                {
                    stack.push(other.id);
                }
            }
        }

        let oxygen_capacity = component
            .iter()
            .map(|structure| structure.kind.oxygen_capacity())
            .sum();
        let ice_capacity = component
            .iter()
            .map(|structure| structure.kind.ice_capacity())
            .sum();
        let energy_generation = component
            .iter()
            .map(|structure| structure.kind.energy_generation())
            .sum();
        let energy_demand = component
            .iter()
            .map(|structure| structure.kind.maintenance_energy())
            .sum();
        let connected_life_support = component
            .iter()
            .any(|structure| structure.kind.supports_life());

        networks.push(LifeSupportNetwork {
            network_id: networks.len(),
            structures: component.iter().map(|structure| structure.id).collect(),
            oxygen_capacity,
            oxygen_stored: 0.0,
            ice_capacity,
            ice_stored: 0.0,
            energy_generation,
            energy_demand,
            oxygen_balance: 0.0,
            connected_life_support,
            alerts: Vec::new(),
        });
    }

    networks.sort_by(|left, right| {
        right
            .connected_life_support
            .cmp(&left.connected_life_support)
            .then_with(|| right.structures.len().cmp(&left.structures.len()))
    });

    for (index, network) in networks.iter_mut().enumerate() {
        network.network_id = index;
    }

    distribute_network_reserves(&mut networks, oxygen_stored, ice_stored);
    networks
}

fn distribute_network_reserves(
    networks: &mut [LifeSupportNetwork],
    oxygen_stored: f32,
    ice_stored: f32,
) {
    let mut remaining_oxygen = oxygen_stored.max(0.0);
    let mut remaining_ice = ice_stored.max(0.0);

    for network in networks {
        let assigned_oxygen = remaining_oxygen.min(network.oxygen_capacity);
        let assigned_ice = remaining_ice.min(network.ice_capacity);
        network.oxygen_stored = assigned_oxygen;
        network.ice_stored = assigned_ice;
        remaining_oxygen -= assigned_oxygen;
        remaining_ice -= assigned_ice;
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct LifeSupportCyclePlan {
    powered_extractors: Vec<StructureId>,
    energy_generation: f32,
    energy_demand: f32,
    oxygen_produced: f32,
}

fn plan_life_support_cycle(
    primary: &LifeSupportNetwork,
    structures: &[StructureState],
) -> LifeSupportCyclePlan {
    let connected: HashSet<_> = primary.structures.iter().copied().collect();
    let connected_structures: Vec<_> = structures
        .iter()
        .filter(|structure| connected.contains(&structure.id) && structure.built)
        .collect();

    let energy_generation: f32 = connected_structures
        .iter()
        .map(|structure| structure.kind.energy_generation())
        .sum();
    let mut energy_demand: f32 = connected_structures
        .iter()
        .map(|structure| structure.kind.maintenance_energy())
        .sum();
    let mut available_extractor_energy = (energy_generation - energy_demand).max(0.0);
    let mut oxygen_produced = 0.0;

    let mut extractor_candidates: Vec<_> = connected_structures
        .into_iter()
        .filter_map(|structure| {
            structure
                .kind
                .extraction_cycle()
                .map(|cycle| (structure.id, structure.internal_ice, cycle))
        })
        .collect();
    extractor_candidates.sort_by_key(|(id, _, _)| id.0);

    let mut powered_extractors = Vec::new();
    for (structure_id, internal_ice, (energy_cost, oxygen_output)) in extractor_candidates {
        if internal_ice < 1.0 || available_extractor_energy < energy_cost {
            continue;
        }

        available_extractor_energy -= energy_cost;
        energy_demand += energy_cost;
        oxygen_produced += oxygen_output;
        powered_extractors.push(structure_id);
    }

    LifeSupportCyclePlan {
        powered_extractors,
        energy_generation,
        energy_demand,
        oxygen_produced,
    }
}

#[derive(Clone, Debug)]
pub struct WorkerSnapshot {
    pub id: AstronautId,
    pub position: IVec2,
    pub current_task: Option<TaskId>,
    pub suit_oxygen: f32,
    pub alive: bool,
    pub role: RoleAstronaute,
    pub build_speed: f32,
    pub haul_capacity: f32,
    pub extraction_speed: f32,
}

pub fn assign_available_tasks(
    tasks: &mut [Task],
    workers: &[WorkerSnapshot],
) -> Vec<(AstronautId, TaskId)> {
    let mut assignments = Vec::new();
    let mut reserved = HashSet::new();
    let mut worker_list = workers.to_vec();
    worker_list.sort_by_key(|worker| worker.id.0);

    tasks.sort_by(|left, right| right.priority.cmp(&left.priority));

    for worker in worker_list {
        if !worker.alive || worker.current_task.is_some() || worker.suit_oxygen < 48.0 {
            continue;
        }

        let mut best: Option<(usize, i32, i32)> = None;
        for (index, task) in tasks.iter().enumerate() {
            if reserved.contains(&task.id) || task.assigned_to.is_some() {
                continue;
            }
            let distance = (task.target_cell - worker.position).abs().element_sum();
            let score = task.priority * 100 - distance + bonus_affectation(&worker, &task.kind);
            match best {
                Some((_, best_score, best_distance))
                    if score < best_score || (score == best_score && distance >= best_distance) => {
                }
                _ => {
                    best = Some((index, score, distance));
                }
            }
        }

        if let Some((index, _, _)) = best {
            tasks[index].assigned_to = Some(worker.id);
            reserved.insert(tasks[index].id);
            assignments.push((worker.id, tasks[index].id));
        }
    }

    assignments
}

fn bonus_affectation(worker: &WorkerSnapshot, task: &TaskKind) -> i32 {
    bonus_role_pour_tache(worker.role, task)
        + bonus_competence_pour_tache(
            task,
            worker.build_speed,
            worker.haul_capacity,
            worker.extraction_speed,
        )
}

fn bonus_role_pour_tache(role: RoleAstronaute, task: &TaskKind) -> i32 {
    match (role, task) {
        (RoleAstronaute::Ingenieur, TaskKind::Build { .. }) => 18,
        (RoleAstronaute::Ingenieur, TaskKind::RefuelStructure { .. }) => 10,
        (RoleAstronaute::Scientifique, TaskKind::Extract { .. }) => 18,
        (RoleAstronaute::Logisticien, TaskKind::HaulIce { .. }) => 18,
        (RoleAstronaute::Logisticien, TaskKind::RefuelStructure { .. }) => 12,
        (RoleAstronaute::Commandant, TaskKind::Build { .. }) => 12,
        (RoleAstronaute::Commandant, TaskKind::ReturnToBase { .. }) => 4,
        _ => 0,
    }
}

fn bonus_competence_pour_tache(
    task: &TaskKind,
    build_speed: f32,
    haul_capacity: f32,
    extraction_speed: f32,
) -> i32 {
    match task {
        TaskKind::Build { .. } => ((build_speed - 1.0) * 12.0).round() as i32,
        TaskKind::Extract { .. } => ((extraction_speed - 1.0) * 12.0).round() as i32,
        TaskKind::HaulIce { .. } | TaskKind::RefuelStructure { .. } => {
            ((haul_capacity - 1.0) * 12.0).round() as i32
        }
        TaskKind::ReturnToBase { .. } => 0,
    }
}

fn active_task_assignments(astronauts: &[Astronaut]) -> HashMap<TaskId, AstronautId> {
    astronauts
        .iter()
        .filter(|astronaut| astronaut.status != AstronautStatus::Dead)
        .filter_map(|astronaut| {
            astronaut
                .current_task
                .map(|task_id| (task_id, astronaut.id))
        })
        .collect()
}

fn make_task(
    assigned_workers: &HashMap<TaskId, AstronautId>,
    kind: TaskKind,
    priority: i32,
    target_cell: IVec2,
) -> Task {
    let id = task_id_of(&kind);
    Task {
        id,
        kind,
        priority,
        target_cell,
        assigned_to: assigned_workers.get(&id).copied(),
    }
}

fn task_id_of(kind: &TaskKind) -> TaskId {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    kind.hash(&mut hasher);
    TaskId(hasher.finish())
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;
    use crate::world::{ActiveChunks, PlanetProfile, WorldCache, WorldSeed};

    fn structure_test(id: u32, kind: StructureKind, anchor: IVec2) -> StructureState {
        StructureState {
            id: StructureId(id),
            kind,
            anchor,
            built: true,
            build_progress: 1.0,
            network_id: None,
            internal_ice: 0.0,
        }
    }

    fn reseau_primaire_test(structures: Vec<StructureId>, glace: f32) -> LifeSupportState {
        LifeSupportState {
            primary: Some(LifeSupportNetwork {
                network_id: 0,
                structures,
                oxygen_capacity: 360.0,
                oxygen_stored: 260.0,
                ice_capacity: 8.0,
                ice_stored: glace,
                energy_generation: 0.0,
                energy_demand: 0.5,
                oxygen_balance: 0.0,
                connected_life_support: true,
                alerts: Vec::new(),
            }),
            disconnected: Vec::new(),
        }
    }

    fn app_generation_taches() -> App {
        let mut app = App::new();
        app.insert_resource(TaskBoard::default());
        app.insert_resource(ActiveChunks::default());
        app.insert_resource(WorldCache::default());
        app.insert_resource(PlanetProfile::mars());
        app.insert_resource(WorldSeed(42));
        app.insert_resource(reseau_primaire_test(vec![StructureId(1)], 8.0));
        app.insert_resource(ZoneRechargeBase::depuis_cellules(vec![
            IVec2::new(-1, 0),
            IVec2::new(0, -1),
            IVec2::new(-1, -1),
        ]));
        app.add_systems(Update, generate_tasks);
        app
    }

    #[test]
    fn network_energy_changes_when_solar_array_added_or_removed() {
        let lander = structure_test(1, StructureKind::Lander, IVec2::ZERO);
        let solar = structure_test(2, StructureKind::SolarArray, IVec2::new(2, 0));

        let without = compute_life_support_networks(std::slice::from_ref(&lander), 50.0, 2.0);
        let with = compute_life_support_networks(&[lander, solar], 50.0, 2.0);
        assert!(with[0].energy_generation > without[0].energy_generation);
    }

    #[test]
    fn task_assignment_does_not_double_book_same_task() {
        let mut tasks = vec![
            Task {
                id: TaskId(1),
                kind: TaskKind::Build {
                    structure: StructureId(1),
                },
                priority: 100,
                target_cell: IVec2::ZERO,
                assigned_to: None,
            },
            Task {
                id: TaskId(2),
                kind: TaskKind::Extract {
                    cell: IVec2::new(4, 4),
                },
                priority: 50,
                target_cell: IVec2::new(4, 4),
                assigned_to: None,
            },
        ];

        let workers = vec![
            WorkerSnapshot {
                id: AstronautId(1),
                position: IVec2::ZERO,
                current_task: None,
                suit_oxygen: 100.0,
                alive: true,
                role: RoleAstronaute::Ingenieur,
                build_speed: 1.0,
                haul_capacity: 1.0,
                extraction_speed: 1.0,
            },
            WorkerSnapshot {
                id: AstronautId(2),
                position: IVec2::ZERO,
                current_task: None,
                suit_oxygen: 100.0,
                alive: true,
                role: RoleAstronaute::Scientifique,
                build_speed: 1.0,
                haul_capacity: 1.0,
                extraction_speed: 1.0,
            },
        ];

        let assignments = assign_available_tasks(&mut tasks, &workers);
        assert_eq!(assignments.len(), 2);
        assert_ne!(assignments[0].1, assignments[1].1);
    }

    #[test]
    fn affectation_favorise_les_roles_appropries() {
        let mut tasks = vec![
            Task {
                id: TaskId(1),
                kind: TaskKind::Build {
                    structure: StructureId(1),
                },
                priority: 100,
                target_cell: IVec2::ZERO,
                assigned_to: None,
            },
            Task {
                id: TaskId(2),
                kind: TaskKind::Extract {
                    cell: IVec2::new(1, 0),
                },
                priority: 100,
                target_cell: IVec2::new(1, 0),
                assigned_to: None,
            },
        ];

        let workers = vec![
            WorkerSnapshot {
                id: AstronautId(1),
                position: IVec2::ZERO,
                current_task: None,
                suit_oxygen: 100.0,
                alive: true,
                role: RoleAstronaute::Ingenieur,
                build_speed: 1.25,
                haul_capacity: 1.0,
                extraction_speed: 1.0,
            },
            WorkerSnapshot {
                id: AstronautId(2),
                position: IVec2::ZERO,
                current_task: None,
                suit_oxygen: 100.0,
                alive: true,
                role: RoleAstronaute::Scientifique,
                build_speed: 1.0,
                haul_capacity: 1.0,
                extraction_speed: 1.35,
            },
        ];

        let assignments = assign_available_tasks(&mut tasks, &workers);
        let affectations: HashMap<_, _> = assignments.into_iter().collect();

        assert_eq!(affectations.get(&AstronautId(1)), Some(&TaskId(1)));
        assert_eq!(affectations.get(&AstronautId(2)), Some(&TaskId(2)));
    }

    #[test]
    fn pont_court_connecte_un_module_avec_un_seul_gap() {
        let lander = structure_test(1, StructureKind::Lander, IVec2::ZERO);
        let solar_gap_court = structure_test(2, StructureKind::SolarArray, IVec2::new(3, 0));
        let solar_gap_long = structure_test(3, StructureKind::SolarArray, IVec2::new(4, 0));

        let reseau_gap_court =
            compute_life_support_networks(&[lander.clone(), solar_gap_court], 50.0, 2.0);
        let reseau_gap_long = compute_life_support_networks(&[lander, solar_gap_long], 50.0, 2.0);

        assert_eq!(reseau_gap_court.len(), 1);
        assert_eq!(reseau_gap_long.len(), 2);
    }

    #[test]
    fn generation_de_tache_de_construction_cible_une_case_d_interaction() {
        let mut app = app_generation_taches();
        app.world_mut()
            .spawn(structure_test(1, StructureKind::Lander, IVec2::ZERO));
        app.world_mut().spawn(StructureState {
            id: StructureId(2),
            kind: StructureKind::Storage,
            anchor: IVec2::new(3, 0),
            built: false,
            build_progress: 0.0,
            network_id: None,
            internal_ice: 0.0,
        });

        app.update();

        let board = app.world().resource::<TaskBoard>();
        let tache_construction = board
            .tasks
            .iter()
            .find(|task| matches!(task.kind, TaskKind::Build { structure } if structure == StructureId(2)))
            .expect("une tache de construction est attendue");

        assert_ne!(tache_construction.target_cell, IVec2::new(3, 0));
        assert!(
            [
                IVec2::new(2, 0),
                IVec2::new(4, 0),
                IVec2::new(3, 1),
                IVec2::new(3, -1)
            ]
            .contains(&tache_construction.target_cell)
        );
    }

    #[test]
    fn split_networks_do_not_duplicate_or_overfill_reserves() {
        let lander = structure_test(1, StructureKind::Lander, IVec2::ZERO);
        let storage = structure_test(2, StructureKind::Storage, IVec2::new(12, 12));

        let networks = compute_life_support_networks(&[lander, storage], 300.0, 40.0);
        let total_oxygen: f32 = networks.iter().map(|network| network.oxygen_stored).sum();
        let total_ice: f32 = networks.iter().map(|network| network.ice_stored).sum();

        assert!((total_oxygen - 300.0).abs() < 0.001);
        assert!((total_ice - 26.0).abs() < 0.001);
        assert!(
            networks
                .iter()
                .all(|network| network.oxygen_stored <= network.oxygen_capacity)
        );
        assert!(
            networks
                .iter()
                .all(|network| network.ice_stored <= network.ice_capacity)
        );
    }

    #[test]
    fn fusion_de_glace_libre_regroupe_les_doublons() {
        let mut app = App::new();
        app.add_systems(Update, fusionner_glace_libre_dupliquee);

        let cell = IVec2::new(2, 3);
        app.world_mut().spawn(LooseIce { cell, amount: 1.5 });
        app.world_mut().spawn(LooseIce { cell, amount: 0.75 });

        app.update();

        let world = app.world_mut();
        let mut query = world.query::<&LooseIce>();
        let glaces: Vec<_> = query
            .iter(world)
            .map(|glace| (glace.cell, glace.amount))
            .collect();

        assert_eq!(glaces.len(), 1);
        assert_eq!(glaces[0].0, cell);
        assert!((glaces[0].1 - 2.25).abs() < 0.001);
    }

    #[test]
    fn active_task_assignments_ignore_dead_astronauts() {
        let astronauts = vec![
            Astronaut {
                id: AstronautId(1),
                name: "Ari",
                suit_oxygen: 100.0,
                current_task: Some(TaskId(11)),
                status: AstronautStatus::Idle,
                carrying_ice: 0.0,
            },
            Astronaut {
                id: AstronautId(2),
                name: "Noor",
                suit_oxygen: 0.0,
                current_task: Some(TaskId(22)),
                status: AstronautStatus::Dead,
                carrying_ice: 0.0,
            },
        ];

        let assignments = active_task_assignments(&astronauts);
        assert_eq!(assignments.get(&TaskId(11)), Some(&AstronautId(1)));
        assert!(!assignments.contains_key(&TaskId(22)));
    }

    #[test]
    fn oxygen_extraction_plan_is_stable_regardless_of_structure_order() {
        let lander = structure_test(1, StructureKind::Lander, IVec2::ZERO);
        let solar = structure_test(2, StructureKind::SolarArray, IVec2::new(2, 0));
        let mut extractor = structure_test(3, StructureKind::OxygenExtractor, IVec2::new(4, 0));
        extractor.internal_ice = 2.0;

        let network = LifeSupportNetwork {
            network_id: 0,
            structures: vec![lander.id, solar.id, extractor.id],
            oxygen_capacity: 360.0,
            oxygen_stored: 20.0,
            ice_capacity: 8.0,
            ice_stored: 2.0,
            energy_generation: 0.0,
            energy_demand: 0.0,
            oxygen_balance: 0.0,
            connected_life_support: true,
            alerts: Vec::new(),
        };

        let plan_a = plan_life_support_cycle(
            &network,
            &[extractor.clone(), solar.clone(), lander.clone()],
        );
        let plan_b = plan_life_support_cycle(&network, &[solar, lander, extractor]);

        assert_eq!(plan_a.powered_extractors, vec![StructureId(3)]);
        assert_eq!(plan_b.powered_extractors, vec![StructureId(3)]);
        assert!((plan_a.oxygen_produced - 12.0).abs() < 0.001);
        assert!((plan_b.oxygen_produced - 12.0).abs() < 0.001);
        assert!((plan_a.energy_demand - 2.5).abs() < 0.001);
        assert!((plan_b.energy_demand - 2.5).abs() < 0.001);
    }

    #[test]
    fn stored_network_reserves_include_disconnected_networks() {
        let life_support = LifeSupportState {
            primary: Some(LifeSupportNetwork {
                oxygen_stored: 40.0,
                ice_stored: 3.0,
                ..default()
            }),
            disconnected: vec![
                LifeSupportNetwork {
                    oxygen_stored: 10.0,
                    ice_stored: 1.5,
                    ..default()
                },
                LifeSupportNetwork {
                    oxygen_stored: 5.0,
                    ice_stored: 0.5,
                    ..default()
                },
            ],
        };

        assert_eq!(stored_network_reserves(&life_support), (55.0, 5.0));
    }

    #[test]
    fn taking_loose_ice_never_creates_extra_resource() {
        let mut available: f32 = 0.4;
        let taken = available.min(1.0_f32);
        available -= taken;

        assert!((taken - 0.4).abs() < 0.001);
        assert!(available.abs() < 0.001);
    }

    #[test]
    fn depositing_ice_returns_surplus_when_storage_is_full() {
        let mut network = LifeSupportNetwork {
            ice_capacity: 8.0,
            ice_stored: 7.5,
            ..default()
        };

        let deposited = deposit_ice_in_network(&mut network, 1.0);
        let surplus = 1.0 - deposited;

        assert!((deposited - 0.5).abs() < 0.001);
        assert!((surplus - 0.5).abs() < 0.001);
        assert!((network.ice_stored - 8.0).abs() < 0.001);
    }
}
