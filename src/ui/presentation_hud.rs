use bevy::prelude::*;

use crate::colony::{
    Astronaut, AstronautStatus, AstronautePromeneur, EtatPromenade, LifeSupportNetwork,
};
use crate::construction::EtatConnexionPlacement;
use crate::core::GameState;

use super::theme_cockpit as theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NiveauMission {
    Nominal,
    Surveillance,
    Critique,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CarteEquipageHud {
    pub(crate) titre: String,
    pub(crate) detail: String,
    pub(crate) accent: Color,
}

pub(crate) fn niveau_mission(primary: Option<&LifeSupportNetwork>) -> NiveauMission {
    let Some(reseau) = primary else {
        return NiveauMission::Critique;
    };

    let ratio_oxygene = theme::ratio(reseau.oxygen_stored, reseau.oxygen_capacity);
    let ratio_glace = theme::ratio(reseau.ice_stored, reseau.ice_capacity);
    let ratio_energie = if reseau.energy_demand <= f32::EPSILON {
        1.0
    } else {
        (reseau.energy_generation / reseau.energy_demand).clamp(0.0, 1.0)
    };

    if ratio_oxygene <= 0.20 || ratio_energie <= 0.55 || reseau.alerts.len() >= 2 {
        NiveauMission::Critique
    } else if ratio_oxygene <= 0.45
        || ratio_glace <= 0.20
        || ratio_energie < 1.0
        || !reseau.alerts.is_empty()
    {
        NiveauMission::Surveillance
    } else {
        NiveauMission::Nominal
    }
}

pub(crate) fn couleur_niveau_mission(niveau: NiveauMission) -> Color {
    match niveau {
        NiveauMission::Nominal => theme::COULEUR_OK,
        NiveauMission::Surveillance => theme::COULEUR_SURVEILLANCE,
        NiveauMission::Critique => theme::COULEUR_DANGER,
    }
}

pub(crate) fn libelle_niveau_mission(niveau: NiveauMission) -> &'static str {
    match niveau {
        NiveauMission::Nominal => "NOMINAL",
        NiveauMission::Surveillance => "SURVEILLANCE",
        NiveauMission::Critique => "CRITIQUE",
    }
}

pub(crate) fn formatter_resume_mission(
    state: &GameState,
    nombre_structures: usize,
    nombre_taches: usize,
) -> String {
    format!(
        "{} | {} structures | {} taches",
        libelle_etat_simulation(state),
        nombre_structures,
        nombre_taches
    )
}

pub(crate) fn objectif_mission(primary: Option<&LifeSupportNetwork>) -> String {
    primary
        .and_then(|reseau| reseau.alerts.first())
        .cloned()
        .unwrap_or_else(|| "Stabilise la colonie et etends le reseau vital.".into())
}

pub(crate) fn formatter_detail_alertes(
    primary: Option<&LifeSupportNetwork>,
    reseaux_secondaires: usize,
) -> String {
    lignes_alertes(primary, reseaux_secondaires).join("\n")
}

pub(crate) fn formatter_case_survolee(cellule: Option<IVec2>) -> String {
    cellule
        .map(|cellule| format!("Cellule cible : {}, {}", cellule.x, cellule.y))
        .unwrap_or_else(|| "Cellule cible : --".into())
}

pub(crate) fn formatter_connexion(etat: EtatConnexionPlacement) -> &'static str {
    match etat {
        EtatConnexionPlacement::Connecte => "Connexion reseau : valide",
        EtatConnexionPlacement::NonConnecte => "Connexion reseau : relais requis",
    }
}

pub(crate) fn carte_astronaut(astronaut: &Astronaut) -> CarteEquipageHud {
    CarteEquipageHud {
        titre: astronaut.name.into(),
        detail: format!(
            "{:>3.0}% O2 | {}{}",
            astronaut.suit_oxygen,
            libelle_statut_astronaute(astronaut.status),
            suffixe_charge_glace(astronaut.carrying_ice)
        ),
        accent: match astronaut.status {
            AstronautStatus::Idle => theme::COULEUR_OK,
            AstronautStatus::Moving => theme::COULEUR_ACCENT_ACIER,
            AstronautStatus::Working => theme::COULEUR_ACCENT_OXYDE,
            AstronautStatus::Returning => theme::COULEUR_SURVEILLANCE,
            AstronautStatus::Dead => theme::COULEUR_DANGER,
        },
    }
}

