use bevy::prelude::*;

pub(crate) const COULEUR_PANNEAU_PRINCIPAL: Color = Color::srgba(0.07, 0.09, 0.12, 0.92);
pub(crate) const COULEUR_PANNEAU_SECONDAIRE: Color = Color::srgba(0.12, 0.15, 0.19, 0.90);
pub(crate) const COULEUR_BORDURE: Color = Color::srgba(0.71, 0.40, 0.24, 0.55);
pub(crate) const COULEUR_BORDURE_ACTIVE: Color = Color::srgb(0.95, 0.58, 0.24);
pub(crate) const COULEUR_TEXTE: Color = Color::srgb(0.92, 0.95, 0.97);
pub(crate) const COULEUR_TEXTE_ATTENUE: Color = Color::srgb(0.63, 0.70, 0.76);
pub(crate) const COULEUR_TEXTE_FAIBLE: Color = Color::srgb(0.47, 0.55, 0.61);
pub(crate) const COULEUR_TEXTURE: Color = Color::srgba(0.79, 0.40, 0.19, 0.10);
pub(crate) const COULEUR_ACCENT_OXYDE: Color = Color::srgb(0.85, 0.44, 0.21);
pub(crate) const COULEUR_ACCENT_CYAN: Color = Color::srgb(0.25, 0.79, 0.91);
pub(crate) const COULEUR_ACCENT_ACIER: Color = Color::srgb(0.54, 0.64, 0.75);
pub(crate) const COULEUR_DANGER: Color = Color::srgb(0.91, 0.28, 0.27);
pub(crate) const COULEUR_SURVEILLANCE: Color = Color::srgb(0.96, 0.67, 0.20);
pub(crate) const COULEUR_OK: Color = Color::srgb(0.23, 0.76, 0.51);
pub(crate) const COULEUR_PISTE: Color = Color::srgb(0.18, 0.22, 0.27);
pub(crate) const COULEUR_FOND_HOTBAR: Color = Color::srgba(0.05, 0.06, 0.08, 0.96);
pub(crate) const COULEUR_SLOT_NORMAL: Color = Color::srgba(0.10, 0.13, 0.17, 0.95);
pub(crate) const COULEUR_SLOT_SURVOL: Color = Color::srgba(0.14, 0.18, 0.22, 0.97);
pub(crate) const COULEUR_SLOT_SELECTION: Color = Color::srgba(0.20, 0.14, 0.10, 0.98);

pub(crate) fn rayon_arrondi(value: Val) -> BorderRadius {
    BorderRadius::new(value, value, value, value)
}

pub(crate) fn style_racine_hud() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(0),
        right: px(0),
        top: px(0),
        bottom: px(0),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::SpaceBetween,
        padding: UiRect::all(px(20.0)),
        ..default()
    }
}

pub(crate) fn style_rangee_superieure() -> Node {
    Node {
        width: percent(100),
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::FlexStart,
        column_gap: px(18.0),
        ..default()
    }
}

pub(crate) fn style_colonne(largeur: f32) -> Node {
    Node {
        width: px(largeur),
        flex_direction: FlexDirection::Column,
        row_gap: px(16.0),
        ..default()
    }
}

pub(crate) fn style_panneau() -> Node {
    Node {
        width: percent(100),
        position_type: PositionType::Relative,
        border: UiRect::all(px(1.0)),
        border_radius: rayon_arrondi(px(20.0)),
        ..default()
    }
}

pub(crate) fn style_texture_fond() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(0),
        right: px(0),
        top: px(0),
        bottom: px(0),
        border_radius: rayon_arrondi(px(20.0)),
        ..default()
    }
}

pub(crate) fn style_contenu_panneau() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Column,
        row_gap: px(12.0),
        padding: UiRect::axes(px(18.0), px(16.0)),
        ..default()
    }
}

