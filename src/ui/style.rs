use bevy::prelude::*;

pub const COULEUR_PANNEAU: Color = Color::srgba(0.94, 0.97, 1.0, 0.88);
pub const COULEUR_PANNEAU_SECONDAIRE: Color = Color::srgba(0.97, 0.985, 1.0, 0.82);
pub const COULEUR_BORDURE: Color = Color::srgba(0.74, 0.81, 0.90, 0.95);
pub const COULEUR_TEXTE: Color = Color::srgb(0.08, 0.12, 0.17);
pub const COULEUR_TEXTE_ATTENUE: Color = Color::srgb(0.38, 0.45, 0.54);
pub const COULEUR_DANGER: Color = Color::srgb(0.84, 0.21, 0.24);
pub const COULEUR_SURVEILLANCE: Color = Color::srgb(0.96, 0.62, 0.18);
pub const COULEUR_OK: Color = Color::srgb(0.14, 0.71, 0.42);
pub const COULEUR_ACCENT_BLEU: Color = Color::srgb(0.16, 0.42, 0.82);
pub const COULEUR_ACCENT_CYAN: Color = Color::srgb(0.20, 0.66, 0.86);
pub const COULEUR_PISTE: Color = Color::srgb(0.84, 0.88, 0.92);

pub fn rayon_arrondi(value: Val) -> BorderRadius {
    BorderRadius::new(value, value, value, value)
}

pub fn style_panneau() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Column,
        row_gap: px(10),
        padding: UiRect::axes(px(16), px(14)),
        border: UiRect::all(px(1)),
        border_radius: rayon_arrondi(px(16)),
        ..default()
    }
}

pub fn style_carte() -> Node {
    Node {
        width: percent(48),
        min_height: px(72),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::SpaceBetween,
        padding: UiRect::axes(px(12), px(10)),
        border: UiRect::all(px(1)),
        border_radius: rayon_arrondi(px(12)),
        ..default()
    }
}

pub fn style_barre_piste() -> Node {
    Node {
        width: percent(100),
        height: px(12),
        border: UiRect::all(px(1)),
        border_radius: rayon_arrondi(px(999)),
        ..default()
    }
}

pub fn style_barre_remplissage() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        border_radius: rayon_arrondi(px(999)),
        ..default()
    }
}

pub fn style_badge() -> Node {
    Node {
        align_self: AlignSelf::FlexStart,
        padding: UiRect::axes(px(12), px(7)),
        border_radius: rayon_arrondi(px(999)),
        ..default()
    }
}

pub fn couleur_ressource(plein: (f32, f32, f32), ratio: f32) -> Color {
    let t = ratio.clamp(0.0, 1.0);
    let rouge = (0.84, 0.21, 0.24);
    Color::srgb(
        interpolation(rouge.0, plein.0, t),
        interpolation(rouge.1, plein.1, t),
        interpolation(rouge.2, plein.2, t),
    )
}

pub fn ratio(numerateur: f32, denominateur: f32) -> f32 {
    if denominateur <= f32::EPSILON {
        0.0
    } else {
        (numerateur / denominateur).clamp(0.0, 1.0)
    }
}

fn interpolation(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
