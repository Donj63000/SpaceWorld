use std::collections::{HashMap, HashSet};
use std::f32::consts::{FRAC_PI_2, PI};
use std::hash::{Hash, Hasher};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

mod animation_realiste;
mod apparence;
mod mouvement_lisse;
mod rendu_realiste;

pub use animation_realiste::{animer_rig_ouvriers, animer_rig_promeneurs};
pub use apparence::{
    ApparenceAstronaute, RoleAstronaute, TypeCombinaison, initialiser_apparences_astronautes,
};
pub use mouvement_lisse::{
    AnimationOuvrier, CibleMondeLisse, PositionMondeLisse, initialiser_ouvriers_lisses,
    interpoler_ouvriers, mettre_a_jour_cibles_ouvriers,
};
pub use rendu_realiste::{
    RigAstronaute, greffer_rig_ouvriers, greffer_rig_promeneurs, synchroniser_rendu_ouvriers_lisse,
    synchroniser_rendu_promeneurs_realistes,
};

use super::{
    AstronautId, ColonyVisualAssets, LifeSupportState, LooseIce, StructureState, TaskBoard, TaskId,
    WorkerSnapshot, add_loose_ice_at_cell, assign_available_tasks, cellules_occupees_structures,
    cellules_origine_base, deposit_ice_in_network, find_structure_mut,
    task_needs_missing_structure, trouver_chemin_vers_interaction,
};
use crate::core::WorldOrigin;
use crate::simulation::{
    AIR_MAX_COMBINAISON, CONSOMMATION_AIR_PAR_CASE_OUVRIER, MARGE_SECURITE_RETOUR_OUVRIER,
    autonomie_aller_retour_max_cases, terrain_est_marchable, trouver_chemin_a_star,
    trouver_chemin_vers_objectif,
};
use crate::world::{
    PlanetProfile, TerrainCell, WorldCache, WorldSeed, continuous_world_to_render_translation,
    footprint_center, world_to_cell, world_to_render_translation,
};

// -----------------------------------------------------------------------------
// Reglages de survie / oxygene / exploration
// -----------------------------------------------------------------------------
//
// Objectif gameplay : environ 3 minutes d'autonomie de base.
// La simulation tourne a 4 Hz, donc 3 min = 180 s = 720 ticks.
// 180 / 720 = 0.25 unite d'air consommee par tick.
//
const CONSOMMATION_AIR_PAR_TICK: f32 = CONSOMMATION_AIR_PAR_CASE_OUVRIER;
const CONSOMMATION_AIR_PAR_SECONDE: f32 = 1.0;

// Seuils minimums de securite.
// On ne se contente plus d'un seuil fixe : on calcule un cout de retour
// dynamique, puis on applique ces minimums comme garde-fou.
const AIR_RETOUR_OUVRIER_MIN: f32 = 18.0;
const AIR_RETOUR_PROMENEUR_MIN: f32 = 24.0;

// Marges de securite pour eviter les retours trop tardifs.
const MARGE_SECURITE_RETOUR_PROMENEUR: f32 = 14.0;

// Le promeneur ne repart de l'abri qu'avec une vraie reserve.
const AIR_SORTIE_ABRI_PROMENEUR: f32 = 150.0;
const AIR_MIN_ABRI_PROMENEUR: f32 = 8.0;

// Debits de recharge.
// On recharge moins violemment qu'avant, mais de facon plus realiste et stable.
const RECHARGE_ZONE_PAR_TICK: f32 = 2.0;
const RECHARGE_MODULE_PAR_TICK: f32 = 4.0;
const DISTANCE_ZONE_RECHARGE: i32 = 2;
const VITESSE_PROMENADE_METRES: f32 = 1.6;
const REACH_DISTANCE_METERS: f32 = 0.08;
const LISSAGE_ORIENTATION: f32 = 7.5;
// Exploration plus large autour de la base.
const DISTANCE_MIN_PROMENADE: i32 = 4;
const DISTANCE_MAX_PROMENADE: i32 = 18;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AstronautStatus {
    Idle,
    Moving,
    Working,
    Returning,
    Dead,
}

