use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

use super::bruit::{rand_signed, rand01, splitmix64, unit_f64_from_hash};
use super::donnees::ChunkState;
use super::generation::DonneesSurfaceCellule;
use super::{CHUNK_SIZE_CELLS, PlanetProfile, WorldSeed};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DecorationMaterial {
    Dark,
    Dust,
    Salt,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SurfaceDecorationSpec {
    pub local_translation: Vec3,
    pub scale: Vec3,
    pub yaw: f32,
    pub normal: Vec3,
    pub tilt_strength: f32,
    pub material: DecorationMaterial,
    pub shape_seed: u64,
}

pub(crate) fn collect_chunk_decorations(
    chunk: &ChunkState,
    profile: &PlanetProfile,
    seed: WorldSeed,
) -> Vec<SurfaceDecorationSpec> {
    let mut decorations = Vec::new();

    for local_y in 0..CHUNK_SIZE_CELLS {
        for local_x in 0..CHUNK_SIZE_CELLS {
            let local = UVec2::new(local_x as u32, local_y as u32);
            let Some(cell) = chunk.cell(local) else {
                continue;
            };
            let Some(surface) = chunk.surface_cellule(local) else {
                continue;
            };
            let Some(normal) = chunk.normale_cellule(local) else {
                continue;
            };

            let world_x = chunk.coord.x * CHUNK_SIZE_CELLS + local_x;
            let world_y = chunk.coord.y * CHUNK_SIZE_CELLS + local_y;

            if cell.blocked {
                decorations.push(make_boulder_spec(
                    local,
                    cell.height,
                    world_x,
                    world_y,
                    surface,
                    normal,
                    profile,
                    seed,
                ));
                continue;
            }

            let decorative_density = (0.002
                + surface.rockiness * 0.026
                + surface.ejecta * 0.028
                + surface.crater_rim * 0.024
                + surface.yardang_mask * 0.014
                + surface.volcanic_mask * 0.008
                - surface.dust_cover * 0.018
                - surface.sediment_mask * 0.012)
                .clamp(0.0, 0.090);

            if rand01(seed.0 ^ 0x4A11_5F1E, world_x, world_y) < decorative_density {
                let count =
                    1 + (rand01(seed.0 ^ 0x4411_8822, world_x, world_y) * 2.6).floor() as usize;
                for index in 0..count {
                    decorations.push(make_rubble_spec(
                        local,
                        cell.height,
                        world_x,
                        world_y,
                        index,
                        surface,
                        normal,
                        profile,
                        seed,
                    ));
                }
            }

            let salt_density = (surface.salts * 0.022
                + surface.sediment_mask * 0.016
                + surface.crater_floor * 0.010
                - surface.rockiness * 0.012)
                .clamp(0.0, 0.040);

            if surface.salts > 0.34 && rand01(seed.0 ^ 0x515A_1711, world_x, world_y) < salt_density
            {
                decorations.push(make_salt_plate_spec(
                    local,
                    cell.height,
                    world_x,
                    world_y,
                    surface,
                    normal,
                    profile,
                    seed,
                ));
            }
        }
    }

    decorations
}

pub(crate) fn build_chunk_decoration_mesh(
    specs: &[SurfaceDecorationSpec],
    material_kind: DecorationMaterial,
    densite_decors: f32,
) -> Option<Mesh> {
    let matching_specs: Vec<_> = specs
        .iter()
        .filter(|spec| spec.material == material_kind && spec_visible(spec, densite_decors))
        .collect();
    if matching_specs.is_empty() {
        return None;
    }

    let mut positions = Vec::with_capacity(matching_specs.len() * 24);
    let mut normals = Vec::with_capacity(matching_specs.len() * 24);
    let mut uvs = Vec::with_capacity(matching_specs.len() * 24);
    let mut indices = Vec::with_capacity(matching_specs.len() * 36);

    for spec in matching_specs {
        append_rock_geometry(&mut positions, &mut normals, &mut uvs, &mut indices, spec);
    }

    Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices)),
    )
}

