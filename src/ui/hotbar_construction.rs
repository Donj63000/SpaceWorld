use bevy::ecs::prelude::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;

use crate::colony::StructureKind;
use crate::construction::SelectedBuild;

use super::polices::PoliceInterface;
use super::references_hud::{
    AssetsHud, ReferencesHotbarHud, ZoneCaptureCurseurUi, exiger_reference,
};
use super::theme_cockpit as theme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MetadonneeSlotConstruction {
    pub(crate) kind: StructureKind,
    pub(crate) numero: u8,
    pub(crate) libelle: &'static str,
    pub(crate) description: &'static str,
    pub(crate) couleur: Color,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BoutonSlotConstruction {
    pub(crate) kind: StructureKind,
    pub(crate) barre_accent: Entity,
}

pub(crate) fn metadonnees_slots_construction() -> [MetadonneeSlotConstruction; 6] {
    [
        MetadonneeSlotConstruction {
            kind: StructureKind::Habitat,
            numero: 1,
            libelle: "Habitat",
            description: "Vie",
            couleur: theme::COULEUR_OK,
        },
        MetadonneeSlotConstruction {
            kind: StructureKind::SolarArray,
            numero: 2,
            libelle: "Solaire",
            description: "Energie",
            couleur: theme::COULEUR_ACCENT_CYAN,
        },
        MetadonneeSlotConstruction {
            kind: StructureKind::OxygenExtractor,
            numero: 3,
            libelle: "Extracteur O2",
            description: "Production",
            couleur: theme::COULEUR_ACCENT_OXYDE,
        },
        MetadonneeSlotConstruction {
            kind: StructureKind::Storage,
            numero: 4,
            libelle: "Stockage",
            description: "Reserve",
            couleur: theme::COULEUR_ACCENT_ACIER,
        },
        MetadonneeSlotConstruction {
            kind: StructureKind::Tube,
            numero: 5,
            libelle: "Tube",
            description: "Relais",
            couleur: theme::COULEUR_SURVEILLANCE,
        },
        MetadonneeSlotConstruction {
            kind: StructureKind::Lander,
            numero: 6,
            libelle: "Lander",
            description: "Base",
            couleur: theme::COULEUR_BORDURE_ACTIVE,
        },
    ]
}

pub(crate) fn metadonnee_slot(kind: StructureKind) -> MetadonneeSlotConstruction {
    metadonnees_slots_construction()
        .into_iter()
        .find(|metadonnees| metadonnees.kind == kind)
        .unwrap_or_else(|| panic!("Slot HUD manquant pour {:?}", kind))
}

pub(crate) fn spawn_hotbar_construction(
    parent: &mut ChildSpawnerCommands,
    police: &PoliceInterface,
    assets: &AssetsHud,
) -> ReferencesHotbarHud {
    let mut racine_hotbar = None;

    parent
        .spawn(Node {
            width: percent(100),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|rangee| {
            racine_hotbar = Some(
                rangee
                    .spawn((
                        ZoneCaptureCurseurUi,
                        theme::style_hotbar_exterieure(),
                        BackgroundColor(theme::COULEUR_FOND_HOTBAR),
                        BorderColor::all(theme::COULEUR_BORDURE),
                    ))
                    .with_children(|panel| {
                        panel.spawn((
                            ImageNode {
                                image: assets.texture_grille.clone(),
                                color: theme::COULEUR_TEXTURE,
                                image_mode: NodeImageMode::Tiled {
                                    tile_x: true,
                                    tile_y: true,
                                    stretch_value: 1.0,
                                },
                                ..default()
                            },
                            theme::style_texture_fond(),
                        ));

                        panel
                            .spawn(theme::style_hotbar_contenu())
                            .with_children(|contenu| {
                                contenu.spawn((
                                    Text::new("Construction rapide // 1-6 ou clic"),
                                    police.titre(12.0),
                                    TextColor(theme::COULEUR_TEXTE_ATTENUE),
                                ));

                                contenu
                                    .spawn(theme::style_ligne_slots())
                                    .with_children(|slots| {
                                        for metadonnees in metadonnees_slots_construction() {
                                            spawn_slot_construction(
                                                slots,
                                                police,
                                                assets,
                                                metadonnees,
                                            );
                                        }
                                    });
                            });
                    })
                    .id(),
            );
        });

    ReferencesHotbarHud {
        racine: exiger_reference(racine_hotbar, "racine_hotbar"),
    }
}

pub(crate) fn traiter_clics_hotbar_construction(
    mut selected: ResMut<SelectedBuild>,
    interactions: Query<
        (&Interaction, &BoutonSlotConstruction),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, bouton) in &interactions {
        if *interaction == Interaction::Pressed {
            selected.0 = bouton.kind;
        }
    }
}

pub(crate) fn synchroniser_styles_hotbar_construction(
    references_hotbar: Res<ReferencesHotbarHud>,
    selected: Res<SelectedBuild>,
    mut boutons: Query<
        (
            &Interaction,
            &BoutonSlotConstruction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
    mut accents: Query<&mut BackgroundColor, Without<Button>>,
    mut bordures_hors_boutons: Query<&mut BorderColor, Without<Button>>,
) {
    if let Ok(mut bordure_hotbar) = bordures_hors_boutons.get_mut(references_hotbar.racine) {
        *bordure_hotbar = BorderColor::all(metadonnee_slot(selected.0).couleur);
    }

    for (interaction, bouton, mut fond, mut bordure) in &mut boutons {
        let metadonnees = metadonnee_slot(bouton.kind);
        let visuel = style_visuel_slot(*interaction, bouton.kind == selected.0, metadonnees);
        *fond = BackgroundColor(visuel.fond);
        *bordure = BorderColor::all(visuel.bordure);
        if let Ok(mut accent) = accents.get_mut(bouton.barre_accent) {
            *accent = BackgroundColor(visuel.accent);
        }
    }
}

fn spawn_slot_construction(
    parent: &mut ChildSpawnerCommands,
    police: &PoliceInterface,
    assets: &AssetsHud,
    metadonnees: MetadonneeSlotConstruction,
) {
    let style_initial = style_visuel_slot(Interaction::None, false, metadonnees);
    let mut barre_accent = None;

    let mut slot = parent.spawn((
        Button,
        FocusPolicy::Block,
        theme::style_slot_construction(),
        BackgroundColor(style_initial.fond),
        BorderColor::all(style_initial.bordure),
    ));
    let id_slot = slot.id();
    slot.with_children(|contenu| {
        contenu.spawn((
            ImageNode {
                image: assets.texture_grille.clone(),
                color: theme::COULEUR_TEXTURE,
                image_mode: NodeImageMode::Tiled {
                    tile_x: true,
                    tile_y: true,
                    stretch_value: 1.0,
                },
                ..default()
            },
            theme::style_texture_fond(),
        ));
        barre_accent = Some(
            contenu
                .spawn((
                    theme::style_barre_accent_slot(),
                    BackgroundColor(style_initial.accent),
                ))
                .id(),
        );
        contenu.spawn((
            Text::new(format!("{:02}", metadonnees.numero)),
            police.titre(18.0),
            TextColor(metadonnees.couleur),
        ));
        contenu.spawn((
            Text::new(metadonnees.libelle),
            police.titre(13.5),
            TextColor(theme::COULEUR_TEXTE),
        ));
        contenu.spawn((
            Text::new(metadonnees.description),
            police.corps(12.0),
            TextColor(theme::COULEUR_TEXTE_ATTENUE),
        ));
    });

    parent.commands().entity(id_slot).insert(BoutonSlotConstruction {
        kind: metadonnees.kind,
        barre_accent: exiger_reference(barre_accent, "barre_accent_slot_construction"),
    });
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StyleVisuelSlot {
    fond: Color,
    bordure: Color,
    accent: Color,
}

fn style_visuel_slot(
    interaction: Interaction,
    selectionne: bool,
    metadonnees: MetadonneeSlotConstruction,
) -> StyleVisuelSlot {
    if selectionne {
        StyleVisuelSlot {
            fond: theme::COULEUR_SLOT_SELECTION,
            bordure: metadonnees.couleur,
            accent: metadonnees.couleur,
        }
    } else if interaction == Interaction::Hovered {
        StyleVisuelSlot {
            fond: theme::COULEUR_SLOT_SURVOL,
            bordure: theme::COULEUR_BORDURE_ACTIVE,
            accent: metadonnees.couleur,
        }
    } else {
        StyleVisuelSlot {
            fond: theme::COULEUR_SLOT_NORMAL,
            bordure: theme::COULEUR_BORDURE,
            accent: Color::srgba(0.30, 0.34, 0.38, 0.95),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadonnees_slots_conservent_lordre_de_construction() {
        let slots = metadonnees_slots_construction();

        assert_eq!(slots.len(), 6);
        assert_eq!(slots[0].kind, StructureKind::Habitat);
        assert_eq!(slots[0].numero, 1);
        assert_eq!(slots[1].libelle, "Solaire");
        assert_eq!(slots[2].libelle, "Extracteur O2");
        assert_eq!(slots[5].kind, StructureKind::Lander);
    }

    #[test]
    fn clic_hotbar_met_a_jour_le_module_selectionne() {
        let mut app = App::new();
        let barre = app.world_mut().spawn_empty().id();
        app.insert_resource(SelectedBuild(StructureKind::Habitat));
        app.add_systems(Update, traiter_clics_hotbar_construction);
        app.world_mut().spawn((
            Button,
            Interaction::Pressed,
            BoutonSlotConstruction {
                kind: StructureKind::SolarArray,
                barre_accent: barre,
            },
        ));

        app.update();

        assert_eq!(
            app.world().resource::<SelectedBuild>().0,
            StructureKind::SolarArray
        );
    }
}
