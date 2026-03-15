use bevy::prelude::*;

use crate::core::WorldOrigin;

use super::generation::sample_height;
use super::{CHUNK_SIZE_CELLS, ChunkCoord, PlanetProfile, WorldCache, WorldSeed};

#[inline]
pub fn footprint_center(cells: &[IVec2], cell_size: f32) -> Vec2 {
    let min_x = cells.iter().map(|cell| cell.x).min().unwrap_or(0) as f32;
    let max_x = cells.iter().map(|cell| cell.x).max().unwrap_or(0) as f32 + 1.0;
    let min_y = cells.iter().map(|cell| cell.y).min().unwrap_or(0) as f32;
    let max_y = cells.iter().map(|cell| cell.y).max().unwrap_or(0) as f32 + 1.0;
    Vec2::new(
        (min_x + max_x) * 0.5 * cell_size,
        (min_y + max_y) * 0.5 * cell_size,
    )
}

#[inline]
pub fn structure_anchor_translation(
    cells: &[IVec2],
    world_cache: &mut WorldCache,
    profile: &PlanetProfile,
    seed: WorldSeed,
    origin: &WorldOrigin,
) -> Vec3 {
    let center = footprint_center(cells, profile.cell_size_meters);
    let height = world_cache.average_height_for_cells(cells, profile, seed);
    Vec3::new(center.x - origin.0.x, height + 0.28, center.y - origin.0.y)
}

#[inline]
pub fn world_to_render_translation(
    cell: IVec2,
    height: f32,
    profile: &PlanetProfile,
    origin: &WorldOrigin,
) -> Vec3 {
    let center = Vec2::new(
        (cell.x as f32 + 0.5) * profile.cell_size_meters,
        (cell.y as f32 + 0.5) * profile.cell_size_meters,
    );
    Vec3::new(center.x - origin.0.x, height, center.y - origin.0.y)
}

#[inline]
pub fn continuous_world_to_render_translation(
    world: Vec2,
    height_offset: f32,
    _world_cache: &mut WorldCache,
    profile: &PlanetProfile,
    seed: WorldSeed,
    origin: &WorldOrigin,
) -> Vec3 {
    Vec3::new(
        world.x - origin.0.x,
        sample_height_at_world(world, profile, seed) + height_offset,
        world.y - origin.0.y,
    )
}

#[inline]
pub fn world_to_cell(world: Vec2, cell_size: f32) -> IVec2 {
    IVec2::new(
        (world.x / cell_size).floor() as i32,
        (world.y / cell_size).floor() as i32,
    )
}

#[inline]
pub fn world_to_chunk_coord(world: Vec2, profile: &PlanetProfile) -> ChunkCoord {
    world_cell_to_chunk(world_to_cell(world, profile.cell_size_meters)).0
}

#[inline]
pub fn world_cell_to_chunk(cell: IVec2) -> (ChunkCoord, UVec2) {
    let chunk_x = cell.x.div_euclid(CHUNK_SIZE_CELLS);
    let chunk_y = cell.y.div_euclid(CHUNK_SIZE_CELLS);
    let local_x = cell.x.rem_euclid(CHUNK_SIZE_CELLS) as u32;
    let local_y = cell.y.rem_euclid(CHUNK_SIZE_CELLS) as u32;
    (
        ChunkCoord {
            x: chunk_x,
            y: chunk_y,
        },
        UVec2::new(local_x, local_y),
    )
}

#[inline]
pub fn chunk_local_to_world_cell(coord: ChunkCoord, local: UVec2) -> IVec2 {
    IVec2::new(
        coord.x * CHUNK_SIZE_CELLS + local.x as i32,
        coord.y * CHUNK_SIZE_CELLS + local.y as i32,
    )
}

#[inline]
pub(crate) fn sample_height_at_world(world: Vec2, profile: &PlanetProfile, seed: WorldSeed) -> f32 {
    sample_height(
        seed.0,
        (world.x / profile.cell_size_meters) as f64,
        (world.y / profile.cell_size_meters) as f64,
        profile,
    )
}

#[inline]
pub(crate) fn chunk_origin_translation(
    coord: ChunkCoord,
    profile: &PlanetProfile,
    origin: &WorldOrigin,
) -> Vec3 {
    Vec3::new(
        coord.x as f32 * profile.chunk_span_meters() - origin.0.x,
        0.0,
        coord.y as f32 * profile.chunk_span_meters() - origin.0.y,
    )
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::core::WorldOrigin;

    use super::*;

    #[test]
    fn la_translation_continue_utilise_la_hauteur_procedurale_au_point_exact() {
        let profile = PlanetProfile::mars();
        let seed = WorldSeed(42);
        let mut cache = WorldCache::default();
        let world = Vec2::new(
            profile.cell_size_meters * 10.25,
            profile.cell_size_meters * -3.75,
        );
        let translation = continuous_world_to_render_translation(
            world,
            0.4,
            &mut cache,
            &profile,
            seed,
            &WorldOrigin::default(),
        );

        let expected = sample_height_at_world(world, &profile, seed) + 0.4;
        assert!((translation.y - expected).abs() < 1e-5);
    }
}
