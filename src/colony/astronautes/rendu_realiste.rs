use bevy::ecs::prelude::ChildSpawnerCommands;
use bevy::prelude::*;

use super::super::ColonyVisualAssets;
use super::{
    AnimationOuvrier, AnimationPromenade, ApparenceAstronaute, Astronaut, AstronautStatus,
    AstronautePromeneur, PositionMonde, PositionMondeLisse, RoleAstronaute, TypeCombinaison,
};
use crate::core::WorldOrigin;
use crate::world::{PlanetProfile, WorldCache, WorldSeed, continuous_world_to_render_translation};

#[derive(Component, Clone, Copy, Debug)]
pub struct RigAstronaute {
    pub torse: Entity,
    pub tete: Entity,
    pub bras_gauche: Entity,
    pub bras_droit: Entity,
    pub jambe_gauche: Entity,
    pub jambe_droite: Entity,
}

#[derive(Clone, Copy, Debug)]
struct GabaritCombinaison {
    coque_torse: Vec3,
    plastron: Vec3,
    bassin: Vec3,
    casque: Vec3,
    visiere: Vec3,
    collier: Vec3,
    sac_dorsal: Vec3,
    reserve_dorsale: Vec3,
    bras_haut: Vec3,
    bras_bas: Vec3,
    cuisse: Vec3,
    mollet: Vec3,
    botte: Vec3,
    semelle: Vec3,
    hauteur_epaule: f32,
    hauteur_hanche: f32,
    epaules_x: f32,
    hanches_x: f32,
    epaisseur_articulation: f32,
}

#[derive(Clone, Copy, Debug)]
struct SignatureRole {
    antenne_sac: bool,
    capteur_casque: bool,
    sacoche_ceinture: bool,
    double_sacoche: bool,
    boitier_bras: bool,
}

pub(crate) fn offset_torse() -> Vec3 {
    Vec3::new(0.0, 1.02, -0.04)
}

pub(crate) fn offset_tete() -> Vec3 {
    Vec3::new(0.0, 1.70, 0.06)
}

pub(crate) fn offset_bras_gauche() -> Vec3 {
    Vec3::new(-0.42, 1.34, 0.0)
}

pub(crate) fn offset_bras_droit() -> Vec3 {
    Vec3::new(0.42, 1.34, 0.0)
}

pub(crate) fn offset_jambe_gauche() -> Vec3 {
    Vec3::new(-0.18, 0.82, 0.0)
}

pub(crate) fn offset_jambe_droite() -> Vec3 {
    Vec3::new(0.18, 0.82, 0.0)
}

pub fn greffer_rig_ouvriers(
    mut commands: Commands,
    visuals: Res<ColonyVisualAssets>,
    query: Query<(Entity, &ApparenceAstronaute), (With<Astronaut>, Without<RigAstronaute>)>,
) {
    for (entity, apparence) in &query {
        commands.entity(entity).insert((
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
        ));
        let rig = attacher_rig_astronaute(entity, &mut commands, &visuals, apparence);
        commands.entity(entity).insert(rig);
    }
}

pub fn greffer_rig_promeneurs(
    mut commands: Commands,
    visuals: Res<ColonyVisualAssets>,
    query: Query<
        (Entity, &ApparenceAstronaute),
        (With<AstronautePromeneur>, Without<RigAstronaute>),
    >,
) {
    for (entity, apparence) in &query {
        commands.entity(entity).insert((
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
        ));
        let rig = attacher_rig_astronaute(entity, &mut commands, &visuals, apparence);
        commands.entity(entity).insert(rig);
    }
}