pub(crate) fn compter_decors_visibles(
    specs: &[SurfaceDecorationSpec],
    material_kind: DecorationMaterial,
    densite_decors: f32,
) -> usize {
    specs
        .iter()
        .filter(|spec| spec.material == material_kind && spec_visible(spec, densite_decors))
        .count()
}

fn make_boulder_spec(
    local: UVec2,
    height: f32,
    world_x: i32,
    world_y: i32,
    surface: &DonneesSurfaceCellule,
    normal: Vec3,
    profile: &PlanetProfile,
    seed: WorldSeed,
) -> SurfaceDecorationSpec {
    let yardang_align = surface.yardang_mask.clamp(0.0, 1.0);
    let elongation = 1.0 + yardang_align * 1.4;
    let compact = 1.0 - yardang_align * 0.30;
    let width = (0.76 + rand01(seed.0 ^ 0x0DD5_9A7E, world_x, world_y) * 1.08) * elongation;
    let depth = (0.60 + rand01(seed.0 ^ 0xB011_DA7A, world_y, world_x) * 0.90) * compact;
    let vertical = 0.30
        + rand01(seed.0 ^ 0x7A11_4B4D, world_x, world_y) * 0.54
        + surface.rockiness * 0.18
        + surface.yardang_mask * 0.06;
    let offset_x = rand_signed(seed.0 ^ 0x91B0_11D3, world_x, world_y) as f32 * 0.22;
    let offset_z = rand_signed(seed.0 ^ 0xA31D_0C7A, world_x, world_y) as f32 * 0.22;
    let dust_bias = (surface.dust_cover * 0.60
        + surface.sediment_mask * 0.12
        + rand01(seed.0 ^ 0xD057_BA51, world_x, world_y) * 0.28)
        .clamp(0.0, 1.0);

    SurfaceDecorationSpec {
        local_translation: rock_local_translation(
            local, height, profile, offset_x, offset_z, vertical, 0.12,
        ),
        scale: Vec3::new(width, vertical, depth),
        yaw: if yardang_align > 0.48 {
            wind_yaw(profile.wind_direction)
                + rand_signed(seed.0 ^ 0x5EED_F00D, world_x, world_y) as f32 * 0.28
        } else {
            rand01(seed.0 ^ 0x5EED_F00D, world_x, world_y) * std::f32::consts::TAU
        },
        normal,
        tilt_strength: (0.12
            + surface.rockiness * 0.24
            + surface.ejecta * 0.08
            + surface.yardang_mask * 0.05)
            .clamp(0.0, 0.44),
        material: if surface.salts > 0.48 && surface.rockiness < 0.40 {
            DecorationMaterial::Salt
        } else if dust_bias > 0.60 {
            DecorationMaterial::Dust
        } else {
            DecorationMaterial::Dark
        },
        shape_seed: compose_shape_seed(seed.0 ^ 0xABCD_3101, world_x, world_y, 0),
    }
}

