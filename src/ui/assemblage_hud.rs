use bevy::ecs::prelude::ChildSpawnerCommands;
use bevy::prelude::*;

use super::hotbar_construction::spawn_hotbar_construction;
use super::polices::PoliceInterface;
use super::references_hud::{
    AssetsHud, ReferencesAlertesHud, ReferencesConstructionHud, ReferencesEquipageHud,
    ReferencesMissionHud, ReferencesReseauxHud, RacineHud, ZoneCaptureCurseurUi,
    exiger_reference,
};
use super::theme_cockpit as theme;

const LARGEUR_COLONNE_GAUCHE: f32 = 436.0;
const LARGEUR_COLONNE_DROITE: f32 = 376.0;

pub(crate) fn setup_hud(
    mut commands: Commands,
    police: Res<PoliceInterface>,
    assets: Res<AssetsHud>,
) {
    let mut mission_entete = None;
    let mut mission_resume = None;
    let mut mission_badge = None;
    let mut mission_objectif = None;
    let mut mission_libelle_oxygene = None;
    let mut mission_barre_oxygene = None;
    let mut mission_glace = None;
    let mut mission_energie = None;
    let mut mission_structures = None;
    let mut mission_taches = None;
    let mut equipage_cartes = None;
    let mut construction_module = None;
    let mut construction_case = None;
    let mut construction_connexion = None;
    let mut construction_feedback = None;
    let mut badge_alerte = None;
    let mut detail_alertes = None;
    let mut libelle_energie = None;
    let mut barre_energie = None;
    let mut libelle_glace = None;
    let mut barre_glace = None;
    let mut references_hotbar = None;

    commands
        .spawn((
            RacineHud,
            Visibility::Hidden,
            GlobalZIndex(30),
            theme::style_racine_hud(),
        ))
        .with_children(|racine| {
            racine
                .spawn(theme::style_rangee_superieure())
                .with_children(|rangee| {
                    rangee
                        .spawn(theme::style_colonne(LARGEUR_COLONNE_GAUCHE))
                        .with_children(|colonne| {
                            spawn_panneau_cockpit(
                                colonne,
                                "Mission",
                                police.as_ref(),
                                assets.as_ref(),
                                true,
                                |panel| {
                                    mission_entete = Some(
                                        panel
                                            .spawn((
                                                Text::new("MARS // SEED 0"),
                                                police.titre(24.0),
                                                TextColor(theme::COULEUR_TEXTE),
                                            ))
                                            .id(),
                                    );

                                    panel
                                        .spawn(theme::style_ligne_entete())
                                        .with_children(|ligne| {
                                            mission_resume = Some(
                                                ligne
                                                    .spawn((
                                                        Text::new(
                                                            "Boot | 0 structures | 0 taches",
                                                        ),
                                                        police.corps(13.0),
                                                        TextColor(theme::COULEUR_TEXTE_ATTENUE),
                                                    ))
                                                    .id(),
                                            );
                                            mission_badge = Some(spawn_badge(
                                                ligne,
                                                "NOMINAL",
                                                theme::COULEUR_OK,
                                                police.as_ref(),
                                            ));
                                        });

                                    mission_objectif = Some(
                                        panel
                                            .spawn((
                                                Text::new(
                                                    "Stabilise la colonie et etends le reseau vital.",
                                                ),
                                                police.corps(13.0),
                                                TextColor(theme::COULEUR_TEXTE_FAIBLE),
                                                Node {
                                                    width: percent(100),
                                                    ..default()
                                                },
                                            ))
                                            .id(),
                                    );

                                    mission_libelle_oxygene = Some(
                                        panel
                                            .spawn((
                                                Text::new("Reserve O2 colonie : 0 / 0"),
                                                police.corps(14.0),
                                                TextColor(theme::COULEUR_TEXTE),
                                            ))
                                            .id(),
                                    );

                                    panel
                                        .spawn((
                                            theme::style_barre_piste(),
                                            BackgroundColor(theme::COULEUR_PISTE),
                                            BorderColor::all(theme::COULEUR_BORDURE),
                                        ))
                                        .with_children(|track| {
                                            mission_barre_oxygene = Some(
                                                track
                                                    .spawn((
                                                        theme::style_barre_remplissage(),
                                                        BackgroundColor(theme::COULEUR_OK),
                                                    ))
                                                    .id(),
                                            );
                                        });

                                    panel
                                        .spawn(theme::style_grille_synthese())
                                        .with_children(|grid| {
                                            mission_glace = Some(spawn_carte_synthese(
                                                grid,
                                                "Glace stockee",
                                                "0 / 0",
                                                police.as_ref(),
                                            ));
                                            mission_energie = Some(spawn_carte_synthese(
                                                grid,
                                                "Energie",
                                                "0.0 / 0.0",
                                                police.as_ref(),
                                            ));
                                            mission_structures = Some(spawn_carte_synthese(
                                                grid,
                                                "Batiments",
                                                "0",
                                                police.as_ref(),
                                            ));
                                            mission_taches = Some(spawn_carte_synthese(
                                                grid,
                                                "Taches",
                                                "0",
                                                police.as_ref(),
                                            ));
                                        });
                                },
                            );

                            spawn_panneau_cockpit(
                                colonne,
                                "Equipage",
                                police.as_ref(),
                                assets.as_ref(),
                                true,
                                |panel| {
                                    panel.spawn((
                                        Text::new("Etat EVA en direct et reserves individuelles."),
                                        police.corps(13.0),
                                        TextColor(theme::COULEUR_TEXTE_ATTENUE),
                                    ));
                                    equipage_cartes = Some(
                                        panel.spawn(theme::style_conteneur_cartes_equipage()).id(),
                                    );
                                },
                            );
                        });

                    rangee
                        .spawn(theme::style_colonne(LARGEUR_COLONNE_DROITE))
                        .with_children(|colonne| {
                            spawn_panneau_cockpit(
                                colonne,
                                "Construction",
                                police.as_ref(),
                                assets.as_ref(),
                                true,
                                |panel| {
                                    construction_module = Some(
                                        panel
                                            .spawn((
                                                Text::new("Habitat"),
                                                police.titre(22.0),
                                                TextColor(theme::COULEUR_OK),
                                            ))
                                            .id(),
                                    );
                                    construction_case = Some(
                                        panel
                                            .spawn((
                                                Text::new("Cellule cible : --"),
                                                police.corps(13.0),
                                                TextColor(theme::COULEUR_TEXTE_ATTENUE),
                                            ))
                                            .id(),
                                    );
                                    construction_connexion = Some(
                                        panel
                                            .spawn((
                                                Text::new("Connexion reseau : valide"),
                                                police.corps(13.0),
                                                TextColor(theme::COULEUR_OK),
                                            ))
                                            .id(),
                                    );
                                    construction_feedback = Some(
                                        panel
                                            .spawn((
                                                Text::new("Deplace le curseur sur le terrain."),
                                                police.corps(14.0),
                                                TextColor(theme::COULEUR_TEXTE),
                                                Node {
                                                    width: percent(100),
                                                    ..default()
                                                },
                                            ))
                                            .id(),
                                    );
                                    panel.spawn((
                                        Text::new(
                                            "Hotbar en bas pour changer de module. ZQSD/WASD pour la camera, molette pour le zoom, Espace pour pause.",
                                        ),
                                        police.corps(12.5),
                                        TextColor(theme::COULEUR_TEXTE_FAIBLE),
                                        Node {
                                            width: percent(100),
                                            ..default()
                                        },
                                    ));
                                },
                            );

                            spawn_panneau_cockpit(
                                colonne,
                                "Alertes",
                                police.as_ref(),
                                assets.as_ref(),
                                true,
                                |panel| {
                                    badge_alerte = Some(spawn_badge(
                                        panel,
                                        "NOMINAL",
                                        theme::COULEUR_OK,
                                        police.as_ref(),
                                    ));
                                    detail_alertes = Some(
                                        panel
                                            .spawn((
                                                Text::new("Aucune alerte active."),
                                                police.corps(13.0),
                                                TextColor(theme::COULEUR_TEXTE),
                                                Node {
                                                    width: percent(100),
                                                    ..default()
                                                },
                                            ))
                                            .id(),
                                    );
                                },
                            );

                            spawn_panneau_cockpit(
                                colonne,
                                "Reseaux vitaux",
                                police.as_ref(),
                                assets.as_ref(),
                                true,
                                |panel| {
                                    let (texte_energie, remplissage_energie) =
                                        spawn_barre_ressource(
                                            panel,
                                            "Energie disponible : 0.0 / 0.0",
                                            theme::COULEUR_ACCENT_CYAN,
                                            police.as_ref(),
                                        );
                                    libelle_energie = Some(texte_energie);
                                    barre_energie = Some(remplissage_energie);

                                    let (texte_glace, remplissage_glace) = spawn_barre_ressource(
                                        panel,
                                        "Reserves de glace : 0 / 0",
                                        theme::COULEUR_ACCENT_ACIER,
                                        police.as_ref(),
                                    );
                                    libelle_glace = Some(texte_glace);
                                    barre_glace = Some(remplissage_glace);
                                },
                            );
                        });
                });

            references_hotbar = Some(spawn_hotbar_construction(
                racine,
                police.as_ref(),
                assets.as_ref(),
            ));
        });

    commands.insert_resource(ReferencesMissionHud {
        entete: exiger_reference(mission_entete, "mission_entete"),
        resume: exiger_reference(mission_resume, "mission_resume"),
        badge_niveau: exiger_reference(mission_badge, "mission_badge"),
        objectif: exiger_reference(mission_objectif, "mission_objectif"),
        libelle_oxygene: exiger_reference(mission_libelle_oxygene, "mission_libelle_oxygene"),
        barre_oxygene: exiger_reference(mission_barre_oxygene, "mission_barre_oxygene"),
        synthese_glace: exiger_reference(mission_glace, "mission_glace"),
        synthese_energie: exiger_reference(mission_energie, "mission_energie"),
        synthese_structures: exiger_reference(mission_structures, "mission_structures"),
        synthese_taches: exiger_reference(mission_taches, "mission_taches"),
    });
    commands.insert_resource(ReferencesEquipageHud {
        conteneur_cartes: exiger_reference(equipage_cartes, "equipage_cartes"),
    });
    commands.insert_resource(ReferencesConstructionHud {
        module_actif: exiger_reference(construction_module, "construction_module"),
        case_survolee: exiger_reference(construction_case, "construction_case"),
        etat_connexion: exiger_reference(construction_connexion, "construction_connexion"),
        feedback: exiger_reference(construction_feedback, "construction_feedback"),
    });
    commands.insert_resource(ReferencesAlertesHud {
        badge_niveau: exiger_reference(badge_alerte, "badge_alerte"),
        detail: exiger_reference(detail_alertes, "detail_alertes"),
    });
    commands.insert_resource(ReferencesReseauxHud {
        libelle_energie: exiger_reference(libelle_energie, "libelle_energie"),
        barre_energie: exiger_reference(barre_energie, "barre_energie"),
        libelle_glace: exiger_reference(libelle_glace, "libelle_glace"),
        barre_glace: exiger_reference(barre_glace, "barre_glace"),
    });
    commands.insert_resource(
        references_hotbar.unwrap_or_else(|| panic!("Reference HUD manquante : hotbar")),
    );
}

