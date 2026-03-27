# AGENTS.md

## Objectif
Ce projet doit rester lisible, modulaire, maintenable et agréable a faire evoluer.
L'agent doit prendre en charge l'architecture du projet de maniere proactive, sans attendre qu'on lui demande a chaque fois de reorganiser les fichiers.

## Langue de travail
- La communication avec l'utilisateur se fait en francais.
- L'architecture du projet doit etre pensee en francais.
- Les noms de fichiers, dossiers, modules, ressources de gameplay, systemes et concepts metier doivent etre en francais autant que possible.
- Les commentaires de code et la documentation interne doivent etre en francais.
- Exception: conserver les noms imposes par Rust, Cargo, Bevy ou une API externe quand ils font partie d'une convention technique obligatoire.

## Convention de nommage
- Utiliser des noms de fichiers en francais, en `snake_case`, sans accents.
- Exemples attendus: `gestion_camera.rs`, `generation_monde.rs`, `reseau_oxygene.rs`, `placement_structures.rs`.
- Preferer des noms explicites et metier plutot que des noms vagues comme `utils.rs`, `helpers.rs`, `misc.rs`, `manager.rs`.
- Eviter les abreviations opaques.

## Regle d'architecture
- Toujours preferer une architecture multi-fichiers et multi-modules plutot qu'un gros bloc monolithique.
- Un fichier doit avoir une responsabilite principale claire.
- Quand un fichier commence a melanger plusieurs sujets, l'agent doit le decouper de lui-meme.
- Quand une fonctionnalite grossit, l'agent doit creer les sous-modules necessaires plutot que d'empiler la logique dans un seul fichier.
- L'agent doit reflechir a l'architecture avant d'implanter une fonctionnalite, puis faire evoluer la structure si besoin.
- Il faut privilegier un decoupage par domaine metier plutot que par commodite temporaire.

## Organisation attendue
- Structurer le code par domaines clairs: `monde`, `colonie`, `construction`, `interface`, `simulation`, `rendu`, `outils`, etc.
- A l'interieur d'un domaine, separer autant que possible:
  - les composants et donnees
  - les systemes
  - la generation
  - la logique metier
  - la configuration
  - les tests
- Les fichiers `mod.rs` doivent servir de point d'entree et d'organisation, pas devenir des fichiers geants.
- Les plugins Bevy doivent rester fins et deleguer la logique a plusieurs fichiers specialises.

## Anti-monolithe
- Eviter les fichiers geants.
- Eviter les fonctions trop longues.
- Eviter les types "fourre-tout" qui centralisent trop de responsabilites.
- Eviter les modules "poubelle" du style `common.rs`, `helpers.rs` ou `god_manager.rs` si le contenu n'est pas coherent.
- Si une logique depasse une taille raisonnable ou devient difficile a lire, l'agent doit proposer puis effectuer un refactor par sous-modules.

## Attentes concretes pour l'agent
- Avant toute implementation significative, reflechir au meilleur decoupage en fichiers et modules.
- Creer automatiquement une architecture propre quand une nouvelle fonctionnalite apparait.
- Ne pas attendre une demande explicite pour reorganiser un fichier devenu trop gros ou trop confus.
- Preserver une API interne simple entre les modules.
- Limiter le couplage entre domaines.
- Favoriser des interfaces claires, des types explicites et des dependances unidirectionnelles quand c'est possible.

## Taille et lisibilite
- Preferer plusieurs petits fichiers coherents a un seul gros fichier.
- Preferer plusieurs fonctions courtes a une grosse fonction difficile a suivre.
- Chaque fichier doit pouvoir se comprendre rapidement.
- Si un fichier devient visiblement trop charge, l'agent doit le scinder.

## Qualite du code
- Le code doit etre propre des le debut, pas "on verra plus tard".
- Toute nouvelle fonctionnalite doit s'inserer dans l'architecture existante proprement.
- Les duplications importantes doivent etre reduites par une bonne factorisation, sans tomber dans l'abstraction prematuree.
- Les optimisations doivent respecter la clarte du code.

## Tests
- Ajouter des tests la ou cela a du sens, surtout pour la logique metier, la generation procedurale, les regles de placement et les reseaux de survie.
- Les tests doivent suivre la meme logique de nommage et d'organisation claire.

## Decision par defaut
Quand plusieurs options sont possibles, l'agent doit choisir celle qui:
1. garde l'architecture la plus propre,
2. evite les gros blocs monolithiques,
3. facilite les evolutions futures,
4. reste coherente avec des noms et une structure en francais.