pub fn synchroniser_rendu_ouvriers_lisse(
    mut cache: ResMut<WorldCache>,
    profile: Res<PlanetProfile>,
    seed: Res<WorldSeed>,
    origin: Res<WorldOrigin>,
    mut query: Query<(
        &Astronaut,
        &ApparenceAstronaute,
        &PositionMondeLisse,
        &AnimationOuvrier,
        &mut Transform,
    )>,
) {
    for (astronaut, apparence, position, animation, mut transform) in &mut query {
        transform.translation = continuous_world_to_render_translation(
            position.0, 0.06, &mut cache, &profile, *seed, &origin,
        );
        if astronaut.status == AstronautStatus::Dead {
            transform.translation.y -= 0.26;
            transform.rotation =
                Quat::from_rotation_y(animation.orientation) * Quat::from_rotation_z(-0.92);
        } else {
            transform.rotation = Quat::from_rotation_y(animation.orientation);
        }
        transform.scale = Vec3::splat(apparence.echelle);
    }
}

pub fn synchroniser_rendu_promeneurs_realistes(
    mut cache: ResMut<WorldCache>,
    profile: Res<PlanetProfile>,
    seed: Res<WorldSeed>,
    origin: Res<WorldOrigin>,
    mut query: Query<
        (
            &ApparenceAstronaute,
            &PositionMonde,
            &AnimationPromenade,
            &mut Transform,
        ),
        With<AstronautePromeneur>,
    >,
) {
    for (apparence, position, animation, mut transform) in &mut query {
        transform.translation = continuous_world_to_render_translation(
            position.0, 0.06, &mut cache, &profile, *seed, &origin,
        );
        transform.rotation = Quat::from_rotation_y(animation.orientation);
        transform.scale = Vec3::splat(apparence.echelle);
    }
}

fn attacher_rig_astronaute(
    entity: Entity,
    commands: &mut Commands,
    visuals: &ColonyVisualAssets,
    apparence: &ApparenceAstronaute,
) -> RigAstronaute {
    let mut torse = None;
    let mut tete = None;
    let mut bras_gauche = None;
    let mut bras_droit = None;
    let mut jambe_gauche = None;
    let mut jambe_droite = None;

    let gabarit = gabarit_combinaison(apparence.combinaison);
    let materiau_role = materiau_role(visuals, apparence.role);
    let signature = signature_role(apparence.role);

    commands.entity(entity).with_children(|parent| {
        torse = Some(
            parent
                .spawn((
                    Transform::from_translation(offset_torse()),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    construire_torse(
                        pivot,
                        visuals,
                        apparence,
                        &gabarit,
                        materiau_role.clone(),
                        signature,
                    );
                })
                .id(),
        );

        tete = Some(
            parent
                .spawn((
                    Transform::from_translation(offset_tete()),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    construire_tete(
                        pivot,
                        visuals,
                        apparence,
                        &gabarit,
                        materiau_role.clone(),
                        signature,
                    );
                })
                .id(),
        );

        bras_gauche = Some(
            parent
                .spawn((
                    Transform::from_translation(offset_bras_gauche()),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    construire_bras(
                        pivot,
                        visuals,
                        apparence,
                        &gabarit,
                        materiau_role.clone(),
                        signature,
                        true,
                    );
                })
                .id(),
        );

        bras_droit = Some(
            parent
                .spawn((
                    Transform::from_translation(offset_bras_droit()),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    construire_bras(
                        pivot,
                        visuals,
                        apparence,
                        &gabarit,
                        materiau_role.clone(),
                        signature,
                        false,
                    );
                })
                .id(),
        );

        jambe_gauche = Some(
            parent
                .spawn((
                    Transform::from_translation(offset_jambe_gauche()),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    construire_jambe(
                        pivot,
                        visuals,
                        apparence,
                        &gabarit,
                        materiau_role.clone(),
                        true,
                    );
                })
                .id(),
        );

        jambe_droite = Some(
            parent
                .spawn((
                    Transform::from_translation(offset_jambe_droite()),
                    GlobalTransform::default(),
                    Visibility::default(),
                ))
                .with_children(|pivot| {
                    construire_jambe(
                        pivot,
                        visuals,
                        apparence,
                        &gabarit,
                        materiau_role.clone(),
                        false,
                    );
                })
                .id(),
        );
    });

    RigAstronaute {
        torse: torse.expect("pivot torse manquant"),
        tete: tete.expect("pivot tete manquante"),
        bras_gauche: bras_gauche.expect("pivot bras gauche manquant"),
        bras_droit: bras_droit.expect("pivot bras droit manquant"),
        jambe_gauche: jambe_gauche.expect("pivot jambe gauche manquant"),
        jambe_droite: jambe_droite.expect("pivot jambe droite manquant"),
    }
}

