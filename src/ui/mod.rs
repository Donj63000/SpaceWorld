mod assemblage_hud;
mod hotbar_construction;
mod polices;
mod presentation_hud;
mod references_hud;
mod systemes_hud;
mod theme_cockpit;

use bevy::prelude::*;
use bevy::ui::UiSystems;

use self::assemblage_hud::setup_hud;
use self::hotbar_construction::{
    synchroniser_styles_hotbar_construction, traiter_clics_hotbar_construction,
};
use self::polices::charger_police_interface;
use self::references_hud::{CadenceHudEquipage, MemoireHudMission, charger_assets_hud};
use self::systemes_hud::{
    appliquer_visibilite_hud, mettre_a_jour_blocage_construction_ui,
    rafraichir_hud_construction, rafraichir_hud_equipage, rafraichir_hud_mission_reseaux,
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MemoireHudMission::default())
            .insert_resource(CadenceHudEquipage::default())
            .add_systems(
                Startup,
                (charger_police_interface, charger_assets_hud, setup_hud).chain(),
            )
            .add_systems(
                PreUpdate,
                (
                    traiter_clics_hotbar_construction.after(UiSystems::Focus),
                    mettre_a_jour_blocage_construction_ui.after(UiSystems::Focus),
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    appliquer_visibilite_hud,
                    rafraichir_hud_mission_reseaux,
                    rafraichir_hud_construction,
                    rafraichir_hud_equipage,
                    synchroniser_styles_hotbar_construction,
                )
                    .chain(),
            );
    }
}
