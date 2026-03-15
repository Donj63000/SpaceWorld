use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

use super::donnees::{ChunkState, WorldCache};
use super::generation::DonneesSurfaceCellule;
use super::{CHUNK_SIZE_CELLS, ChunkCoord, PlanetProfile};

pub(crate) fn build_chunk_mesh(
    chunk: &ChunkState,
    cache: &WorldCache,
    profile: &PlanetProfile,
    subdivisions_maillage: u32,
) -> Mesh {
    let subdivisions_maillage = subdivisions_maillage.max(1);
    let mesh_cells = CHUNK_SIZE_CELLS as u32 * subdivisions_maillage;
    let side = mesh_cells as usize + 1;
    let step_cells = 1.0 / subdivisions_maillage as f32;
    let step_meters = profile.cell_size_meters / subdivisions_maillage as f32;

    let mut positions = Vec::with_capacity(side * side);
    let mut normals = Vec::with_capacity(side * side);
    let mut colors = Vec::with_capacity(side * side);
    let mut uvs = Vec::with_capacity(side * side);
    let mut indices = Vec::with_capacity(mesh_cells as usize * mesh_cells as usize * 6);

    for y in 0..=mesh_cells {
        for x in 0..=mesh_cells {
            let local_x = x as f32 * step_cells;
            let local_y = y as f32 * step_cells;
            let height = hauteur_interpolee_locale(cache, chunk, local_x, local_y);
            let left = hauteur_interpolee_locale(cache, chunk, local_x - step_cells, local_y);
            let right = hauteur_interpolee_locale(cache, chunk, local_x + step_cells, local_y);
            let down = hauteur_interpolee_locale(cache, chunk, local_x, local_y - step_cells);
            let up = hauteur_interpolee_locale(cache, chunk, local_x, local_y + step_cells);
            let normal = Vec3::new(left - right, 2.0 * step_meters, down - up).normalize();
            let slope = (1.0 - normal.y).clamp(0.0, 1.0);
            let curvature = ((left + right + down + up) - 4.0 * height)
                / profile.cell_size_meters.max(0.0001)
                / subdivisions_maillage as f32;
            let surface = surface_proche(chunk, local_x, local_y);

            positions.push([x as f32 * step_meters, height, y as f32 * step_meters]);
            normals.push([normal.x, normal.y, normal.z]);
            colors.push(terrain_vertex_color(
                surface, height, slope, curvature, profile,
            ));
            uvs.push([
                x as f32 / mesh_cells.max(1) as f32,
                y as f32 / mesh_cells.max(1) as f32,
            ]);
        }
    }

    for y in 0..mesh_cells {
        for x in 0..mesh_cells {
            let i0 = (y as usize * side + x as usize) as u32;
            let i1 = i0 + 1;
            let i2 = i0 + side as u32;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

fn surface_proche(chunk: &ChunkState, local_x: f32, local_y: f32) -> DonneesSurfaceCellule {
    let x = local_x.floor().clamp(0.0, (CHUNK_SIZE_CELLS - 1) as f32) as u32;
    let y = local_y.floor().clamp(0.0, (CHUNK_SIZE_CELLS - 1) as f32) as u32;
    chunk
        .surface_cellule(UVec2::new(x, y))
        .copied()
        .unwrap_or_default()
}

fn hauteur_interpolee_locale(
    cache: &WorldCache,
    chunk: &ChunkState,
    local_x: f32,
    local_y: f32,
) -> f32 {
    let x0 = local_x.floor() as i32;
    let y0 = local_y.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let tx = local_x - x0 as f32;
    let ty = local_y - y0 as f32;

    let h00 = hauteur_sommet_locale(cache, chunk, x0, y0);
    let h10 = hauteur_sommet_locale(cache, chunk, x1, y0);
    let h01 = hauteur_sommet_locale(cache, chunk, x0, y1);
    let h11 = hauteur_sommet_locale(cache, chunk, x1, y1);

    let hx0 = h00 + (h10 - h00) * tx;
    let hx1 = h01 + (h11 - h01) * tx;
    hx0 + (hx1 - hx0) * ty
}

fn hauteur_sommet_locale(
    cache: &WorldCache,
    chunk: &ChunkState,
    local_x: i32,
    local_y: i32,
) -> f32 {
    if let Some(hauteur) = lire_sommet_local_chunk(chunk, local_x, local_y) {
        return hauteur;
    }

    let monde_x = chunk.coord.x * CHUNK_SIZE_CELLS + local_x;
    let monde_y = chunk.coord.y * CHUNK_SIZE_CELLS + local_y;
    let coord_voisin = ChunkCoord {
        x: monde_x.div_euclid(CHUNK_SIZE_CELLS),
        y: monde_y.div_euclid(CHUNK_SIZE_CELLS),
    };
    let local_voisin = UVec2::new(
        monde_x.rem_euclid(CHUNK_SIZE_CELLS) as u32,
        monde_y.rem_euclid(CHUNK_SIZE_CELLS) as u32,
    );

    cache
        .chunk(coord_voisin)
        .and_then(|voisin| voisin.hauteur_sommet(local_voisin))
        .unwrap_or_else(|| {
            let local_replie = UVec2::new(
                local_x.clamp(0, CHUNK_SIZE_CELLS) as u32,
                local_y.clamp(0, CHUNK_SIZE_CELLS) as u32,
            );
            chunk
                .hauteur_sommet(local_replie)
                .unwrap_or(chunk.average_height)
        })
}

fn lire_sommet_local_chunk(chunk: &ChunkState, local_x: i32, local_y: i32) -> Option<f32> {
    if !(0..=CHUNK_SIZE_CELLS).contains(&local_x) || !(0..=CHUNK_SIZE_CELLS).contains(&local_y) {
        return None;
    }

    chunk.hauteur_sommet(UVec2::new(local_x as u32, local_y as u32))
}

fn terrain_vertex_color(
    surface: DonneesSurfaceCellule,
    height: f32,
    slope: f32,
    curvature: f32,
    profile: &PlanetProfile,
) -> [f32; 4] {
    let height_factor = ((height + 6.0) / 12.0).clamp(0.0, 1.0);
    let slope_factor = slope.clamp(0.0, 1.0);
    let convexity = (-curvature * 0.70).clamp(0.0, 1.0);
    let concavity = (curvature * 0.70).clamp(0.0, 1.0);

    let mut color = blend_color(
        profile.palette.ground_shadow,
        profile.palette.ground_mid,
        0.18 + surface.dust_cover * 0.28 + surface.sediment_mask * 0.10,
    );
    color = blend_color(
        color,
        profile.palette.basalt_exposed,
        surface.basalt_exposure * (0.20 + surface.volcanic_mask * 0.26) + slope_factor * 0.08,
    );
    color = blend_color(
        color,
        profile.palette.rock_dark,
        surface.rockiness * 0.24 + surface.crater_rim * 0.12 + surface.yardang_mask * 0.10,
    );
    color = blend_color(
        color,
        profile.palette.oxide_dark,
        surface.hematite * 0.24 + surface.wind_streaks * 0.14,
    );
    color = blend_color(
        color,
        profile.palette.salt_light,
        surface.salts * 0.30 + surface.sediment_mask * 0.08,
    );
    color = blend_color(
        color,
        profile.palette.dust_light,
        surface.dust_cover * 0.42 + surface.wind_streaks * 0.12,
    );
    color = blend_color(
        color,
        profile.palette.accent,
        surface.crater_rim * 0.08 + surface.yardang_mask * 0.06,
    );
    color = blend_color(
        color,
        profile.palette.ground_highlight,
        height_factor * 0.24 + convexity * 0.18 + surface.sediment_mask * 0.06,
    );
    color = blend_color(
        color,
        profile.palette.ground_shadow,
        concavity * 0.18 + surface.crater_floor * 0.06,
    );

    if surface.cold_trap > 0.72 && height < -2.2 {
        color = blend_color(
            color,
            profile.palette.ice,
            ((surface.cold_trap - 0.72) / 0.28).clamp(0.0, 1.0) * 0.32,
        );
    }

    color_to_array(color)
}

fn blend_color(left: Color, right: Color, factor: f32) -> Color {
    let factor = factor.clamp(0.0, 1.0);
    let left = left.to_srgba();
    let right = right.to_srgba();
    Color::srgba(
        left.red + (right.red - left.red) * factor,
        left.green + (right.green - left.green) * factor,
        left.blue + (right.blue - left.blue) * factor,
        left.alpha + (right.alpha - left.alpha) * factor,
    )
}

fn color_to_array(color: Color) -> [f32; 4] {
    let color = color.to_srgba();
    [color.red, color.green, color.blue, color.alpha]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{ChunkCoord, PlanetProfile, WorldCache, WorldSeed, generate_chunk};

    #[test]
    fn le_maillage_reutilise_les_hauteurs_mises_en_cache() {
        let profile = PlanetProfile::mars();
        let seed = WorldSeed(42);
        let coord = ChunkCoord { x: 0, y: 0 };
        let chunk = generate_chunk(&profile, seed, coord);
        let mut cache = WorldCache::default();
        cache.chunks.insert(coord, chunk.clone());

        let mesh = build_chunk_mesh(&chunk, &cache, &profile, 1);
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|valeur| valeur.as_float3())
            .expect("les positions du maillage doivent exister");

        assert_eq!(
            positions[0][1],
            chunk.hauteur_sommet(UVec2::new(0, 0)).unwrap()
        );
        assert_eq!(
            positions[(CHUNK_SIZE_CELLS as usize + 1) * (CHUNK_SIZE_CELLS as usize + 1) - 1][1],
            chunk
                .hauteur_sommet(UVec2::new(CHUNK_SIZE_CELLS as u32, CHUNK_SIZE_CELLS as u32))
                .unwrap()
        );
    }
}
