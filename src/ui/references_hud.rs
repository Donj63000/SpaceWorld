use bevy::prelude::*;

const CHEMIN_TEXTURE_GRILLE_HUD: &str = "textures/hud_grille.ppm";

#[derive(Component)]
pub(crate) struct RacineHud;

#[derive(Component)]
pub(crate) struct ZoneCaptureCurseurUi;

#[derive(Resource, Clone)]
pub(crate) struct AssetsHud {
    pub(crate) texture_grille: Handle<Image>,
}

impl AssetsHud {
    #[cfg(test)]
    pub(crate) fn factices() -> Self {
        Self {
            texture_grille: Handle::default(),
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub(crate) struct ReferencesMissionHud {
    pub(crate) entete: Entity,
    pub(crate) resume: Entity,
    pub(crate) badge_niveau: Entity,
    pub(crate) objectif: Entity,
    pub(crate) libelle_oxygene: Entity,
    pub(crate) barre_oxygene: Entity,
    pub(crate) synthese_glace: Entity,
    pub(crate) synthese_energie: Entity,
    pub(crate) synthese_structures: Entity,
    pub(crate) synthese_taches: Entity,
}

#[derive(Resource, Clone, Copy)]
pub(crate) struct ReferencesEquipageHud {
    pub(crate) conteneur_cartes: Entity,
}

#[derive(Resource, Clone, Copy)]
pub(crate) struct ReferencesConstructionHud {
    pub(crate) module_actif: Entity,
    pub(crate) case_survolee: Entity,
    pub(crate) etat_connexion: Entity,
    pub(crate) feedback: Entity,
}

#[derive(Resource, Clone, Copy)]
pub(crate) struct ReferencesAlertesHud {
    pub(crate) badge_niveau: Entity,
    pub(crate) detail: Entity,
}

#[derive(Resource, Clone, Copy)]
pub(crate) struct ReferencesReseauxHud {
    pub(crate) libelle_energie: Entity,
    pub(crate) barre_energie: Entity,
    pub(crate) libelle_glace: Entity,
    pub(crate) barre_glace: Entity,
}

#[derive(Resource, Clone, Copy)]
pub(crate) struct ReferencesHotbarHud {
    pub(crate) racine: Entity,
}

#[derive(Resource, Default, Clone, Copy)]
pub(crate) struct MemoireHudMission {
    pub(crate) dernier_nombre_structures: usize,
}

#[derive(Resource, Default, Clone, Copy)]
pub(crate) struct CadenceHudEquipage {
    pub(crate) prochain_rafraichissement: f64,
    pub(crate) dernier_effectif: usize,
}

pub(crate) fn charger_assets_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(AssetsHud {
        texture_grille: asset_server.load(CHEMIN_TEXTURE_GRILLE_HUD),
    });
}

pub(crate) fn exiger_reference(reference: Option<Entity>, nom: &str) -> Entity {
    reference.unwrap_or_else(|| panic!("Reference HUD manquante : {nom}"))
}
