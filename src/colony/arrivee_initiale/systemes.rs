use std::f32::consts::PI;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::super::astronautes::{AnimationOuvrier, CibleMondeLisse, PositionMondeLisse};
use super::super::{
    AnimationPromenade, Astronaut, AstronautStatus, AstronautePromeneur, ColonyVisualAssets,
    EtatPromenade, GridPosition, PositionMonde, StructureKind, StructureState, ZoneRechargeBase,
};
use super::donnees::{
    AstronauteDebarquement, COULEUR_AMBIANTE_INTRO, COULEUR_AMBIANTE_JEU, COULEUR_CIEL_INTRO,
    COULEUR_CIEL_JEU, DISTANCE_ARRIVEE_DEBARQUEMENT, DUREE_FONDU_FINAL, EQUIPAGE_ARRIVEE,
    EffetArriveeInitiale, FlammePropulseur, LUMINOSITE_AMBIANTE_INTRO, LUMINOSITE_AMBIANTE_JEU,
    LumierePropulseur, MateriauxArriveeInitiale, NuagePoussiere, SequenceArriveeInitiale,
    VITESSE_DEBARQUEMENT, ZOOM_CAMERA_FINAL, duree_totale_intro, ease_in_out_cubic, ease_out_cubic,
    etat_vaisseau, focus_camera_final, temps_sortie_equipage,
};
use crate::core::{
    CAMERA_ORBIT_DIRECTION, CameraController, CameraOverride, GameState, WorldOrigin,
};
use crate::world::{
    PlanetProfile, WorldCache, WorldSeed, continuous_world_to_render_translation, footprint_center,
    structure_anchor_translation,
};

type FiltreSansEffets = (
    Without<FlammePropulseur>,
    Without<NuagePoussiere>,
    Without<LumierePropulseur>,
);

type FiltreFlammes = (
    Without<NuagePoussiere>,
    Without<LumierePropulseur>,
    Without<StructureState>,
);

type FiltrePoussieres = (
    Without<FlammePropulseur>,
    Without<LumierePropulseur>,
    Without<StructureState>,
);

type FiltreLumieres = (
    With<LumierePropulseur>,
    Without<FlammePropulseur>,
    Without<NuagePoussiere>,
    Without<StructureState>,
);

#[derive(SystemParam)]
pub(crate) struct ContextePilotageArriveeInitiale<'w, 's> {
    commands: Commands<'w, 's>,
    next_state: ResMut<'w, NextState<GameState>>,
    controller: ResMut<'w, CameraController>,
    camera_override: Option<ResMut<'w, CameraOverride>>,
    clear_color: ResMut<'w, ClearColor>,
    ambient: ResMut<'w, GlobalAmbientLight>,
    cache: ResMut<'w, WorldCache>,
    profile: Res<'w, PlanetProfile>,
    seed: Res<'w, WorldSeed>,
    origin: Res<'w, WorldOrigin>,
    sequence: Option<ResMut<'w, SequenceArriveeInitiale>>,
    lander_query:
        Query<'w, 's, (&'static StructureState, &'static mut Transform), FiltreSansEffets>,
    flames: Query<
        'w,
        's,
        (
            &'static FlammePropulseur,
            &'static mut Transform,
            &'static mut Visibility,
        ),
        FiltreFlammes,
    >,
    poussieres: Query<
        'w,
        's,
        (
            &'static NuagePoussiere,
            &'static mut Transform,
            &'static mut Visibility,
        ),
        FiltrePoussieres,
    >,
    lumiere: Query<'w, 's, (&'static mut PointLight, &'static mut Visibility), FiltreLumieres>,
    debarquements: Query<'w, 's, (), With<AstronauteDebarquement>>,
}

