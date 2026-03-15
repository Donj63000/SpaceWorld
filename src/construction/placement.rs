use std::collections::HashSet;

use bevy::prelude::*;

use crate::colony::{
    StructureKind, StructureState, ZoneRechargeBase,
    cellules_interaction_structure as cellules_interaction_colonie, cellules_origine_base,
    meilleure_cellule_interaction as meilleure_interaction_colonie, structure_cells,
    structure_rejoint_reseau_principal,
};
use crate::simulation::{autonomie_aller_retour_max_cases, carte_distances_depuis_origines};
use crate::world::TerrainCell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EtatConnexionPlacement {
    #[default]
    Connecte,
    NonConnecte,
}

#[derive(Clone, Debug, Default)]
pub struct PlacementValidation {
    pub valid: bool,
    pub reason: String,
    pub cellule_interaction: Option<IVec2>,
    pub etat_connexion: EtatConnexionPlacement,
}

pub fn cellules_interaction_structure<F>(
    kind: StructureKind,
    anchor: IVec2,
    structures: &[StructureState],
    terrain_at: F,
) -> Vec<IVec2>
where
    F: FnMut(IVec2) -> TerrainCell,
{
    cellules_interaction_colonie(kind, anchor, structures, terrain_at)
}

pub fn meilleure_cellule_interaction<F>(
    kind: StructureKind,
    anchor: IVec2,
    structures: &[StructureState],
    origines: &[IVec2],
    limite_cout: i32,
    terrain_at: F,
) -> Option<(IVec2, i32)>
where
    F: FnMut(IVec2) -> TerrainCell,
{
    meilleure_interaction_colonie(kind, anchor, structures, origines, limite_cout, terrain_at)
}

pub fn cout_trajet_base_vers<F>(
    cellule: IVec2,
    origines: &[IVec2],
    cellules_occupees: &HashSet<IVec2>,
    limite_cout: i32,
    terrain_at: F,
) -> Option<i32>
where
    F: FnMut(IVec2) -> TerrainCell,
{
    carte_distances_depuis_origines(origines, cellules_occupees, limite_cout, terrain_at)
        .get(&cellule)
        .copied()
}

