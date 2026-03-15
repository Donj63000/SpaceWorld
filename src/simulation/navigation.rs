use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use bevy::prelude::*;

use crate::world::TerrainCell;

pub const AIR_MAX_COMBINAISON: f32 = 180.0;
pub const CONSOMMATION_AIR_PAR_CASE_OUVRIER: f32 = 0.25;
pub const MARGE_SECURITE_RETOUR_OUVRIER: f32 = 10.0;
pub const PENTE_MAX_MARCHABLE: f32 = 0.82;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheminCellulaire {
    pub cellules: Vec<IVec2>,
    pub cout: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EtatOuvert {
    position: IVec2,
    cout: i32,
    score_total: i32,
}

impl Ord for EtatOuvert {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .score_total
            .cmp(&self.score_total)
            .then_with(|| other.cout.cmp(&self.cout))
            .then_with(|| other.position.x.cmp(&self.position.x))
            .then_with(|| other.position.y.cmp(&self.position.y))
    }
}

impl PartialOrd for EtatOuvert {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn autonomie_aller_retour_max_cases() -> i32 {
    ((AIR_MAX_COMBINAISON - MARGE_SECURITE_RETOUR_OUVRIER)
        / (CONSOMMATION_AIR_PAR_CASE_OUVRIER * 2.0))
        .floor() as i32
}

pub fn terrain_est_marchable(terrain: &TerrainCell) -> bool {
    !terrain.blocked && terrain.slope <= PENTE_MAX_MARCHABLE
}

pub fn voisins_cardinaux(cellule: IVec2) -> [IVec2; 4] {
    [
        cellule + IVec2::X,
        cellule - IVec2::X,
        cellule + IVec2::Y,
        cellule - IVec2::Y,
    ]
}

pub fn trouver_chemin_a_star<F>(
    depart: IVec2,
    arrivee: IVec2,
    cellules_occupees: &HashSet<IVec2>,
    limite_cout: i32,
    terrain_at: F,
) -> Option<CheminCellulaire>
where
    F: FnMut(IVec2) -> TerrainCell,
{
    trouver_chemin_vers_objectif(
        depart,
        &[arrivee],
        cellules_occupees,
        limite_cout,
        terrain_at,
    )
}

pub fn trouver_chemin_vers_objectif<F>(
    depart: IVec2,
    objectifs: &[IVec2],
    cellules_occupees: &HashSet<IVec2>,
    mut limite_cout: i32,
    mut terrain_at: F,
) -> Option<CheminCellulaire>
where
    F: FnMut(IVec2) -> TerrainCell,
{
    if objectifs.is_empty() {
        return None;
    }

    let objectifs_set: HashSet<_> = objectifs.iter().copied().collect();
    if objectifs_set.contains(&depart) {
        return Some(CheminCellulaire {
            cellules: vec![depart],
            cout: 0,
        });
    }

    if cellules_occupees.contains(&depart) || !terrain_est_marchable(&terrain_at(depart)) {
        return None;
    }

    limite_cout = limite_cout.max(0);

    let mut ouverts = BinaryHeap::new();
    let mut parents = HashMap::<IVec2, IVec2>::new();
    let mut meilleurs_couts = HashMap::<IVec2, i32>::new();

    meilleurs_couts.insert(depart, 0);
    ouverts.push(EtatOuvert {
        position: depart,
        cout: 0,
        score_total: heuristique(depart, &objectifs_set),
    });

    while let Some(etat) = ouverts.pop() {
        if objectifs_set.contains(&etat.position) {
            return Some(reconstruire_chemin(
                depart,
                etat.position,
                etat.cout,
                &parents,
            ));
        }

        if etat.cout > *meilleurs_couts.get(&etat.position).unwrap_or(&i32::MAX) {
            continue;
        }

        for voisin in voisins_cardinaux(etat.position) {
            if cellules_occupees.contains(&voisin) {
                continue;
            }

            let terrain = terrain_at(voisin);
            if !terrain_est_marchable(&terrain) {
                continue;
            }

            let nouveau_cout = etat.cout + 1;
            if nouveau_cout > limite_cout {
                continue;
            }

            if nouveau_cout >= *meilleurs_couts.get(&voisin).unwrap_or(&i32::MAX) {
                continue;
            }

            parents.insert(voisin, etat.position);
            meilleurs_couts.insert(voisin, nouveau_cout);
            ouverts.push(EtatOuvert {
                position: voisin,
                cout: nouveau_cout,
                score_total: nouveau_cout + heuristique(voisin, &objectifs_set),
            });
        }
    }

    None
}

pub fn carte_distances_depuis_origines<F>(
    origines: &[IVec2],
    cellules_occupees: &HashSet<IVec2>,
    limite_cout: i32,
    mut terrain_at: F,
) -> HashMap<IVec2, i32>
where
    F: FnMut(IVec2) -> TerrainCell,
{
    let mut distances = HashMap::new();
    let mut file = VecDeque::new();
    let mut origines_tries = origines.to_vec();
    origines_tries.sort_by_key(|cellule| (cellule.x, cellule.y));
    origines_tries.dedup();

    for origine in origines_tries {
        if cellules_occupees.contains(&origine) {
            continue;
        }

        let terrain = terrain_at(origine);
        if !terrain_est_marchable(&terrain) {
            continue;
        }

        if distances.insert(origine, 0).is_none() {
            file.push_back(origine);
        }
    }

    while let Some(cellule) = file.pop_front() {
        let cout = distances[&cellule];
        if cout >= limite_cout {
            continue;
        }

        for voisin in voisins_cardinaux(cellule) {
            if cellules_occupees.contains(&voisin) || distances.contains_key(&voisin) {
                continue;
            }

            let terrain = terrain_at(voisin);
            if !terrain_est_marchable(&terrain) {
                continue;
            }

            distances.insert(voisin, cout + 1);
            file.push_back(voisin);
        }
    }

    distances
}

fn heuristique(position: IVec2, objectifs: &HashSet<IVec2>) -> i32 {
    objectifs
        .iter()
        .map(|objectif| (*objectif - position).abs().element_sum())
        .min()
        .unwrap_or(0)
}

fn reconstruire_chemin(
    depart: IVec2,
    arrivee: IVec2,
    cout: i32,
    parents: &HashMap<IVec2, IVec2>,
) -> CheminCellulaire {
    let mut cellules = vec![arrivee];
    let mut courant = arrivee;

    while courant != depart {
        let precedent = parents[&courant];
        cellules.push(precedent);
        courant = precedent;
    }

    cellules.reverse();
    CheminCellulaire { cellules, cout }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terrain_libre(_: IVec2) -> TerrainCell {
        TerrainCell {
            height: 0.0,
            slope: 0.0,
            constructible: true,
            resource: None,
            blocked: false,
        }
    }

    #[test]
    fn a_star_contourne_un_obstacle() {
        let cellules_occupees = HashSet::from([IVec2::new(1, 0), IVec2::new(1, 1)]);
        let chemin = trouver_chemin_a_star(
            IVec2::ZERO,
            IVec2::new(2, 1),
            &cellules_occupees,
            12,
            terrain_libre,
        )
        .expect("un chemin est attendu");

        assert_eq!(chemin.cout, 5);
        assert_eq!(chemin.cellules.first().copied(), Some(IVec2::ZERO));
        assert_eq!(chemin.cellules.last().copied(), Some(IVec2::new(2, 1)));
        assert!(
            chemin
                .cellules
                .iter()
                .all(|cellule| !cellules_occupees.contains(cellule))
        );
    }

    #[test]
    fn la_carte_de_distances_part_dune_origine_multiple() {
        let distances = carte_distances_depuis_origines(
            &[IVec2::new(-1, 0), IVec2::new(3, 0)],
            &HashSet::from([IVec2::new(1, 0)]),
            6,
            terrain_libre,
        );

        assert_eq!(distances.get(&IVec2::new(-1, 0)), Some(&0));
        assert_eq!(distances.get(&IVec2::new(3, 0)), Some(&0));
        assert_eq!(distances.get(&IVec2::new(0, 0)), Some(&1));
        assert!(!distances.contains_key(&IVec2::new(1, 0)));
    }
}
