use std::collections::{HashMap, HashSet, VecDeque};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::core::{
    CameraController, ParametresPerformanceJeu, StatistiquesPerformance, WorldOrigin,
};

use super::coordonnees::{chunk_origin_translation, world_to_chunk_coord};
use super::decorations::{
    DecorationMaterial, build_chunk_decoration_mesh, compter_decors_visibles,
};
use super::donnees::{ActiveChunks, ChunkCoord, ChunkState, WorldCache};
use super::maillage::build_chunk_mesh;
use super::performance_monde::{FileAttenteStreamingChunks, coordonnees_chunks_requises};
use super::{PlanetProfile, WorldSeed};

#[derive(Resource, Default)]
pub(crate) struct ChunkVisuals {
    entries: HashMap<ChunkCoord, ChunkVisualEntry>,
}

#[derive(Default)]
struct ChunkVisualEntry {
    terrain_entity: Option<Entity>,
    decoration_entities: Vec<Entity>,
    decors_initialises: bool,
}

#[derive(Component)]
pub(crate) struct ChunkMesh {
    coord: ChunkCoord,
}

#[derive(Component)]
pub(crate) struct ChunkDecoration {
    coord: ChunkCoord,
}

#[derive(Resource)]
struct WorldVisualAssets {
    terrain_material: Handle<StandardMaterial>,
    rock_dark_material: Handle<StandardMaterial>,
    rock_dust_material: Handle<StandardMaterial>,
    rock_salt_material: Handle<StandardMaterial>,
}

#[derive(SystemParam)]
pub(crate) struct ContextePlanificationStreaming<'w, 's> {
    commands: Commands<'w, 's>,
    controller: Res<'w, CameraController>,
    profile: Res<'w, PlanetProfile>,
    perf: Res<'w, ParametresPerformanceJeu>,
    active_chunks: ResMut<'w, ActiveChunks>,
    visuals: ResMut<'w, ChunkVisuals>,
    file: ResMut<'w, FileAttenteStreamingChunks>,
}

#[derive(SystemParam)]
pub(crate) struct ContexteTraitementStreaming<'w, 's> {
    commands: Commands<'w, 's>,
    origin: Res<'w, WorldOrigin>,
    profile: Res<'w, PlanetProfile>,
    seed: Res<'w, WorldSeed>,
    perf: Res<'w, ParametresPerformanceJeu>,
    cache: ResMut<'w, WorldCache>,
    file: ResMut<'w, FileAttenteStreamingChunks>,
    visuals: ResMut<'w, ChunkVisuals>,
    visual_assets: Res<'w, WorldVisualAssets>,
    meshes: ResMut<'w, Assets<Mesh>>,
    stats: ResMut<'w, StatistiquesPerformance>,
}

pub(crate) fn setup_world_visuals(
    mut commands: Commands,
    profile: Res<PlanetProfile>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(WorldVisualAssets {
        terrain_material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 1.0, 1.0),
            perceptual_roughness: 0.97,
            reflectance: 0.08,
            metallic: 0.0,
            ..default()
        }),
        rock_dark_material: materials.add(StandardMaterial {
            base_color: blend_color(
                profile.palette.rock_dark,
                profile.palette.basalt_exposed,
                0.24,
            ),
            perceptual_roughness: 0.99,
            reflectance: 0.06,
            metallic: 0.0,
            ..default()
        }),
        rock_dust_material: materials.add(StandardMaterial {
            base_color: blend_color(profile.palette.ground_mid, profile.palette.dust_light, 0.68),
            perceptual_roughness: 0.98,
            reflectance: 0.09,
            metallic: 0.0,
            ..default()
        }),
        rock_salt_material: materials.add(StandardMaterial {
            base_color: blend_color(profile.palette.salt_light, profile.palette.dust_light, 0.18),
            perceptual_roughness: 0.92,
            reflectance: 0.16,
            metallic: 0.0,
            ..default()
        }),
    });
}