pub(crate) fn carte_promeneur(promeneur: &AstronautePromeneur) -> CarteEquipageHud {
    CarteEquipageHud {
        titre: promeneur.nom.into(),
        detail: format!(
            "{:>3.0}% O2 | {}",
            promeneur.air_combinaison,
            libelle_etat_promeneur(promeneur.etat)
        ),
        accent: match promeneur.etat {
            EtatPromenade::Promenade => theme::COULEUR_ACCENT_ACIER,
            EtatPromenade::Pause => theme::COULEUR_ACCENT_CYAN,
            EtatPromenade::RetourAbri => theme::COULEUR_SURVEILLANCE,
            EtatPromenade::Abri => theme::COULEUR_OK,
        },
    }
}

pub(crate) fn trier_cartes(cartes: &mut [CarteEquipageHud]) {
    cartes.sort_by(|gauche, droite| gauche.titre.cmp(&droite.titre));
}

fn lignes_alertes(
    primary: Option<&LifeSupportNetwork>,
    reseaux_secondaires: usize,
) -> Vec<String> {
    let mut lignes = Vec::new();

    match primary {
        Some(reseau) if reseau.alerts.is_empty() => {
            lignes.push("Aucune alerte active. Reseau vital stable.".to_string())
        }
        Some(reseau) => lignes.extend(reseau.alerts.iter().cloned()),
        None => lignes.push("Aucun reseau vital connecte.".to_string()),
    }

    if reseaux_secondaires > 0 {
        lignes.push(format!(
            "{} reseau(x) secondaire(s) isole(s).",
            reseaux_secondaires
        ));
    }

    lignes
}

fn libelle_etat_simulation(state: &GameState) -> &'static str {
    match state {
        GameState::Boot => "Boot",
        GameState::Intro => "Atterrissage",
        GameState::InGame => "En jeu",
        GameState::Paused => "Pause",
    }
}

fn libelle_statut_astronaute(status: AstronautStatus) -> &'static str {
    match status {
        AstronautStatus::Idle => "en attente",
        AstronautStatus::Moving => "en deplacement",
        AstronautStatus::Working => "au travail",
        AstronautStatus::Returning => "retour base",
        AstronautStatus::Dead => "hors service",
    }
}

fn libelle_etat_promeneur(etat: EtatPromenade) -> &'static str {
    match etat {
        EtatPromenade::Promenade => "promenade",
        EtatPromenade::Pause => "pause",
        EtatPromenade::RetourAbri => "retour abri",
        EtatPromenade::Abri => "abri",
    }
}

fn suffixe_charge_glace(charge: f32) -> String {
    if charge <= 0.0 {
        String::new()
    } else {
        format!(" | +{:.0} glace", charge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::{AstronautId, TaskId};

    #[test]
    fn niveau_mission_detecte_une_situation_critique() {
        let reseau = LifeSupportNetwork {
            oxygen_capacity: 100.0,
            oxygen_stored: 12.0,
            ice_capacity: 10.0,
            ice_stored: 6.0,
            energy_generation: 2.0,
            energy_demand: 6.0,
            ..default()
        };

        assert_eq!(niveau_mission(Some(&reseau)), NiveauMission::Critique);
    }

    #[test]
    fn formatter_detail_alertes_ajoute_les_reseaux_secondaires() {
        let reseau = LifeSupportNetwork {
            alerts: vec!["O2 faible".into()],
            ..default()
        };

        assert_eq!(
            formatter_detail_alertes(Some(&reseau), 2),
            "O2 faible\n2 reseau(x) secondaire(s) isole(s)."
        );
    }

    #[test]
    fn carte_astronaut_affiche_oxygene_statut_et_charge() {
        let astronaut = Astronaut {
            id: AstronautId(7),
            name: "Ariane",
            suit_oxygen: 84.0,
            current_task: Some(TaskId(1)),
            status: AstronautStatus::Working,
            carrying_ice: 3.0,
        };

        let carte = carte_astronaut(&astronaut);
        assert_eq!(carte.titre, "Ariane");
        assert_eq!(carte.detail, " 84% O2 | au travail | +3 glace");
        assert_eq!(carte.accent, theme::COULEUR_ACCENT_OXYDE);
    }
}