fn construire_torse(
    pivot: &mut ChildSpawnerCommands,
    visuals: &ColonyVisualAssets,
    apparence: &ApparenceAstronaute,
    gabarit: &GabaritCombinaison,
    materiau_role: Handle<StandardMaterial>,
    signature: SignatureRole,
) {
    capsule(
        pivot,
        visuals,
        visuals.suit_material.clone(),
        Vec3::new(0.0, 0.02, 0.00),
        Quat::IDENTITY,
        gabarit.coque_torse,
    );
    cube(
        pivot,
        visuals,
        visuals.suit_material.clone(),
        Vec3::new(0.0, 0.18, 0.04),
        Vec3::new(
            gabarit.plastron.x + 0.04,
            gabarit.plastron.y + 0.02,
            gabarit.plastron.z,
        ),
    );
    cube(
        pivot,
        visuals,
        visuals.hull_secondary_material.clone(),
        Vec3::new(0.0, -0.08, 0.26),
        gabarit.plastron,
    );
    cube(
        pivot,
        visuals,
        visuals.glass_material.clone(),
        Vec3::new(0.0, 0.00, 0.30),
        Vec3::new(gabarit.plastron.x - 0.08, gabarit.plastron.y - 0.12, 0.02),
    );
    cube(
        pivot,
        visuals,
        visuals.frame_material.clone(),
        Vec3::new(0.0, -0.34, 0.02),
        gabarit.bassin,
    );
    cube(
        pivot,
        visuals,
        visuals.suit_fabric_material.clone(),
        Vec3::new(0.0, -0.18, 0.02),
        Vec3::new(gabarit.bassin.x - 0.10, 0.14, gabarit.bassin.z - 0.08),
    );

    for &x in &[-gabarit.epaules_x, gabarit.epaules_x] {
        sphere(
            pivot,
            visuals,
            visuals.hull_secondary_material.clone(),
            Vec3::new(x, gabarit.hauteur_epaule, 0.0),
            Vec3::splat(gabarit.epaisseur_articulation + 0.03),
        );
    }
    for &x in &[-gabarit.hanches_x, gabarit.hanches_x] {
        sphere(
            pivot,
            visuals,
            visuals.hull_secondary_material.clone(),
            Vec3::new(x, gabarit.hauteur_hanche, 0.0),
            Vec3::splat(gabarit.epaisseur_articulation),
        );
    }

    for &x in &[-0.17, 0.17] {
        cube(
            pivot,
            visuals,
            visuals.frame_material.clone(),
            Vec3::new(x, -0.02, 0.18),
            Vec3::new(0.04, 0.52, 0.04),
        );
    }

    cube(
        pivot,
        visuals,
        materiau_role.clone(),
        Vec3::new(0.0, 0.20, 0.31),
        Vec3::new(gabarit.plastron.x - 0.02, 0.05, 0.02),
    );
    cube(
        pivot,
        visuals,
        visuals.lit_material.clone(),
        Vec3::new(0.0, -0.10, 0.31),
        Vec3::new(0.08, 0.08, 0.03),
    );

    if apparence.combinaison.sac_dorsal() {
        cube(
            pivot,
            visuals,
            visuals.frame_material.clone(),
            Vec3::new(0.0, 0.02, -0.34),
            gabarit.sac_dorsal,
        );
        cube(
            pivot,
            visuals,
            visuals.hull_secondary_material.clone(),
            Vec3::new(0.0, 0.28, -0.52),
            Vec3::new(gabarit.sac_dorsal.x - 0.08, 0.14, 0.06),
        );
        cube(
            pivot,
            visuals,
            visuals.hull_secondary_material.clone(),
            Vec3::new(0.0, -0.18, -0.30),
            Vec3::new(gabarit.sac_dorsal.x - 0.18, 0.10, 0.12),
        );
        for &x in &[-0.18, 0.18] {
            capsule(
                pivot,
                visuals,
                visuals.hull_secondary_material.clone(),
                Vec3::new(x, 0.00, -0.46),
                Quat::IDENTITY,
                gabarit.reserve_dorsale,
            );
            cube(
                pivot,
                visuals,
                visuals.lit_material.clone(),
                Vec3::new(x, 0.18, -0.60),
                Vec3::new(0.05, 0.05, 0.03),
            );
        }
        for &x in &[-0.17, 0.17] {
            cube(
                pivot,
                visuals,
                visuals.frame_material.clone(),
                Vec3::new(x, 0.12, -0.20),
                Vec3::new(0.05, 0.30, 0.05),
            );
        }
    }

    match apparence.role {
        RoleAstronaute::Commandant => {
            cube(
                pivot,
                visuals,
                materiau_role,
                Vec3::new(-0.18, 0.04, 0.31),
                Vec3::new(0.10, 0.22, 0.02),
            );
        }
        RoleAstronaute::Ingenieur => {
            cube(
                pivot,
                visuals,
                materiau_role,
                Vec3::new(0.18, -0.18, 0.22),
                Vec3::new(0.10, 0.12, 0.06),
            );
        }
        RoleAstronaute::Scientifique => {
            cube(
                pivot,
                visuals,
                materiau_role,
                Vec3::new(0.0, 0.06, 0.34),
                Vec3::new(0.10, 0.10, 0.02),
            );
            cube(
                pivot,
                visuals,
                visuals.glass_material.clone(),
                Vec3::new(0.0, 0.06, 0.36),
                Vec3::new(0.05, 0.05, 0.01),
            );
        }
        RoleAstronaute::Logisticien => {
            cube(
                pivot,
                visuals,
                materiau_role.clone(),
                Vec3::new(-0.22, -0.32, 0.18),
                Vec3::new(0.12, 0.10, 0.08),
            );
            cube(
                pivot,
                visuals,
                materiau_role,
                Vec3::new(0.22, -0.32, 0.18),
                Vec3::new(0.12, 0.10, 0.08),
            );
        }
        RoleAstronaute::Civil => {
            cube(
                pivot,
                visuals,
                materiau_role,
                Vec3::new(0.0, -0.32, 0.18),
                Vec3::new(0.26, 0.05, 0.04),
            );
        }
    }

    if signature.sacoche_ceinture {
        cube(
            pivot,
            visuals,
            visuals.storage_material.clone(),
            Vec3::new(-0.24, -0.34, 0.10),
            Vec3::new(0.12, 0.12, 0.12),
        );
    }
    if signature.double_sacoche {
        cube(
            pivot,
            visuals,
            visuals.storage_material.clone(),
            Vec3::new(0.24, -0.34, 0.10),
            Vec3::new(0.12, 0.12, 0.12),
        );
    }
    if signature.antenne_sac && apparence.combinaison.sac_dorsal() {
        cube(
            pivot,
            visuals,
            visuals.frame_material.clone(),
            Vec3::new(-0.12, 0.42, -0.44),
            Vec3::new(0.03, 0.18, 0.03),
        );
        sphere(
            pivot,
            visuals,
            visuals.lit_material.clone(),
            Vec3::new(-0.12, 0.54, -0.44),
            Vec3::splat(0.04),
        );
    }
}

