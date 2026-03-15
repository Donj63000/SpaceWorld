use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::PrimaryWindow;

use crate::colony::{
    Astronaut, AstronautePromeneur, LifeSupportState, StructureState, TaskBoard,
};
use crate::construction::{
    BlocageConstructionParUi, EtatConnexionPlacement, PlacementFeedback, SelectedBuild,
};
use crate::core::{GameState, ParametresPerformanceJeu};
use crate::world::{HoveredCell, PlanetProfile, WorldSeed};

use super::hotbar_construction::metadonnee_slot;
use super::polices::PoliceInterface;
use super::presentation_hud::{
    carte_astronaut, carte_promeneur, couleur_niveau_mission, formatter_case_survolee,
    formatter_connexion, formatter_detail_alertes, formatter_resume_mission,
    libelle_niveau_mission, niveau_mission, objectif_mission, trier_cartes,
};
use super::references_hud::{
    CadenceHudEquipage, MemoireHudMission, RacineHud, ReferencesAlertesHud,
    ReferencesConstructionHud, ReferencesEquipageHud, ReferencesMissionHud,
    ReferencesReseauxHud, ZoneCaptureCurseurUi,
};
use super::theme_cockpit as theme;

pub(crate) fn appliquer_visibilite_hud(
    state: Res<State<GameState>>,
    mut racine: Single<&mut Visibility, With<RacineHud>>,
) {
    **racine = if matches!(state.get(), GameState::InGame | GameState::Paused) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

pub(crate) fn mettre_a_jour_blocage_construction_ui(
    state: Res<State<GameState>>,
    window: Single<&Window, With<PrimaryWindow>>,
    zones: Query<
        (&ComputedNode, &UiGlobalTransform, Option<&Visibility>),
        With<ZoneCaptureCurseurUi>,
    >,
    mut blocage: ResMut<BlocageConstructionParUi>,
) {
    if !matches!(state.get(), GameState::InGame | GameState::Paused) {
        blocage.0 = false;
        return;
    }

    let Some(curseur) = window.physical_cursor_position() else {
        blocage.0 = false;
        return;
    };

    blocage.0 = zones.iter().any(|(node, transform, visibilite)| {
        visibilite.copied().unwrap_or(Visibility::Visible) != Visibility::Hidden
            && node.contains_point(*transform, curseur)
    });
}

pub(crate) fn rafraichir_hud_mission_reseaux(
    state: Res<State<GameState>>,
    seed: Res<WorldSeed>,
    planet: Res<PlanetProfile>,
    tasks: Res<TaskBoard>,
    life_support: Res<LifeSupportState>,
    structures: Query<&StructureState>,
    references_mission: Res<ReferencesMissionHud>,
    references_alertes: Res<ReferencesAlertesHud>,
    references_reseaux: Res<ReferencesReseauxHud>,
    mut memoire: ResMut<MemoireHudMission>,
    mut textes: Query<&mut Text>,
    mut fonds: Query<&mut BackgroundColor>,
    mut bordures: Query<&mut BorderColor>,
    mut noeuds: Query<&mut Node>,
) {
    if !matches!(state.get(), GameState::InGame | GameState::Paused) {
        return;
    }

    let nombre_structures = structures.iter().count();
    let doit_rafraichir = state.is_changed()
        || tasks.is_changed()
        || life_support.is_changed()
        || memoire.dernier_nombre_structures != nombre_structures;
    if !doit_rafraichir {
        return;
    }
    memoire.dernier_nombre_structures = nombre_structures;

    let primary = life_support.primary.as_ref();
    let oxygene_stocke = primary.map(|reseau| reseau.oxygen_stored).unwrap_or(0.0);
    let oxygene_capacite = primary.map(|reseau| reseau.oxygen_capacity).unwrap_or(0.0);
    let glace_stockee = primary.map(|reseau| reseau.ice_stored).unwrap_or(0.0);
    let glace_capacite = primary.map(|reseau| reseau.ice_capacity).unwrap_or(0.0);
    let energie_generation = primary
        .map(|reseau| reseau.energy_generation)
        .unwrap_or(0.0);
    let energie_demande = primary.map(|reseau| reseau.energy_demand).unwrap_or(0.0);

    let ratio_oxygene = theme::ratio(oxygene_stocke, oxygene_capacite);
    let ratio_glace = theme::ratio(glace_stockee, glace_capacite);
    let ratio_energie = if energie_demande <= f32::EPSILON {
        1.0
    } else {
        (energie_generation / energie_demande).clamp(0.0, 1.0)
    };

    let niveau = niveau_mission(primary);
    let couleur_niveau = couleur_niveau_mission(niveau);

    remplacer_texte(
        &mut textes,
        references_mission.entete,
        format!("{} // SEED {}", planet.name.to_uppercase(), seed.0),
    );
    remplacer_texte(
        &mut textes,
        references_mission.resume,
        formatter_resume_mission(state.get(), nombre_structures, tasks.tasks.len()),
    );
    remplacer_texte(
        &mut textes,
        references_mission.badge_niveau,
        libelle_niveau_mission(niveau),
    );
    remplacer_fond(&mut fonds, references_mission.badge_niveau, couleur_niveau);
    remplacer_bordure(
        &mut bordures,
        references_mission.badge_niveau,
        couleur_niveau,
    );
    remplacer_texte(
        &mut textes,
        references_mission.objectif,
        objectif_mission(primary),
    );
    remplacer_texte(
        &mut textes,
        references_mission.libelle_oxygene,
        format!(
            "Reserve O2 colonie : {:.0} / {:.0}",
            oxygene_stocke, oxygene_capacite
        ),
    );
    remplacer_largeur_barre(
        &mut noeuds,
        references_mission.barre_oxygene,
        ratio_oxygene,
    );
    remplacer_fond(
        &mut fonds,
        references_mission.barre_oxygene,
        theme::couleur_ressource((0.23, 0.76, 0.51), ratio_oxygene),
    );

    remplacer_texte(
        &mut textes,
        references_mission.synthese_glace,
        format!("{:.0} / {:.0}", glace_stockee, glace_capacite),
    );
    remplacer_texte(
        &mut textes,
        references_mission.synthese_energie,
        format!("{:.1} / {:.1}", energie_generation, energie_demande),
    );
    remplacer_texte(
        &mut textes,
        references_mission.synthese_structures,
        format!("{nombre_structures}"),
    );
    remplacer_texte(
        &mut textes,
        references_mission.synthese_taches,
        format!("{}", tasks.tasks.len()),
    );

    remplacer_texte(
        &mut textes,
        references_alertes.badge_niveau,
        libelle_niveau_mission(niveau),
    );
    remplacer_fond(&mut fonds, references_alertes.badge_niveau, couleur_niveau);
    remplacer_bordure(
        &mut bordures,
        references_alertes.badge_niveau,
        couleur_niveau,
    );
    remplacer_texte(
        &mut textes,
        references_alertes.detail,
        formatter_detail_alertes(primary, life_support.disconnected.len()),
    );

    remplacer_texte(
        &mut textes,
        references_reseaux.libelle_energie,
        format!(
            "Energie disponible : {:.1} / {:.1}",
            energie_generation, energie_demande
        ),
    );
    remplacer_largeur_barre(
        &mut noeuds,
        references_reseaux.barre_energie,
        ratio_energie,
    );
    remplacer_fond(
        &mut fonds,
        references_reseaux.barre_energie,
        theme::couleur_ressource((0.25, 0.79, 0.91), ratio_energie),
    );

    remplacer_texte(
        &mut textes,
        references_reseaux.libelle_glace,
        format!("Reserves de glace : {:.0} / {:.0}", glace_stockee, glace_capacite),
    );
    remplacer_largeur_barre(
        &mut noeuds,
        references_reseaux.barre_glace,
        ratio_glace,
    );
    remplacer_fond(
        &mut fonds,
        references_reseaux.barre_glace,
        theme::couleur_ressource((0.54, 0.64, 0.75), ratio_glace),
    );
}

pub(crate) fn rafraichir_hud_construction(
    state: Res<State<GameState>>,
    selected: Res<SelectedBuild>,
    hovered: Res<HoveredCell>,
    feedback: Res<PlacementFeedback>,
    references: Res<ReferencesConstructionHud>,
    mut textes: Query<&mut Text>,
    mut couleurs_texte: Query<&mut TextColor>,
) {
    if !matches!(state.get(), GameState::InGame | GameState::Paused) {
        return;
    }

    if !state.is_changed()
        && !selected.is_changed()
        && !hovered.is_changed()
        && !feedback.is_changed()
    {
        return;
    }

    let metadonnees = metadonnee_slot(selected.0);
    remplacer_texte(
        &mut textes,
        references.module_actif,
        format!("{} // slot {:02}", metadonnees.libelle, metadonnees.numero),
    );
    remplacer_couleur_texte(
        &mut couleurs_texte,
        references.module_actif,
        metadonnees.couleur,
    );

    remplacer_texte(
        &mut textes,
        references.case_survolee,
        formatter_case_survolee(hovered.0),
    );
    remplacer_texte(
        &mut textes,
        references.etat_connexion,
        formatter_connexion(feedback.etat_connexion),
    );
    remplacer_couleur_texte(
        &mut couleurs_texte,
        references.etat_connexion,
        match feedback.etat_connexion {
            EtatConnexionPlacement::Connecte => theme::COULEUR_OK,
            EtatConnexionPlacement::NonConnecte => theme::COULEUR_SURVEILLANCE,
        },
    );

    remplacer_texte(&mut textes, references.feedback, feedback.reason.clone());
    remplacer_couleur_texte(
        &mut couleurs_texte,
        references.feedback,
        match (feedback.valid, feedback.etat_connexion) {
            (true, EtatConnexionPlacement::Connecte) => theme::COULEUR_OK,
            (true, EtatConnexionPlacement::NonConnecte) => theme::COULEUR_SURVEILLANCE,
            (false, _) => theme::COULEUR_DANGER,
        },
    );
}

pub(crate) fn rafraichir_hud_equipage(
    mut commands: Commands,
    state: Res<State<GameState>>,
    time: Res<Time>,
    perf: Res<ParametresPerformanceJeu>,
    police: Res<PoliceInterface>,
    astronauts: Query<&Astronaut>,
    promeneurs: Query<&AstronautePromeneur>,
    enfants: Query<&Children>,
    references: Res<ReferencesEquipageHud>,
    mut cadence: ResMut<CadenceHudEquipage>,
) {
    if !matches!(state.get(), GameState::InGame | GameState::Paused) {
        return;
    }

    let effectif = astronauts.iter().count() + promeneurs.iter().count();
    let doit_rafraichir = effectif != cadence.dernier_effectif
        || time.elapsed_secs_f64() >= cadence.prochain_rafraichissement;
    if !doit_rafraichir {
        return;
    }

    cadence.dernier_effectif = effectif;
    cadence.prochain_rafraichissement =
        time.elapsed_secs_f64() + 1.0 / perf.frequence_hud_equipage_hz.max(1.0);

    if let Ok(enfants_existants) = enfants.get(references.conteneur_cartes) {
        for enfant in enfants_existants.iter() {
            commands.entity(enfant).despawn();
        }
    }

    let mut cartes = astronauts.iter().map(carte_astronaut).collect::<Vec<_>>();
    cartes.extend(promeneurs.iter().map(carte_promeneur));
    trier_cartes(&mut cartes);

    commands
        .entity(references.conteneur_cartes)
        .with_children(|parent| {
            if cartes.is_empty() {
                parent.spawn((
                    Text::new("Aucun astronaute detecte."),
                    police.corps(13.0),
                    TextColor(theme::COULEUR_TEXTE_ATTENUE),
                    theme::style_carte_equipage(),
                    BackgroundColor(theme::COULEUR_PANNEAU_SECONDAIRE),
                    BorderColor::all(theme::COULEUR_BORDURE),
                ));
                return;
            }

            for carte in cartes {
                parent.spawn((
                    Text::new(format!("{}\n{}", carte.titre, carte.detail)),
                    police.corps(13.0),
                    TextColor(theme::COULEUR_TEXTE),
                    theme::style_carte_equipage(),
                    BackgroundColor(theme::COULEUR_PANNEAU_SECONDAIRE),
                    BorderColor::all(carte.accent),
                ));
            }
        });
}

fn remplacer_texte(query: &mut Query<&mut Text>, entite: Entity, valeur: impl Into<String>) {
    let Ok(mut texte) = query.get_mut(entite) else {
        return;
    };
    *texte = valeur.into().into();
}

fn remplacer_fond(query: &mut Query<&mut BackgroundColor>, entite: Entity, couleur: Color) {
    let Ok(mut fond) = query.get_mut(entite) else {
        return;
    };
    *fond = BackgroundColor(couleur);
}

fn remplacer_bordure(query: &mut Query<&mut BorderColor>, entite: Entity, couleur: Color) {
    let Ok(mut bordure) = query.get_mut(entite) else {
        return;
    };
    *bordure = BorderColor::all(couleur);
}

fn remplacer_couleur_texte(query: &mut Query<&mut TextColor>, entite: Entity, couleur: Color) {
    let Ok(mut texte) = query.get_mut(entite) else {
        return;
    };
    texte.0 = couleur;
}

fn remplacer_largeur_barre(query: &mut Query<&mut Node>, entite: Entity, ratio: f32) {
    let Ok(mut noeud) = query.get_mut(entite) else {
        return;
    };
    noeud.width = percent(100.0 * ratio);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::DVec2;

    #[test]
    fn blocage_ui_repose_sur_les_zones_hud_reelles() {
        let mut app = App::new();
        let mut window = Window::default();
        window.resolution.set_physical_resolution(1600, 900);
        window.set_physical_cursor_position(Some(DVec2::new(120.0, 80.0)));
        app.world_mut().spawn((window, PrimaryWindow));
        app.insert_resource(BlocageConstructionParUi(false));
        app.insert_resource(State::new(GameState::InGame));
        app.add_systems(Update, mettre_a_jour_blocage_construction_ui);

        let mut zone = ComputedNode::default();
        zone.size = Vec2::new(220.0, 120.0);
        app.world_mut().spawn((
            ZoneCaptureCurseurUi,
            zone,
            UiGlobalTransform::from_translation(Vec2::new(120.0, 80.0)),
            Visibility::Visible,
        ));

        app.update();
        assert!(app.world().resource::<BlocageConstructionParUi>().0);

        let mut requete = app
            .world_mut()
            .query_filtered::<&mut Window, With<PrimaryWindow>>();
        let mut fenetre = requete
            .single_mut(app.world_mut())
            .expect("fenetre primaire manquante");
        fenetre.set_physical_cursor_position(Some(DVec2::new(500.0, 500.0)));
        drop(fenetre);

        app.update();
        assert!(!app.world().resource::<BlocageConstructionParUi>().0);
    }
}
