use std::f32::consts::PI;

use bevy::prelude::*;

use crate::colony::AstronautId;

pub(super) const DUREE_APPROCHE: f32 = 2.7;
pub(super) const DUREE_DESCENTE_FINALE: f32 = 2.4;
pub(super) const DUREE_CONTACT: f32 = 0.9;
pub(super) const DUREE_SORTIE_EQUIPE: f32 = 3.4;
pub(super) const DUREE_FONDU_FINAL: f32 = 1.1;
pub(super) const VITESSE_DEBARQUEMENT: f32 = 1.9;
pub(super) const DISTANCE_ARRIVEE_DEBARQUEMENT: f32 = 0.05;
pub(super) const ZOOM_CAMERA_FINAL: f32 = 38.0;

pub(super) const COULEUR_CIEL_INTRO: (f32, f32, f32) = (0.23, 0.12, 0.11);
pub(super) const COULEUR_CIEL_JEU: (f32, f32, f32) = (0.47, 0.28, 0.20);
pub(super) const COULEUR_AMBIANTE_INTRO: (f32, f32, f32) = (0.46, 0.30, 0.24);
pub(super) const COULEUR_AMBIANTE_JEU: (f32, f32, f32) = (0.64, 0.48, 0.41);
pub(super) const LUMINOSITE_AMBIANTE_INTRO: f32 = 610.0;
pub(super) const LUMINOSITE_AMBIANTE_JEU: f32 = 920.0;

#[derive(Resource, Debug)]
pub(crate) struct SequenceArriveeInitiale {
    pub lander: Entity,
    pub centre_monde: Vec2,
    pub point_sortie: Vec2,
    pub position_mila: Vec2,
    pub temps: f32,
    pub equipage_deploie: usize,
    pub mila_creee: bool,
}

#[derive(Resource, Clone)]
pub(crate) struct MateriauxArriveeInitiale {
    pub flamme: Handle<StandardMaterial>,
    pub poussiere: Handle<StandardMaterial>,
}

