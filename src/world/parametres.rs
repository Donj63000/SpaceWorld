use bevy::prelude::*;

pub const CELL_SIZE_METERS: f32 = 2.0;
pub const CHUNK_SIZE_CELLS: i32 = 32;

#[derive(Resource, Clone, Copy, Debug)]
pub struct WorldSeed(pub u64);

#[derive(Clone, Copy, Debug)]
pub struct PlanetPalette {
    pub ground_shadow: Color,
    pub ground_mid: Color,
    pub ground_highlight: Color,
    pub dust_light: Color,
    pub rock_dark: Color,
    pub ice: Color,
    pub accent: Color,
    pub basalt_exposed: Color,
    pub oxide_dark: Color,
    pub salt_light: Color,
}

#[derive(Resource, Clone, Debug)]
pub struct PlanetProfile {
    pub name: &'static str,
    pub gravity: f32,
    pub cell_size_meters: f32,
    pub chunk_size_cells: i32,
    pub origin_recenter_distance: f32,
    pub noise_scale: f64,
    pub crater_scale: f64,
    pub resource_frequency: f64,
    pub wind_direction: Vec2,
    pub palette: PlanetPalette,
}

impl PlanetProfile {
    pub fn mars() -> Self {
        Self {
            name: "Mars",
            gravity: 3.71,
            cell_size_meters: CELL_SIZE_METERS,
            chunk_size_cells: CHUNK_SIZE_CELLS,
            origin_recenter_distance: CELL_SIZE_METERS * CHUNK_SIZE_CELLS as f32 * 1.75,
            // Cette echelle reste volontairement assez petite pour laisser vivre de grands
            // ensembles geomorphologiques visibles sur plusieurs chunks.
            noise_scale: 0.036,
            // La valeur sert de coefficient de calibration pour les bandes de crateres.
            crater_scale: 0.052,
            resource_frequency: 0.118,
            // Direction dominante des formes eoliennes (dunes / yardangs) pour une lecture forte.
            wind_direction: Vec2::new(0.87, 0.49).normalize(),
            palette: PlanetPalette {
                ground_shadow: Color::srgb(0.22, 0.12, 0.10),
                ground_mid: Color::srgb(0.54, 0.31, 0.20),
                ground_highlight: Color::srgb(0.79, 0.58, 0.42),
                dust_light: Color::srgb(0.88, 0.70, 0.55),
                rock_dark: Color::srgb(0.16, 0.13, 0.12),
                ice: Color::srgb(0.83, 0.89, 0.93),
                accent: Color::srgb(0.67, 0.30, 0.18),
                basalt_exposed: Color::srgb(0.21, 0.18, 0.17),
                oxide_dark: Color::srgb(0.46, 0.21, 0.15),
                salt_light: Color::srgb(0.92, 0.85, 0.78),
            },
        }
    }

    #[inline]
    pub fn chunk_span_meters(&self) -> f32 {
        self.cell_size_meters * self.chunk_size_cells as f32
    }
}