pub(crate) fn recenter_origin(
    profile: Res<PlanetProfile>,
    controller: Res<CameraController>,
    mut origin: ResMut<WorldOrigin>,
) {
    if (controller.focus_world - origin.0).length() < profile.origin_recenter_distance {
        return;
    }

    let chunk_span = profile.chunk_span_meters();
    origin.0 = Vec2::new(
        (controller.focus_world.x / chunk_span).round() * chunk_span,
        (controller.focus_world.y / chunk_span).round() * chunk_span,
    );
}

pub(crate) fn planifier_streaming_chunks(mut contexte: ContextePlanificationStreaming) {
    let centre = world_to_chunk_coord(contexte.controller.focus_world, &contexte.profile);
    let required_coords = coordonnees_chunks_requises(centre, contexte.perf.rayon_chunks_actifs);
    let required_set: HashSet<ChunkCoord> = required_coords.iter().copied().collect();

    for coord in contexte
        .active_chunks
        .set
        .iter()
        .copied()
        .collect::<Vec<_>>()
    {
        if !required_set.contains(&coord)
            && let Some(entry) = contexte.visuals.entries.remove(&coord)
        {
            if let Some(entity) = entry.terrain_entity {
                contexte.commands.entity(entity).despawn();
            }
            for entity in entry.decoration_entities {
                contexte.commands.entity(entity).despawn();
            }
        }
    }

    contexte.file.centre_planifie = Some(centre);
    contexte.file.coords_requis = required_coords.clone();
    contexte.file.ensemble_requis = required_set.clone();
    contexte.file.terrains = reconstruire_file_terrains_requise(&required_coords, &contexte.visuals);
    contexte.file.decors = reconstruire_file_decors_requise(&required_coords, &contexte.visuals);

    contexte.active_chunks.center = Some(centre);
    contexte.active_chunks.coords = required_coords;
    contexte.active_chunks.set = required_set;
}

fn reconstruire_file_terrains_requise(
    required_coords: &[ChunkCoord],
    visuals: &ChunkVisuals,
) -> VecDeque<ChunkCoord> {
    required_coords
        .iter()
        .copied()
        .filter(|coord| {
            visuals
                .entries
                .get(coord)
                .and_then(|entry| entry.terrain_entity)
                .is_none()
        })
        .collect()
}

fn reconstruire_file_decors_requise(
    required_coords: &[ChunkCoord],
    visuals: &ChunkVisuals,
) -> VecDeque<ChunkCoord> {
    required_coords
        .iter()
        .copied()
        .filter(|coord| {
            visuals
                .entries
                .get(coord)
                .map(|entry| entry.terrain_entity.is_some() && !entry.decors_initialises)
                .unwrap_or(false)
        })
        .collect()
}