fn make_rubble_spec(
    local: UVec2,
    height: f32,
    world_x: i32,
    world_y: i32,
    index: usize,
    surface: &DonneesSurfaceCellule,
    normal: Vec3,
    profile: &PlanetProfile,
    seed: WorldSeed,
) -> SurfaceDecorationSpec {
    let index_i32 = index as i32;
    let yardang_align = surface.yardang_mask.clamp(0.0, 1.0);
    let width = (0.12 + rand01(seed.0 ^ 0x2244_5011, world_x + index_i32, world_y) * 0.28)
        * (1.0 + yardang_align * 0.45);
    let depth = (0.10 + rand01(seed.0 ^ 0x3311_7EED, world_y, world_x + index_i32) * 0.22)
        * (1.0 - yardang_align * 0.18);
    let vertical = 0.07
        + rand01(seed.0 ^ 0x9EED_A115, world_x, world_y + index_i32) * 0.14
        + surface.rockiness * 0.04;
    let offset_x =
        rand_signed(seed.0 ^ 0x0199_1A5E, world_x + index_i32 * 3, world_y) as f32 * 0.42;
    let offset_z =
        rand_signed(seed.0 ^ 0xC4A7_115E, world_x, world_y + index_i32 * 5) as f32 * 0.42;
    let dust_mix = (surface.dust_cover * 0.70
        + surface.sediment_mask * 0.12
        + rand01(seed.0 ^ 0xDADA_7711, world_x + index_i32, world_y) * 0.18)
        .clamp(0.0, 1.0);

    SurfaceDecorationSpec {
        local_translation: rock_local_translation(
            local, height, profile, offset_x, offset_z, vertical, 0.18,
        ),
        scale: Vec3::new(width, vertical, depth),
        yaw: if yardang_align > 0.58 {
            wind_yaw(profile.wind_direction)
                + rand_signed(seed.0 ^ 0x551A_0042, world_x + index_i32, world_y) as f32 * 0.32
        } else {
            rand01(seed.0 ^ 0x551A_0042, world_x + index_i32, world_y) * std::f32::consts::TAU
        },
        normal,
        tilt_strength: (0.14 + surface.rockiness * 0.16 + surface.yardang_mask * 0.08)
            .clamp(0.0, 0.50),
        material: if surface.salts > 0.44 && surface.rockiness < 0.34 {
            DecorationMaterial::Salt
        } else if dust_mix > 0.70 && surface.basalt_exposure < 0.42 {
            DecorationMaterial::Dust
        } else {
            DecorationMaterial::Dark
        },
        shape_seed: compose_shape_seed(seed.0 ^ 0xABCD_9901, world_x, world_y, index as u64 + 1),
    }
}

fn make_salt_plate_spec(
    local: UVec2,
    height: f32,
    world_x: i32,
    world_y: i32,
    surface: &DonneesSurfaceCellule,
    normal: Vec3,
    profile: &PlanetProfile,
    seed: WorldSeed,
) -> SurfaceDecorationSpec {
    let width = 0.18 + rand01(seed.0 ^ 0xBADA_0011, world_x, world_y) * 0.28;
    let depth = 0.16 + rand01(seed.0 ^ 0xBADA_0012, world_y, world_x) * 0.24;
    let vertical = 0.03 + rand01(seed.0 ^ 0xBADA_0013, world_x, world_y) * 0.05;
    let offset_x = rand_signed(seed.0 ^ 0xBADA_0014, world_x, world_y) as f32 * 0.30;
    let offset_z = rand_signed(seed.0 ^ 0xBADA_0015, world_y, world_x) as f32 * 0.30;

    SurfaceDecorationSpec {
        local_translation: rock_local_translation(
            local, height, profile, offset_x, offset_z, vertical, 0.35,
        ),
        scale: Vec3::new(width, vertical, depth),
        yaw: rand01(seed.0 ^ 0xBADA_0016, world_x, world_y) * std::f32::consts::TAU,
        normal,
        tilt_strength: (0.08 + surface.sediment_mask * 0.08).clamp(0.0, 0.20),
        material: DecorationMaterial::Salt,
        shape_seed: compose_shape_seed(seed.0 ^ 0xBADA_0017, world_x, world_y, 17),
    }
}

fn rock_local_translation(
    local: UVec2,
    height: f32,
    profile: &PlanetProfile,
    offset_x: f32,
    offset_z: f32,
    vertical_scale: f32,
    bury_ratio: f32,
) -> Vec3 {
    Vec3::new(
        (local.x as f32 + 0.5) * profile.cell_size_meters + offset_x,
        height + vertical_scale * (0.5 - bury_ratio),
        (local.y as f32 + 0.5) * profile.cell_size_meters + offset_z,
    )
}

fn compose_shape_seed(seed: u64, world_x: i32, world_y: i32, index: u64) -> u64 {
    splitmix64(
        seed ^ ((world_x as i64 as u64).rotate_left(17))
            ^ ((world_y as i64 as u64).rotate_left(41))
            ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15),
    )
}

