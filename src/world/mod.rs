mod bruit;
mod coordonnees;
mod decorations;
mod donnees;
mod generation;
mod maillage;
mod parametres;
mod performance_monde;
mod streaming_chunks;
mod survol_terrain;

use bevy::prelude::*;

pub use coordonnees::{
    chunk_local_to_world_cell, continuous_world_to_render_translation, footprint_center,
    structure_anchor_translation, world_cell_to_chunk, world_to_cell, world_to_chunk_coord,
    world_to_render_translation,
};
pub use donnees::{
    ActiveChunks, ChunkCoord, ChunkState, HoveredCell, ResourceDeposit, ResourceKind, TerrainCell,
    WorldCache,
};
pub use generation::generate_chunk;
pub use parametres::{CELL_SIZE_METERS, CHUNK_SIZE_CELLS, PlanetPalette, PlanetProfile, WorldSeed};

use performance_monde::{EtatSurvolTerrain, FileAttenteStreamingChunks};
use streaming_chunks::{
    ChunkVisuals, planifier_streaming_chunks, recenter_origin, setup_world_visuals,
    sync_chunk_decorations, sync_chunk_transforms, traiter_streaming_chunks,
};
use survol_terrain::update_hovered_cell;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WorldSeed(0xA11C_E5EED))
            .insert_resource(PlanetProfile::mars())
            .insert_resource(WorldCache::default())
            .insert_resource(HoveredCell::default())
            .insert_resource(ActiveChunks::default())
            .insert_resource(ChunkVisuals::default())
            .insert_resource(EtatSurvolTerrain::default())
            .insert_resource(FileAttenteStreamingChunks::default())
            .add_systems(Startup, setup_world_visuals)
            .add_systems(
                Update,
                (
                    recenter_origin,
                    planifier_streaming_chunks,
                    traiter_streaming_chunks,
                    sync_chunk_transforms,
                    sync_chunk_decorations,
                    update_hovered_cell,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::decorations::collect_chunk_decorations;

    fn find_resource_cell(
        cache: &mut WorldCache,
        profile: &PlanetProfile,
        seed: WorldSeed,
    ) -> IVec2 {
        (-64..64)
            .flat_map(|y| (-64..64).map(move |x| IVec2::new(x, y)))
            .find(|cell| cache.terrain_at(*cell, profile, seed).resource.is_some())
            .expect("le monde de test doit exposer au moins un gisement de glace")
    }

    #[test]
    fn la_generation_de_chunk_est_deterministe_pour_une_seed() {
        let profile = PlanetProfile::mars();
        let seed = WorldSeed(42);
        let a = generate_chunk(&profile, seed, ChunkCoord { x: 1, y: -2 });
        let b = generate_chunk(&profile, seed, ChunkCoord { x: 1, y: -2 });
        assert_eq!(a.cells, b.cells);
        assert_eq!(a.resource_cells(), b.resource_cells());
    }

    #[test]
    fn un_chunk_modifie_reste_persistant_dans_le_cache() {
        let profile = PlanetProfile::mars();
        let seed = WorldSeed(42);
        let mut cache = WorldCache::default();
        let target = find_resource_cell(&mut cache, &profile, seed);
        let extracted_before = cache.extract_resource(target, &profile, seed, 1);
        let after_reload = cache.extract_resource(target, &profile, seed, 1);
        assert_eq!(extracted_before, 1);
        assert!(after_reload <= 1);
        let terrain = cache.terrain_at(target, &profile, seed);
        let remaining = terrain
            .resource
            .map(|resource| resource.amount)
            .unwrap_or(0);
        assert!(remaining < 14);
    }

    #[test]
    fn les_decorations_de_chunk_sont_deterministes() {
        let profile = PlanetProfile::mars();
        let seed = WorldSeed(42);
        let chunk = generate_chunk(&profile, seed, ChunkCoord { x: 2, y: -1 });

        let a = collect_chunk_decorations(&chunk, &profile, seed);
        let b = collect_chunk_decorations(&chunk, &profile, seed);

        assert_eq!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn un_gisement_epuise_est_retire_du_cache_des_ressources() {
        let profile = PlanetProfile::mars();
        let seed = WorldSeed(42);
        let mut cache = WorldCache::default();
        let target = find_resource_cell(&mut cache, &profile, seed);
        let (coord, local) = world_cell_to_chunk(target);
        let starting_amount = cache
            .terrain_at(target, &profile, seed)
            .resource
            .map(|resource| resource.amount)
            .unwrap_or(0);

        let extracted = cache.extract_resource(target, &profile, seed, starting_amount);
        let chunk = cache.ensure_chunk(coord, &profile, seed);

        assert_eq!(extracted, starting_amount);
        assert!(!chunk.resource_cells().contains(&local));
    }

    #[test]
    fn une_extraction_partielle_conserve_lentree_du_gisement() {
        let profile = PlanetProfile::mars();
        let seed = WorldSeed(42);
        let mut cache = WorldCache::default();
        let target = find_resource_cell(&mut cache, &profile, seed);
        let (coord, local) = world_cell_to_chunk(target);

        let extracted = cache.extract_resource(target, &profile, seed, 1);
        let chunk = cache.ensure_chunk(coord, &profile, seed);

        assert_eq!(extracted, 1);
        assert!(chunk.resource_cells().contains(&local));
    }
}
