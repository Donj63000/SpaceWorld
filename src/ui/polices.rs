use bevy::prelude::*;

const CHEMIN_POLICE_CORPS: &str = "polices/noto_sans_regular.ttf";
const CHEMIN_POLICE_TITRE: &str = "polices/orbitron_titre.ttf";

#[derive(Resource, Clone)]
pub(crate) struct PoliceInterface {
    reguliere: Handle<Font>,
    titre: Handle<Font>,
}

impl PoliceInterface {
    pub(crate) fn titre(&self, size: f32) -> TextFont {
        TextFont::from_font_size(size).with_font(self.titre.clone())
    }

    pub(crate) fn corps(&self, size: f32) -> TextFont {
        TextFont::from_font_size(size).with_font(self.reguliere.clone())
    }

    #[cfg(test)]
    pub(crate) fn factices() -> Self {
        Self {
            reguliere: Handle::default(),
            titre: Handle::default(),
        }
    }
}

pub(crate) fn charger_police_interface(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(PoliceInterface {
        reguliere: asset_server.load(CHEMIN_POLICE_CORPS),
        titre: asset_server.load(CHEMIN_POLICE_TITRE),
    });
}