fn append_rock_geometry(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    spec: &SurfaceDecorationSpec,
) {
    let base_index = positions.len() as u32;
    let up = if spec.normal.length_squared() > 1e-6 {
        spec.normal.normalize()
    } else {
        Vec3::Y
    };
    let align_full = Quat::from_rotation_arc(Vec3::Y, up);
    let align = Quat::IDENTITY.slerp(align_full, spec.tilt_strength.clamp(0.0, 1.0));
    let yaw = Quat::from_axis_angle(up, spec.yaw);
    let rotation = yaw * align;

    let mut corners = build_irregular_corners(spec.scale, spec.shape_seed);
    for corner in &mut corners {
        *corner = rotation * *corner + spec.local_translation;
    }

    let faces: [([usize; 4], [[f32; 2]; 4]); 6] = [
        (
            [0, 1, 2, 3],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ),
        (
            [7, 6, 5, 4],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ),
        (
            [3, 2, 6, 7],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ),
        (
            [1, 0, 4, 5],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ),
        (
            [0, 3, 7, 4],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ),
        (
            [2, 1, 5, 6],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ),
    ];

    for (face_index, (ids, face_uvs)) in faces.into_iter().enumerate() {
        let a = corners[ids[0]];
        let b = corners[ids[1]];
        let c = corners[ids[2]];
        let normal = (b - a).cross(c - a).normalize_or_zero();

        for (corner_index, id) in ids.into_iter().enumerate() {
            let v = corners[id];
            positions.push([v.x, v.y, v.z]);
            normals.push([normal.x, normal.y, normal.z]);
            uvs.push(face_uvs[corner_index]);
        }

        let face_base = base_index + face_index as u32 * 4;
        indices.extend_from_slice(&[
            face_base,
            face_base + 1,
            face_base + 2,
            face_base,
            face_base + 2,
            face_base + 3,
        ]);
    }
}

fn build_irregular_corners(scale: Vec3, shape_seed: u64) -> [Vec3; 8] {
    let half = scale * 0.5;
    let base = [
        Vec3::new(-half.x, -half.y, -half.z),
        Vec3::new(half.x, -half.y, -half.z),
        Vec3::new(half.x, -half.y, half.z),
        Vec3::new(-half.x, -half.y, half.z),
        Vec3::new(-half.x, half.y, -half.z),
        Vec3::new(half.x, half.y, -half.z),
        Vec3::new(half.x, half.y, half.z),
        Vec3::new(-half.x, half.y, half.z),
    ];

    let min_axis = scale.x.min(scale.y).min(scale.z);
    let radial_amount = min_axis * 0.18;
    let lateral_amount = min_axis * 0.14;

    let mut corners = [Vec3::ZERO; 8];
    for (index, corner) in base.into_iter().enumerate() {
        let sign = Vec3::new(corner.x.signum(), corner.y.signum(), corner.z.signum());
        let radial = (hash_unit(shape_seed, index as u64) * 2.0 - 1.0) * radial_amount;
        let lateral = Vec3::new(
            (hash_unit(shape_seed ^ 0x1111_AAAA, index as u64) * 2.0 - 1.0) * lateral_amount,
            (hash_unit(shape_seed ^ 0x2222_BBBB, index as u64) * 2.0 - 1.0) * lateral_amount * 0.55,
            (hash_unit(shape_seed ^ 0x3333_CCCC, index as u64) * 2.0 - 1.0) * lateral_amount,
        );

        let mut deformed = corner + sign * radial + lateral;

        if corner.y > 0.0 {
            deformed.y *= 0.86 + hash_unit(shape_seed ^ 0x4444_DDDD, index as u64) * 0.28;
        } else {
            deformed.y *= 0.72 + hash_unit(shape_seed ^ 0x5555_EEEE, index as u64) * 0.12;
        }

        corners[index] = deformed;
    }

    corners
}

#[inline]
fn hash_unit(seed: u64, index: u64) -> f32 {
    unit_f64_from_hash(splitmix64(seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15))) as f32
}

#[inline]
fn spec_visible(spec: &SurfaceDecorationSpec, densite_decors: f32) -> bool {
    if densite_decors >= 1.0 {
        return true;
    }
    if densite_decors <= 0.0 {
        return false;
    }

    hash_unit(spec.shape_seed ^ 0x55AA_1010, 0) <= densite_decors.clamp(0.0, 1.0)
}

#[inline]
fn wind_yaw(direction: Vec2) -> f32 {
    direction.y.atan2(direction.x)
}