fn construire_tete(
    pivot: &mut ChildSpawnerCommands,
    visuals: &ColonyVisualAssets,
    apparence: &ApparenceAstronaute,
    gabarit: &GabaritCombinaison,
    materiau_role: Handle<StandardMaterial>,
    signature: SignatureRole,
) {
    sphere(
        pivot,
        visuals,
        visuals.suit_material.clone(),
        Vec3::ZERO,
        gabarit.casque,
    );
    cube(
        pivot,
        visuals,
        visuals.hull_secondary_material.clone(),
        Vec3::new(0.0, -0.02, -0.18),
        Vec3::new(0.22, 0.20, 0.12),
    );
    cube(
        pivot,
        visuals,
        visuals.frame_material.clone(),
        Vec3::new(0.0, -0.28, 0.0),
        gabarit.collier,
    );

    if apparence.combinaison.casque_integral() {
        sphere(
            pivot,
            visuals,
            visuals.visor_material.clone(),
            Vec3::new(0.0, 0.02, 0.20),
            gabarit.visiere,
        );
        cube(
            pivot,
            visuals,
            visuals.glass_material.clone(),
            Vec3::new(0.0, 0.02, 0.28),
            Vec3::new(gabarit.visiere.x - 0.12, gabarit.visiere.y - 0.12, 0.02),
        );
        cube(
            pivot,
            visuals,
            visuals.lit_material.clone(),
            Vec3::new(0.0, 0.20, 0.24),
            Vec3::new(0.18, 0.04, 0.03),
        );
    } else {
        cube(
            pivot,
            visuals,
            visuals.glass_material.clone(),
            Vec3::new(0.0, 0.0, 0.18),
            Vec3::new(0.18, 0.18, 0.04),
        );
    }

    for &x in &[-0.28, 0.28] {
        sphere(
            pivot,
            visuals,
            visuals.hull_secondary_material.clone(),
            Vec3::new(x, 0.02, 0.0),
            Vec3::splat(0.08),
        );
    }

    cube(
        pivot,
        visuals,
        materiau_role,
        Vec3::new(0.0, -0.18, 0.22),
        Vec3::new(0.14, 0.04, 0.02),
    );

    if signature.capteur_casque {
        cube(
            pivot,
            visuals,
            visuals.frame_material.clone(),
            Vec3::new(0.0, 0.30, 0.0),
            Vec3::new(0.04, 0.10, 0.04),
        );
        cube(
            pivot,
            visuals,
            visuals.mission_blue_material.clone(),
            Vec3::new(0.0, 0.38, 0.08),
            Vec3::new(0.10, 0.03, 0.03),
        );
    }
}