fn spawn_panneau_cockpit<F>(
    parent: &mut ChildSpawnerCommands,
    titre: &str,
    police: &PoliceInterface,
    assets: &AssetsHud,
    capture_curseur: bool,
    contenu: F,
) where
    F: FnOnce(&mut ChildSpawnerCommands),
{
    let mut panneau = parent.spawn((
        theme::style_panneau(),
        BackgroundColor(theme::COULEUR_PANNEAU_PRINCIPAL),
        BorderColor::all(theme::COULEUR_BORDURE),
    ));
    if capture_curseur {
        panneau.insert(ZoneCaptureCurseurUi);
    }

    panneau.with_children(|panel| {
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
            .spawn(theme::style_contenu_panneau())
            .with_children(|contenu_panel| {
                contenu_panel.spawn((
                    Text::new(titre.to_uppercase()),
                    police.titre(12.0),
                    TextColor(theme::COULEUR_TEXTE_ATTENUE),
                ));
                contenu(contenu_panel);
            });
    });
}

fn spawn_carte_synthese(
    parent: &mut ChildSpawnerCommands,
    libelle: &str,
    valeur_initiale: &str,
    police: &PoliceInterface,
) -> Entity {
    let mut valeur = None;

    parent
        .spawn((
            theme::style_carte_synthese(),
            BackgroundColor(theme::COULEUR_PANNEAU_SECONDAIRE),
            BorderColor::all(theme::COULEUR_BORDURE),
        ))
        .with_children(|carte| {
            carte.spawn((
                Text::new(libelle),
                police.corps(12.5),
                TextColor(theme::COULEUR_TEXTE_ATTENUE),
            ));
            valeur = Some(
                carte
                    .spawn((
                        Text::new(valeur_initiale),
                        police.titre(19.0),
                        TextColor(theme::COULEUR_TEXTE),
                    ))
                    .id(),
            );
        });

    exiger_reference(valeur, libelle)
}

