use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::world::generation::{DonneesSurfaceCellule, generate_chunk};

use super::decorations::SurfaceDecorationSpec;
use super::{CHUNK_SIZE_CELLS, PlanetProfile, WorldSeed, world_cell_to_chunk};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Ice,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResourceDeposit {
    pub kind: ResourceKind,
    pub amount: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerrainCell {
    pub height: f32,
    pub slope: f32,
    pub constructible: bool,
    pub resource: Option<ResourceDeposit>,
    pub blocked: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug)]
pub struct ChunkState {
    pub coord: ChunkCoord,
    pub cells: Vec<TerrainCell>,
    pub average_height: f32,
    pub(crate) resource_cells: Vec<UVec2>,
    pub(crate) surfaces_cellules: Vec<DonneesSurfaceCellule>,
    pub(crate) normales_cellules: Vec<Vec3>,
    pub(crate) vertex_heights: Vec<f32>,
    pub(crate) decoration_specs: Vec<SurfaceDecorationSpec>,
}

impl ChunkState {
    #[inline]
    fn index(local: UVec2) -> usize {
        local.y as usize * CHUNK_SIZE_CELLS as usize + local.x as usize
    }

    pub fn cell(&self, local: UVec2) -> Option<&TerrainCell> {
        if local.x >= CHUNK_SIZE_CELLS as u32 || local.y >= CHUNK_SIZE_CELLS as u32 {
            return None;
        }
        self.cells.get(Self::index(local))
    }

    pub fn cell_mut(&mut self, local: UVec2) -> Option<&mut TerrainCell> {
        if local.x >= CHUNK_SIZE_CELLS as u32 || local.y >= CHUNK_SIZE_CELLS as u32 {
            return None;
        }
        let index = Self::index(local);
        self.cells.get_mut(index)
    }

    pub fn resource_cells(&self) -> &[UVec2] {
        &self.resource_cells
    }

    pub(crate) fn surface_cellule(&self, local: UVec2) -> Option<&DonneesSurfaceCellule> {
        if local.x >= CHUNK_SIZE_CELLS as u32 || local.y >= CHUNK_SIZE_CELLS as u32 {
            return None;
        }
        self.surfaces_cellules.get(Self::index(local))
    }

    pub(crate) fn normale_cellule(&self, local: UVec2) -> Option<Vec3> {
        if local.x >= CHUNK_SIZE_CELLS as u32 || local.y >= CHUNK_SIZE_CELLS as u32 {
            return None;
        }
        self.normales_cellules.get(Self::index(local)).copied()
    }

    pub(crate) fn hauteur_sommet(&self, local: UVec2) -> Option<f32> {
        if local.x > CHUNK_SIZE_CELLS as u32 || local.y > CHUNK_SIZE_CELLS as u32 {
            return None;
        }
        let side = (CHUNK_SIZE_CELLS + 1) as usize;
        self.vertex_heights
            .get(local.y as usize * side + local.x as usize)
            .copied()
    }

    pub(crate) fn decoration_specs(&self) -> &[SurfaceDecorationSpec] {
        &self.decoration_specs
    }

    pub(crate) fn set_decorations(&mut self, specs: Vec<SurfaceDecorationSpec>) {
        self.decoration_specs = specs;
    }
}

#[derive(Resource, Default)]
pub struct WorldCache {
    pub chunks: HashMap<ChunkCoord, ChunkState>,
}

impl WorldCache {
    pub fn ensure_chunk(
        &mut self,
        coord: ChunkCoord,
        profile: &PlanetProfile,
        seed: WorldSeed,
    ) -> &ChunkState {
        self.chunks
            .entry(coord)
            .or_insert_with(|| generate_chunk(profile, seed, coord))
    }

    pub fn chunk(&self, coord: ChunkCoord) -> Option<&ChunkState> {
        self.chunks.get(&coord)
    }

    pub fn ensure_chunk_mut(
        &mut self,
        coord: ChunkCoord,
        profile: &PlanetProfile,
        seed: WorldSeed,
    ) -> &mut ChunkState {
        self.chunks
            .entry(coord)
            .or_insert_with(|| generate_chunk(profile, seed, coord))
    }

    pub fn terrain_at(
        &mut self,
        cell: IVec2,
        profile: &PlanetProfile,
        seed: WorldSeed,
    ) -> TerrainCell {
        let (coord, local) = world_cell_to_chunk(cell);
        self.ensure_chunk(coord, profile, seed)
            .cell(local)
            .cloned()
            .expect("la cellule doit rester dans les bornes du chunk")
    }

    pub fn average_height_for_cells(
        &mut self,
        cells: &[IVec2],
        profile: &PlanetProfile,
        seed: WorldSeed,
    ) -> f32 {
        if cells.is_empty() {
            return 0.0;
        }

        let mut total = 0.0;
        for &cell in cells {
            total += self.terrain_at(cell, profile, seed).height;
        }
        total / cells.len() as f32
    }

    pub fn extract_resource(
        &mut self,
        cell: IVec2,
        profile: &PlanetProfile,
        seed: WorldSeed,
        amount: u16,
    ) -> u16 {
        let (coord, local) = world_cell_to_chunk(cell);
        let chunk = self.ensure_chunk_mut(coord, profile, seed);
        let (extracted, depleted) = {
            let Some(cell) = chunk.cell_mut(local) else {
                return 0;
            };
            let Some(mut resource) = cell.resource else {
                return 0;
            };

            let extracted = resource.amount.min(amount);
            resource.amount -= extracted;
            let depleted = resource.amount == 0;
            cell.resource = if depleted { None } else { Some(resource) };
            (extracted, depleted)
        };

        if depleted {
            chunk.resource_cells.retain(|entry| *entry != local);
        }
        extracted
    }
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct HoveredCell(pub Option<IVec2>);

#[derive(Resource, Default, Clone)]
pub struct ActiveChunks {
    pub center: Option<ChunkCoord>,
    pub coords: Vec<ChunkCoord>,
    pub set: HashSet<ChunkCoord>,
}
