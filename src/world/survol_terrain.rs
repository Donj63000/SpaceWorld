use bevy::prelude::*;

use crate::core::{MainCamera, ParametresPerformanceJeu, StatistiquesPerformance, WorldOrigin};

use super::coordonnees::{sample_height_at_world, world_to_cell};
use super::donnees::HoveredCell;
use super::performance_monde::EtatSurvolTerrain;
use super::{PlanetProfile, WorldSeed};

pub(crate) fn update_hovered_cell(
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    window: Single<&Window>,
    time: Res<Time>,
    origin: Res<WorldOrigin>,
    profile: Res<PlanetProfile>,
    seed: Res<WorldSeed>,
    perf: Res<ParametresPerformanceJeu>,
    mut etat: ResMut<EtatSurvolTerrain>,
    mut hovered: ResMut<HoveredCell>,
    mut stats: ResMut<StatistiquesPerformance>,
) {
    let (camera, camera_transform) = *camera_query;
    let (_, rotation, translation) = camera_transform.to_scale_rotation_translation();
    let cursor_position = window.cursor_position();

    if cursor_position.is_none() {
        hovered.0 = None;
        memoriser_etat_survol(
            &mut etat,
            cursor_position,
            translation,
            rotation,
            origin.0,
            time.elapsed_secs_f64(),
            perf.frequence_survol_hz,
        );
        return;
    }

    if !survol_doit_etre_recalcule(
        &etat,
        cursor_position,
        translation,
        rotation,
        origin.0,
        time.elapsed_secs_f64(),
    ) {
        return;
    }

    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position.unwrap()) else {
        hovered.0 = None;
        memoriser_etat_survol(
            &mut etat,
            cursor_position,
            translation,
            rotation,
            origin.0,
            time.elapsed_secs_f64(),
            perf.frequence_survol_hz,
        );
        return;
    };

    let world_ray_origin = ray.origin + Vec3::new(origin.0.x, 0.0, origin.0.y);
    let ray_direction = *ray.direction;

    let point =
        raycast_terrain_point(world_ray_origin, ray_direction, &profile, *seed).or_else(|| {
            ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
                .map(|point| Vec3::new(point.x + origin.0.x, point.y, point.z + origin.0.y))
        });

    hovered.0 =
        point.map(|point| world_to_cell(Vec2::new(point.x, point.z), profile.cell_size_meters));
    stats.survols_recalcules_frame += 1;
    memoriser_etat_survol(
        &mut etat,
        cursor_position,
        translation,
        rotation,
        origin.0,
        time.elapsed_secs_f64(),
        perf.frequence_survol_hz,
    );
}

fn memoriser_etat_survol(
    etat: &mut EtatSurvolTerrain,
    cursor_position: Option<Vec2>,
    translation: Vec3,
    rotation: Quat,
    origine: Vec2,
    temps: f64,
    frequence_hz: f64,
) {
    etat.dernier_curseur = cursor_position;
    etat.derniere_camera_translation = Some(translation);
    etat.derniere_camera_rotation = Some(rotation);
    etat.derniere_origine = origine;
    etat.prochain_raycast_a = temps + 1.0 / frequence_hz.max(1.0);
}

fn survol_doit_etre_recalcule(
    etat: &EtatSurvolTerrain,
    cursor_position: Option<Vec2>,
    translation_camera: Vec3,
    rotation_camera: Quat,
    origine: Vec2,
    temps: f64,
) -> bool {
    let cursor_a_change = etat.dernier_curseur != cursor_position;
    let camera_a_change = etat.derniere_camera_translation != Some(translation_camera)
        || etat.derniere_camera_rotation != Some(rotation_camera);
    let origine_a_change = etat.derniere_origine != origine;
    let presence_curseur_a_change = etat.dernier_curseur.is_some() != cursor_position.is_some();
    let premier_echantillon =
        etat.derniere_camera_translation.is_none() || etat.derniere_camera_rotation.is_none();

    if cursor_position.is_some() && (presence_curseur_a_change || premier_echantillon) {
        return true;
    }

    (cursor_a_change || camera_a_change || origine_a_change) && temps >= etat.prochain_raycast_a
}

fn raycast_terrain_point(
    ray_origin_world: Vec3,
    ray_direction: Vec3,
    profile: &PlanetProfile,
    seed: WorldSeed,
) -> Option<Vec3> {
    if ray_direction.y >= -f32::EPSILON {
        return None;
    }

    let distance_max = ((ray_origin_world.y + 64.0) / -ray_direction.y)
        .max(profile.chunk_span_meters())
        .max(profile.cell_size_meters);
    let pas = (profile.cell_size_meters * 0.35).max(0.25);

    let mut distance_precedente = 0.0;
    let mut hauteur_precedente = distance_to_terrain(
        ray_origin_world,
        ray_direction,
        distance_precedente,
        profile,
        seed,
    );
    if hauteur_precedente <= 0.0 {
        return Some(ray_origin_world);
    }

    let mut distance = pas;
    while distance <= distance_max {
        let hauteur = distance_to_terrain(ray_origin_world, ray_direction, distance, profile, seed);
        if hauteur <= 0.0 {
            let mut bas = distance_precedente;
            let mut haut = distance;
            for _ in 0..8 {
                let milieu = (bas + haut) * 0.5;
                if distance_to_terrain(ray_origin_world, ray_direction, milieu, profile, seed) > 0.0
                {
                    bas = milieu;
                } else {
                    haut = milieu;
                }
            }
            return Some(ray_origin_world + ray_direction * haut);
        }

        distance_precedente = distance;
        hauteur_precedente = hauteur;
        distance += pas;
    }

    let _ = hauteur_precedente;
    None
}