pub(crate) fn style_ligne_entete() -> Node {
    Node {
        width: percent(100),
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        column_gap: px(12.0),
        flex_wrap: FlexWrap::Wrap,
        ..default()
    }
}

pub(crate) fn style_grille_synthese() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        justify_content: JustifyContent::SpaceBetween,
        column_gap: px(12.0),
        row_gap: px(12.0),
        ..default()
    }
}

pub(crate) fn style_carte_synthese() -> Node {
    Node {
        width: percent(48),
        min_height: px(82.0),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::SpaceBetween,
        padding: UiRect::axes(px(12.0), px(11.0)),
        border: UiRect::all(px(1.0)),
        border_radius: rayon_arrondi(px(14.0)),
        ..default()
    }
}

pub(crate) fn style_barre_piste() -> Node {
    Node {
        width: percent(100),
        height: px(12.0),
        border: UiRect::all(px(1.0)),
        border_radius: rayon_arrondi(px(999.0)),
        ..default()
    }
}

pub(crate) fn style_barre_remplissage() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        border_radius: rayon_arrondi(px(999.0)),
        ..default()
    }
}

pub(crate) fn style_badge() -> Node {
    Node {
        align_self: AlignSelf::FlexStart,
        padding: UiRect::axes(px(12.0), px(8.0)),
        border: UiRect::all(px(1.0)),
        border_radius: rayon_arrondi(px(999.0)),
        ..default()
    }
}

pub(crate) fn style_conteneur_cartes_equipage() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Column,
        row_gap: px(8.0),
        ..default()
    }
}

pub(crate) fn style_carte_equipage() -> Node {
    Node {
        width: percent(100),
        padding: UiRect::axes(px(12.0), px(10.0)),
        border: UiRect::all(px(1.0)),
        border_radius: rayon_arrondi(px(14.0)),
        ..default()
    }
}

pub(crate) fn style_hotbar_exterieure() -> Node {
    Node {
        width: px(930.0),
        max_width: percent(100),
        align_self: AlignSelf::Center,
        position_type: PositionType::Relative,
        border: UiRect::all(px(1.0)),
        border_radius: rayon_arrondi(px(24.0)),
        ..default()
    }
}

pub(crate) fn style_hotbar_contenu() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Column,
        row_gap: px(14.0),
        padding: UiRect::axes(px(18.0), px(16.0)),
        ..default()
    }
}

pub(crate) fn style_ligne_slots() -> Node {
    Node {
        width: percent(100),
        justify_content: JustifyContent::SpaceBetween,
        column_gap: px(10.0),
        ..default()
    }
}

pub(crate) fn style_slot_construction() -> Node {
    Node {
        width: percent(16.0),
        min_width: px(120.0),
        min_height: px(96.0),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::SpaceBetween,
        row_gap: px(8.0),
        padding: UiRect::axes(px(12.0), px(10.0)),
        border: UiRect::all(px(1.0)),
        border_radius: rayon_arrondi(px(15.0)),
        position_type: PositionType::Relative,
        ..default()
    }
}

pub(crate) fn style_barre_accent_slot() -> Node {
    Node {
        width: percent(100),
        height: px(5.0),
        border_radius: BorderRadius::new(px(12.0), px(12.0), px(0.0), px(0.0)),
        ..default()
    }
}

pub(crate) fn couleur_ressource(plein: (f32, f32, f32), ratio: f32) -> Color {
    let t = ratio.clamp(0.0, 1.0);
    let rouge = (0.91, 0.28, 0.27);
    Color::srgb(
        interpolation(rouge.0, plein.0, t),
        interpolation(rouge.1, plein.1, t),
        interpolation(rouge.2, plein.2, t),
    )
}

pub(crate) fn ratio(numerateur: f32, denominateur: f32) -> f32 {
    if denominateur <= f32::EPSILON {
        0.0
    } else {
        (numerateur / denominateur).clamp(0.0, 1.0)
    }
}

fn interpolation(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
