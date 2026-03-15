use bevy::prelude::*;

use super::{Astronaut, AstronautePromeneur};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeCombinaison {
    Interieure,
    Vehicule,
    EvaPlanetaireLegere,
    EvaPlanetaireLourde,
}

impl TypeCombinaison {
    pub fn label(self) -> &'static str {
        match self {
            Self::Interieure => "interieure",
            Self::Vehicule => "vehicule",
            Self::EvaPlanetaireLegere => "eva_legere",
            Self::EvaPlanetaireLourde => "eva_lourde",
        }
    }

    pub fn est_eva(self) -> bool {
        matches!(self, Self::EvaPlanetaireLegere | Self::EvaPlanetaireLourde)
    }

    pub fn casque_integral(self) -> bool {
        !matches!(self, Self::Interieure)
    }

    pub fn sac_dorsal(self) -> bool {
        matches!(self, Self::EvaPlanetaireLegere | Self::EvaPlanetaireLourde)
    }

    pub fn amplitude_pas(self) -> f32 {
        match self {
            Self::Interieure => 1.10,
            Self::Vehicule => 0.94,
            Self::EvaPlanetaireLegere => 0.88,
            Self::EvaPlanetaireLourde => 0.76,
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct ApparenceAstronaute {
    pub role: RoleAstronaute,
    pub combinaison: TypeCombinaison,
    pub echelle: f32,
}

impl ApparenceAstronaute {
    pub fn eva_lourde(role: RoleAstronaute, echelle: f32) -> Self {
        Self {
            role,
            combinaison: TypeCombinaison::EvaPlanetaireLourde,
            echelle,
        }
    }

    pub fn eva_legere(role: RoleAstronaute, echelle: f32) -> Self {
        Self {
            role,
            combinaison: TypeCombinaison::EvaPlanetaireLegere,
            echelle,
        }
    }

    pub fn vehicule(role: RoleAstronaute, echelle: f32) -> Self {
        Self {
            role,
            combinaison: TypeCombinaison::Vehicule,
            echelle,
        }
    }

    pub fn interieure(role: RoleAstronaute, echelle: f32) -> Self {
        Self {
            role,
            combinaison: TypeCombinaison::Interieure,
            echelle,
        }
    }
}

pub fn initialiser_apparences_astronautes(
    mut commands: Commands,
    ouvriers: Query<(Entity, &Astronaut), Without<ApparenceAstronaute>>,
    promeneurs: Query<(Entity, &AstronautePromeneur), Without<ApparenceAstronaute>>,
) {
    for (entity, astronaut) in &ouvriers {
        commands
            .entity(entity)
            .insert(apparence_ouvrier_par_defaut(astronaut));
    }

    for (entity, promeneur) in &promeneurs {
        commands
            .entity(entity)
            .insert(apparence_promeneur_par_defaut(promeneur));
    }
}

fn apparence_ouvrier_par_defaut(astronaut: &Astronaut) -> ApparenceAstronaute {
    match astronaut.name {
        "Ari" => ApparenceAstronaute::eva_lourde(RoleAstronaute::Commandant, 1.03),
        "Noor" => ApparenceAstronaute::eva_lourde(RoleAstronaute::Ingenieur, 0.99),
        "Sora" => ApparenceAstronaute::eva_lourde(RoleAstronaute::Scientifique, 0.97),
        _ => match astronaut.id.0 % 4 {
            0 => ApparenceAstronaute::eva_lourde(RoleAstronaute::Ingenieur, 1.0),
            1 => ApparenceAstronaute::eva_lourde(RoleAstronaute::Scientifique, 0.98),
            2 => ApparenceAstronaute::eva_lourde(RoleAstronaute::Logisticien, 1.01),
            _ => ApparenceAstronaute::eva_lourde(RoleAstronaute::Commandant, 1.02),
        },
    }
}

fn apparence_promeneur_par_defaut(promeneur: &AstronautePromeneur) -> ApparenceAstronaute {
    match promeneur.nom {
        "Mila" => ApparenceAstronaute::eva_legere(RoleAstronaute::Civil, 0.96),
        _ => ApparenceAstronaute::eva_legere(RoleAstronaute::Civil, 0.98),
    }
}