fn construire_bras(
    pivot: &mut ChildSpawnerCommands,
    visuals: &ColonyVisualAssets,
    apparence: &ApparenceAstronaute,
    gabarit: &GabaritCombinaison,
    materiau_role: Handle<StandardMaterial>,
    signature: SignatureRole,
    gauche: bool,
) {
    let signe = if gauche { -1.0 } else { 1.0 };

    sphere(
        pivot,
        visuals,
        visuals.hull_secondary_material.clone(),
        Vec3::ZERO,
        Vec3::splat(gabarit.epaisseur_articulation + 0.02),
    );
    capsule(
        pivot,
        visuals,
        visuals.suit_material.clone(),
        Vec3::new(0.0, -0.22, 0.0),
        Quat::IDENTITY,
        gabarit.bras_haut,
    );
    sphere(
        pivot,
        visuals,
        visuals.hull_secondary_material.clone(),
        Vec3::new(0.0, -0.44, 0.0),
        Vec3::splat(gabarit.epaisseur_articulation),
    );
    capsule(
        pivot,
        visuals,
        visuals.suit_fabric_material.clone(),
        Vec3::new(0.0, -0.64, 0.03),
        Quat::IDENTITY,
        gabarit.bras_bas,
    );
    sphere(
        pivot,
        visuals,
        visuals.boot_material.clone(),
        Vec3::new(0.0, -0.88, 0.12),
        Vec3::splat(0.10),
    );
    cube(
        pivot,
        visuals,
        visuals.frame_material.clone(),
        Vec3::new(0.0, -0.94, 0.20),
        Vec3::new(0.14, 0.06, 0.18),
    );

    cube(
        pivot,
        visuals,
        materiau_role.clone(),
        Vec3::new(0.0, -0.12, 0.14),
        Vec3::new(0.22, 0.04, 0.02),
    );

    if signature.boitier_bras && gauche {
        cube(
            pivot,
            visuals,
            visuals.storage_material.clone(),
            Vec3::new(0.05 * signe, -0.76, 0.18),
            Vec3::new(0.12, 0.10, 0.06),
        );
        cube(
            pivot,
            visuals,
            visuals.lit_material.clone(),
            Vec3::new(0.05 * signe, -0.76, 0.22),
            Vec3::new(0.05, 0.03, 0.01),
        );
    }

    if apparence.role == RoleAstronaute::Ingenieur && !gauche {
        cube(
            pivot,
            visuals,
            materiau_role,
            Vec3::new(0.0, -0.88, 0.18),
            Vec3::new(0.20, 0.05, 0.02),
        );
    }
}