pub fn validate_structure_placement<F>(
    kind: StructureKind,
    anchor: IVec2,
    structures: &[StructureState],
    zone_recharge: &ZoneRechargeBase,
    mut terrain_at: F,
) -> PlacementValidation
where
    F: FnMut(IVec2) -> TerrainCell,
{
    let emprise = structure_cells(kind, anchor);

    for structure in structures {
        let emprise_existante = structure.occupied_cells();
        if emprise
            .iter()
            .any(|cellule| emprise_existante.contains(cellule))
        {
            return PlacementValidation {
                valid: false,
                reason: "Case deja occupee.".into(),
                cellule_interaction: None,
                etat_connexion: EtatConnexionPlacement::Connecte,
            };
        }
    }

    for &cellule in &emprise {
        let terrain = terrain_at(cellule);
        if !terrain.constructible || terrain.blocked {
            return PlacementValidation {
                valid: false,
                reason: "Terrain trop pentu ou bloque.".into(),
                cellule_interaction: None,
                etat_connexion: EtatConnexionPlacement::Connecte,
            };
        }
    }

    let interactions = cellules_interaction_structure(kind, anchor, structures, &mut terrain_at);
    if interactions.is_empty() {
        return PlacementValidation {
            valid: false,
            reason: "Aucun acces pieton adjacent.".into(),
            cellule_interaction: None,
            etat_connexion: EtatConnexionPlacement::Connecte,
        };
    }

    let origines = cellules_origine_base(zone_recharge);
    if origines.is_empty() {
        return PlacementValidation {
            valid: false,
            reason: "Aucune zone de travail disponible depuis la base.".into(),
            cellule_interaction: None,
            etat_connexion: EtatConnexionPlacement::Connecte,
        };
    }

    let mut cellules_occupees = structures
        .iter()
        .flat_map(|structure| structure.occupied_cells())
        .collect::<HashSet<_>>();
    cellules_occupees.extend(emprise.iter().copied());

    let limite_eva = autonomie_aller_retour_max_cases();
    if let Some((cellule_interaction, cout)) = meilleure_cellule_interaction(
        kind,
        anchor,
        structures,
        &origines,
        limite_eva,
        &mut terrain_at,
    ) {
        let etat_connexion = if structure_rejoint_reseau_principal(kind, anchor, structures) {
            EtatConnexionPlacement::Connecte
        } else {
            EtatConnexionPlacement::NonConnecte
        };

        let reason = if etat_connexion == EtatConnexionPlacement::Connecte {
            format!("Placement valide. Acces ouvrier en {cout} case(s).")
        } else {
            format!(
                "Placement valide mais module non relie au reseau principal. Acces ouvrier en {cout} case(s)."
            )
        };

        return PlacementValidation {
            valid: true,
            reason,
            cellule_interaction: Some(cellule_interaction),
            etat_connexion,
        };
    }

    let limite_recherche = limite_eva + 128;
    if interactions.iter().any(|cellule| {
        cout_trajet_base_vers(
            *cellule,
            &origines,
            &cellules_occupees,
            limite_recherche,
            &mut terrain_at,
        )
        .is_some()
    }) {
        return PlacementValidation {
            valid: false,
            reason: "Hors portee EVA sure.".into(),
            cellule_interaction: None,
            etat_connexion: EtatConnexionPlacement::Connecte,
        };
    }

    PlacementValidation {
        valid: false,
        reason: "Zone inaccessible a pied depuis la base.".into(),
        cellule_interaction: None,
        etat_connexion: EtatConnexionPlacement::Connecte,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terrain_libre(_: IVec2) -> TerrainCell {
        TerrainCell {
            height: 0.0,
            slope: 0.1,
            constructible: true,
            resource: None,
            blocked: false,
        }
    }

    #[test]
    fn refuse_un_placement_sur_case_occupee() {
        let structures = vec![StructureState {
            id: crate::colony::StructureId(1),
            kind: StructureKind::Lander,
            anchor: IVec2::ZERO,
            built: true,
            build_progress: 1.0,
            network_id: Some(0),
            internal_ice: 0.0,
        }];

        let validation = validate_structure_placement(
            StructureKind::Storage,
            IVec2::new(1, 0),
            &structures,
            &ZoneRechargeBase::default(),
            terrain_libre,
        );

        assert!(!validation.valid);
    }

    #[test]
    fn signale_un_module_valide_mais_non_connecte() {
        let structures = vec![StructureState {
            id: crate::colony::StructureId(1),
            kind: StructureKind::Lander,
            anchor: IVec2::ZERO,
            built: true,
            build_progress: 1.0,
            network_id: Some(0),
            internal_ice: 0.0,
        }];
        let zone = ZoneRechargeBase::depuis_cellules(vec![IVec2::new(-1, 0), IVec2::new(0, -1)]);

        let validation = validate_structure_placement(
            StructureKind::Storage,
            IVec2::new(4, 0),
            &structures,
            &zone,
            terrain_libre,
        );

        assert!(validation.valid);
        assert_eq!(
            validation.etat_connexion,
            EtatConnexionPlacement::NonConnecte
        );
    }

    #[test]
    fn refuse_un_module_inaccessible_depuis_la_base() {
        let zone = ZoneRechargeBase::depuis_cellules(vec![IVec2::new(0, 0)]);
        let validation = validate_structure_placement(
            StructureKind::Storage,
            IVec2::new(2, 0),
            &[],
            &zone,
            |cellule| TerrainCell {
                height: 0.0,
                slope: 0.1,
                constructible: true,
                resource: None,
                blocked: cellule.x == 1,
            },
        );

        assert!(!validation.valid);
        assert!(validation.reason.contains("inaccessible"));
    }

    #[test]
    fn refuse_un_module_hors_portee_eva() {
        let zone = ZoneRechargeBase::depuis_cellules(vec![IVec2::new(0, 0)]);
        let validation = validate_structure_placement(
            StructureKind::Storage,
            IVec2::new(360, 0),
            &[],
            &zone,
            terrain_libre,
        );

        assert!(!validation.valid);
        assert!(validation.reason.contains("EVA"));
    }
}
