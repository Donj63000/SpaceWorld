use bevy::prelude::*;

use super::StructureKind;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CycleExtraction {
    pub cout_energie: f32,
    pub production_oxygene: f32,
}

impl CycleExtraction {
    pub fn en_tuple(self) -> (f32, f32) {
        (self.cout_energie, self.production_oxygene)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DefinitionStructure {
    pub kind: StructureKind,
    pub libelle: &'static str,
    pub emprise: UVec2,
    pub travail_construction: f32,
    pub relais_reseau: bool,
    pub support_vie: bool,
    pub capacite_oxygene: f32,
    pub capacite_glace: f32,
    pub generation_energie: f32,
    pub maintenance_energie: f32,
    pub capacite_stockage_solide: f32,
    pub capacite_lits: u32,
    pub cycle_extraction: Option<CycleExtraction>,
    pub couleur_materiau: Color,
    pub echelle_rendu: Vec3,
}

pub fn definition_structure(kind: StructureKind) -> DefinitionStructure {
    match kind {
        StructureKind::Lander => DefinitionStructure {
            kind,
            libelle: "Lander",
            emprise: UVec2::splat(2),
            travail_construction: 6.0,
            relais_reseau: true,
            support_vie: true,
            capacite_oxygene: 360.0,
            capacite_glace: 8.0,
            generation_energie: 0.0,
            maintenance_energie: 0.5,
            capacite_stockage_solide: 0.0,
            capacite_lits: 4,
            cycle_extraction: None,
            couleur_materiau: Color::srgb(0.86, 0.83, 0.79),
            echelle_rendu: Vec3::new(3.2, 1.7, 3.2),
        },
        StructureKind::Habitat => DefinitionStructure {
            kind,
            libelle: "Habitat",
            emprise: UVec2::splat(2),
            travail_construction: 4.0,
            relais_reseau: true,
            support_vie: true,
            capacite_oxygene: 220.0,
            capacite_glace: 0.0,
            generation_energie: 0.0,
            maintenance_energie: 1.0,
            capacite_stockage_solide: 0.0,
            capacite_lits: 4,
            cycle_extraction: None,
            couleur_materiau: Color::srgb(0.91, 0.92, 0.93),
            echelle_rendu: Vec3::new(3.1, 1.4, 3.1),
        },
        StructureKind::SolarArray => DefinitionStructure {
            kind,
            libelle: "Solar Array",
            emprise: UVec2::splat(2),
            travail_construction: 3.0,
            relais_reseau: true,
            support_vie: false,
            capacite_oxygene: 0.0,
            capacite_glace: 0.0,
            generation_energie: 6.0,
            maintenance_energie: 0.0,
            capacite_stockage_solide: 0.0,
            capacite_lits: 0,
            cycle_extraction: None,
            couleur_materiau: Color::srgb(0.20, 0.32, 0.58),
            echelle_rendu: Vec3::new(3.3, 0.25, 3.3),
        },
        StructureKind::OxygenExtractor => DefinitionStructure {
            kind,
            libelle: "O2 Extractor",
            emprise: UVec2::splat(2),
            travail_construction: 5.0,
            relais_reseau: true,
            support_vie: false,
            capacite_oxygene: 0.0,
            capacite_glace: 0.0,
            generation_energie: 0.0,
            maintenance_energie: 0.0,
            capacite_stockage_solide: 0.0,
            capacite_lits: 0,
            cycle_extraction: Some(CycleExtraction {
                cout_energie: 2.0,
                production_oxygene: 12.0,
            }),
            couleur_materiau: Color::srgb(0.84, 0.48, 0.22),
            echelle_rendu: Vec3::new(2.8, 1.1, 2.8),
        },
        StructureKind::Storage => DefinitionStructure {
            kind,
            libelle: "Storage",
            emprise: UVec2::splat(1),
            travail_construction: 2.0,
            relais_reseau: true,
            support_vie: false,
            capacite_oxygene: 0.0,
            capacite_glace: 18.0,
            generation_energie: 0.0,
            maintenance_energie: 0.0,
            capacite_stockage_solide: 24.0,
            capacite_lits: 0,
            cycle_extraction: None,
            couleur_materiau: Color::srgb(0.44, 0.47, 0.53),
            echelle_rendu: Vec3::new(1.2, 0.9, 1.2),
        },
        StructureKind::Tube => DefinitionStructure {
            kind,
            libelle: "Tube",
            emprise: UVec2::splat(1),
            travail_construction: 1.0,
            relais_reseau: true,
            support_vie: false,
            capacite_oxygene: 0.0,
            capacite_glace: 0.0,
            generation_energie: 0.0,
            maintenance_energie: 0.0,
            capacite_stockage_solide: 0.0,
            capacite_lits: 0,
            cycle_extraction: None,
            couleur_materiau: Color::srgb(0.78, 0.79, 0.82),
            echelle_rendu: Vec3::new(1.4, 0.25, 1.4),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lextracteur_porte_son_cycle_dans_la_definition() {
        let definition = definition_structure(StructureKind::OxygenExtractor);

        assert_eq!(definition.kind, StructureKind::OxygenExtractor);
        assert_eq!(definition.emprise, UVec2::splat(2));
        assert_eq!(
            definition.cycle_extraction,
            Some(CycleExtraction {
                cout_energie: 2.0,
                production_oxygene: 12.0,
            })
        );
    }

    #[test]
    fn les_wrappers_de_structurekind_restent_alignes_sur_la_definition() {
        let definition = definition_structure(StructureKind::Storage);

        assert_eq!(StructureKind::Storage.label(), definition.libelle);
        assert_eq!(StructureKind::Storage.footprint(), definition.emprise);
        assert_eq!(
            StructureKind::Storage.ice_capacity(),
            definition.capacite_glace
        );
        assert_eq!(StructureKind::Storage.scale(), definition.echelle_rendu);
    }
}