fn construire_jambe(
    pivot: &mut ChildSpawnerCommands,
    visuals: &ColonyVisualAssets,
    apparence: &ApparenceAstronaute,
    gabarit: &GabaritCombinaison,
    materiau_role: Handle<StandardMaterial>,
    gauche: bool,
) {
    let signe = if gauche { -1.0 } else { 1.0 };

    sphere(
        pivot,
        visuals,
        visuals.hull_secondary_material.clone(),
        Vec3::ZERO,
        Vec3::splat(gabarit.epaisseur_articulation),
    );
    capsule(
        pivot,
        visuals,
        visuals.suit_material.clone(),
        Vec3::new(0.0, -0.28, 0.0),
        Quat::IDENTITY,
        gabarit.cuisse,
    );
    sphere(
        pivot,
        visuals,
        visuals.hull_secondary_material.clone(),
        Vec3::new(0.0, -0.56, 0.03),
        Vec3::splat(gabarit.epaisseur_articulation + 0.01),
    );
    capsule(
        pivot,
        visuals,
        visuals.suit_fabric_material.clone(),
        Vec3::new(0.0, -0.80, 0.04),
        Quat::IDENTITY,
        gabarit.mollet,
    );
    cube(
        pivot,
        visuals,
        visuals.boot_material.clone(),
        Vec3::new(0.0, -1.00, 0.08),
        gabarit.botte,
    );
    cube(
        pivot,
        visuals,
        visuals.frame_material.clone(),
        Vec3::new(0.0, -1.06, 0.20),
        gabarit.semelle,
    );
    cube(
        pivot,
        visuals,
        visuals.frame_material.clone(),
        Vec3::new(0.0, -0.98, -0.04),
        Vec3::new(gabarit.semelle.x - 0.10, 0.08, 0.10),
    );

    cube(
        pivot,
        visuals,
        materiau_role,
        Vec3::new(0.04 * signe, -0.58, 0.16),
        Vec3::new(0.10, 0.06, 0.02),
    );

    if apparence.role == RoleAstronaute::Logisticien && !gauche {
        cube(
            pivot,
            visuals,
            visuals.storage_material.clone(),
            Vec3::new(0.0, -0.86, 0.18),
            Vec3::new(0.14, 0.12, 0.03),
        );
    }
}

