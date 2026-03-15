use bevy::prelude::*;

use super::bruit::{
    billow_fbm, directional_ripples, domain_warp, fbm, rand01, remap_clamped, ridged_fbm, saturate,
    smootherstep01, smoothstep01,
};
use super::decorations::collect_chunk_decorations;
use super::donnees::{ChunkState, ResourceDeposit, ResourceKind, TerrainCell};
use super::{CHUNK_SIZE_CELLS, ChunkCoord, PlanetProfile, WorldSeed};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DonneesSurfaceCellule {
    pub height: f32,
    pub dust_cover: f32,
    pub rockiness: f32,
    pub basalt_exposure: f32,
    pub cold_trap: f32,
    pub crater_floor: f32,
    pub crater_rim: f32,
    pub ejecta: f32,
    pub wind_streaks: f32,
    pub salts: f32,
    pub volcanic_mask: f32,
    pub sediment_mask: f32,
    pub yardang_mask: f32,
    pub hematite: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct ReliefSample {
    height: f64,
    crater_floor: f64,
    crater_rim: f64,
    ejecta: f64,
    ridge_mask: f64,
    dune_field: f64,
    plateau_mask: f64,
    highland_mask: f64,
    volcanic_mask: f64,
    sediment_mask: f64,
    yardang_mask: f64,
    lava_flow: f64,
    layered_mask: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct ProvinceBlend {
    highland: f64,
    volcanic: f64,
    basin: f64,
    yardang: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct CraterContribution {
    height: f64,
    floor: f64,
    rim: f64,
    ejecta: f64,
}

#[derive(Clone, Copy, Debug)]
struct CraterBand {
    maille: f64,
    rayon_min: f64,
    rayon_max: f64,
    densite: f64,
    profondeur: f64,
    force_bourrelet: f64,
    largeur_bourrelet: f64,
    force_ejecta: f64,
    largeur_ejecta: f64,
    pic_central: f64,
}

impl CraterBand {
    fn scaled(self, factor: f64) -> Self {
        let factor = factor.max(1e-4);
        Self {
            maille: self.maille / factor,
            rayon_min: self.rayon_min / factor,
            rayon_max: self.rayon_max / factor,
            ..self
        }
    }
}

const BANDES_CRATERES: [CraterBand; 6] = [
    CraterBand {
        maille: 20.0,
        rayon_min: 1.8,
        rayon_max: 4.0,
        densite: 0.62,
        profondeur: 0.18,
        force_bourrelet: 0.40,
        largeur_bourrelet: 0.16,
        force_ejecta: 0.06,
        largeur_ejecta: 0.34,
        pic_central: 0.0,
    },
    CraterBand {
        maille: 46.0,
        rayon_min: 4.0,
        rayon_max: 8.6,
        densite: 0.46,
        profondeur: 0.42,
        force_bourrelet: 0.36,
        largeur_bourrelet: 0.17,
        force_ejecta: 0.10,
        largeur_ejecta: 0.42,
        pic_central: 0.0,
    },
    CraterBand {
        maille: 112.0,
        rayon_min: 8.5,
        rayon_max: 18.0,
        densite: 0.28,
        profondeur: 0.88,
        force_bourrelet: 0.33,
        largeur_bourrelet: 0.19,
        force_ejecta: 0.14,
        largeur_ejecta: 0.50,
        pic_central: 0.06,
    },
    CraterBand {
        maille: 260.0,
        rayon_min: 18.0,
        rayon_max: 40.0,
        densite: 0.18,
        profondeur: 1.30,
        force_bourrelet: 0.30,
        largeur_bourrelet: 0.22,
        force_ejecta: 0.18,
        largeur_ejecta: 0.58,
        pic_central: 0.08,
    },
    CraterBand {
        maille: 620.0,
        rayon_min: 40.0,
        rayon_max: 82.0,
        densite: 0.10,
        profondeur: 1.90,
        force_bourrelet: 0.24,
        largeur_bourrelet: 0.24,
        force_ejecta: 0.14,
        largeur_ejecta: 0.60,
        pic_central: 0.04,
    },
    CraterBand {
        maille: 1300.0,
        rayon_min: 78.0,
        rayon_max: 150.0,
        densite: 0.05,
        profondeur: 2.60,
        force_bourrelet: 0.18,
        largeur_bourrelet: 0.26,
        force_ejecta: 0.10,
        largeur_ejecta: 0.64,
        pic_central: 0.0,
    },
];

pub fn generate_chunk(profile: &PlanetProfile, seed: WorldSeed, coord: ChunkCoord) -> ChunkState {
    let mut cells = Vec::with_capacity((CHUNK_SIZE_CELLS * CHUNK_SIZE_CELLS) as usize);
    let mut resource_cells = Vec::new();
    let mut surfaces_cellules = Vec::with_capacity((CHUNK_SIZE_CELLS * CHUNK_SIZE_CELLS) as usize);
    let mut normales_cellules = Vec::with_capacity((CHUNK_SIZE_CELLS * CHUNK_SIZE_CELLS) as usize);
    let mut total_height = 0.0;
    let vertex_heights = sample_chunk_vertex_heights(profile, seed, coord);

    for local_y in 0..CHUNK_SIZE_CELLS {
        for local_x in 0..CHUNK_SIZE_CELLS {
            let world_x = coord.x * CHUNK_SIZE_CELLS + local_x;
            let world_y = coord.y * CHUNK_SIZE_CELLS + local_y;
            let surface =
                sample_surface(seed.0, world_x as f64 + 0.5, world_y as f64 + 0.5, profile);
            let normal =
                estimate_normal(seed.0, world_x as f64 + 0.5, world_y as f64 + 0.5, profile);
            let slope = (1.0 - normal.y).clamp(0.0, 1.0);

            let obstacle_chance = (surface.rockiness * 0.056
                + surface.ejecta * 0.020
                + surface.crater_rim * 0.018
                + surface.yardang_mask * 0.016
                + surface.volcanic_mask * 0.008
                - surface.dust_cover * 0.020
                - surface.sediment_mask * 0.006)
                .clamp(0.0, 0.12);
            let blocked =
                slope < 0.92 && rand01(seed.0 ^ 0x5D1E_5A1E, world_x, world_y) < obstacle_chance;

            let ice_probability = (surface.cold_trap * 0.50
                + surface.crater_floor * 0.16
                + surface.sediment_mask * 0.10
                + (1.0 - surface.rockiness) * 0.08)
                .clamp(0.0, 0.95)
                * profile.resource_frequency as f32
                * 2.5;
            let resource = if surface.cold_trap > 0.34
                && slope < 0.66
                && rand01(seed.0 ^ 0xBADA_551C, world_x, world_y) < ice_probability
            {
                let richness = (surface.cold_trap * 5.0
                    + surface.crater_floor * 2.0
                    + surface.sediment_mask * 2.0)
                    .round() as u16;
                Some(ResourceDeposit {
                    kind: ResourceKind::Ice,
                    amount: 4
                        + richness
                        + (rand01(seed.0 ^ 0xC001_CE11, world_x, world_y) * 4.0) as u16,
                })
            } else {
                None
            };

            let constructible = slope < 0.80
                && !blocked
                && !(surface.crater_rim > 0.78 && surface.rockiness > 0.64)
                && !(surface.yardang_mask > 0.82 && surface.rockiness > 0.46);

            total_height += surface.height;
            if resource.is_some() {
                resource_cells.push(UVec2::new(local_x as u32, local_y as u32));
            }
            surfaces_cellules.push(surface);
            normales_cellules.push(normal);
            cells.push(TerrainCell {
                height: surface.height,
                slope,
                constructible,
                resource,
                blocked,
            });
        }
    }

    let mut chunk = ChunkState {
        coord,
        average_height: total_height / cells.len() as f32,
        cells,
        resource_cells,
        surfaces_cellules,
        normales_cellules,
        vertex_heights,
        decoration_specs: Vec::new(),
    };
    chunk.set_decorations(collect_chunk_decorations(&chunk, profile, seed));
    chunk
}

fn sample_chunk_vertex_heights(
    profile: &PlanetProfile,
    seed: WorldSeed,
    coord: ChunkCoord,
) -> Vec<f32> {
    let side = (CHUNK_SIZE_CELLS + 1) as usize;
    let mut heights = vec![0.0; side * side];

    for y in 0..=CHUNK_SIZE_CELLS {
        for x in 0..=CHUNK_SIZE_CELLS {
            let world_x = coord.x * CHUNK_SIZE_CELLS + x;
            let world_y = coord.y * CHUNK_SIZE_CELLS + y;
            heights[y as usize * side + x as usize] =
                sample_height(seed.0, world_x as f64, world_y as f64, profile);
        }
    }

    heights
}

#[inline]
pub(crate) fn sample_height(seed: u64, x: f64, y: f64, profile: &PlanetProfile) -> f32 {
    sample_relief(seed, x, y, profile).height as f32
}

pub(crate) fn sample_surface(
    seed: u64,
    x: f64,
    y: f64,
    profile: &PlanetProfile,
) -> DonneesSurfaceCellule {
    let relief = sample_relief(seed, x, y, profile);
    let dust_noise = fbm(
        seed ^ 0xFADE_CAFE,
        x * profile.noise_scale * 0.48 + 23.0,
        y * profile.noise_scale * 0.48 - 17.0,
        4,
        2.0,
        0.52,
    ) * 0.5
        + 0.5;
    let dark_patches = billow_fbm(
        seed ^ 0x0BAD_1DEA,
        x * profile.noise_scale * 0.22 - 61.0,
        y * profile.noise_scale * 0.22 + 14.0,
        3,
        2.0,
        0.55,
    ) * 0.5
        + 0.5;
    let salt_noise = fbm(
        seed ^ 0x51A7_11DE,
        x * profile.noise_scale * 0.13 + 7.0,
        y * profile.noise_scale * 0.13 - 11.0,
        2,
        2.0,
        0.55,
    ) * 0.5
        + 0.5;

    let wind_streaks = (directional_ripples(
        seed ^ 0xA11C_5F11,
        x,
        y,
        profile.wind_direction.x as f64,
        profile.wind_direction.y as f64,
        profile.noise_scale * 17.0,
        profile.noise_scale * 0.70,
    ) * (relief.dune_field * 0.55
        + relief.sediment_mask * 0.28
        + dust_noise * 0.12)) as f32;

    let dust_cover = saturate(
        0.18 + dust_noise * 0.22
            + relief.sediment_mask * 0.24
            + relief.dune_field * 0.30
            + relief.crater_floor * 0.16
            + relief.plateau_mask * 0.05
            - relief.crater_rim * 0.42
            - relief.ejecta * 0.18
            - relief.ridge_mask * 0.26
            - relief.volcanic_mask * 0.08,
    ) as f32;

    let rockiness = saturate(
        0.08 + relief.ridge_mask * 0.40
            + relief.crater_rim * 0.22
            + relief.ejecta * 0.26
            + relief.highland_mask * 0.20
            + relief.yardang_mask * 0.18
            + dark_patches * 0.10
            - dust_cover as f64 * 0.30
            - relief.sediment_mask * 0.12,
    ) as f32;

    let cold_height_mask = remap_clamped(-4.4, 1.4, 1.0, 0.0, relief.height);
    let cold_trap = saturate(
        relief.crater_floor * 0.72 + cold_height_mask * 0.22 + relief.sediment_mask * 0.10
            - relief.crater_rim * 0.16
            - rockiness as f64 * 0.12,
    ) as f32;

    let salts = saturate(
        relief.crater_floor * 0.10
            + cold_trap as f64 * 0.12
            + relief.sediment_mask * 0.24
            + relief.plateau_mask * 0.06
            + dust_cover as f64 * 0.10 * salt_noise
            - relief.volcanic_mask * 0.08,
    ) as f32;

    let basalt_exposure = saturate(
        0.06 + dark_patches * 0.16
            + relief.volcanic_mask * 0.44
            + relief.lava_flow * 0.26
            + rockiness as f64 * 0.22
            + relief.ejecta * 0.10
            - dust_cover as f64 * 0.26
            - salts as f64 * 0.10,
    ) as f32;

    let hematite = saturate(
        0.04 + relief.sediment_mask * 0.18
            + wind_streaks as f64 * 0.22
            + relief.layered_mask * 0.10
            + relief.crater_floor * 0.06
            - basalt_exposure as f64 * 0.10,
    ) as f32;

    DonneesSurfaceCellule {
        height: relief.height as f32,
        dust_cover,
        rockiness,
        basalt_exposure,
        cold_trap,
        crater_floor: relief.crater_floor as f32,
        crater_rim: relief.crater_rim as f32,
        ejecta: relief.ejecta as f32,
        wind_streaks,
        salts,
        volcanic_mask: relief.volcanic_mask as f32,
        sediment_mask: relief.sediment_mask as f32,
        yardang_mask: relief.yardang_mask as f32,
        hematite,
    }
}

pub(crate) fn estimate_normal(seed: u64, x: f64, y: f64, profile: &PlanetProfile) -> Vec3 {
    let left = sample_height(seed, x - 1.0, y, profile);
    let right = sample_height(seed, x + 1.0, y, profile);
    let down = sample_height(seed, x, y - 1.0, profile);
    let up = sample_height(seed, x, y + 1.0, profile);
    Vec3::new(left - right, 2.0 * profile.cell_size_meters, down - up).normalize()
}

fn sample_relief(seed: u64, x: f64, y: f64, profile: &PlanetProfile) -> ReliefSample {
    let (macro_x, macro_y) =
        domain_warp(seed ^ 0x11A1_900D, x, y, profile.noise_scale * 0.30, 15.0);
    let (detail_x, detail_y) =
        domain_warp(seed ^ 0x9154_C7A1, x, y, profile.noise_scale * 1.02, 3.4);

    let broad = fbm(
        seed ^ 0x6A09_E667,
        macro_x * profile.noise_scale * 0.36,
        macro_y * profile.noise_scale * 0.36,
        5,
        2.02,
        0.52,
    );
    let plateaus = billow_fbm(
        seed ^ 0xBB67_AE85,
        macro_x * profile.noise_scale * 0.20,
        macro_y * profile.noise_scale * 0.20,
        4,
        2.0,
        0.55,
    );
    let ridges = ridged_fbm(
        seed ^ 0x3C6E_F372,
        detail_x * profile.noise_scale * 1.06,
        detail_y * profile.noise_scale * 1.06,
        4,
        2.06,
        0.55,
    );
    let basin_noise = billow_fbm(
        seed ^ 0xA54F_F53A,
        x * profile.noise_scale * 0.16 - 11.0,
        y * profile.noise_scale * 0.16 + 7.0,
        3,
        2.0,
        0.55,
    );
    let lava_flow = billow_fbm(
        seed ^ 0x0FF1_0A77,
        macro_x * profile.noise_scale * 0.46 - 7.0,
        macro_y * profile.noise_scale * 0.46 + 13.0,
        4,
        2.0,
        0.53,
    );
    let layered = billow_fbm(
        seed ^ 0x67AE_8584,
        detail_x * profile.noise_scale * 0.84 + 43.0,
        detail_y * profile.noise_scale * 0.84 - 21.0,
        3,
        2.0,
        0.55,
    );

    let province = province_blend(seed, x, y, profile);
    let highland_mask = smoothstep01(remap_clamped(0.08, 0.26, 0.0, 1.0, province.highland));
    let volcanic_mask = smoothstep01(remap_clamped(0.22, 0.52, 0.0, 1.0, province.volcanic));
    let sediment_mask = smoothstep01(remap_clamped(0.08, 0.28, 0.0, 1.0, province.basin));
    let yardang_mask = smoothstep01(remap_clamped(0.18, 0.40, 0.0, 1.0, province.yardang));

    let plateau_mask = smootherstep01(remap_clamped(
        -0.10,
        0.44,
        0.0,
        1.0,
        plateaus * 0.60 + basin_noise * 0.14 + highland_mask * 0.16,
    ));
    let ridge_mask = smoothstep01(remap_clamped(
        0.32,
        0.80,
        0.0,
        1.0,
        ridges + highland_mask * 0.10 + yardang_mask * 0.15,
    ));
    let dune_field = smoothstep01(remap_clamped(
        -0.18,
        0.34,
        0.0,
        1.0,
        basin_noise * 0.34 + sediment_mask * 0.58 + broad * 0.08 - highland_mask * 0.16,
    ));

    let aeolian_relief = (directional_ripples(
        seed ^ 0xD00D_5EED,
        x,
        y,
        profile.wind_direction.x as f64,
        profile.wind_direction.y as f64,
        profile.noise_scale * 14.5,
        profile.noise_scale * 0.68,
    ) - 0.5)
        * (0.12 + dune_field * 0.52);

    let yardang_relief = (directional_ripples(
        seed ^ 0x4E6F_7274,
        x,
        y,
        profile.wind_direction.x as f64,
        profile.wind_direction.y as f64,
        profile.noise_scale * 7.0,
        profile.noise_scale * 0.40,
    ) - 0.5)
        * (0.08 + yardang_mask * 1.00);

    let wrinkle_ridges = (directional_ripples(
        seed ^ 0x7788_99AA,
        x,
        y,
        0.54,
        -0.84,
        profile.noise_scale * 5.0,
        profile.noise_scale * 0.22,
    ) - 0.5)
        * (0.06 + volcanic_mask * 0.40);

    let crater = sample_craters(seed, x, y, profile);
    let crater_weight = 0.76 + highland_mask * 0.48 + sediment_mask * 0.16 + yardang_mask * 0.12
        - volcanic_mask * 0.20;
    let micro_relief = fbm(
        seed ^ 0x5BE0_CD19,
        detail_x * profile.noise_scale * 2.65 + 91.0,
        detail_y * profile.noise_scale * 2.65 - 37.0,
        2,
        2.0,
        0.5,
    ) * (0.08 + highland_mask * 0.04 + yardang_mask * 0.03);

    let basin_depth = sediment_mask * (0.50 + (basin_noise * 0.5 + 0.5) * 1.25);
    let height = broad * (1.90 + highland_mask * 0.62 - sediment_mask * 0.06)
        + (plateau_mask * 2.0 - 1.0) * (0.30 + highland_mask * 0.66 + yardang_mask * 0.10)
        + (ridges * 2.0 - 1.0) * (0.16 + highland_mask * 0.40 + yardang_mask * 0.18)
        - basin_depth * 1.25
        + lava_flow * (0.12 + volcanic_mask * 1.00)
        + layered * sediment_mask * 0.24
        + aeolian_relief
        + yardang_relief
        + wrinkle_ridges
        + crater.height * crater_weight
        + micro_relief;

    ReliefSample {
        height,
        crater_floor: crater.floor,
        crater_rim: crater.rim,
        ejecta: crater.ejecta,
        ridge_mask,
        dune_field,
        plateau_mask,
        highland_mask,
        volcanic_mask,
        sediment_mask,
        yardang_mask,
        lava_flow: (lava_flow * 0.5 + 0.5).max(0.0),
        layered_mask: (layered * 0.5 + 0.5).max(0.0),
    }
}

fn province_blend(seed: u64, x: f64, y: f64, profile: &PlanetProfile) -> ProvinceBlend {
    let (province_x, province_y) =
        domain_warp(seed ^ 0x72AA_5511, x, y, profile.noise_scale * 0.09, 32.0);

    let a = fbm(
        seed ^ 0x510E_527F,
        province_x * profile.noise_scale * 0.08 + 18.0,
        province_y * profile.noise_scale * 0.08 - 31.0,
        4,
        2.0,
        0.54,
    );
    let b = billow_fbm(
        seed ^ 0x9B05_688C,
        province_x * profile.noise_scale * 0.06 - 27.0,
        province_y * profile.noise_scale * 0.06 + 14.0,
        4,
        2.0,
        0.55,
    );
    let c = ridged_fbm(
        seed ^ 0x1F83_D9AB,
        province_x * profile.noise_scale * 0.14 + 9.0,
        province_y * profile.noise_scale * 0.14 + 5.0,
        3,
        2.04,
        0.55,
    );
    let d = fbm(
        seed ^ 0x428A_2F98,
        province_x * profile.noise_scale * 0.11 - 7.0,
        province_y * profile.noise_scale * 0.11 + 27.0,
        4,
        2.0,
        0.55,
    );

    let highland = (0.55 + b * 0.72 + c * 0.18 - a * 0.12).max(0.05);
    let volcanic = (0.48 + a * 0.78 - b * 0.12 - c * 0.06 + d * 0.10).max(0.05);
    let basin = (0.40 - a * 0.40 + b * 0.18 + d * 0.75).max(0.05);
    let yardang = (0.10 + c * 0.50 + d * 0.12 - b * 0.08).max(0.02);
    let total = (highland + volcanic + basin + yardang).max(1e-6);

    ProvinceBlend {
        highland: highland / total,
        volcanic: volcanic / total,
        basin: basin / total,
        yardang: yardang / total,
    }
}

fn sample_craters(seed: u64, x: f64, y: f64, profile: &PlanetProfile) -> CraterContribution {
    let mut total = CraterContribution::default();
    let crater_scale = (profile.crater_scale / 0.052).clamp(0.55, 1.85);

    for (band_index, band) in BANDES_CRATERES.into_iter().enumerate() {
        let band_seed = seed
            ^ profile.crater_scale.to_bits()
            ^ (band_index as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let crater = sample_crater_band(band_seed, x, y, band.scaled(crater_scale));
        total.height += crater.height;
        total.floor = total.floor.max(crater.floor);
        total.rim = total.rim.max(crater.rim);
        total.ejecta = total.ejecta.max(crater.ejecta);
    }

    total
}

fn sample_crater_band(seed: u64, x: f64, y: f64, band: CraterBand) -> CraterContribution {
    let cell_x = (x / band.maille).floor() as i32;
    let cell_y = (y / band.maille).floor() as i32;
    let search_radius = ((band.rayon_max * 2.0 / band.maille).ceil() as i32).max(1) + 1;
    let mut result = CraterContribution::default();

    for dy in -search_radius..=search_radius {
        for dx in -search_radius..=search_radius {
            let gx = cell_x + dx;
            let gy = cell_y + dy;

            if rand01(seed ^ 0xA11C_7001, gx, gy) as f64 > band.densite {
                continue;
            }

            let center_x =
                (gx as f64 + 0.5 + (rand01(seed ^ 0xCAFE_0001, gx, gy) as f64 - 0.5) * 0.82)
                    * band.maille;
            let center_y =
                (gy as f64 + 0.5 + (rand01(seed ^ 0xCAFE_0002, gx, gy) as f64 - 0.5) * 0.82)
                    * band.maille;
            let radius = remap_clamped(
                0.0,
                1.0,
                band.rayon_min,
                band.rayon_max,
                rand01(seed ^ 0xCAFE_0003, gx, gy) as f64,
            );
            let depth = band.profondeur
                * remap_clamped(
                    0.0,
                    1.0,
                    0.78,
                    1.26,
                    rand01(seed ^ 0xCAFE_0004, gx, gy) as f64,
                );

            let dx = x - center_x;
            let dy = y - center_y;
            let distance = (dx * dx + dy * dy).sqrt();
            let normalized = distance / radius;
            if normalized > 2.25 {
                continue;
            }

            let bowl = if normalized < 1.0 {
                -(1.0 - normalized * normalized).powf(2.2) * depth
            } else {
                0.0
            };
            let floor = if normalized < 0.62 {
                (1.0 - normalized / 0.62).powf(2.1)
            } else {
                0.0
            };
            let rim =
                gaussian(normalized, 1.03, band.largeur_bourrelet) * depth * band.force_bourrelet;
            let ejecta =
                gaussian(normalized, 1.34, band.largeur_ejecta) * depth * band.force_ejecta;
            let central_peak = if band.pic_central > 0.0 {
                gaussian(normalized, 0.0, 0.16) * depth * band.pic_central
            } else {
                0.0
            };

            result.height += bowl + rim + ejecta + central_peak;
            result.floor = result.floor.max(floor);
            result.rim = result
                .rim
                .max(saturate(rim / (depth * band.force_bourrelet + 1e-5)));
            result.ejecta = result
                .ejecta
                .max(saturate(ejecta / (depth * band.force_ejecta + 1e-5)));
        }
    }

    result
}

#[inline]
fn gaussian(x: f64, mean: f64, sigma: f64) -> f64 {
    let delta = (x - mean) / sigma.max(1e-5);
    (-delta * delta).exp()
}