#[derive(Component, Clone, Debug)]
pub struct Astronaut {
    pub id: AstronautId,
    pub name: &'static str,
    pub suit_oxygen: f32,
    pub current_task: Option<TaskId>,
    pub status: AstronautStatus,
    pub carrying_ice: f32,
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GridPosition(pub IVec2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatPromenade {
    Promenade,
    Pause,
    RetourAbri,
    Abri,
}

impl EtatPromenade {
    pub fn label(self) -> &'static str {
        match self {
            Self::Promenade => "promenade",
            Self::Pause => "pause",
            Self::RetourAbri => "retour",
            Self::Abri => "abri",
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PositionMonde(pub Vec2);

#[derive(Component, Clone, Debug)]
pub struct AstronautePromeneur {
    pub id: AstronautId,
    pub nom: &'static str,
    pub air_combinaison: f32,
    pub etat: EtatPromenade,
    pub cellule_cible: Option<IVec2>,
    pub compteur_promenade: u32,
    pub pause_restante: f32,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AnimationPromenade {
    pub phase_pas: f32,
    pub orientation: f32,
    pub vitesse_normalisee: f32,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct ZoneRechargeBase {
    cellules: Vec<IVec2>,
    index: HashSet<IVec2>,
}

impl ZoneRechargeBase {
    pub fn contient(&self, cell: IVec2) -> bool {
        self.index.contains(&cell)
    }

    pub fn cellules(&self) -> &[IVec2] {
        &self.cellules
    }

    pub fn est_vide(&self) -> bool {
        self.cellules.is_empty()
    }

    pub fn cellule_la_plus_proche(&self, from: IVec2) -> Option<IVec2> {
        self.cellules
            .iter()
            .min_by_key(|cell| (**cell - from).abs().element_sum())
            .copied()
    }

    pub(crate) fn depuis_cellules(mut cellules: Vec<IVec2>) -> Self {
        cellules.sort_by_key(|cell| (cell.x, cell.y));
        cellules.dedup();
        let index = cellules.iter().copied().collect();
        Self { cellules, index }
    }
}

#[derive(SystemParam)]
pub(super) struct ContexteSimulationAstronautes<'w, 's> {
    commands: Commands<'w, 's>,
    board: Res<'w, TaskBoard>,
    life_support: ResMut<'w, LifeSupportState>,
    zone_recharge: Res<'w, ZoneRechargeBase>,
    profile: Res<'w, PlanetProfile>,
    seed: Res<'w, WorldSeed>,
    cache: ResMut<'w, WorldCache>,
}

type RequeteVisuelsAstronautes<'w, 's> = Query<
    'w,
    's,
    (
        &'static Astronaut,
        Ref<'static, Astronaut>,
        &'static GridPosition,
        Ref<'static, GridPosition>,
        &'static mut Transform,
    ),
>;

type RequeteVisuelsPromeneur<'w, 's> = Query<
    'w,
    's,
    (
        &'static AstronautePromeneur,
        Ref<'static, AstronautePromeneur>,
        &'static PositionMonde,
        Ref<'static, PositionMonde>,
        &'static AnimationPromenade,
        Ref<'static, AnimationPromenade>,
        &'static mut Transform,
    ),
>;

#[derive(Component)]
pub(super) struct PivotTorse;

#[derive(Component)]
pub(super) struct PivotTete;

#[derive(Component)]
pub(super) struct PivotBrasGauche;

#[derive(Component)]
pub(super) struct PivotBrasDroit;

#[derive(Component)]
pub(super) struct PivotJambeGauche;

#[derive(Component)]
pub(super) struct PivotJambeDroite;

type RequetesPivotsPromeneur<'w, 's> = (
    Query<'w, 's, &'static mut Transform, With<PivotTorse>>,
    Query<'w, 's, &'static mut Transform, With<PivotTete>>,
    Query<'w, 's, &'static mut Transform, With<PivotBrasGauche>>,
    Query<'w, 's, &'static mut Transform, With<PivotBrasDroit>>,
    Query<'w, 's, &'static mut Transform, With<PivotJambeGauche>>,
    Query<'w, 's, &'static mut Transform, With<PivotJambeDroite>>,
);

#[derive(SystemParam)]
pub(super) struct ContexteAnimationPromeneur<'w, 's> {
    pivots: ParamSet<'w, 's, RequetesPivotsPromeneur<'w, 's>>,
}

pub fn calculer_zone_recharge_base<F>(
    structures: &[StructureState],
    life_support: &LifeSupportState,
    mut terrain_at: F,
) -> ZoneRechargeBase
where
    F: FnMut(IVec2) -> TerrainCell,
{
    let connected: HashSet<_> = life_support
        .primary
        .as_ref()
        .map(|network| network.structures.iter().copied().collect())
        .unwrap_or_default();

    if connected.is_empty() {
        return ZoneRechargeBase::default();
    }

    let occupied: HashSet<_> = structures
        .iter()
        .filter(|structure| structure.built && connected.contains(&structure.id))
        .flat_map(|structure| structure.occupied_cells())
        .collect();

    if occupied.is_empty() {
        return ZoneRechargeBase::default();
    }

    let mut cellules = Vec::new();
    let mut vues = HashSet::new();
    for origine in &occupied {
        for delta_y in -DISTANCE_ZONE_RECHARGE..=DISTANCE_ZONE_RECHARGE {
            for delta_x in -DISTANCE_ZONE_RECHARGE..=DISTANCE_ZONE_RECHARGE {
                let candidate = *origine + IVec2::new(delta_x, delta_y);
                if occupied.contains(&candidate) || !vues.insert(candidate) {
                    continue;
                }

                let terrain = terrain_at(candidate);
                if !terrain_est_marchable(&terrain) {
                    continue;
                }

                cellules.push(candidate);
            }
        }
    }

    ZoneRechargeBase::depuis_cellules(cellules)
}

pub fn astronaute_dans_zone_recharge(zone_recharge: &ZoneRechargeBase, position: IVec2) -> bool {
    zone_recharge.contient(position)
}

pub fn recharger_automatiquement_en_air(
    astronaut: &mut Astronaut,
    position: IVec2,
    structures: &[StructureState],
    zone_recharge: &ZoneRechargeBase,
    life_support: &mut LifeSupportState,
) -> f32 {
    recharger_reserve_air(
        &mut astronaut.suit_oxygen,
        position,
        structures,
        zone_recharge,
        life_support,
    )
}

fn recharger_promeneur_en_air(
    promeneur: &mut AstronautePromeneur,
    position: IVec2,
    structures: &[StructureState],
    zone_recharge: &ZoneRechargeBase,
    life_support: &mut LifeSupportState,
) -> f32 {
    recharger_reserve_air(
        &mut promeneur.air_combinaison,
        position,
        structures,
        zone_recharge,
        life_support,
    )
}

fn recharger_reserve_air(
    reserve: &mut f32,
    position: IVec2,
    structures: &[StructureState],
    zone_recharge: &ZoneRechargeBase,
    life_support: &mut LifeSupportState,
) -> f32 {
    let debit = if position_sur_module_support(structures, position) {
        RECHARGE_MODULE_PAR_TICK
    } else if astronaute_dans_zone_recharge(zone_recharge, position) {
        RECHARGE_ZONE_PAR_TICK
    } else {
        0.0
    };

    if debit <= 0.0 {
        return 0.0;
    }

    let Some(primary) = life_support.primary.as_mut() else {
        return 0.0;
    };

    let besoin = (AIR_MAX_COMBINAISON - *reserve).max(0.0);
    if besoin <= 0.0 {
        return 0.0;
    }

    let draw = besoin.min(primary.oxygen_stored).min(debit);
    *reserve = (*reserve + draw).min(AIR_MAX_COMBINAISON);
    primary.oxygen_stored -= draw;
    draw
}

pub fn recompute_zone_recharge_base(
    mut zone_recharge: ResMut<ZoneRechargeBase>,
    life_support: Res<LifeSupportState>,
    mut cache: ResMut<WorldCache>,
    profile: Res<PlanetProfile>,
    seed: Res<WorldSeed>,
    structures: Query<&StructureState>,
) {
    let structure_snapshots: Vec<_> = structures.iter().cloned().collect();
    *zone_recharge = calculer_zone_recharge_base(&structure_snapshots, &life_support, |cell| {
        cache.terrain_at(cell, &profile, *seed)
    });
}

pub fn ensure_astronaut_visuals(
    mut commands: Commands,
    visuals: Res<ColonyVisualAssets>,
    query: Query<Entity, Added<Astronaut>>,
) {
    for entity in &query {
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

            piece(
                visuals.cube_mesh.clone(),
                visuals.suit_material.clone(),
                Vec3::new(0.0, 0.98, -0.18),
                Quat::IDENTITY,
                Vec3::new(0.52, 0.74, 0.34),
            );
            piece(
                visuals.cube_mesh.clone(),
                visuals.hull_secondary_material.clone(),
                Vec3::new(0.0, 0.98, -0.52),
                Quat::IDENTITY,
                Vec3::new(0.50, 0.68, 0.28),
            );
            piece(
                visuals.cube_mesh.clone(),
                visuals.hull_secondary_material.clone(),
                Vec3::new(0.0, 0.78, 0.10),
                Quat::IDENTITY,
                Vec3::new(0.24, 0.24, 0.12),
            );
            piece(
                visuals.cube_mesh.clone(),
                visuals.frame_material.clone(),
                Vec3::new(0.0, 0.66, 0.0),
                Quat::IDENTITY,
                Vec3::new(0.44, 0.22, 0.30),
            );
            piece(
                visuals.sphere_mesh.clone(),
                visuals.suit_material.clone(),
                Vec3::new(0.0, 1.54, 0.0),
                Quat::IDENTITY,
                Vec3::new(0.40, 0.40, 0.40),
            );
            piece(
                visuals.cube_mesh.clone(),
                visuals.frame_material.clone(),
                Vec3::new(0.0, 1.24, 0.0),
                Quat::IDENTITY,
                Vec3::new(0.28, 0.08, 0.28),
            );
            piece(
                visuals.sphere_mesh.clone(),
                visuals.visor_material.clone(),
                Vec3::new(0.0, 1.50, 0.20),
                Quat::IDENTITY,
                Vec3::new(0.26, 0.23, 0.18),
            );
            piece(
                visuals.sphere_mesh.clone(),
                visuals.glass_material.clone(),
                Vec3::new(0.0, 1.57, 0.10),
                Quat::IDENTITY,
                Vec3::new(0.08, 0.08, 0.08),
            );
            for &(x, y) in &[(-0.32, 1.06), (0.32, 1.06)] {
                piece(
                    visuals.sphere_mesh.clone(),
                    visuals.suit_fabric_material.clone(),
                    Vec3::new(x, y, 0.0),
                    Quat::IDENTITY,
                    Vec3::new(0.13, 0.13, 0.13),
                );
            }
            for &(x, rotation) in &[(-0.42, 0.95), (0.42, -0.95)] {
                piece(
                    visuals.capsule_mesh.clone(),
                    visuals.suit_fabric_material.clone(),
                    Vec3::new(x, 0.98, 0.0),
                    Quat::from_rotation_z(rotation),
                    Vec3::new(0.24, 0.52, 0.24),
                );
            }
            for &(x, rotation) in &[(-0.66, 0.74), (0.66, -0.74)] {
                piece(
                    visuals.capsule_mesh.clone(),
                    visuals.suit_fabric_material.clone(),
                    Vec3::new(x, 0.70, 0.02),
                    Quat::from_rotation_z(rotation),
                    Vec3::new(0.20, 0.42, 0.20),
                );
            }
            piece(
                visuals.cube_mesh.clone(),
                visuals.mission_red_material.clone(),
                Vec3::new(-0.52, 1.02, 0.0),
                Quat::from_rotation_z(0.95),
                Vec3::new(0.24, 0.04, 0.24),
            );
            piece(
                visuals.cube_mesh.clone(),
                visuals.mission_blue_material.clone(),
                Vec3::new(0.30, 1.00, 0.19),
                Quat::IDENTITY,
                Vec3::new(0.10, 0.10, 0.02),
            );
            for &(x, z) in &[(-0.80, 0.02), (0.80, 0.02)] {
                piece(
                    visuals.sphere_mesh.clone(),
                    visuals.suit_material.clone(),
                    Vec3::new(x, 0.46, z),
                    Quat::IDENTITY,
                    Vec3::new(0.10, 0.10, 0.10),
                );
            }
            for &(x, z) in &[(-0.16, 0.00), (0.16, 0.00)] {
                piece(
                    visuals.capsule_mesh.clone(),
                    visuals.suit_fabric_material.clone(),
                    Vec3::new(x, 0.26, z),
                    Quat::IDENTITY,
                    Vec3::new(0.22, 0.52, 0.22),
                );
                piece(
                    visuals.cube_mesh.clone(),
                    visuals.boot_material.clone(),
                    Vec3::new(x, -0.08, 0.05),
                    Quat::IDENTITY,
                    Vec3::new(0.22, 0.12, 0.34),
                );
            }
            piece(
                visuals.cube_mesh.clone(),
                visuals.lit_material.clone(),
                Vec3::new(0.0, 1.10, -0.71),
                Quat::IDENTITY,
                Vec3::new(0.09, 0.09, 0.06),
            );
            for &x in &[-0.18, 0.18] {
                piece(
                    visuals.cube_mesh.clone(),
                    visuals.frame_material.clone(),
                    Vec3::new(x, 1.00, -0.36),
                    Quat::from_rotation_x(0.22),
                    Vec3::new(0.04, 0.04, 0.34),
                );
            }
        });
    }
}

pub fn sync_astronaut_visuals(
    mut cache: ResMut<WorldCache>,
    profile: Res<PlanetProfile>,
    seed: Res<WorldSeed>,
    origin: Res<WorldOrigin>,
    mut query: RequeteVisuelsAstronautes,
) {
    let origin_changed = origin.is_changed();
    for (astronaut, astronaut_ref, position, position_ref, mut transform) in &mut query {
        if !origin_changed && !astronaut_ref.is_changed() && !position_ref.is_changed() {
            continue;
        }

        let terrain = cache.terrain_at(position.0, &profile, *seed);
        transform.translation =
            world_to_render_translation(position.0, terrain.height + 0.10, &profile, &origin);
        transform.rotation = if astronaut.status == AstronautStatus::Dead {
            Quat::from_rotation_z(-0.92)
        } else {
            Quat::IDENTITY
        };
    }
}

pub fn ensure_promeneur_visuals(
    mut commands: Commands,
    visuals: Res<ColonyVisualAssets>,
    query: Query<Entity, Added<AstronautePromeneur>>,
) {
    for entity in &query {
        commands.entity(entity).insert((
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
        ));
        commands.entity(entity).with_children(|parent| {
            parent
                .spawn((
                    PivotTorse,
                    Transform::from_xyz(0.0, 0.96, -0.03),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    pivot.spawn((
                        Mesh3d(visuals.cube_mesh.clone()),
                        MeshMaterial3d(visuals.suit_material.clone()),
                        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(0.54, 0.78, 0.38)),
                    ));
                    pivot.spawn((
                        Mesh3d(visuals.cube_mesh.clone()),
                        MeshMaterial3d(visuals.hull_secondary_material.clone()),
                        Transform::from_xyz(0.0, 0.02, -0.33)
                            .with_scale(Vec3::new(0.48, 0.66, 0.26)),
                    ));
                    pivot.spawn((
                        Mesh3d(visuals.cube_mesh.clone()),
                        MeshMaterial3d(visuals.frame_material.clone()),
                        Transform::from_xyz(0.0, -0.36, 0.02)
                            .with_scale(Vec3::new(0.34, 0.12, 0.32)),
                    ));
                    pivot.spawn((
                        Mesh3d(visuals.cube_mesh.clone()),
                        MeshMaterial3d(visuals.mission_red_material.clone()),
                        Transform::from_xyz(-0.18, 0.10, 0.22)
                            .with_rotation(Quat::from_rotation_z(0.44))
                            .with_scale(Vec3::new(0.22, 0.04, 0.18)),
                    ));
                });

            parent
                .spawn((
                    PivotTete,
                    Transform::from_xyz(0.0, 1.63, 0.05),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    pivot.spawn((
                        Mesh3d(visuals.sphere_mesh.clone()),
                        MeshMaterial3d(visuals.suit_material.clone()),
                        Transform::from_scale(Vec3::new(0.42, 0.42, 0.42)),
                    ));
                    pivot.spawn((
                        Mesh3d(visuals.sphere_mesh.clone()),
                        MeshMaterial3d(visuals.visor_material.clone()),
                        Transform::from_xyz(0.0, 0.00, 0.18)
                            .with_scale(Vec3::new(0.26, 0.24, 0.18)),
                    ));
                    pivot.spawn((
                        Mesh3d(visuals.cube_mesh.clone()),
                        MeshMaterial3d(visuals.frame_material.clone()),
                        Transform::from_xyz(0.0, -0.26, 0.0)
                            .with_scale(Vec3::new(0.24, 0.08, 0.24)),
                    ));
                    pivot.spawn((
                        Mesh3d(visuals.cube_mesh.clone()),
                        MeshMaterial3d(visuals.lit_material.clone()),
                        Transform::from_xyz(0.14, 0.12, -0.18)
                            .with_scale(Vec3::new(0.05, 0.05, 0.12)),
                    ));
                });

            parent
                .spawn((
                    PivotBrasGauche,
                    Transform::from_xyz(-0.34, 1.28, 0.0),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    pivot.spawn((
                        Mesh3d(visuals.capsule_mesh.clone()),
                        MeshMaterial3d(visuals.suit_fabric_material.clone()),
                        Transform::from_xyz(0.0, -0.30, 0.0)
                            .with_rotation(Quat::from_rotation_z(FRAC_PI_2))
                            .with_scale(Vec3::new(0.18, 0.52, 0.18)),
                    ));
                    pivot.spawn((
                        Mesh3d(visuals.cube_mesh.clone()),
                        MeshMaterial3d(visuals.boot_material.clone()),
                        Transform::from_xyz(0.0, -0.68, 0.06)
                            .with_scale(Vec3::new(0.12, 0.14, 0.20)),
                    ));
                });

            parent
                .spawn((
                    PivotBrasDroit,
                    Transform::from_xyz(0.34, 1.28, 0.0),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    pivot.spawn((
                        Mesh3d(visuals.capsule_mesh.clone()),
                        MeshMaterial3d(visuals.suit_fabric_material.clone()),
                        Transform::from_xyz(0.0, -0.30, 0.0)
                            .with_rotation(Quat::from_rotation_z(FRAC_PI_2))
                            .with_scale(Vec3::new(0.18, 0.52, 0.18)),
                    ));
                    pivot.spawn((
                        Mesh3d(visuals.cube_mesh.clone()),
                        MeshMaterial3d(visuals.boot_material.clone()),
                        Transform::from_xyz(0.0, -0.68, 0.06)
                            .with_scale(Vec3::new(0.12, 0.14, 0.20)),
                    ));
                });

            parent
                .spawn((
                    PivotJambeGauche,
                    Transform::from_xyz(-0.16, 0.78, 0.0),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    pivot.spawn((
                        Mesh3d(visuals.capsule_mesh.clone()),
                        MeshMaterial3d(visuals.suit_fabric_material.clone()),
                        Transform::from_xyz(0.0, -0.40, 0.0)
                            .with_scale(Vec3::new(0.20, 0.56, 0.20)),
                    ));
                    pivot.spawn((
                        Mesh3d(visuals.cube_mesh.clone()),
                        MeshMaterial3d(visuals.boot_material.clone()),
                        Transform::from_xyz(0.0, -0.78, 0.10)
                            .with_scale(Vec3::new(0.22, 0.12, 0.34)),
                    ));
                });

            parent
                .spawn((
                    PivotJambeDroite,
                    Transform::from_xyz(0.16, 0.78, 0.0),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    pivot.spawn((
                        Mesh3d(visuals.capsule_mesh.clone()),
                        MeshMaterial3d(visuals.suit_fabric_material.clone()),
                        Transform::from_xyz(0.0, -0.40, 0.0)
                            .with_scale(Vec3::new(0.20, 0.56, 0.20)),
                    ));
                    pivot.spawn((
                        Mesh3d(visuals.cube_mesh.clone()),
                        MeshMaterial3d(visuals.boot_material.clone()),
                        Transform::from_xyz(0.0, -0.78, 0.10)
                            .with_scale(Vec3::new(0.22, 0.12, 0.34)),
                    ));
                });
        });
    }
}

pub fn sync_promeneur_visuals(
    mut cache: ResMut<WorldCache>,
    profile: Res<PlanetProfile>,
    seed: Res<WorldSeed>,
    origin: Res<WorldOrigin>,
    mut query: RequeteVisuelsPromeneur,
) {
    let origin_changed = origin.is_changed();
    for (_, promeneur_ref, position, position_ref, animation, animation_ref, mut transform) in
        &mut query
    {
        if !origin_changed
            && !promeneur_ref.is_changed()
            && !position_ref.is_changed()
            && !animation_ref.is_changed()
        {
            continue;
        }

        transform.translation = continuous_world_to_render_translation(
            position.0, 0.06, &mut cache, &profile, *seed, &origin,
        );
        transform.rotation = Quat::from_rotation_y(animation.orientation);
    }
}

pub fn animer_promeneur(
    time: Res<Time>,
    promeneur: Query<(&AstronautePromeneur, &AnimationPromenade)>,
    mut contexte: ContexteAnimationPromeneur,
) {
    let Ok((promeneur, animation)) = promeneur.single() else {
        return;
    };

    let phase = animation.phase_pas;
    let intensite_pas = animation.vitesse_normalisee.clamp(0.0, 1.0);
    let balancement = phase.sin() * 0.58 * intensite_pas;
    let rebond = (phase * 2.0).sin().abs() * 0.07 * intensite_pas;
    let respiration = (time.elapsed_secs() * 2.1).sin() * 0.02;
    let regard_pause = if promeneur.etat == EtatPromenade::Pause {
        (time.elapsed_secs() * 1.7).sin() * 0.18
    } else {
        0.0
    };

    if let Ok(mut transform) = contexte.pivots.p0().single_mut() {
        transform.translation = Vec3::new(0.0, 0.96 + rebond + respiration * 0.4, -0.03);
        transform.rotation = Quat::from_rotation_x(respiration * 0.08);
    }
    if let Ok(mut transform) = contexte.pivots.p1().single_mut() {
        transform.translation = Vec3::new(0.0, 1.63 + rebond * 0.45, 0.05);
        transform.rotation =
            Quat::from_rotation_y(regard_pause) * Quat::from_rotation_x(respiration * 0.12);
    }
    if let Ok(mut transform) = contexte.pivots.p2().single_mut() {
        transform.translation = Vec3::new(-0.34, 1.28 + rebond * 0.25, 0.0);
        transform.rotation = Quat::from_rotation_x(-balancement);
    }
    if let Ok(mut transform) = contexte.pivots.p3().single_mut() {
        transform.translation = Vec3::new(0.34, 1.28 + rebond * 0.25, 0.0);
        transform.rotation = Quat::from_rotation_x(balancement);
    }
    if let Ok(mut transform) = contexte.pivots.p4().single_mut() {
        transform.translation = Vec3::new(-0.16, 0.78 + rebond * 0.1, 0.0);
        transform.rotation = Quat::from_rotation_x(balancement * 0.85);
    }
    if let Ok(mut transform) = contexte.pivots.p5().single_mut() {
        transform.translation = Vec3::new(0.16, 0.78 + rebond * 0.1, 0.0);
        transform.rotation = Quat::from_rotation_x(-balancement * 0.85);
    }
}

pub fn assign_tasks_system(
    mut board: ResMut<TaskBoard>,
    mut astronauts: Query<(&mut Astronaut, &GridPosition)>,
) {
    let workers = astronauts
        .iter_mut()
        .map(|(astronaut, position)| WorkerSnapshot {
            id: astronaut.id,
            position: position.0,
            current_task: astronaut.current_task,
            suit_oxygen: astronaut.suit_oxygen,
            alive: astronaut.status != AstronautStatus::Dead,
        })
        .collect::<Vec<_>>();

    let assignments = assign_available_tasks(&mut board.tasks, &workers);
    let assignment_map: HashMap<_, _> = assignments.into_iter().collect();

    for (mut astronaut, _) in &mut astronauts {
        if astronaut.current_task.is_some() {
            continue;
        }
        if let Some(task_id) = assignment_map.get(&astronaut.id) {
            astronaut.current_task = Some(*task_id);
        }
    }
}

fn abandonner_tache_en_cours(
    depots_glace_en_attente: &mut HashMap<IVec2, f32>,
    astronaut: &mut Astronaut,
    position: IVec2,
) {
    if astronaut.carrying_ice > 0.0 {
        accumuler_glace_libre_en_attente(
            depots_glace_en_attente,
            position,
            astronaut.carrying_ice,
        );
        astronaut.carrying_ice = 0.0;
    }

    astronaut.current_task = None;
    astronaut.status = AstronautStatus::Idle;
}

fn accumuler_glace_libre_en_attente(
    depots_glace_en_attente: &mut HashMap<IVec2, f32>,
    cell: IVec2,
    amount: f32,
) {
    if amount <= 0.0 {
        return;
    }

    *depots_glace_en_attente.entry(cell).or_insert(0.0) += amount;
}

fn prelever_glace_libre(
    commands: &mut Commands,
    loose_ice_query: &mut Query<(Entity, &mut LooseIce)>,
    depots_glace_en_attente: &mut HashMap<IVec2, f32>,
    cell: IVec2,
    requested: f32,
) -> f32 {
    if requested <= 0.0 {
        return 0.0;
    }

    let mut restant = requested;
    let mut preleve = 0.0;
    let mut a_supprimer = Vec::new();

    for (ice_entity, mut loose) in loose_ice_query
        .iter_mut()
        .filter(|(_, loose)| loose.cell == cell && loose.amount > 0.0)
    {
        if restant <= 0.0 {
            break;
        }

        let pris = loose.amount.min(restant);
        loose.amount -= pris;
        preleve += pris;
        restant -= pris;

        if loose.amount <= 0.0 {
            a_supprimer.push(ice_entity);
        }
    }

    for entity in a_supprimer {
        commands.entity(entity).despawn();
    }

    if restant > 0.0 {
        let mut supprimer_entree = false;
        if let Some(disponible) = depots_glace_en_attente.get_mut(&cell) {
            let pris = (*disponible).min(restant);
            *disponible -= pris;
            preleve += pris;
            restant -= pris;
            supprimer_entree = *disponible <= 0.0;
        }
        if supprimer_entree {
            depots_glace_en_attente.remove(&cell);
        }
    }

    let _ = restant;
    preleve
}

fn appliquer_depots_glace_en_attente(
    commands: &mut Commands,
    loose_ice_query: &mut Query<(Entity, &mut LooseIce)>,
    depots_glace_en_attente: HashMap<IVec2, f32>,
) {
    for (cell, amount) in depots_glace_en_attente {
        add_loose_ice_at_cell(commands, loose_ice_query, cell, amount);
    }
}

pub fn advance_astronauts(
    mut contexte: ContexteSimulationAstronautes,
    mut structures: ParamSet<(Query<&StructureState>, Query<&mut StructureState>)>,
    mut astronauts: Query<(Entity, &mut Astronaut, &mut GridPosition)>,
    mut loose_ice_query: Query<(Entity, &mut LooseIce)>,
) {
    let structure_snapshots: Vec<_> = structures.p0().iter().cloned().collect();
    let structure_positions: HashMap<_, _> = structure_snapshots
        .iter()
        .map(|structure| (structure.id, structure.anchor))
        .collect();
    let cellules_occupees = cellules_occupees_structures(&structure_snapshots);
    let limite_navigation = autonomie_aller_retour_max_cases() + 128;
    let _origines_base = cellules_origine_base(&contexte.zone_recharge);
    let mut depots_glace_en_attente = HashMap::new();

    for (_, mut astronaut, mut position) in &mut astronauts {
        if astronaut.status == AstronautStatus::Dead {
            continue;
        }

        astronaut.suit_oxygen = (astronaut.suit_oxygen - CONSOMMATION_AIR_PAR_TICK).max(0.0);
        if astronaut.suit_oxygen <= 0.0 {
            if astronaut.carrying_ice > 0.0 {
                accumuler_glace_libre_en_attente(
                    &mut depots_glace_en_attente,
                    position.0,
                    astronaut.carrying_ice,
                );
                astronaut.carrying_ice = 0.0;
            }
            astronaut.status = AstronautStatus::Dead;
            astronaut.current_task = None;
            continue;
        }

        if doit_rentrer_ouvrier_avec_terrain(
            &astronaut,
            &contexte.zone_recharge,
            &structure_snapshots,
            position.0,
            |cellule| {
                contexte
                    .cache
                    .terrain_at(cellule, &contexte.profile, *contexte.seed)
            },
        ) {
            astronaut.status = AstronautStatus::Returning;

            if let Some(chemin) = chemin_vers_recharge_ouvrier(
                &contexte.zone_recharge,
                &structure_snapshots,
                position.0,
                limite_navigation,
                |cellule| {
                    contexte
                        .cache
                        .terrain_at(cellule, &contexte.profile, *contexte.seed)
                },
            ) {
                if let Some(prochain_pas) = chemin.cellules.get(1).copied() {
                    position.0 = prochain_pas;
                }
            }

            recharger_automatiquement_en_air(
                &mut astronaut,
                position.0,
                &structure_snapshots,
                &contexte.zone_recharge,
                &mut contexte.life_support,
            );
            continue;
        }

        let Some(task_id) = astronaut.current_task else {
            astronaut.status = AstronautStatus::Idle;
            recharger_automatiquement_en_air(
                &mut astronaut,
                position.0,
                &structure_snapshots,
                &contexte.zone_recharge,
                &mut contexte.life_support,
            );
            continue;
        };

        let task = contexte
            .board
            .tasks
            .iter()
            .find(|task| task.id == task_id)
            .cloned();
        let Some(task) = task else {
            astronaut.current_task = None;
            astronaut.status = AstronautStatus::Idle;
            continue;
        };

        if task_needs_missing_structure(&task, astronaut.carrying_ice, &structure_positions) {
            abandonner_tache_en_cours(
                &mut depots_glace_en_attente,
                &mut astronaut,
                position.0,
            );
            continue;
        }

        let target_cell = match task.kind {
            super::TaskKind::Build { .. } | super::TaskKind::ReturnToBase { .. } => {
                Some(task.target_cell)
            }
            super::TaskKind::Extract { cell } => Some(cell),
            super::TaskKind::HaulIce { source_cell, .. } => Some(if astronaut.carrying_ice > 0.0 {
                task.target_cell
            } else {
                source_cell
            }),
            super::TaskKind::RefuelStructure { source, target: _ } => {
                if astronaut.carrying_ice > 0.0 {
                    Some(task.target_cell)
                } else {
                    structure_snapshots
                        .iter()
                        .find(|structure| structure.id == source)
                        .and_then(|source_state| {
                            trouver_chemin_vers_interaction(
                                position.0,
                                source_state.kind,
                                source_state.anchor,
                                &structure_snapshots,
                                limite_navigation,
                                |cellule| {
                                    contexte.cache.terrain_at(
                                        cellule,
                                        &contexte.profile,
                                        *contexte.seed,
                                    )
                                },
                            )
                        })
                        .and_then(|chemin| chemin.cellules.last().copied())
                }
            }
        };
        let Some(target_cell) = target_cell else {
            abandonner_tache_en_cours(
                &mut depots_glace_en_attente,
                &mut astronaut,
                position.0,
            );
            continue;
        };

        if position.0 != target_cell {
            let Some(chemin) = trouver_chemin_a_star(
                position.0,
                target_cell,
                &cellules_occupees,
                limite_navigation,
                |cellule| {
                    contexte
                        .cache
                        .terrain_at(cellule, &contexte.profile, *contexte.seed)
                },
            ) else {
                abandonner_tache_en_cours(
                    &mut depots_glace_en_attente,
                    &mut astronaut,
                    position.0,
                );
                continue;
            };
            let Some(cout_retour) = cout_retour_ouvrier_avec_terrain(
                &contexte.zone_recharge,
                &structure_snapshots,
                target_cell,
                |cellule| {
                    contexte
                        .cache
                        .terrain_at(cellule, &contexte.profile, *contexte.seed)
                },
            ) else {
                abandonner_tache_en_cours(
                    &mut depots_glace_en_attente,
                    &mut astronaut,
                    position.0,
                );
                continue;
            };

            let oxygene_necessaire = chemin.cout as f32 * CONSOMMATION_AIR_PAR_TICK + cout_retour;
            if oxygene_necessaire > astronaut.suit_oxygen {
                abandonner_tache_en_cours(
                    &mut depots_glace_en_attente,
                    &mut astronaut,
                    position.0,
                );
                continue;
            }

            astronaut.status = AstronautStatus::Moving;
            if let Some(prochain_pas) = chemin.cellules.get(1).copied() {
                position.0 = prochain_pas;
            }
            recharger_automatiquement_en_air(
                &mut astronaut,
                position.0,
                &structure_snapshots,
                &contexte.zone_recharge,
                &mut contexte.life_support,
            );
            continue;
        }

        astronaut.status = AstronautStatus::Working;
        match task.kind {
            super::TaskKind::Build { structure } => {
                if let Some(mut state) = find_structure_mut(&mut structures.p1(), structure) {
                    state.build_progress += 1.0 / state.kind.build_ticks();
                    if state.build_progress >= 1.0 {
                        state.build_progress = 1.0;
                        state.built = true;
                        astronaut.current_task = None;
                    }
                } else {
                    astronaut.current_task = None;
                }
            }
            super::TaskKind::Extract { cell } => {
                let extracted =
                    contexte
                        .cache
                        .extract_resource(cell, &contexte.profile, *contexte.seed, 1);
                if extracted > 0 {
                    accumuler_glace_libre_en_attente(
                        &mut depots_glace_en_attente,
                        cell,
                        extracted as f32,
                    );
                }
                astronaut.current_task = None;
            }
            super::TaskKind::HaulIce {
                source_cell,
                target: _,
            } => {
                if astronaut.carrying_ice <= 0.0 {
                    let carried = prelever_glace_libre(
                        &mut contexte.commands,
                        &mut loose_ice_query,
                        &mut depots_glace_en_attente,
                        source_cell,
                        1.0,
                    );
                    if carried > 0.0 {
                        astronaut.carrying_ice = carried;
                    } else {
                        astronaut.current_task = None;
                    }
                } else {
                    if let Some(primary) = contexte.life_support.primary.as_mut() {
                        let deposited = deposit_ice_in_network(primary, astronaut.carrying_ice);
                        astronaut.carrying_ice -= deposited;
                    }

                    if astronaut.carrying_ice > 0.0 {
                        accumuler_glace_libre_en_attente(
                            &mut depots_glace_en_attente,
                            position.0,
                            astronaut.carrying_ice,
                        );
                        astronaut.carrying_ice = 0.0;
                    }

                    astronaut.current_task = None;
                }
            }
            super::TaskKind::RefuelStructure { source: _, target } => {
                if astronaut.carrying_ice <= 0.0 {
                    if let Some(primary) = contexte.life_support.primary.as_mut() {
                        if primary.ice_stored >= 1.0 {
                            primary.ice_stored -= 1.0;
                            astronaut.carrying_ice = 1.0;
                        } else {
                            astronaut.current_task = None;
                        }
                    }
                } else {
                    if let Some(mut state) = find_structure_mut(&mut structures.p1(), target) {
                        state.internal_ice += astronaut.carrying_ice;
                    } else {
                        accumuler_glace_libre_en_attente(
                            &mut depots_glace_en_attente,
                            position.0,
                            astronaut.carrying_ice,
                        );
                    }
                    astronaut.carrying_ice = 0.0;
                    astronaut.current_task = None;
                }
            }
            super::TaskKind::ReturnToBase { target: _ } => {
                recharger_automatiquement_en_air(
                    &mut astronaut,
                    position.0,
                    &structure_snapshots,
                    &contexte.zone_recharge,
                    &mut contexte.life_support,
                );
                astronaut.current_task = None;
            }
        }

        if astronaut.current_task.is_none() {
            astronaut.status = AstronautStatus::Idle;
        }
    }

    appliquer_depots_glace_en_attente(
        &mut contexte.commands,
        &mut loose_ice_query,
        depots_glace_en_attente,
    );
}

pub fn simuler_promeneur(
    zone_recharge: Res<ZoneRechargeBase>,
    mut life_support: ResMut<LifeSupportState>,
    mut cache: ResMut<WorldCache>,
    profile: Res<PlanetProfile>,
    seed: Res<WorldSeed>,
    structures: Query<&StructureState>,
    mut promeneurs: Query<(&mut AstronautePromeneur, &PositionMonde)>,
) {
    let structure_snapshots: Vec<_> = structures.iter().cloned().collect();

    for (mut promeneur, position_monde) in &mut promeneurs {
        let cellule = world_to_cell(position_monde.0, profile.cell_size_meters);

        if promeneur.etat != EtatPromenade::Abri || !zone_recharge.contient(cellule) {
            promeneur.air_combinaison =
                (promeneur.air_combinaison - CONSOMMATION_AIR_PAR_TICK).max(0.0);
        }

        if promeneur.air_combinaison <= 0.0 {
            promeneur.air_combinaison = AIR_MIN_ABRI_PROMENEUR;
            promeneur.etat = EtatPromenade::RetourAbri;
            promeneur.pause_restante = 0.0;
            promeneur.cellule_cible = cible_recharge_promeneur(
                &zone_recharge,
                &structure_snapshots,
                &life_support,
                cellule,
            );
        }

        let seuil_retour = seuil_retour_promeneur(
            position_monde.0,
            &zone_recharge,
            &structure_snapshots,
            &life_support,
            &profile,
        );

        if promeneur.air_combinaison <= seuil_retour
            && promeneur.etat != EtatPromenade::RetourAbri
            && promeneur.etat != EtatPromenade::Abri
        {
            promeneur.etat = EtatPromenade::RetourAbri;
            promeneur.pause_restante = 0.0;
            promeneur.cellule_cible = cible_recharge_promeneur(
                &zone_recharge,
                &structure_snapshots,
                &life_support,
                cellule,
            );
        }

        if promeneur.etat == EtatPromenade::Promenade
            && !cible_promenade_est_encore_sure(
                &promeneur,
                position_monde.0,
                &zone_recharge,
                &structure_snapshots,
                &life_support,
                &profile,
            )
        {
            promeneur.etat = EtatPromenade::RetourAbri;
            promeneur.pause_restante = 0.0;
            promeneur.cellule_cible = cible_recharge_promeneur(
                &zone_recharge,
                &structure_snapshots,
                &life_support,
                cellule,
            );
        }

        match promeneur.etat {
            EtatPromenade::Promenade => {
                if promeneur.cellule_cible.is_none() {
                    promeneur.cellule_cible = choisir_cible_promenade(
                        &promeneur,
                        &structure_snapshots,
                        &mut cache,
                        &profile,
                        *seed,
                    )
                    .or_else(|| {
                        cible_recharge_promeneur(
                            &zone_recharge,
                            &structure_snapshots,
                            &life_support,
                            cellule,
                        )
                    });

                    if promeneur.cellule_cible.is_some() {
                        promeneur.compteur_promenade += 1;
                    }
                }
            }
            EtatPromenade::Pause => {}
            EtatPromenade::RetourAbri => {
                if zone_recharge.contient(cellule) {
                    promeneur.etat = EtatPromenade::Abri;
                    promeneur.cellule_cible = None;
                } else if promeneur.cellule_cible.is_none() {
                    promeneur.cellule_cible = cible_recharge_promeneur(
                        &zone_recharge,
                        &structure_snapshots,
                        &life_support,
                        cellule,
                    );
                }
            }
            EtatPromenade::Abri => {}
        }

        let recharge = recharger_promeneur_en_air(
            &mut promeneur,
            cellule,
            &structure_snapshots,
            &zone_recharge,
            &mut life_support,
        );

        if promeneur.etat == EtatPromenade::RetourAbri && zone_recharge.contient(cellule) {
            promeneur.etat = EtatPromenade::Abri;
            promeneur.cellule_cible = None;
        }

        if promeneur.etat == EtatPromenade::Abri {
            if recharge <= 0.0 {
                promeneur.air_combinaison = promeneur.air_combinaison.max(AIR_MIN_ABRI_PROMENEUR);
            }

            if promeneur.air_combinaison >= AIR_SORTIE_ABRI_PROMENEUR {
                promeneur.etat = EtatPromenade::Promenade;
                promeneur.cellule_cible = choisir_cible_promenade(
                    &promeneur,
                    &structure_snapshots,
                    &mut cache,
                    &profile,
                    *seed,
                )
                .or_else(|| {
                    cible_recharge_promeneur(
                        &zone_recharge,
                        &structure_snapshots,
                        &life_support,
                        cellule,
                    )
                });

                if promeneur.cellule_cible.is_some() {
                    promeneur.compteur_promenade += 1;
                }
            }
        }
    }
}

pub fn deplacer_promeneur(
    time: Res<Time>,
    profile: Res<PlanetProfile>,
    seed: Res<WorldSeed>,
    mut promeneurs: Query<(
        &mut AstronautePromeneur,
        &mut PositionMonde,
        &mut AnimationPromenade,
    )>,
) {
    let delta = time.delta_secs();
    for (mut promeneur, mut position, mut animation) in &mut promeneurs {
        if promeneur.etat == EtatPromenade::Pause {
            promeneur.pause_restante = (promeneur.pause_restante - delta).max(0.0);
            if promeneur.pause_restante <= 0.0 {
                promeneur.etat = EtatPromenade::Promenade;
            }
            animation.vitesse_normalisee = 0.0;
            animation.phase_pas += delta * 1.8;
            continue;
        }

        let Some(cellule_cible) = promeneur.cellule_cible else {
            animation.vitesse_normalisee = 0.0;
            animation.phase_pas += delta * 1.4;
            continue;
        };

        let cible_monde = footprint_center(&[cellule_cible], profile.cell_size_meters);
        let delta_cible = cible_monde - position.0;
        let distance = delta_cible.length();
        if distance <= REACH_DISTANCE_METERS {
            position.0 = cible_monde;
            animation.vitesse_normalisee = 0.0;
            if promeneur.etat == EtatPromenade::Promenade {
                promeneur.etat = EtatPromenade::Pause;
                promeneur.pause_restante =
                    duree_pause_deterministe(*seed, promeneur.id, promeneur.compteur_promenade);
            } else if promeneur.etat == EtatPromenade::RetourAbri {
                promeneur.etat = EtatPromenade::Abri;
            }
            promeneur.cellule_cible = None;
            animation.phase_pas += delta * 1.5;
            continue;
        }

        let pas = VITESSE_PROMENADE_METRES * delta;
        let direction = delta_cible.normalize();
        let mouvement = direction * pas.min(distance);
        position.0 += mouvement;

        let orientation_cible = direction.x.atan2(direction.y);
        animation.orientation = lerp_angle(
            animation.orientation,
            orientation_cible,
            (delta * LISSAGE_ORIENTATION).clamp(0.0, 1.0),
        );
        animation.vitesse_normalisee =
            (mouvement.length() / (VITESSE_PROMENADE_METRES * delta.max(0.001))).clamp(0.0, 1.0);
        animation.phase_pas += delta * (3.4 + 5.2 * animation.vitesse_normalisee);
    }
}

fn position_sur_module_support(structures: &[StructureState], position: IVec2) -> bool {
    structures.iter().any(|structure| {
        structure.built
            && structure.kind.supports_life()
            && structure.occupied_cells().contains(&position)
            && structure.network_id == Some(0)
    })
}

fn chemin_vers_recharge_ouvrier<F>(
    zone_recharge: &ZoneRechargeBase,
    structures: &[StructureState],
    from: IVec2,
    limite_cout: i32,
    terrain_at: F,
) -> Option<crate::simulation::CheminCellulaire>
where
    F: FnMut(IVec2) -> TerrainCell,
{
    let objectifs = cellules_origine_base(zone_recharge);
    if objectifs.is_empty() {
        return None;
    }

    let cellules_occupees = cellules_occupees_structures(structures);
    trouver_chemin_vers_objectif(
        from,
        &objectifs,
        &cellules_occupees,
        limite_cout,
        terrain_at,
    )
}

fn consommation_air_promeneur_par_metre() -> f32 {
    CONSOMMATION_AIR_PAR_SECONDE / VITESSE_PROMENADE_METRES
}

fn cout_retour_ouvrier_avec_terrain<F>(
    zone_recharge: &ZoneRechargeBase,
    structures: &[StructureState],
    from: IVec2,
    terrain_at: F,
) -> Option<f32>
where
    F: FnMut(IVec2) -> TerrainCell,
{
    chemin_vers_recharge_ouvrier(
        zone_recharge,
        structures,
        from,
        autonomie_aller_retour_max_cases() + 128,
        terrain_at,
    )
    .map(|chemin| chemin.cout as f32 * CONSOMMATION_AIR_PAR_TICK + MARGE_SECURITE_RETOUR_OUVRIER)
}

fn seuil_retour_ouvrier_avec_terrain<F>(
    zone_recharge: &ZoneRechargeBase,
    structures: &[StructureState],
    from: IVec2,
    terrain_at: F,
) -> f32
where
    F: FnMut(IVec2) -> TerrainCell,
{
    cout_retour_ouvrier_avec_terrain(zone_recharge, structures, from, terrain_at)
        .unwrap_or(AIR_RETOUR_OUVRIER_MIN)
        .max(AIR_RETOUR_OUVRIER_MIN)
}

fn doit_rentrer_ouvrier_avec_terrain<F>(
    astronaut: &Astronaut,
    zone_recharge: &ZoneRechargeBase,
    structures: &[StructureState],
    from: IVec2,
    terrain_at: F,
) -> bool
where
    F: FnMut(IVec2) -> TerrainCell,
{
    astronaut.suit_oxygen
        <= seuil_retour_ouvrier_avec_terrain(zone_recharge, structures, from, terrain_at)
}

fn cible_recharge_promeneur(
    zone_recharge: &ZoneRechargeBase,
    _structures: &[StructureState],
    _life_support: &LifeSupportState,
    from: IVec2,
) -> Option<IVec2> {
    zone_recharge.cellule_la_plus_proche(from)
}

fn cout_retour_promeneur(
    position_monde: Vec2,
    zone_recharge: &ZoneRechargeBase,
    structures: &[StructureState],
    life_support: &LifeSupportState,
    profile: &PlanetProfile,
) -> Option<f32> {
    let mut meilleur: Option<f32> = None;

    for &cellule in zone_recharge.cellules() {
        let centre = footprint_center(&[cellule], profile.cell_size_meters);
        let distance = position_monde.distance(centre);
        let cout =
            distance * consommation_air_promeneur_par_metre() + MARGE_SECURITE_RETOUR_PROMENEUR;

        meilleur = Some(match meilleur {
            Some(valeur) => valeur.min(cout),
            None => cout,
        });
    }

    let connected: HashSet<_> = life_support
        .primary
        .as_ref()
        .map(|network| network.structures.iter().copied().collect())
        .unwrap_or_default();

    for structure in structures.iter().filter(|structure| {
        structure.built && structure.kind.supports_life() && connected.contains(&structure.id)
    }) {
        let centre = structure.center_world(profile.cell_size_meters);
        let distance = position_monde.distance(centre);
        let cout =
            distance * consommation_air_promeneur_par_metre() + MARGE_SECURITE_RETOUR_PROMENEUR;

        meilleur = Some(match meilleur {
            Some(valeur) => valeur.min(cout),
            None => cout,
        });
    }

    meilleur
}

fn seuil_retour_promeneur(
    position_monde: Vec2,
    zone_recharge: &ZoneRechargeBase,
    structures: &[StructureState],
    life_support: &LifeSupportState,
    profile: &PlanetProfile,
) -> f32 {
    cout_retour_promeneur(
        position_monde,
        zone_recharge,
        structures,
        life_support,
        profile,
    )
    .unwrap_or(AIR_RETOUR_PROMENEUR_MIN)
    .max(AIR_RETOUR_PROMENEUR_MIN)
}

fn cible_promenade_est_encore_sure(
    promeneur: &AstronautePromeneur,
    position_monde: Vec2,
    zone_recharge: &ZoneRechargeBase,
    structures: &[StructureState],
    life_support: &LifeSupportState,
    profile: &PlanetProfile,
) -> bool {
    let Some(cellule_cible) = promeneur.cellule_cible else {
        return true;
    };

    let cible_monde = footprint_center(&[cellule_cible], profile.cell_size_meters);
    let cout_aller = position_monde.distance(cible_monde) * consommation_air_promeneur_par_metre();
    let cout_retour_depuis_cible = cout_retour_promeneur(
        cible_monde,
        zone_recharge,
        structures,
        life_support,
        profile,
    )
    .unwrap_or(f32::INFINITY);

    promeneur.air_combinaison
        > (cout_aller + cout_retour_depuis_cible + MARGE_SECURITE_RETOUR_PROMENEUR * 0.5)
}

fn choisir_cible_promenade(
    promeneur: &AstronautePromeneur,
    structures: &[StructureState],
    cache: &mut WorldCache,
    profile: &PlanetProfile,
    seed: WorldSeed,
) -> Option<IVec2> {
    let occupes: HashSet<_> = structures
        .iter()
        .filter(|structure| structure.built && structure.network_id == Some(0))
        .flat_map(|structure| structure.occupied_cells())
        .collect();

    if occupes.is_empty() {
        return None;
    }

    let min_x = occupes.iter().map(|cell| cell.x).min()?;
    let max_x = occupes.iter().map(|cell| cell.x).max()?;
    let min_y = occupes.iter().map(|cell| cell.y).min()?;
    let max_y = occupes.iter().map(|cell| cell.y).max()?;

    let cout_aller_retour_par_cellule =
        (profile.cell_size_meters / VITESSE_PROMENADE_METRES) * CONSOMMATION_AIR_PAR_SECONDE * 2.0;

    let distance_max_autorisee = (((promeneur.air_combinaison - MARGE_SECURITE_RETOUR_PROMENEUR)
        .max(0.0))
        / cout_aller_retour_par_cellule)
        .floor() as i32;

    let distance_max = distance_max_autorisee.clamp(DISTANCE_MIN_PROMENADE, DISTANCE_MAX_PROMENADE);

    let distance_preferee_min = (distance_max - 3).max(DISTANCE_MIN_PROMENADE);

    let mut candidates_frontiere = Vec::new();
    let mut candidates_secours = Vec::new();

    for y in (min_y - distance_max)..=(max_y + distance_max) {
        for x in (min_x - distance_max)..=(max_x + distance_max) {
            let cell = IVec2::new(x, y);
            if occupes.contains(&cell) {
                continue;
            }

            let dx = if x < min_x {
                min_x - x
            } else if x > max_x {
                x - max_x
            } else {
                0
            };
            let dy = if y < min_y {
                min_y - y
            } else if y > max_y {
                y - max_y
            } else {
                0
            };
            let distance_anneau = dx.max(dy);
            if !(DISTANCE_MIN_PROMENADE..=distance_max).contains(&distance_anneau) {
                continue;
            }

            let terrain = cache.terrain_at(cell, profile, seed);
            if !terrain_est_marchable(&terrain) {
                continue;
            }

            if distance_anneau >= distance_preferee_min {
                candidates_frontiere.push(cell);
            } else {
                candidates_secours.push(cell);
            }
        }
    }

    let pool = if !candidates_frontiere.is_empty() {
        candidates_frontiere
    } else {
        candidates_secours
    };

    pool.into_iter().min_by_key(|cell| {
        score_deterministe_cellule(seed.0, promeneur.id, promeneur.compteur_promenade, *cell)
    })
}

fn duree_pause_deterministe(seed: WorldSeed, id: AstronautId, compteur: u32) -> f32 {
    let raw = score_deterministe_cellule(seed.0, id, compteur, IVec2::new(3, 7));
    let fraction = raw as f64 / u64::MAX as f64;
    1.2 + (fraction as f32 * 1.8)
}

fn score_deterministe_cellule(seed: u64, id: AstronautId, compteur: u32, cell: IVec2) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    id.hash(&mut hasher);
    compteur.hash(&mut hasher);
    cell.hash(&mut hasher);
    hasher.finish()
}

fn lerp_angle(from: f32, to: f32, factor: f32) -> f32 {
    let mut delta = (to - from) % (PI * 2.0);
    if delta > PI {
        delta -= PI * 2.0;
    } else if delta < -PI {
        delta += PI * 2.0;
    }
    from + delta * factor
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;
    use crate::colony::{AstronautId, ColonyVisualAssets, StructureId, StructureKind};
    use crate::world::PlanetProfile;

    fn structure_test(id: u32, kind: StructureKind, anchor: IVec2) -> StructureState {
        StructureState {
            id: StructureId(id),
            kind,
            anchor,
            built: true,
            build_progress: 1.0,
            network_id: Some(0),
            internal_ice: 0.0,
        }
    }

    fn terrain_marchable(cell: IVec2) -> TerrainCell {
        TerrainCell {
            height: (cell.x + cell.y) as f32 * 0.01,
            slope: 0.15,
            constructible: true,
            resource: None,
            blocked: false,
        }
    }

    fn visuels_colonie_test() -> ColonyVisualAssets {
        ColonyVisualAssets {
            cube_mesh: Handle::default(),
            capsule_mesh: Handle::default(),
            sphere_mesh: Handle::default(),
            ice_mesh: Handle::default(),
            hull_material: Handle::default(),
            hull_secondary_material: Handle::default(),
            frame_material: Handle::default(),
            solar_material: Handle::default(),
            solar_frame_material: Handle::default(),
            glass_material: Handle::default(),
            accent_material: Handle::default(),
            storage_material: Handle::default(),
            landing_material: Handle::default(),
            lit_material: Handle::default(),
            suit_material: Handle::default(),
            suit_fabric_material: Handle::default(),
            boot_material: Handle::default(),
            visor_material: Handle::default(),
            mission_red_material: Handle::default(),
            mission_blue_material: Handle::default(),
            ice_material: Handle::default(),
        }
    }

    fn app_initialisation_astronautes() -> App {
        let mut app = App::new();
        app.insert_resource(PlanetProfile::mars());
        app.insert_resource(visuels_colonie_test());
        app.add_systems(
            Update,
            (
                initialiser_apparences_astronautes,
                initialiser_ouvriers_lisses,
                greffer_rig_ouvriers,
                greffer_rig_promeneurs,
            )
                .chain(),
        );
        app
    }

    #[test]
    fn zone_recharge_suit_la_base_et_exclut_les_cases_occupees() {
        let structures = vec![
            structure_test(1, StructureKind::Lander, IVec2::ZERO),
            structure_test(2, StructureKind::Habitat, IVec2::new(2, 0)),
        ];
        let life_support = LifeSupportState {
            primary: Some(super::super::LifeSupportNetwork {
                structures: vec![StructureId(1), StructureId(2)],
                ..default()
            }),
            ..default()
        };

        let zone_a = calculer_zone_recharge_base(&structures, &life_support, terrain_marchable);
        let zone_b = calculer_zone_recharge_base(&structures, &life_support, terrain_marchable);

        assert!(zone_a.contient(IVec2::new(-1, 0)));
        assert!(zone_a.contient(IVec2::new(4, 1)));
        assert!(!zone_a.contient(IVec2::new(0, 0)));
        assert_eq!(zone_a.cellules(), zone_b.cellules());
    }

    #[test]
    fn recharge_automatique_consomme_l_oxygene_du_reseau() {
        let structures = vec![structure_test(1, StructureKind::Lander, IVec2::ZERO)];
        let zone = ZoneRechargeBase::depuis_cellules(vec![IVec2::new(-1, 0)]);
        let mut life_support = LifeSupportState {
            primary: Some(super::super::LifeSupportNetwork {
                oxygen_stored: 50.0,
                ..default()
            }),
            ..default()
        };
        let mut astronaut = Astronaut {
            id: AstronautId(0),
            name: "Ari",
            suit_oxygen: 40.0,
            current_task: None,
            status: AstronautStatus::Idle,
            carrying_ice: 0.0,
        };
        let mut promeneur = AstronautePromeneur {
            id: AstronautId(99),
            nom: "Mila",
            air_combinaison: 60.0,
            etat: EtatPromenade::Abri,
            cellule_cible: None,
            compteur_promenade: 0,
            pause_restante: 0.0,
        };

        let ouvrier_recharge = recharger_automatiquement_en_air(
            &mut astronaut,
            IVec2::new(-1, 0),
            &structures,
            &zone,
            &mut life_support,
        );
        let promeneur_recharge = recharger_promeneur_en_air(
            &mut promeneur,
            IVec2::new(-1, 0),
            &structures,
            &zone,
            &mut life_support,
        );

        assert!((ouvrier_recharge - 2.0).abs() < 0.001);
        assert!((promeneur_recharge - 2.0).abs() < 0.001);
        assert!((astronaut.suit_oxygen - 42.0).abs() < 0.001);
        assert!((promeneur.air_combinaison - 62.0).abs() < 0.001);
        assert!((life_support.primary.as_ref().unwrap().oxygen_stored - 46.0).abs() < 0.001);
    }

    #[test]
    fn promeneur_passe_par_retour_abri_puis_abri_sans_mourir() {
        let structures = vec![structure_test(1, StructureKind::Lander, IVec2::ZERO)];
        let zone = ZoneRechargeBase::depuis_cellules(vec![IVec2::new(-1, 0), IVec2::new(0, -1)]);
        let mut life_support = LifeSupportState {
            primary: Some(super::super::LifeSupportNetwork {
                oxygen_stored: 40.0,
                structures: vec![StructureId(1)],
                ..default()
            }),
            ..default()
        };
        let mut promeneur = AstronautePromeneur {
            id: AstronautId(99),
            nom: "Mila",
            air_combinaison: 20.0,
            etat: EtatPromenade::Promenade,
            cellule_cible: None,
            compteur_promenade: 0,
            pause_restante: 0.0,
        };

        let cell_exterieure = IVec2::new(4, 0);
        if promeneur.air_combinaison < AIR_RETOUR_PROMENEUR_MIN {
            promeneur.etat = EtatPromenade::RetourAbri;
            promeneur.cellule_cible = zone.cellule_la_plus_proche(cell_exterieure);
        }
        assert_eq!(promeneur.etat, EtatPromenade::RetourAbri);
        assert!(
            [Some(IVec2::new(-1, 0)), Some(IVec2::new(0, -1))].contains(&promeneur.cellule_cible)
        );

        promeneur.etat = EtatPromenade::Abri;
        let recharge = recharger_promeneur_en_air(
            &mut promeneur,
            IVec2::new(0, -1),
            &structures,
            &zone,
            &mut life_support,
        );
        assert!(recharge > 0.0);
        assert_eq!(promeneur.etat, EtatPromenade::Abri);
        assert!(promeneur.air_combinaison > 20.0);
    }

    #[test]
    fn promeneur_reste_en_abri_si_le_reseau_est_a_sec() {
        let structures = vec![structure_test(1, StructureKind::Lander, IVec2::ZERO)];
        let zone = ZoneRechargeBase::depuis_cellules(vec![IVec2::new(-1, 0)]);
        let mut life_support = LifeSupportState {
            primary: Some(super::super::LifeSupportNetwork {
                oxygen_stored: 0.0,
                structures: vec![StructureId(1)],
                ..default()
            }),
            ..default()
        };
        let mut promeneur = AstronautePromeneur {
            id: AstronautId(99),
            nom: "Mila",
            air_combinaison: 2.0,
            etat: EtatPromenade::Abri,
            cellule_cible: None,
            compteur_promenade: 0,
            pause_restante: 0.0,
        };

        let recharge = recharger_promeneur_en_air(
            &mut promeneur,
            IVec2::new(-1, 0),
            &structures,
            &zone,
            &mut life_support,
        );
        if recharge <= 0.0 {
            promeneur.air_combinaison = promeneur.air_combinaison.max(AIR_MIN_ABRI_PROMENEUR);
        }

        assert_eq!(promeneur.etat, EtatPromenade::Abri);
        assert!((promeneur.air_combinaison - AIR_MIN_ABRI_PROMENEUR).abs() < 0.001);
    }

    #[test]
    fn cible_promenade_reste_dans_l_anneau_autour_de_la_base() {
        let mut cache = WorldCache::default();
        let profile = PlanetProfile::mars();
        let seed = WorldSeed(42);
        let structures = vec![
            structure_test(1, StructureKind::Lander, IVec2::ZERO),
            structure_test(2, StructureKind::Habitat, IVec2::new(2, 0)),
        ];
        let promeneur = AstronautePromeneur {
            id: AstronautId(99),
            nom: "Mila",
            air_combinaison: 100.0,
            etat: EtatPromenade::Promenade,
            cellule_cible: None,
            compteur_promenade: 3,
            pause_restante: 0.0,
        };

        let cible = choisir_cible_promenade(&promeneur, &structures, &mut cache, &profile, seed)
            .expect("une cible attendue");

        assert!(
            !structures
                .iter()
                .flat_map(|structure| structure.occupied_cells())
                .any(|cell| cell == cible)
        );

        let occupes: HashSet<_> = structures
            .iter()
            .flat_map(|structure| structure.occupied_cells())
            .collect();
        let min_x = occupes.iter().map(|cell| cell.x).min().unwrap();
        let max_x = occupes.iter().map(|cell| cell.x).max().unwrap();
        let min_y = occupes.iter().map(|cell| cell.y).min().unwrap();
        let max_y = occupes.iter().map(|cell| cell.y).max().unwrap();

        let dx = if cible.x < min_x {
            min_x - cible.x
        } else if cible.x > max_x {
            cible.x - max_x
        } else {
            0
        };
        let dy = if cible.y < min_y {
            min_y - cible.y
        } else if cible.y > max_y {
            cible.y - max_y
        } else {
            0
        };
        let distance_anneau = dx.max(dy);

        assert!((DISTANCE_MIN_PROMENADE..=DISTANCE_MAX_PROMENADE).contains(&distance_anneau));
    }

    #[test]
    fn un_ouvrier_cree_apres_un_premier_cycle_recoit_son_rendu() {
        let mut app = app_initialisation_astronautes();

        app.update();

        let entite = app
            .world_mut()
            .spawn((
                Astronaut {
                    id: AstronautId(7),
                    name: "Late",
                    suit_oxygen: 180.0,
                    current_task: None,
                    status: AstronautStatus::Idle,
                    carrying_ice: 0.0,
                },
                GridPosition(IVec2::new(2, -1)),
            ))
            .id();

        app.update();

        let monde = app.world();
        let astronaute = monde.entity(entite);

        assert!(astronaute.contains::<ApparenceAstronaute>());
        assert!(astronaute.contains::<PositionMondeLisse>());
        assert!(astronaute.contains::<CibleMondeLisse>());
        assert!(astronaute.contains::<AnimationOuvrier>());
        assert!(astronaute.contains::<RigAstronaute>());
        assert!(astronaute.contains::<Transform>());
        assert!(astronaute.contains::<GlobalTransform>());
        assert!(astronaute.contains::<Visibility>());
    }

    #[test]
    fn un_promeneur_cree_apres_un_premier_cycle_recoit_son_rendu() {
        let mut app = app_initialisation_astronautes();

        app.update();

        let entite = app
            .world_mut()
            .spawn((
                AstronautePromeneur {
                    id: AstronautId(10),
                    nom: "Mila",
                    air_combinaison: 180.0,
                    etat: EtatPromenade::Promenade,
                    cellule_cible: None,
                    compteur_promenade: 0,
                    pause_restante: 0.0,
                },
                PositionMonde(Vec2::new(0.0, 0.0)),
                AnimationPromenade::default(),
            ))
            .id();

        app.update();

        let monde = app.world();
        let promeneur = monde.entity(entite);

        assert!(promeneur.contains::<ApparenceAstronaute>());
        assert!(promeneur.contains::<RigAstronaute>());
        assert!(promeneur.contains::<Transform>());
        assert!(promeneur.contains::<GlobalTransform>());
        assert!(promeneur.contains::<Visibility>());
    }
}