#[derive(Component)]
pub(crate) struct EffetArriveeInitiale;

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct FlammePropulseur {
    pub intensite_base: f32,
    pub decalage_temps: f32,
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct NuagePoussiere {
    pub multiplicateur_taille: f32,
    pub decalage_temps: f32,
}

#[derive(Component)]
pub(crate) struct LumierePropulseur;

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct AstronauteDebarquement {
    pub cible: Vec2,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DonneeEquipageArrivee {
    pub id: AstronautId,
    pub nom: &'static str,
    pub position_finale: IVec2,
    pub delai_sortie: f32,
    pub decalage_sortie: Vec2,
}

pub(super) const EQUIPAGE_ARRIVEE: [DonneeEquipageArrivee; 3] = [
    DonneeEquipageArrivee {
        id: AstronautId(0),
        nom: "Ari",
        position_finale: IVec2::new(1, -1),
        delai_sortie: 0.25,
        decalage_sortie: Vec2::new(-0.35, -0.55),
    },
    DonneeEquipageArrivee {
        id: AstronautId(1),
        nom: "Noor",
        position_finale: IVec2::new(2, -1),
        delai_sortie: 0.68,
        decalage_sortie: Vec2::new(0.28, -0.65),
    },
    DonneeEquipageArrivee {
        id: AstronautId(2),
        nom: "Sora",
        position_finale: IVec2::new(-1, 1),
        delai_sortie: 1.18,
        decalage_sortie: Vec2::new(-0.10, 0.18),
    },
];

#[derive(Clone, Copy, Debug)]
pub(super) struct EtatVaisseau {
    pub decalage_monde: Vec2,
    pub altitude: f32,
    pub rotation: Quat,
    pub intensite_propulseurs: f32,
    pub intensite_poussiere: f32,
}

pub(super) fn duree_totale_intro() -> f32 {
    DUREE_APPROCHE + DUREE_DESCENTE_FINALE + DUREE_CONTACT + DUREE_SORTIE_EQUIPE + DUREE_FONDU_FINAL
}

pub(super) fn temps_sortie_equipage(temps: f32) -> f32 {
    (temps - (DUREE_APPROCHE + DUREE_DESCENTE_FINALE + DUREE_CONTACT * 0.35)).max(0.0)
}

pub(super) fn focus_camera_final(centre_monde: Vec2) -> Vec2 {
    centre_monde + Vec2::new(2.2, 1.8)
}

pub(super) fn etat_vaisseau(temps: f32) -> EtatVaisseau {
    if temps <= DUREE_APPROCHE {
        let progression = ease_out_cubic((temps / DUREE_APPROCHE).clamp(0.0, 1.0));
        let decalage = Vec2::new(17.0, -14.0).lerp(Vec2::new(3.1, -2.8), progression);
        let altitude = 34.0 + (12.0 - 34.0) * progression;
        let intensite = 0.96 - 0.12 * progression;
        let balancement = (temps * 2.5).sin() * (1.0 - progression) * 0.08;
        return EtatVaisseau {
            decalage_monde: decalage,
            altitude,
            rotation: Quat::from_rotation_x(-0.11 * (1.0 - progression))
                * Quat::from_rotation_z(balancement)
                * Quat::from_rotation_y(0.08 * (1.0 - progression)),
            intensite_propulseurs: intensite,
            intensite_poussiere: 0.0,
        };
    }

    if temps <= DUREE_APPROCHE + DUREE_DESCENTE_FINALE {
        let progression = ((temps - DUREE_APPROCHE) / DUREE_DESCENTE_FINALE).clamp(0.0, 1.0);
        let progression_lissee = ease_in_out_cubic(progression);
        let tremblement =
            Vec2::new((temps * 4.3).sin(), (temps * 3.1).cos()) * (1.0 - progression) * 0.38;
        let decalage = Vec2::new(3.1, -2.8).lerp(Vec2::ZERO, progression_lissee) + tremblement;
        let altitude = 12.0 + (0.9 - 12.0) * progression_lissee + (1.0 - progression) * 0.55;
        let intensite = 0.84 - 0.22 * progression_lissee;
        let proximite_sol = ((8.0 - altitude) / 8.0).clamp(0.0, 1.0);
        return EtatVaisseau {
            decalage_monde: decalage,
            altitude,
            rotation: Quat::from_rotation_x(-0.07 * (1.0 - progression))
                * Quat::from_rotation_z((temps * 5.4).sin() * (1.0 - progression) * 0.05)
                * Quat::from_rotation_y(-0.03 * progression),
            intensite_propulseurs: intensite,
            intensite_poussiere: proximite_sol.powf(1.4) * intensite,
        };
    }

    let progression =
        ((temps - DUREE_APPROCHE - DUREE_DESCENTE_FINALE) / DUREE_CONTACT).clamp(0.0, 1.0);
    let rebond = ((1.0 - progression) * PI * 2.4).sin().abs() * (1.0 - progression) * 0.22;
    let intensite = (1.0 - progression).powf(1.6) * 0.52;
    EtatVaisseau {
        decalage_monde: Vec2::new(0.0, 0.0),
        altitude: rebond,
        rotation: Quat::from_rotation_z((1.0 - progression) * 0.018),
        intensite_propulseurs: intensite,
        intensite_poussiere: (0.55 + rebond * 2.0) * (1.0 - progression * 0.55),
    }
}

pub(super) fn ease_out_cubic(t: f32) -> f32 {
    let inv = 1.0 - t.clamp(0.0, 1.0);
    1.0 - inv * inv * inv
}

pub(super) fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_vaisseau_termine_sa_descente_au_sol() {
        let etat = etat_vaisseau(DUREE_APPROCHE + DUREE_DESCENTE_FINALE + DUREE_CONTACT);
        assert!(etat.altitude <= 0.001);
        assert!(etat.intensite_propulseurs <= 0.001);
    }

    #[test]
    fn la_poussiere_napparait_qua_proximite_du_sol() {
        let haut = etat_vaisseau(0.4);
        let bas = etat_vaisseau(DUREE_APPROCHE + DUREE_DESCENTE_FINALE * 0.88);

        assert!(haut.intensite_poussiere < 0.01);
        assert!(bas.intensite_poussiere > haut.intensite_poussiere);
    }

    #[test]
    fn la_sortie_dequipage_attend_la_phase_de_pose() {
        assert_eq!(temps_sortie_equipage(DUREE_APPROCHE * 0.9), 0.0);
        assert!(temps_sortie_equipage(DUREE_APPROCHE + DUREE_DESCENTE_FINALE + 0.6) > 0.0);
    }
}
