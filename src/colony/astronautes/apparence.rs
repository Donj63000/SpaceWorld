use bevy::prelude::*;

use crate::colony::{
    ProfilAstronaute, RoleAstronaute, profil_astronaute_par_defaut, profil_promeneur_par_defaut,
};

use super::{Astronaut, AstronautePromeneur};

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
    ouvriers: Query<(Entity, &Astronaut, Option<&ProfilAstronaute>), Without<ApparenceAstronaute>>,
    promeneurs: Query<
        (Entity, &AstronautePromeneur, Option<&ProfilAstronaute>),
        Without<ApparenceAstronaute>,
    >,
) {
    for (entity, astronaut, profil) in &ouvriers {
        commands
            .entity(entity)
            .insert(apparence_ouvrier_par_defaut(astronaut, profil.copied()));
    }

    for (entity, promeneur, profil) in &promeneurs {
        commands
            .entity(entity)
            .insert(apparence_promeneur_par_defaut(promeneur, profil.copied()));
    }
}

fn apparence_ouvrier_par_defaut(
    astronaut: &Astronaut,
    profil: Option<ProfilAstronaute>,
) -> ApparenceAstronaute {
    let profil =
        profil.unwrap_or_else(|| profil_astronaute_par_defaut(astronaut.id, astronaut.name));
    let echelle = match astronaut.name {
        "Ari" => 1.03,
        "Noor" => 0.99,
        "Sora" => 0.97,
        _ => echelle_par_role(profil.role),
    };

    ApparenceAstronaute::eva_lourde(profil.role, echelle)
}

fn apparence_promeneur_par_defaut(
    promeneur: &AstronautePromeneur,
    profil: Option<ProfilAstronaute>,
) -> ApparenceAstronaute {
    let profil = profil.unwrap_or_else(|| profil_promeneur_par_defaut(promeneur.id, promeneur.nom));
    let echelle = match promeneur.nom {
        "Mila" => 0.96,
        _ => 0.98,
    };

    ApparenceAstronaute::eva_legere(profil.role, echelle)
}

fn echelle_par_role(role: RoleAstronaute) -> f32 {
    match role {
        RoleAstronaute::Commandant => 1.02,
        RoleAstronaute::Ingenieur => 1.0,
        RoleAstronaute::Scientifique => 0.98,
        RoleAstronaute::Logisticien => 1.01,
        RoleAstronaute::Civil => 0.98,
    }
}
