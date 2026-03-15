use bevy::prelude::*;

use super::AstronautId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoleAstronaute {
    Commandant,
    Ingenieur,
    Scientifique,
    Logisticien,
    Civil,
}

impl RoleAstronaute {
    pub fn label(self) -> &'static str {
        match self {
            Self::Commandant => "commandant",
            Self::Ingenieur => "ingenieur",
            Self::Scientifique => "scientifique",
            Self::Logisticien => "logisticien",
            Self::Civil => "civil",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfilCompetences {
    pub vitesse_construction: f32,
    pub capacite_transport: f32,
    pub vitesse_extraction: f32,
    pub vitesse_prospection: f32,
    pub vitesse_reparation: f32,
}

impl ProfilCompetences {
    pub const fn new(
        vitesse_construction: f32,
        capacite_transport: f32,
        vitesse_extraction: f32,
        vitesse_prospection: f32,
        vitesse_reparation: f32,
    ) -> Self {
        Self {
            vitesse_construction,
            capacite_transport,
            vitesse_extraction,
            vitesse_prospection,
            vitesse_reparation,
        }
    }
}

impl Default for ProfilCompetences {
    fn default() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfilTraits {
    pub prudence: f32,
    pub discipline: f32,
    pub sociabilite: f32,
    pub curiosite: f32,
    pub endurance: f32,
}

impl ProfilTraits {
    pub const fn new(
        prudence: f32,
        discipline: f32,
        sociabilite: f32,
        curiosite: f32,
        endurance: f32,
    ) -> Self {
        Self {
            prudence,
            discipline,
            sociabilite,
            curiosite,
            endurance,
        }
    }
}

impl Default for ProfilTraits {
    fn default() -> Self {
        Self::new(0.5, 0.5, 0.5, 0.5, 0.5)
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ProfilAstronaute {
    pub role: RoleAstronaute,
    pub competences: ProfilCompetences,
    pub traits: ProfilTraits,
}

impl ProfilAstronaute {
    pub const fn new(
        role: RoleAstronaute,
        competences: ProfilCompetences,
        traits: ProfilTraits,
    ) -> Self {
        Self {
            role,
            competences,
            traits,
        }
    }
}

pub fn profil_astronaute_par_defaut(id: AstronautId, nom: &str) -> ProfilAstronaute {
    match nom {
        "Ari" => ProfilAstronaute::new(
            RoleAstronaute::Commandant,
            ProfilCompetences::new(1.0, 1.0, 0.95, 1.0, 1.0),
            ProfilTraits::new(0.80, 0.90, 0.60, 0.55, 0.75),
        ),
        "Noor" => ProfilAstronaute::new(
            RoleAstronaute::Ingenieur,
            ProfilCompetences::new(1.25, 1.0, 1.0, 0.95, 1.30),
            ProfilTraits::new(0.72, 0.82, 0.48, 0.52, 0.80),
        ),
        "Sora" => ProfilAstronaute::new(
            RoleAstronaute::Scientifique,
            ProfilCompetences::new(0.95, 0.90, 1.10, 1.35, 0.90),
            ProfilTraits::new(0.62, 0.66, 0.42, 0.95, 0.62),
        ),
        _ => profil_role_par_cycle(id),
    }
}

pub fn profil_promeneur_par_defaut(id: AstronautId, nom: &str) -> ProfilAstronaute {
    match nom {
        "Mila" => ProfilAstronaute::new(
            RoleAstronaute::Civil,
            ProfilCompetences::new(0.75, 0.70, 0.70, 0.95, 0.65),
            ProfilTraits::new(0.38, 0.40, 0.95, 0.78, 0.45),
        ),
        _ => profil_role_par_cycle(id),
    }
}

fn profil_role_par_cycle(id: AstronautId) -> ProfilAstronaute {
    match id.0 % 4 {
        0 => ProfilAstronaute::new(
            RoleAstronaute::Ingenieur,
            ProfilCompetences::new(1.10, 1.0, 0.95, 0.90, 1.15),
            ProfilTraits::new(0.70, 0.74, 0.44, 0.48, 0.72),
        ),
        1 => ProfilAstronaute::new(
            RoleAstronaute::Scientifique,
            ProfilCompetences::new(0.92, 0.90, 1.05, 1.18, 0.88),
            ProfilTraits::new(0.60, 0.62, 0.46, 0.86, 0.60),
        ),
        2 => ProfilAstronaute::new(
            RoleAstronaute::Logisticien,
            ProfilCompetences::new(0.95, 1.20, 0.92, 0.88, 0.95),
            ProfilTraits::new(0.74, 0.68, 0.58, 0.40, 0.76),
        ),
        _ => ProfilAstronaute::new(
            RoleAstronaute::Commandant,
            ProfilCompetences::new(1.0, 1.0, 0.95, 1.0, 1.0),
            ProfilTraits::new(0.82, 0.84, 0.55, 0.54, 0.74),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_profil_dari_reflète_le_role_et_les_traits_attendus() {
        let profil = profil_astronaute_par_defaut(AstronautId(0), "Ari");

        assert_eq!(profil.role, RoleAstronaute::Commandant);
        assert!((profil.traits.discipline - 0.90).abs() < 0.001);
        assert!((profil.traits.prudence - 0.80).abs() < 0.001);
        assert!((profil.traits.sociabilite - 0.60).abs() < 0.001);
    }

    #[test]
    fn le_profil_de_mila_reste_civil_et_tres_social() {
        let profil = profil_promeneur_par_defaut(AstronautId(10), "Mila");

        assert_eq!(profil.role, RoleAstronaute::Civil);
        assert!((profil.traits.sociabilite - 0.95).abs() < 0.001);
        assert!((profil.traits.endurance - 0.45).abs() < 0.001);
    }
}