fn gabarit_combinaison(combinaison: TypeCombinaison) -> GabaritCombinaison {
    match combinaison {
        TypeCombinaison::Interieure => GabaritCombinaison {
            coque_torse: Vec3::new(0.28, 0.34, 0.22),
            plastron: Vec3::new(0.22, 0.26, 0.06),
            bassin: Vec3::new(0.34, 0.10, 0.24),
            casque: Vec3::new(0.32, 0.36, 0.32),
            visiere: Vec3::new(0.16, 0.16, 0.04),
            collier: Vec3::new(0.24, 0.08, 0.24),
            sac_dorsal: Vec3::new(0.22, 0.28, 0.16),
            reserve_dorsale: Vec3::new(0.08, 0.16, 0.08),
            bras_haut: Vec3::new(0.10, 0.22, 0.10),
            bras_bas: Vec3::new(0.09, 0.20, 0.09),
            cuisse: Vec3::new(0.12, 0.26, 0.12),
            mollet: Vec3::new(0.11, 0.24, 0.11),
            botte: Vec3::new(0.18, 0.10, 0.22),
            semelle: Vec3::new(0.20, 0.05, 0.28),
            hauteur_epaule: 0.22,
            hauteur_hanche: -0.34,
            epaules_x: 0.34,
            hanches_x: 0.16,
            epaisseur_articulation: 0.10,
        },
        TypeCombinaison::Vehicule => GabaritCombinaison {
            coque_torse: Vec3::new(0.30, 0.36, 0.24),
            plastron: Vec3::new(0.24, 0.28, 0.06),
            bassin: Vec3::new(0.36, 0.10, 0.26),
            casque: Vec3::new(0.38, 0.40, 0.38),
            visiere: Vec3::new(0.22, 0.18, 0.10),
            collier: Vec3::new(0.28, 0.09, 0.28),
            sac_dorsal: Vec3::new(0.28, 0.34, 0.18),
            reserve_dorsale: Vec3::new(0.09, 0.18, 0.09),
            bras_haut: Vec3::new(0.12, 0.24, 0.12),
            bras_bas: Vec3::new(0.10, 0.22, 0.10),
            cuisse: Vec3::new(0.13, 0.28, 0.13),
            mollet: Vec3::new(0.12, 0.26, 0.12),
            botte: Vec3::new(0.20, 0.10, 0.24),
            semelle: Vec3::new(0.22, 0.05, 0.30),
            hauteur_epaule: 0.24,
            hauteur_hanche: -0.36,
            epaules_x: 0.35,
            hanches_x: 0.16,
            epaisseur_articulation: 0.10,
        },
        TypeCombinaison::EvaPlanetaireLegere => GabaritCombinaison {
            coque_torse: Vec3::new(0.32, 0.40, 0.26),
            plastron: Vec3::new(0.26, 0.30, 0.06),
            bassin: Vec3::new(0.40, 0.11, 0.28),
            casque: Vec3::new(0.40, 0.42, 0.40),
            visiere: Vec3::new(0.26, 0.20, 0.14),
            collier: Vec3::new(0.30, 0.10, 0.30),
            sac_dorsal: Vec3::new(0.34, 0.56, 0.20),
            reserve_dorsale: Vec3::new(0.10, 0.22, 0.10),
            bras_haut: Vec3::new(0.13, 0.26, 0.13),
            bras_bas: Vec3::new(0.11, 0.24, 0.11),
            cuisse: Vec3::new(0.14, 0.30, 0.14),
            mollet: Vec3::new(0.13, 0.28, 0.13),
            botte: Vec3::new(0.22, 0.11, 0.28),
            semelle: Vec3::new(0.24, 0.05, 0.34),
            hauteur_epaule: 0.24,
            hauteur_hanche: -0.36,
            epaules_x: 0.36,
            hanches_x: 0.16,
            epaisseur_articulation: 0.11,
        },
        TypeCombinaison::EvaPlanetaireLourde => GabaritCombinaison {
            coque_torse: Vec3::new(0.35, 0.44, 0.28),
            plastron: Vec3::new(0.30, 0.32, 0.07),
            bassin: Vec3::new(0.44, 0.12, 0.30),
            casque: Vec3::new(0.44, 0.44, 0.44),
            visiere: Vec3::new(0.30, 0.22, 0.18),
            collier: Vec3::new(0.32, 0.10, 0.32),
            sac_dorsal: Vec3::new(0.40, 0.68, 0.24),
            reserve_dorsale: Vec3::new(0.11, 0.26, 0.11),
            bras_haut: Vec3::new(0.14, 0.28, 0.14),
            bras_bas: Vec3::new(0.12, 0.26, 0.12),
            cuisse: Vec3::new(0.15, 0.32, 0.15),
            mollet: Vec3::new(0.14, 0.30, 0.14),
            botte: Vec3::new(0.24, 0.12, 0.30),
            semelle: Vec3::new(0.26, 0.06, 0.36),
            hauteur_epaule: 0.26,
            hauteur_hanche: -0.38,
            epaules_x: 0.38,
            hanches_x: 0.18,
            epaisseur_articulation: 0.12,
        },
    }
}