pub(crate) fn preparer_arrivee_initiale(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut controller: ResMut<CameraController>,
    mut clear_color: ResMut<ClearColor>,
    mut ambient: ResMut<GlobalAmbientLight>,
    zone_recharge: Res<ZoneRechargeBase>,
    profile: Res<PlanetProfile>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    visuals: Res<ColonyVisualAssets>,
    structures: Query<(
        Entity,
        &StructureState,
        Option<&Transform>,
        Option<&GlobalTransform>,
        Option<&Visibility>,
    )>,
) {
    let Some((lander, structure, transform, global_transform, visibility)) = structures
        .iter()
        .find(|(_, structure, _, _, _)| structure.kind == StructureKind::Lander)
    else {
        next_state.set(GameState::InGame);
        return;
    };

    let centre_monde = structure.center_world(profile.cell_size_meters);
    let point_sortie = centre_monde + Vec2::new(0.7, -1.35);
    let cellule_mila = zone_recharge
        .cellule_la_plus_proche(structure.anchor)
        .unwrap_or(IVec2::new(-1, 0));
    let position_mila = footprint_center(&[cellule_mila], profile.cell_size_meters);

    let materiaux = MateriauxArriveeInitiale {
        flamme: materials.add(StandardMaterial {
            base_color: Color::srgb(0.98, 0.75, 0.34),
            emissive: Color::srgb(1.40, 0.58, 0.16).into(),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
        poussiere: materials.add(StandardMaterial {
            base_color: Color::srgba(0.81, 0.61, 0.46, 0.34),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
    };

    commands.insert_resource(materiaux.clone());
    commands.insert_resource(SequenceArriveeInitiale {
        lander,
        centre_monde,
        point_sortie,
        position_mila,
        temps: 0.0,
        equipage_deploie: 0,
        mila_creee: false,
    });
    commands.insert_resource(CameraOverride {
        transform: Transform::default(),
    });

    if transform.is_none() {
        commands.entity(lander).insert(Transform::default());
    }
    if global_transform.is_none() {
        commands.entity(lander).insert(GlobalTransform::default());
    }
    if visibility.is_none() {
        commands.entity(lander).insert(Visibility::default());
    }

    commands.entity(lander).with_children(|parent| {
        for &(x, z, intensite, decalage) in &[
            (-0.68, -0.68, 1.0, 0.0),
            (0.68, -0.68, 0.92, 0.9),
            (-0.68, 0.68, 0.90, 1.7),
            (0.68, 0.68, 1.0, 2.5),
        ] {
            parent.spawn((
                Name::new("Flamme Propulseur"),
                EffetArriveeInitiale,
                FlammePropulseur {
                    intensite_base: intensite,
                    decalage_temps: decalage,
                },
                Mesh3d(visuals.sphere_mesh.clone()),
                MeshMaterial3d(materiaux.flamme.clone()),
                Transform::from_xyz(x, -0.78, z).with_scale(Vec3::ZERO),
                Visibility::Hidden,
            ));
        }

        for &(taille, decalage) in &[(1.0, 0.0), (0.62, 1.2)] {
            parent.spawn((
                Name::new("Nuage Poussiere"),
                EffetArriveeInitiale,
                NuagePoussiere {
                    multiplicateur_taille: taille,
                    decalage_temps: decalage,
                },
                Mesh3d(visuals.sphere_mesh.clone()),
                MeshMaterial3d(materiaux.poussiere.clone()),
                Transform::from_xyz(0.0, -0.18, 0.0).with_scale(Vec3::ZERO),
                Visibility::Hidden,
            ));
        }

        parent.spawn((
            Name::new("Lumiere Propulseur"),
            EffetArriveeInitiale,
            LumierePropulseur,
            PointLight {
                color: Color::srgb(1.0, 0.61, 0.28),
                intensity: 0.0,
                range: 12.0,
                radius: 1.6,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, -0.42, 0.0),
            Visibility::Hidden,
        ));
    });

    controller.focus_world = focus_camera_final(centre_monde);
    controller.zoom = ZOOM_CAMERA_FINAL;
    clear_color.0 = couleur_rgb(COULEUR_CIEL_INTRO);
    ambient.color = couleur_rgb(COULEUR_AMBIANTE_INTRO);
    ambient.brightness = LUMINOSITE_AMBIANTE_INTRO;
}

pub(crate) fn piloter_arrivee_initiale(
    time: Res<Time>,
    mut contexte: ContextePilotageArriveeInitiale,
) {
    let Some(mut sequence) = contexte.sequence.take() else {
        contexte.next_state.set(GameState::InGame);
        return;
    };
    let Some(mut camera_override) = contexte.camera_override.take() else {
        contexte.next_state.set(GameState::InGame);
        return;
    };

    sequence.temps += time.delta_secs();
    let etat = etat_vaisseau(sequence.temps);
    let position_vaisseau = sequence.centre_monde + etat.decalage_monde;
    let focus_final = focus_camera_final(sequence.centre_monde);

    contexte.controller.focus_world = focus_final;
    contexte.controller.zoom = ZOOM_CAMERA_FINAL;

    if let Ok((_, mut transform)) = contexte.lander_query.get_mut(sequence.lander) {
        transform.translation = continuous_world_to_render_translation(
            position_vaisseau,
            0.28 + etat.altitude,
            &mut contexte.cache,
            &contexte.profile,
            *contexte.seed,
            &contexte.origin,
        );
        transform.rotation = etat.rotation;
        transform.scale = Vec3::ONE;
    }

    mettre_a_jour_effets_propulseurs(
        sequence.temps,
        etat.altitude,
        etat.intensite_propulseurs,
        etat.intensite_poussiere,
        &mut contexte.flames,
        &mut contexte.poussieres,
        &mut contexte.lumiere,
    );
    camera_override.transform = construire_camera_cinematique(
        sequence.temps,
        position_vaisseau,
        etat.altitude,
        sequence.centre_monde,
        &focus_final,
        &mut contexte.cache,
        &contexte.profile,
        *contexte.seed,
        &contexte.origin,
    );

    let progression_couleurs = ease_out_cubic(
        (sequence.temps / (duree_totale_intro() - DUREE_FONDU_FINAL * 0.35)).clamp(0.0, 1.0),
    );
    contexte.clear_color.0 =
        melanger_couleurs(COULEUR_CIEL_INTRO, COULEUR_CIEL_JEU, progression_couleurs);
    contexte.ambient.color = melanger_couleurs(
        COULEUR_AMBIANTE_INTRO,
        COULEUR_AMBIANTE_JEU,
        progression_couleurs,
    );
    contexte.ambient.brightness = LUMINOSITE_AMBIANTE_INTRO
        + (LUMINOSITE_AMBIANTE_JEU - LUMINOSITE_AMBIANTE_INTRO) * progression_couleurs;

    while sequence.equipage_deploie < EQUIPAGE_ARRIVEE.len() {
        let data = EQUIPAGE_ARRIVEE[sequence.equipage_deploie];
        if temps_sortie_equipage(sequence.temps) < data.delai_sortie {
            break;
        }

        let cible_monde = footprint_center(&[data.position_finale], contexte.profile.cell_size_meters);
        let origine_monde = sequence.point_sortie + data.decalage_sortie;
        let orientation = (cible_monde - origine_monde)
            .x
            .atan2((cible_monde - origine_monde).y);

        contexte.commands.spawn((
            Astronaut {
                id: data.id,
                name: data.nom,
                suit_oxygen: 180.0,
                current_task: None,
                status: AstronautStatus::Moving,
                carrying_ice: 0.0,
            },
            GridPosition(data.position_finale),
            PositionMondeLisse(origine_monde),
            CibleMondeLisse(cible_monde),
            AnimationOuvrier {
                orientation,
                ..default()
            },
            AstronauteDebarquement { cible: cible_monde },
            Name::new(format!("Astronaute {}", data.nom)),
        ));
        sequence.equipage_deploie += 1;
    }

    if !sequence.mila_creee && sequence.temps >= duree_totale_intro() - 0.95 {
        contexte.commands.spawn((
            AstronautePromeneur {
                id: super::super::AstronautId(10),
                nom: "Mila",
                air_combinaison: 180.0,
                etat: EtatPromenade::Promenade,
                cellule_cible: None,
                compteur_promenade: 0,
                pause_restante: 0.0,
            },
            PositionMonde(sequence.position_mila),
            AnimationPromenade::default(),
            Name::new("Astronaute Mila"),
        ));
        sequence.mila_creee = true;
    }

    if sequence.temps >= duree_totale_intro()
        && sequence.equipage_deploie == EQUIPAGE_ARRIVEE.len()
        && contexte.debarquements.is_empty()
    {
        contexte.next_state.set(GameState::InGame);
    }
}

pub(crate) fn animer_debarquement_initial(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut Astronaut,
        &mut PositionMondeLisse,
        &mut AnimationOuvrier,
        &AstronauteDebarquement,
    )>,
) {
    let delta = time.delta_secs();
    if delta <= 0.0 {
        return;
    }

    for (entity, mut astronaut, mut position, mut animation, debarquement) in &mut query {
        let delta_cible = debarquement.cible - position.0;
        let distance = delta_cible.length();

        if distance <= DISTANCE_ARRIVEE_DEBARQUEMENT {
            position.0 = debarquement.cible;
            astronaut.status = AstronautStatus::Idle;
            animation.vitesse_normalisee = 0.0;
            animation.intensite_travail = 0.0;
            animation.phase_pas += delta * 1.4;
            commands.entity(entity).remove::<AstronauteDebarquement>();
            continue;
        }

        let direction = delta_cible.normalize();
        let pas = VITESSE_DEBARQUEMENT * delta;
        let mouvement = direction * pas.min(distance);
        position.0 += mouvement;

        let orientation_cible = direction.x.atan2(direction.y);
        animation.orientation = lerp_angle(
            animation.orientation,
            orientation_cible,
            (delta * 8.4).clamp(0.0, 1.0),
        );
        animation.vitesse_normalisee =
            (mouvement.length() / (VITESSE_DEBARQUEMENT * delta.max(0.001))).clamp(0.0, 1.0);
        animation.intensite_travail = 0.0;
        animation.phase_pas += delta * (2.8 + 4.6 * animation.vitesse_normalisee);
        astronaut.status = AstronautStatus::Moving;
    }
}

pub(crate) fn nettoyer_arrivee_initiale(
    mut commands: Commands,
    mut controller: ResMut<CameraController>,
    mut clear_color: ResMut<ClearColor>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut cache: ResMut<WorldCache>,
    profile: Res<PlanetProfile>,
    seed: Res<WorldSeed>,
    origin: Res<WorldOrigin>,
    sequence: Option<Res<SequenceArriveeInitiale>>,
    mut lander_query: Query<(&StructureState, &mut Transform)>,
    effets: Query<Entity, With<EffetArriveeInitiale>>,
    debarquements: Query<Entity, With<AstronauteDebarquement>>,
) {
    if let Some(sequence) = sequence {
        controller.focus_world = focus_camera_final(sequence.centre_monde);
        controller.zoom = ZOOM_CAMERA_FINAL;

        if let Ok((structure, mut transform)) = lander_query.get_mut(sequence.lander) {
            let cells = structure.occupied_cells();
            transform.translation =
                structure_anchor_translation(&cells, &mut cache, &profile, *seed, &origin);
            transform.rotation = Quat::IDENTITY;
            transform.scale = Vec3::ONE;
        }
    }

    clear_color.0 = couleur_rgb(COULEUR_CIEL_JEU);
    ambient.color = couleur_rgb(COULEUR_AMBIANTE_JEU);
    ambient.brightness = LUMINOSITE_AMBIANTE_JEU;

    for entity in &effets {
        commands.entity(entity).despawn();
    }
    for entity in &debarquements {
        commands.entity(entity).remove::<AstronauteDebarquement>();
    }

    commands.remove_resource::<CameraOverride>();
    commands.remove_resource::<MateriauxArriveeInitiale>();
    commands.remove_resource::<SequenceArriveeInitiale>();
}

fn construire_camera_cinematique(
    temps: f32,
    position_vaisseau: Vec2,
    altitude_vaisseau: f32,
    centre_monde: Vec2,
    focus_final: &Vec2,
    cache: &mut WorldCache,
    profile: &PlanetProfile,
    seed: WorldSeed,
    origin: &WorldOrigin,
) -> Transform {
    let centre_render = continuous_world_to_render_translation(
        position_vaisseau,
        0.32 + altitude_vaisseau,
        cache,
        profile,
        seed,
        origin,
    );
    let cible_cinema = centre_render + Vec3::new(0.0, 1.55, 0.0);

    let progression_pose = ((temps)
        / (super::donnees::DUREE_APPROCHE + super::donnees::DUREE_DESCENTE_FINALE))
        .clamp(0.0, 1.0);
    let progression_pose = ease_in_out_cubic(progression_pose);
    let oeil_cinema = centre_render
        + Vec3::new(-29.0, 17.0, 23.0).lerp(Vec3::new(-17.0, 10.0, 13.0), progression_pose)
        + Vec3::new((temps * 0.6).sin() * 1.4, 0.0, (temps * 0.4).cos() * 0.8);

    let cible_finale =
        continuous_world_to_render_translation(*focus_final, 0.80, cache, profile, seed, origin);
    let oeil_final = cible_finale + CAMERA_ORBIT_DIRECTION.normalize() * ZOOM_CAMERA_FINAL;
    let transform_final = Transform::from_translation(oeil_final).looking_at(cible_finale, Vec3::Y);

    let debut_fondu = duree_totale_intro() - DUREE_FONDU_FINAL;
    if temps <= debut_fondu {
        return Transform::from_translation(oeil_cinema).looking_at(cible_cinema, Vec3::Y);
    }

    let facteur = ease_in_out_cubic(((temps - debut_fondu) / DUREE_FONDU_FINAL).clamp(0.0, 1.0));
    let oeil = oeil_cinema.lerp(transform_final.translation, facteur);
    let cible = cible_cinema.lerp(cible_finale, facteur);

    let _ = centre_monde;
    Transform::from_translation(oeil).looking_at(cible, Vec3::Y)
}

fn mettre_a_jour_effets_propulseurs(
    temps: f32,
    altitude: f32,
    intensite_propulseurs: f32,
    intensite_poussiere: f32,
    flames: &mut Query<(&FlammePropulseur, &mut Transform, &mut Visibility), FiltreFlammes>,
    poussieres: &mut Query<(&NuagePoussiere, &mut Transform, &mut Visibility), FiltrePoussieres>,
    lumiere: &mut Query<(&mut PointLight, &mut Visibility), FiltreLumieres>,
) {
    for (flamme, mut transform, mut visibility) in flames {
        let pulsation = 0.86 + 0.14 * ((temps * 16.0 + flamme.decalage_temps).sin() * 0.5 + 0.5);
        let intensite = intensite_propulseurs * flamme.intensite_base * pulsation;
        transform.translation.y = -0.84 - intensite * 1.25;
        transform.scale = Vec3::new(
            0.18 + intensite * 0.12,
            intensite * 1.85,
            0.18 + intensite * 0.12,
        );
        *visibility = if intensite > 0.03 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    for (poussiere, mut transform, mut visibility) in poussieres {
        let pulsation = 0.92 + 0.16 * ((temps * 3.4 + poussiere.decalage_temps).sin() * 0.5 + 0.5);
        let intensite = intensite_poussiere * pulsation;
        transform.translation.y = -0.18 - altitude;
        transform.scale = Vec3::new(
            3.2 * poussiere.multiplicateur_taille * intensite.max(0.02),
            0.08 + intensite * 0.10,
            3.2 * poussiere.multiplicateur_taille * intensite.max(0.02),
        );
        *visibility = if intensite > 0.05 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    for (mut point_light, mut visibility) in lumiere {
        let intensite = intensite_propulseurs * intensite_propulseurs;
        point_light.intensity = 1_800.0 + intensite * 4_200.0;
        point_light.range = 10.0 + intensite * 8.0;
        *visibility = if intensite_propulseurs > 0.04 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn lerp_angle(from: f32, to: f32, facteur: f32) -> f32 {
    let mut delta = (to - from) % (PI * 2.0);
    if delta > PI {
        delta -= PI * 2.0;
    } else if delta < -PI {
        delta += PI * 2.0;
    }
    from + delta * facteur
}

fn couleur_rgb(rgb: (f32, f32, f32)) -> Color {
    Color::srgb(rgb.0, rgb.1, rgb.2)
}

fn melanger_couleurs(source: (f32, f32, f32), cible: (f32, f32, f32), facteur: f32) -> Color {
    let facteur = facteur.clamp(0.0, 1.0);
    couleur_rgb((
        source.0 + (cible.0 - source.0) * facteur,
        source.1 + (cible.1 - source.1) * facteur,
        source.2 + (cible.2 - source.2) * facteur,
    ))
}
