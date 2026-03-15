use bevy::prelude::*;

use super::rendu_realiste::{
    offset_bras_droit, offset_bras_gauche, offset_jambe_droite, offset_jambe_gauche, offset_tete,
    offset_torse,
};
use super::{
    AnimationOuvrier, AnimationPromenade, ApparenceAstronaute, Astronaut, AstronautStatus,
    AstronautePromeneur, EtatPromenade, RigAstronaute,
};

pub fn animer_rig_ouvriers(
    time: Res<Time>,
    mut transforms: Query<&mut Transform>,
    query: Query<(
        &Astronaut,
        &ApparenceAstronaute,
        &AnimationOuvrier,
        &RigAstronaute,
    )>,
) {
    let temps = time.elapsed_secs();
    for (astronaut, apparence, animation, rig) in &query {
        appliquer_pose_rig(
            temps,
            rig,
            apparence,
            animation.phase_pas,
            animation.vitesse_normalisee,
            animation.intensite_travail,
            false,
            astronaut.status == AstronautStatus::Dead,
            &mut transforms,
        );
    }
}

pub fn animer_rig_promeneurs(
    time: Res<Time>,
    mut transforms: Query<&mut Transform>,
    query: Query<(
        &AstronautePromeneur,
        &ApparenceAstronaute,
        &AnimationPromenade,
        &RigAstronaute,
    )>,
) {
    let temps = time.elapsed_secs();
    for (promeneur, apparence, animation, rig) in &query {
        appliquer_pose_rig(
            temps,
            rig,
            apparence,
            animation.phase_pas,
            animation.vitesse_normalisee,
            0.0,
            promeneur.etat == EtatPromenade::Pause,
            false,
            &mut transforms,
        );
    }
}

fn appliquer_pose_rig(
    temps: f32,
    rig: &RigAstronaute,
    apparence: &ApparenceAstronaute,
    phase_pas: f32,
    vitesse_normalisee: f32,
    intensite_travail: f32,
    est_en_pause: bool,
    est_mort: bool,
    transforms: &mut Query<&mut Transform>,
) {
    if est_mort {
        ecrire_transform(
            transforms,
            rig.torse,
            offset_torse() + Vec3::new(0.0, -0.18, 0.0),
            Quat::from_rotation_z(-0.18) * Quat::from_rotation_x(0.32),
        );
        ecrire_transform(
            transforms,
            rig.tete,
            offset_tete() + Vec3::new(0.0, -0.16, -0.06),
            Quat::from_rotation_z(0.10) * Quat::from_rotation_x(-0.34),
        );
        ecrire_transform(
            transforms,
            rig.bras_gauche,
            offset_bras_gauche() + Vec3::new(-0.04, -0.02, 0.0),
            Quat::from_rotation_z(-0.28) * Quat::from_rotation_x(-1.08),
        );
        ecrire_transform(
            transforms,
            rig.bras_droit,
            offset_bras_droit() + Vec3::new(0.05, -0.02, 0.0),
            Quat::from_rotation_z(0.30) * Quat::from_rotation_x(-0.84),
        );
        ecrire_transform(
            transforms,
            rig.jambe_gauche,
            offset_jambe_gauche(),
            Quat::from_rotation_z(-0.06) * Quat::from_rotation_x(0.24),
        );
        ecrire_transform(
            transforms,
            rig.jambe_droite,
            offset_jambe_droite(),
            Quat::from_rotation_z(0.04) * Quat::from_rotation_x(-0.10),
        );
        return;
    }

    let amplitude_combinaison = apparence.combinaison.amplitude_pas();
    let intensite_pas = vitesse_normalisee.clamp(0.0, 1.0);
    let intensite_travail = intensite_travail.clamp(0.0, 1.0);

    let balancement = phase_pas.sin() * 0.74 * intensite_pas * amplitude_combinaison;
    let rebond = (phase_pas * 2.0).sin().abs() * 0.06 * intensite_pas;
    let roulis = phase_pas.sin() * 0.09 * intensite_pas;
    let decalage_lateral = (phase_pas * 2.0).sin() * 0.018 * intensite_pas;
    let respiration = (temps * 2.1).sin() * 0.018;
    let regard = if est_en_pause {
        (temps * 1.6).sin() * 0.20
    } else {
        0.0
    };
    let oscillation_travail = (temps * 6.2).sin() * 0.24 * intensite_travail;
    let inclinaison_avant = 0.12 * intensite_pas + 0.20 * intensite_travail;
    let rotation_tete_travail = (temps * 4.2).sin() * 0.05 * intensite_travail;

    ecrire_transform(
        transforms,
        rig.torse,
        offset_torse() + Vec3::new(decalage_lateral, rebond + respiration * 0.35, 0.0),
        Quat::from_rotation_z(-roulis * 0.50)
            * Quat::from_rotation_x(-inclinaison_avant + respiration * 0.10),
    );
    ecrire_transform(
        transforms,
        rig.tete,
        offset_tete() + Vec3::new(decalage_lateral * 0.4, rebond * 0.40, 0.0),
        Quat::from_rotation_y(regard + rotation_tete_travail)
            * Quat::from_rotation_z(roulis * 0.22)
            * Quat::from_rotation_x(respiration * 0.14 + intensite_travail * 0.05),
    );
    ecrire_transform(
        transforms,
        rig.bras_gauche,
        offset_bras_gauche() + Vec3::new(0.0, rebond * 0.14, 0.0),
        Quat::from_rotation_z(-0.10 - roulis * 0.35)
            * Quat::from_rotation_x(-balancement - oscillation_travail - intensite_travail * 0.12),
    );
    ecrire_transform(
        transforms,
        rig.bras_droit,
        offset_bras_droit() + Vec3::new(0.0, rebond * 0.14, 0.0),
        Quat::from_rotation_z(0.10 - roulis * 0.35)
            * Quat::from_rotation_x(balancement + oscillation_travail - intensite_travail * 0.28),
    );
    ecrire_transform(
        transforms,
        rig.jambe_gauche,
        offset_jambe_gauche() + Vec3::new(0.0, rebond * 0.10, 0.0),
        Quat::from_rotation_z(-0.04 + roulis * 0.10)
            * Quat::from_rotation_x(balancement * 0.84 - intensite_travail * 0.04),
    );
    ecrire_transform(
        transforms,
        rig.jambe_droite,
        offset_jambe_droite() + Vec3::new(0.0, rebond * 0.10, 0.0),
        Quat::from_rotation_z(0.04 + roulis * 0.10)
            * Quat::from_rotation_x(-balancement * 0.84 + intensite_travail * 0.04),
    );
}

fn ecrire_transform(
    transforms: &mut Query<&mut Transform>,
    entity: Entity,
    translation: Vec3,
    rotation: Quat,
) {
    if let Ok(mut transform) = transforms.get_mut(entity) {
        transform.translation = translation;
        transform.rotation = rotation;
    }
}