pub(crate) fn traiter_streaming_chunks(mut contexte: ContexteTraitementStreaming) {
    for coord in prelever_budget(
        &mut contexte.file.terrains,
        contexte.perf.budget_chunks_terrain_par_frame,
    ) {
        if !contexte.file.ensemble_requis.contains(&coord) {
            continue;
        }

        let chunk_absent = contexte.cache.chunk(coord).is_none();
        contexte
            .cache
            .ensure_chunk(coord, &contexte.profile, *contexte.seed);
        if chunk_absent {
            contexte.stats.chunks_generes_frame += 1;
        }

        let terrain_deja_present = contexte
            .visuals
            .entries
            .get(&coord)
            .and_then(|entry| entry.terrain_entity)
            .is_some();
        if terrain_deja_present {
            pousser_decors_si_necessaire(&mut contexte.file, &contexte.visuals, coord);
            continue;
        }

        let terrain_mesh = {
            let cache = &*contexte.cache;
            let chunk = cache
                .chunk(coord)
                .expect("le chunk doit exister apres ensure_chunk");
            build_chunk_mesh(
                chunk,
                cache,
                &contexte.profile,
                contexte.perf.subdivisions_maillage_terrain,
            )
        };

        let terrain_entity = contexte
            .commands
            .spawn((
                Mesh3d(contexte.meshes.add(terrain_mesh)),
                MeshMaterial3d(contexte.visual_assets.terrain_material.clone()),
                Transform::from_translation(chunk_origin_translation(
                    coord,
                    &contexte.profile,
                    &contexte.origin,
                )),
                ChunkMesh { coord },
            ))
            .id();

        let entry = contexte.visuals.entries.entry(coord).or_default();
        entry.terrain_entity = Some(terrain_entity);
        contexte.stats.chunks_mailles_frame += 1;
        pousser_decors_si_necessaire(&mut contexte.file, &contexte.visuals, coord);
    }

    for coord in prelever_budget(
        &mut contexte.file.decors,
        contexte.perf.budget_batches_decors_par_frame,
    ) {
        if !contexte.file.ensemble_requis.contains(&coord) {
            continue;
        }

        let decors_deja_initialises = contexte
            .visuals
            .entries
            .get(&coord)
            .map(|entry| entry.decors_initialises)
            .unwrap_or(false);
        if decors_deja_initialises {
            continue;
        }

        let (decoration_entities, nombre_decors) = {
            let cache = &*contexte.cache;
            let Some(chunk) = cache.chunk(coord) else {
                continue;
            };
            spawn_chunk_decorations(
                &mut contexte.commands,
                &mut contexte.meshes,
                chunk,
                &contexte.profile,
                &contexte.origin,
                &contexte.visual_assets,
                contexte.perf.densite_decors,
            )
        };

        let entry = contexte.visuals.entries.entry(coord).or_default();
        entry.decoration_entities = decoration_entities;
        entry.decors_initialises = true;
        contexte.stats.decors_spawnes_frame += nombre_decors as u32;
    }
}

fn pousser_decors_si_necessaire(
    file: &mut FileAttenteStreamingChunks,
    visuals: &ChunkVisuals,
    coord: ChunkCoord,
) {
    let decors_initialises = visuals
        .entries
        .get(&coord)
        .map(|entry| entry.decors_initialises)
        .unwrap_or(false);
    if !decors_initialises && !file.decors.contains(&coord) {
        file.decors.push_back(coord);
    }
}

fn prelever_budget(
    file: &mut std::collections::VecDeque<ChunkCoord>,
    budget: usize,
) -> Vec<ChunkCoord> {
    let mut coords = Vec::new();
    for _ in 0..budget {
        let Some(coord) = file.pop_front() else {
            break;
        };
        coords.push(coord);
    }
    coords
}

fn spawn_chunk_decorations(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    chunk: &ChunkState,
    profile: &PlanetProfile,
    origin: &WorldOrigin,
    assets: &WorldVisualAssets,
    densite_decors: f32,
) -> (Vec<Entity>, usize) {
    let mut entities = Vec::new();
    let mut nombre_decors = 0;

    for material_kind in [
        DecorationMaterial::Dark,
        DecorationMaterial::Dust,
        DecorationMaterial::Salt,
    ] {
        let Some(mesh) =
            build_chunk_decoration_mesh(chunk.decoration_specs(), material_kind, densite_decors)
        else {
            continue;
        };

        let material = match material_kind {
            DecorationMaterial::Dark => assets.rock_dark_material.clone(),
            DecorationMaterial::Dust => assets.rock_dust_material.clone(),
            DecorationMaterial::Salt => assets.rock_salt_material.clone(),
        };

        let entity = commands
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::from_translation(chunk_origin_translation(chunk.coord, profile, origin)),
                ChunkDecoration { coord: chunk.coord },
            ))
            .id();
        entities.push(entity);
        nombre_decors +=
            compter_decors_visibles(chunk.decoration_specs(), material_kind, densite_decors);
    }

    (entities, nombre_decors)
}