#[inline]
fn distance_to_terrain(
    ray_origin_world: Vec3,
    ray_direction: Vec3,
    distance: f32,
    profile: &PlanetProfile,
    seed: WorldSeed,
) -> f32 {
    let point = ray_origin_world + ray_direction * distance;
    point.y - sample_height_at_world(Vec2::new(point.x, point.z), profile, seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_slanted_ray_case(profile: &PlanetProfile, seed: WorldSeed) -> (Vec2, Vec3, Vec3) {
        let direction = Vec3::new(0.72, -0.66, 0.21).normalize();

        for cell_y in -24..24 {
            for cell_x in -24..24 {
                for offset in [
                    Vec2::new(0.15, 0.15),
                    Vec2::new(0.85, 0.15),
                    Vec2::new(0.15, 0.85),
                    Vec2::new(0.85, 0.85),
                ] {
                    let world = Vec2::new(
                        (cell_x as f32 + offset.x) * profile.cell_size_meters,
                        (cell_y as f32 + offset.y) * profile.cell_size_meters,
                    );
                    let height = sample_height_at_world(world, profile, seed);
                    if height.abs() < 0.7 {
                        continue;
                    }

                    let target = Vec3::new(world.x, height, world.y);
                    let origin = target - direction * 18.0;
                    let plane_hit =
                        Ray3d::new(origin, Dir3::new(direction).expect("direction valide"))
                            .plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
                            .map(|point| {
                                world_to_cell(Vec2::new(point.x, point.z), profile.cell_size_meters)
                            });

                    let target_cell = world_to_cell(world, profile.cell_size_meters);
                    if plane_hit != Some(target_cell) {
                        return (world, origin, direction);
                    }
                }
            }
        }

        panic!("aucun cas de rayon oblique distinct du plan y=0 n'a ete trouve");
    }

    #[test]
    fn le_raycast_terrain_retrouve_la_cellule_sur_un_relief_non_plat() {
        let profile = PlanetProfile::mars();
        let seed = WorldSeed(42);
        let (world, origin, direction) = find_slanted_ray_case(&profile, seed);

        let hit = raycast_terrain_point(origin, direction, &profile, seed)
            .map(|point| world_to_cell(Vec2::new(point.x, point.z), profile.cell_size_meters));

        assert_eq!(hit, Some(world_to_cell(world, profile.cell_size_meters)));
    }

    #[test]
    fn le_cache_de_survol_ignore_un_nouveau_tick_sans_changement() {
        let etat = EtatSurvolTerrain {
            dernier_curseur: Some(Vec2::new(100.0, 80.0)),
            derniere_camera_translation: Some(Vec3::new(1.0, 2.0, 3.0)),
            derniere_camera_rotation: Some(Quat::IDENTITY),
            derniere_origine: Vec2::new(4.0, -2.0),
            prochain_raycast_a: 10.0,
        };

        assert!(!survol_doit_etre_recalcule(
            &etat,
            Some(Vec2::new(100.0, 80.0)),
            Vec3::new(1.0, 2.0, 3.0),
            Quat::IDENTITY,
            Vec2::new(4.0, -2.0),
            12.0,
        ));
    }

    #[test]
    fn le_survol_est_recalcule_des_que_le_curseur_revient() {
        let etat = EtatSurvolTerrain {
            dernier_curseur: None,
            derniere_camera_translation: Some(Vec3::new(1.0, 2.0, 3.0)),
            derniere_camera_rotation: Some(Quat::IDENTITY),
            derniere_origine: Vec2::new(4.0, -2.0),
            prochain_raycast_a: 10.0,
        };

        assert!(survol_doit_etre_recalcule(
            &etat,
            Some(Vec2::new(100.0, 80.0)),
            Vec3::new(1.0, 2.0, 3.0),
            Quat::IDENTITY,
            Vec2::new(4.0, -2.0),
            5.0,
        ));
    }

    #[test]
    fn le_premier_survol_ne_subit_pas_la_cadence() {
        let etat = EtatSurvolTerrain::default();

        assert!(survol_doit_etre_recalcule(
            &etat,
            Some(Vec2::new(12.0, 18.0)),
            Vec3::new(1.0, 2.0, 3.0),
            Quat::IDENTITY,
            Vec2::ZERO,
            0.0,
        ));
    }
}