fn spawn_badge(
    parent: &mut ChildSpawnerCommands,
    texte: &str,
    couleur: Color,
    police: &PoliceInterface,
) -> Entity {
    parent
        .spawn((
            Text::new(texte),
            police.titre(11.5),
            TextColor(Color::WHITE),
            theme::style_badge(),
            BackgroundColor(couleur),
            BorderColor::all(couleur),
        ))
        .id()
}

fn spawn_barre_ressource(
    parent: &mut ChildSpawnerCommands,
    libelle_initial: &str,
    couleur_initiale: Color,
    police: &PoliceInterface,
) -> (Entity, Entity) {
    let texte = parent
        .spawn((
            Text::new(libelle_initial),
            police.corps(13.5),
            TextColor(theme::COULEUR_TEXTE),
        ))
        .id();

    let mut remplissage = None;
    parent
        .spawn((
            theme::style_barre_piste(),
            BackgroundColor(theme::COULEUR_PISTE),
            BorderColor::all(theme::COULEUR_BORDURE),
        ))
        .with_children(|track| {
            remplissage = Some(
                track
                    .spawn((
                        theme::style_barre_remplissage(),
                        BackgroundColor(couleur_initiale),
                    ))
                    .id(),
            );
        });

    (texte, exiger_reference(remplissage, libelle_initial))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::hotbar_construction::BoutonSlotConstruction;

    #[test]
    fn setup_hud_cree_les_references_et_six_slots_de_hotbar() {
        let mut app = App::new();
        app.insert_resource(PoliceInterface::factices());
        app.insert_resource(AssetsHud::factices());
        app.add_systems(Startup, setup_hud);

        app.update();

        assert!(app.world().get_resource::<ReferencesMissionHud>().is_some());
        assert!(app.world().get_resource::<ReferencesConstructionHud>().is_some());
        assert!(app.world().get_resource::<ReferencesAlertesHud>().is_some());
        assert!(app.world().get_resource::<ReferencesReseauxHud>().is_some());
        assert!(app.world().get_resource::<ReferencesEquipageHud>().is_some());
        assert!(app.world().get_resource::<super::super::references_hud::ReferencesHotbarHud>().is_some());
        let mut requete = app.world_mut().query::<&BoutonSlotConstruction>();
        assert_eq!(
            requete.iter(app.world()).count(),
            6
        );
    }
}
