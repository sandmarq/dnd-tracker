# D&D Tracker

Gestionnaire de personnages D&D 5e en Rust avec interface terminal (TUI). Le compendium XML est généré à partir de [FightClub5eXML](https://github.com/vidalvanbergen/FightClub5eXML); il n'est pas inclus dans ce dépôt. Après l'import initial, les données du compendium sont mises en cache dans une base SQLite locale pour accélérer les lancements suivants.

## Fonctionnalités

- Plusieurs personnages avec sauvegarde JSON automatique.
- Création guidée depuis les espèces, classes et compétences présentes dans le compendium.
- Fiche avec caractéristiques, modificateurs, compétences, sauvegardes, maîtrise, PV, CA et statistiques d'incantation.
- Progression de classe, dons et sous-classe.
- Inventaire lié aux objets XML, équipement et calcul de CA.
- Liste de sorts et suivi des emplacements.
- Suivi de PV, PV temporaires, dés de vie et ressources; repos court et long.

## Prérequis

- Rust et Cargo (canal stable).
- Un compendium XML compilé depuis le dépôt [FightClub5eXML](https://github.com/vidalvanbergen/FightClub5eXML).
- `xsltproc` pour compiler le compendium sous Linux/macOS. Le dépôt source fournit `build-collections.ps1` pour Windows.

## Préparer le compendium

Le dépôt FightClub5eXML contient des collections de fichiers XML source, pas un compendium directement lisible par l'application. Compile la collection WotC 5.5e officielle avec les outils et le script fournis par ce dépôt :

```sh
git clone https://github.com/vidalvanbergen/FightClub5eXML.git
cd FightClub5eXML
./build-collections.sh WotC_5.5e_only.xml
```

Le fichier compilé est créé ici :

```text
FightClub5eXML/Compendiums/WotC_5.5e_only.xml
```

La compilation complète inclut les contenus tiers/homebrew d'autres collections; cette commande sélectionne la collection officielle `WotC_5.5e_only.xml`.

## Lancer

Depuis la racine de ce projet, importe un compendium XML de ton choix :

```sh
cargo run -- --import /chemin/vers/FightClub5eXML/Compendiums/WotC_5.5e_only.xml
```

Un chemin passé seul reste accepté comme raccourci d'import :

```sh
cargo run -- /chemin/vers/autre-compendium.xml
```

Après l'import, démarre ou redémarre normalement :

```sh
cargo run
```

L'application lit alors le compendium actif depuis SQLite. Si le XML source enregistré existe toujours et a changé, il est réimporté automatiquement. Tu peux aussi choisir le XML au lancement avec `DND_COMPENDIUM`. Sans cache existant, l'application recherche `Compendiums/WotC_5.5e_only.xml`, `data/compendium.xml` ou `WotC_5.5e+Legacy_5e.xml` dans le dossier du projet.

Chaque remplacement est effectué dans une transaction : le cache précédent reste actif si la lecture XML ou l'import échoue. L'empreinte SHA-256 permet de réutiliser le cache lorsque le fichier est inchangé.

La base de compendium est sauvegardée dans `$XDG_DATA_HOME/dnd-tracker/compendium.sqlite3`, ou dans `~/.local/share/dnd-tracker/compendium.sqlite3` si `XDG_DATA_HOME` n'est pas défini. Sous Windows, l'application utilise `%APPDATA%\dnd-tracker\compendium.sqlite3`. Les personnages restent sauvegardés séparément en JSON dans le même dossier.

Sur Windows, passe le chemin au programme avec `cargo run -- --import C:\chemin\WotC_5.5e_only.xml`, ou définis `$env:DND_COMPENDIUM`.

Les fiches sont sauvegardées dans `$XDG_DATA_HOME/dnd-tracker/characters.json`, ou dans `~/.local/share/dnd-tracker/characters.json` si `XDG_DATA_HOME` n'est pas défini. Sous Windows, l'application utilise `%APPDATA%\dnd-tracker\characters.json`.

## Commandes

- `n` : créer une fiche; saisir le nom, choisir l'espèce et la classe, entrer les caractéristiques et choisir les compétences permises par la classe.
- `Tab` : passer entre fiche, équipement, sorts et progression.
- `↑` / `↓` : changer de personnage; dans les vues d'équipement, sorts et progression, déplacer la sélection.
- `1` à `6` dans la fiche : modifier FOR, DEX, CON, INT, SAG ou CHA.
- `i` : ajouter un objet; `Espace` équipe/déséquipe; `x` retire l'objet sélectionné.
- `p` : ajouter un sort de la classe; dans la vue des sorts, `1` à `9` choisit le niveau d'emplacement, `]` dépense un emplacement et `[` le restaure.
- `f` : ajouter un don; `x` retire le don sélectionné dans la vue de progression.
- `+` : gagner un niveau; `b` : saisir ou modifier une sous-classe.
- `h` : saisir des dégâts (nombre négatif) ou des soins (nombre positif); les dégâts épuisent d'abord les PV temporaires.
- `t` : définir les PV temporaires; `g` : lancer un dé de vie; `u` : dépenser la ressource sélectionnée.
- `[` / `]` dans la fiche : choisir une ressource; `v` : repos court; `r` : repos long.
- `PgUp` / `PgDn` : faire défiler le panneau de détails; `Esc` annule une saisie; `q` quitte.

## Développement

```sh
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Licence et données

Le code de D&D Tracker est distribué sous licence MIT (voir [LICENSE](LICENSE)). Cette licence ne s'étend pas au contenu d'un compendium XML externe.

Les fichiers de données sont obtenus séparément depuis [FightClub5eXML](https://github.com/vidalvanbergen/FightClub5eXML) et restent soumis aux conditions applicables à leurs sources respectives. Consulte le dépôt source et ses mentions de licence avant toute redistribution; ce projet ne contient ni ne republie ces données. Les règles et marques D&D appartiennent à leurs ayants droit. Ce projet est indépendant, non affilié à Wizards of the Coast et non affilié à FightClub5eXML.

## État du projet

Cette première version fournit les flux de gestion courants mais n'offre pas encore une parité complète avec Fight Club 5. Les choix de sous-classe et de dons restent manuels; les historiques, le multiclassage, les choix d'espèce, la préparation des sorts et les règles détaillées de combat sont à développer.
