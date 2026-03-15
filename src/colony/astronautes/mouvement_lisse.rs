use std::f32::consts::PI;

use bevy::prelude::*;

use super::{Astronaut, AstronautStatus, GridPosition};
use crate::core::SIMULATION_HZ;
use crate::world::{PlanetProfile, footprint_center};

const MULTIPLICATEUR_VITESSE_MARCHE: f32 = 1.15;
const MULTIPLICATEUR_VITESSE_RETOUR: f32 = 1.30;
const LISSAGE_ORIENTATION_OUVRIER: f32 = 9.0;

#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PositionMondeLisse(pub Vec2);

#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct CibleMondeLisse(pub Vec2);

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AnimationOuvrier {
    pub phase_pas: f32,
    pub orientation: f32,
    pub vitesse_normalisee: f32,
    pub intensite_travail: f32,
}

pub fn initialiser_ouvriers_lisses(
    mut commands: Commands,
    profile: Res<PlanetProfile>,
    query: Query<(Entity, &GridPosition), (With<Astronaut>, Without<PositionMondeLisse>)>,
) {
    for (entity, position) in &query {
        let monde = footprint_center(&[position.0], profile.cell_size_meters);
        commands.entity(entity).insert((
            PositionMondeLisse(monde),
            CibleMondeLisse(monde),
            AnimationOuvrier::default(),
        ));
    }
}

pub fn mettre_a_jour_cibles_ouvriers(
    profile: Res<PlanetProfile>,
    mut query: Query<(&GridPosition, Ref<GridPosition>, &mut CibleMondeLisse), With<Astronaut>>,
) {
    for (position, position_ref, mut cible) in &mut query {
        if !position_ref.is_changed() {
            continue;
        }

        cible.0 = footprint_center(&[position.0], profile.cell_size_meters);
    }
}

pub fn interpoler_ouvriers(
    profile: Res<PlanetProfile>,
    time: Res<Time>,
    mut query: Query<(
        &Astronaut,
        &mut PositionMondeLisse,
        &CibleMondeLisse,
        &mut AnimationOuvrier,
    )>,
) {
    let delta = time.delta_secs();
    if delta <= 0.0 {
        return;
    }

    for (astronaut, mut position, cible, mut animation) in &mut query {
        let delta_cible = cible.0 - position.0;
        let distance = delta_cible.length();

        if astronaut.status == AstronautStatus::Dead {
            animation.vitesse_normalisee = 0.0;
            animation.intensite_travail = 0.0;
            animation.phase_pas += delta * 0.4;
            continue;
        }

        if distance <= 0.0001 {
            position.0 = cible.0;
            animation.vitesse_normalisee = 0.0;
            animation.intensite_travail = if astronaut.status == AstronautStatus::Working {
                1.0
            } else {
                0.0
            };
            animation.phase_pas += delta
                * if animation.intensite_travail > 0.0 {
                    4.3
                } else {
                    1.2
                };
            continue;
        }

        let vitesse_max = vitesse_lissage_ouvrier(astronaut.status, &profile);
        if vitesse_max <= 0.0 {
            animation.vitesse_normalisee = 0.0;
            animation.intensite_travail = 0.0;
            animation.phase_pas += delta * 1.2;
            continue;
        }

        let direction = delta_cible.normalize();
        let pas = vitesse_max * delta;
        let mouvement = direction * pas.min(distance);
        position.0 += mouvement;

        let orientation_cible = direction.x.atan2(direction.y);
        animation.orientation = lerp_angle(
            animation.orientation,
            orientation_cible,
            (delta * LISSAGE_ORIENTATION_OUVRIER).clamp(0.0, 1.0),
        );
        animation.vitesse_normalisee =
            (mouvement.length() / (vitesse_max * delta.max(0.001))).clamp(0.0, 1.0);
        animation.intensite_travail = 0.0;
        animation.phase_pas += delta * (3.0 + 5.4 * animation.vitesse_normalisee);
    }
}

fn vitesse_lissage_ouvrier(status: AstronautStatus, profile: &PlanetProfile) -> f32 {
    // Le rendu doit pouvoir rattraper au moins un pas logique de grille par tick fixe,
    // sinon l'ouvrier accumule du retard puis finit par "sauter" a l'arrivee.
    let vitesse_base = profile.cell_size_meters * SIMULATION_HZ as f32;
    match status {
        AstronautStatus::Idle | AstronautStatus::Moving | AstronautStatus::Working => {
            vitesse_base * MULTIPLICATEUR_VITESSE_MARCHE
        }
        AstronautStatus::Returning => vitesse_base * MULTIPLICATEUR_VITESSE_RETOUR,
        AstronautStatus::Dead => 0.0,
    }
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
    use super::*;
    use std::time::Duration;

    use crate::colony::AstronautId;

    fn astronaute_test(statut: AstronautStatus) -> Astronaut {
        Astronaut {
            id: AstronautId(1),
            name: "Test",
            suit_oxygen: 100.0,
            current_task: None,
            status: statut,
            carrying_ice: 0.0,
        }
    }

    #[test]
    fn un_ouvrier_en_travail_termine_son_approche_sans_snap() {
        let mut monde = World::default();
        let mut temps = Time::<()>::default();
        temps.advance_by(Duration::from_millis(100));

        monde.insert_resource(temps);
        monde.insert_resource(PlanetProfile::mars());

        let entite = monde
            .spawn((
                astronaute_test(AstronautStatus::Working),
                PositionMondeLisse(Vec2::ZERO),
                CibleMondeLisse(Vec2::new(2.0, 0.0)),
                AnimationOuvrier::default(),
            ))
            .id();

        let mut systeme = IntoSystem::into_system(interpoler_ouvriers);
        systeme.initialize(&mut monde);
        let _ = systeme.run((), &mut monde);
        systeme.apply_deferred(&mut monde);

        let entity_ref = monde.entity(entite);
        let position = entity_ref
            .get::<PositionMondeLisse>()
            .copied()
            .expect("position lisse manquante");
        let animation = entity_ref
            .get::<AnimationOuvrier>()
            .copied()
            .expect("animation ouvrier manquante");

        assert!(position.0.x > 0.0);
        assert!(position.0.x < 2.0);
        assert_eq!(animation.intensite_travail, 0.0);
    }

    #[test]
    fn la_vitesse_lissee_peut_rattraper_un_pas_logique_par_tick() {
        let profile = PlanetProfile::mars();
        let vitesse = vitesse_lissage_ouvrier(AstronautStatus::Moving, &profile);
        let distance_par_tick = profile.cell_size_meters * (SIMULATION_HZ as f32).recip();

        assert!(vitesse * (SIMULATION_HZ as f32).recip() >= distance_par_tick);
    }
}
