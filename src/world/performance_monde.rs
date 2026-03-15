use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;

use super::donnees::ChunkCoord;

#[derive(Resource, Debug, Default, Clone)]
pub(crate) struct EtatSurvolTerrain {
    pub dernier_curseur: Option<Vec2>,
    pub derniere_camera_translation: Option<Vec3>,
    pub derniere_camera_rotation: Option<Quat>,
    pub derniere_origine: Vec2,
    pub prochain_raycast_a: f64,
}

#[derive(Resource, Debug, Default, Clone)]
pub(crate) struct FileAttenteStreamingChunks {
    pub centre_planifie: Option<ChunkCoord>,
    pub coords_requis: Vec<ChunkCoord>,
    pub ensemble_requis: HashSet<ChunkCoord>,
    pub terrains: VecDeque<ChunkCoord>,
    pub decors: VecDeque<ChunkCoord>,
}

pub(crate) fn coordonnees_chunks_requises(centre: ChunkCoord, rayon: i32) -> Vec<ChunkCoord> {
    let mut coords = Vec::new();
    for dx in -rayon..=rayon {
        for dy in -rayon..=rayon {
            coords.push(ChunkCoord {
                x: centre.x + dx,
                y: centre.y + dy,
            });
        }
    }

    coords.sort_by_key(|coord| {
        (
            (coord.x - centre.x).abs() + (coord.y - centre.y).abs(),
            coord.y,
            coord.x,
        )
    });
    coords
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_chunks_requis_sont_tries_par_distance_puis_coordonnees() {
        let centre = ChunkCoord { x: 4, y: -2 };
        let coords = coordonnees_chunks_requises(centre, 1);

        assert_eq!(coords[0], centre);
        assert_eq!(
            coords,
            vec![
                ChunkCoord { x: 4, y: -2 },
                ChunkCoord { x: 4, y: -3 },
                ChunkCoord { x: 3, y: -2 },
                ChunkCoord { x: 5, y: -2 },
                ChunkCoord { x: 4, y: -1 },
                ChunkCoord { x: 3, y: -3 },
                ChunkCoord { x: 5, y: -3 },
                ChunkCoord { x: 3, y: -1 },
                ChunkCoord { x: 5, y: -1 },
            ]
        );
    }
}