pub(crate) fn sync_chunk_transforms(
    origin: Res<WorldOrigin>,
    profile: Res<PlanetProfile>,
    mut query: Query<(&ChunkMesh, &mut Transform)>,
) {
    if !origin.is_changed() {
        return;
    }

    for (chunk, mut transform) in &mut query {
        transform.translation = chunk_origin_translation(chunk.coord, &profile, &origin);
    }
}

pub(crate) fn sync_chunk_decorations(
    origin: Res<WorldOrigin>,
    profile: Res<PlanetProfile>,
    mut query: Query<(&ChunkDecoration, &mut Transform)>,
) {
    if !origin.is_changed() {
        return;
    }

    for (chunk, mut transform) in &mut query {
        transform.translation = chunk_origin_translation(chunk.coord, &profile, &origin);
    }
}

fn blend_color(left: Color, right: Color, factor: f32) -> Color {
    let factor = factor.clamp(0.0, 1.0);
    let left = left.to_srgba();
    let right = right.to_srgba();
    Color::srgba(
        left.red + (right.red - left.red) * factor,
        left.green + (right.green - left.green) * factor,
        left.blue + (right.blue - left.blue) * factor,
        left.alpha + (right.alpha - left.alpha) * factor,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_budget_de_streaming_preleve_un_chunk_par_frame() {
        let mut file = VecDeque::from(vec![
            ChunkCoord { x: 0, y: 0 },
            ChunkCoord { x: 1, y: 0 },
            ChunkCoord { x: 2, y: 0 },
        ]);

        let premier = prelever_budget(&mut file, 1);
        let second = prelever_budget(&mut file, 1);

        assert_eq!(premier, vec![ChunkCoord { x: 0, y: 0 }]);
        assert_eq!(second, vec![ChunkCoord { x: 1, y: 0 }]);
        assert_eq!(file, VecDeque::from(vec![ChunkCoord { x: 2, y: 0 }]));
    }

    #[test]
    fn la_file_des_terrains_reste_complete_tant_que_des_chunks_manquent() {
        let coords = vec![
            ChunkCoord { x: 0, y: 0 },
            ChunkCoord { x: 1, y: 0 },
            ChunkCoord { x: 0, y: 1 },
        ];
        let mut visuals = ChunkVisuals::default();
        visuals.entries.insert(
            ChunkCoord { x: 0, y: 0 },
            ChunkVisualEntry {
                terrain_entity: Some(Entity::from_bits(1)),
                ..default()
            },
        );

        let file = reconstruire_file_terrains_requise(&coords, &visuals);
        assert_eq!(
            file,
            VecDeque::from(vec![ChunkCoord { x: 1, y: 0 }, ChunkCoord { x: 0, y: 1 }])
        );
    }

    #[test]
    fn la_file_des_decors_ne_contient_que_les_chunks_terrain_deja_charges() {
        let coords = vec![
            ChunkCoord { x: 0, y: 0 },
            ChunkCoord { x: 1, y: 0 },
            ChunkCoord { x: 0, y: 1 },
        ];
        let mut visuals = ChunkVisuals::default();
        visuals.entries.insert(
            ChunkCoord { x: 0, y: 0 },
            ChunkVisualEntry {
                terrain_entity: Some(Entity::from_bits(1)),
                decors_initialises: true,
                ..default()
            },
        );
        visuals.entries.insert(
            ChunkCoord { x: 1, y: 0 },
            ChunkVisualEntry {
                terrain_entity: Some(Entity::from_bits(2)),
                decors_initialises: false,
                ..default()
            },
        );
        visuals.entries.insert(
            ChunkCoord { x: 0, y: 1 },
            ChunkVisualEntry {
                terrain_entity: None,
                decors_initialises: false,
                ..default()
            },
        );

        let file = reconstruire_file_decors_requise(&coords, &visuals);
        assert_eq!(file, VecDeque::from(vec![ChunkCoord { x: 1, y: 0 }]));
    }
}