fn signature_role(role: RoleAstronaute) -> SignatureRole {
    match role {
        RoleAstronaute::Commandant => SignatureRole {
            antenne_sac: true,
            capteur_casque: false,
            sacoche_ceinture: false,
            double_sacoche: false,
            boitier_bras: false,
        },
        RoleAstronaute::Ingenieur => SignatureRole {
            antenne_sac: false,
            capteur_casque: false,
            sacoche_ceinture: true,
            double_sacoche: false,
            boitier_bras: true,
        },
        RoleAstronaute::Scientifique => SignatureRole {
            antenne_sac: false,
            capteur_casque: true,
            sacoche_ceinture: false,
            double_sacoche: false,
            boitier_bras: true,
        },
        RoleAstronaute::Logisticien => SignatureRole {
            antenne_sac: false,
            capteur_casque: false,
            sacoche_ceinture: true,
            double_sacoche: true,
            boitier_bras: false,
        },
        RoleAstronaute::Civil => SignatureRole {
            antenne_sac: false,
            capteur_casque: false,
            sacoche_ceinture: false,
            double_sacoche: false,
            boitier_bras: false,
        },
    }
}

fn materiau_role(visuals: &ColonyVisualAssets, role: RoleAstronaute) -> Handle<StandardMaterial> {
    match role {
        RoleAstronaute::Commandant => visuals.mission_red_material.clone(),
        RoleAstronaute::Ingenieur => visuals.accent_material.clone(),
        RoleAstronaute::Scientifique => visuals.mission_blue_material.clone(),
        RoleAstronaute::Logisticien => visuals.lit_material.clone(),
        RoleAstronaute::Civil => visuals.storage_material.clone(),
    }
}

fn cube(
    parent: &mut ChildSpawnerCommands,
    visuals: &ColonyVisualAssets,
    material: Handle<StandardMaterial>,
    translation: Vec3,
    scale: Vec3,
) {
    piece(
        parent,
        visuals.cube_mesh.clone(),
        material,
        translation,
        Quat::IDENTITY,
        scale,
    );
}

fn sphere(
    parent: &mut ChildSpawnerCommands,
    visuals: &ColonyVisualAssets,
    material: Handle<StandardMaterial>,
    translation: Vec3,
    scale: Vec3,
) {
    piece(
        parent,
        visuals.sphere_mesh.clone(),
        material,
        translation,
        Quat::IDENTITY,
        scale,
    );
}

fn capsule(
    parent: &mut ChildSpawnerCommands,
    visuals: &ColonyVisualAssets,
    material: Handle<StandardMaterial>,
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
) {
    piece(
        parent,
        visuals.capsule_mesh.clone(),
        material,
        translation,
        rotation,
        scale,
    );
}

fn piece(
    parent: &mut ChildSpawnerCommands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
) {
    parent.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform {
            translation,
            rotation,
            scale,
        },
    ));
}

#[allow(dead_code)]
fn _silhouette_est_pressurisee(combinaison: TypeCombinaison) -> bool {
    combinaison != TypeCombinaison::Interieure
}
