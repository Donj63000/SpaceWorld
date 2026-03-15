use std::collections::HashSet;

use bevy::prelude::*;

use crate::simulation::{
    carte_distances_depuis_origines, terrain_est_marchable, trouver_chemin_vers_objectif,
};
use crate::world::TerrainCell;

use super::astronautes::ZoneRechargeBase;
use super::{StructureId, StructureKind, StructureState, structure_cells};

pub fn cellules_occupees_structures(structures: &[StructureState]) -> HashSet<IVec2> {
    structures
        .iter()
        .flat_map(|structure| structure.occupied_cells())
        .collect()
}

pub fn cellules_origine_base(zone_recharge: &ZoneRechargeBase) -> Vec<IVec2> {
    let mut origines = zone_recharge.cellules().to_vec();
    origines.sort_by_key(|cellule| (cellule.x, cellule.y));
    origines.dedup();
    origines
}

pub fn cellules_interaction_structure<F>(
    kind: StructureKind,
    anchor: IVec2,
    structures: &[StructureState],
    mut terrain_at: F,
) -> Vec<IVec2>
where
    F: FnMut(IVec2) -> TerrainCell,
{
    let emprise = structure_cells(kind, anchor);
    let occupees = cellules_occupees_structures(structures);
    let mut interactions = HashSet::new();

    for cellule in &emprise {
        for voisin in crate::simulation::voisins_cardinaux(*cellule) {
            if emprise.contains(&voisin) || occupees.contains(&voisin) {
                continue;
            }

            let terrain = terrain_at(voisin);
            if terrain_est_marchable(&terrain) {
                interactions.insert(voisin);
            }
        }
    }

    let mut interactions = interactions.into_iter().collect::<Vec<_>>();
    interactions.sort_by_key(|cellule| {
        (
            (*cellule - anchor).abs().element_sum(),
            cellule.x,
            cellule.y,
        )
    });
    interactions
}

pub fn meilleure_cellule_interaction<F>(
    kind: StructureKind,
    anchor: IVec2,
    structures: &[StructureState],
    origines: &[IVec2],
    limite_cout: i32,
    mut terrain_at: F,
) -> Option<(IVec2, i32)>
where
    F: FnMut(IVec2) -> TerrainCell,
{
    let interactions = cellules_interaction_structure(kind, anchor, structures, &mut terrain_at);
    if interactions.is_empty() {
        return None;
    }

    let mut cellules_occupees = cellules_occupees_structures(structures);
    cellules_occupees.extend(structure_cells(kind, anchor));
    let distances =
        carte_distances_depuis_origines(origines, &cellules_occupees, limite_cout, &mut terrain_at);

    interactions
        .into_iter()
        .filter_map(|cellule| distances.get(&cellule).copied().map(|cout| (cellule, cout)))
        .min_by_key(|(cellule, cout)| (*cout, cellule.x, cellule.y))
}

pub fn meilleure_cellule_interaction_structure<F>(
    structure: StructureId,
    structures: &[StructureState],
    origines: &[IVec2],
    limite_cout: i32,
    terrain_at: F,
) -> Option<(IVec2, i32)>
where
    F: FnMut(IVec2) -> TerrainCell,
{
    let structure = structures.iter().find(|etat| etat.id == structure)?;
    meilleure_cellule_interaction(
        structure.kind,
        structure.anchor,
        structures,
        origines,
        limite_cout,
        terrain_at,
    )
}

pub fn trouver_chemin_vers_interaction<F>(
    depart: IVec2,
    kind: StructureKind,
    anchor: IVec2,
    structures: &[StructureState],
    limite_cout: i32,
    mut terrain_at: F,
) -> Option<crate::simulation::CheminCellulaire>
where
    F: FnMut(IVec2) -> TerrainCell,
{
    let interactions = cellules_interaction_structure(kind, anchor, structures, &mut terrain_at);
    if interactions.is_empty() {
        return None;
    }

    let mut cellules_occupees = cellules_occupees_structures(structures);
    cellules_occupees.extend(structure_cells(kind, anchor));
    trouver_chemin_vers_objectif(
        depart,
        &interactions,
        &cellules_occupees,
        limite_cout,
        terrain_at,
    )
}

pub fn structure_rejoint_reseau_principal(
    kind: StructureKind,
    anchor: IVec2,
    structures: &[StructureState],
) -> bool {
    let structures_primaires = structures
        .iter()
        .filter(|structure| structure.built && structure.network_id == Some(0))
        .collect::<Vec<_>>();

    if structures_primaires.is_empty() {
        return true;
    }

    let cellules_occupees = cellules_occupees_structures(structures);
    structures_primaires.into_iter().any(|structure| {
        structures_connectees_par_proximite(
            kind,
            anchor,
            structure.kind,
            structure.anchor,
            &cellules_occupees,
        )
    })
}

pub fn structures_connectees_par_proximite(
    kind_a: StructureKind,
    anchor_a: IVec2,
    kind_b: StructureKind,
    anchor_b: IVec2,
    cellules_occupees: &HashSet<IVec2>,
) -> bool {
    let cellules_a = structure_cells(kind_a, anchor_a);
    let cellules_b = structure_cells(kind_b, anchor_b);

    cellules_a.iter().any(|cellule_a| {
        cellules_b.iter().any(|cellule_b| {
            let delta = *cellule_b - *cellule_a;
            let distance = delta.abs().element_sum();

            if distance == 1 {
                return true;
            }

            if distance != 2 || (delta.x != 0 && delta.y != 0) {
                return false;
            }

            let pont = *cellule_a + IVec2::new(delta.x.signum(), delta.y.signum());
            !cellules_occupees.contains(&pont)
        })
    })
}
