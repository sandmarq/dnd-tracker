# D&D Tracker

Gestionnaire de personnages D&D 5e en Rust avec interface terminal (TUI). Le compendium XML est généré à partir de [FightClub5eXML](https://github.com/vidalvanbergen/FightClub5eXML); il n'est pas inclus dans ce dépôt. Après l'import initial, les données du compendium sont mises en cache dans une base SQLite locale pour accélérer les lancements suivants.

Consultez le [guide utilisateur complet](docs/guide-utilisateur.md) pour les
prérequis, l'installation, l'import du compendium, les sections de la fiche,
tous les raccourcis clavier, les sauvegardes et les limites connues.

## Fonctionnalités

- Plusieurs personnages avec sauvegarde JSON automatique.
- Création guidée depuis les espèces, classes, historiques et compétences du compendium.
- Application des maîtrises, du don et des bonus de caractéristiques de l'historique.
- Choix des paquets d'équipement de départ A/B, objets complémentaires et sorts de classe.
- Fiche avec caractéristiques, modificateurs, compétences, sauvegardes, maîtrise, PV, CA et statistiques d'incantation.
- Sections de fiche pour les caractéristiques, jets de sauvegarde, compétences, sens, vitesse, défenses, capacités et traits, maîtrises et formations.
- Progression guidée : choix de sous-classe, améliorations de caractéristiques ou dons éligibles aux niveaux débloqués; au niveau 19, le choix est limité aux dons épiques.
- Baisse de niveau avec retrait des choix de progression enregistrés et recalcul des ressources et emplacements encore accessibles.
- Inventaire lié aux objets XML, équipement et calcul de CA.
- Liste de sorts et suivi des emplacements.
- Suivi de PV, PV temporaires, dés de vie et ressources; repos court et long.
- Suivi de l'XP avec calcul automatique du niveau selon les seuils 5e 2024, avec modification manuelle du niveau.
- Conditions actives et journal des actions de combat (sorts lancés, emplacements et ressources dépensés, dégâts/soins, repos et notes manuelles).
- Journal de personnage avec histoire personnelle modifiable sur plusieurs lignes, notes d'aventure, défenses, sens, vitesse personnalisée, maîtrises supplémentaires et créatures/compagnons.

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

Le compendium SQLite existant est actualisé depuis son XML source pour inclure les historiques. Si ce fichier source n'est plus disponible, réimporte-le avec `--import`.

## Commandes

- `n` : créer une fiche; saisir le nom, choisir l'espèce, la classe et l'historique, répartir les caractéristiques et compétences, puis choisir l'équipement et les sorts de départ. Les maîtrises et le don de l'historique sont ajoutés automatiquement; les paquets A/B sont proposés quand le compendium les décrit.
- Dans les sélecteurs d'équipement et de sorts de création : `Espace` sélectionne/désélectionne une option et `Entrée` passe à l'étape suivante.
- `Tab` : passer entre fiche, inventaire, sorts, progression et journal.
- La barre d'aide au bas de l'écran affiche les raccourcis disponibles selon l'onglet ou le dialogue ouvert; la touche est mise en évidence sous la forme `(I)nventaire`.
- `↑` / `↓` : changer de personnage; dans les vues d'inventaire, sorts, progression et journal, déplacer la sélection.
- `1` à `6` dans la fiche : modifier FOR, DEX, CON, INT, SAG ou CHA.
- `i` : ajouter un objet; `Espace` équipe/déséquipe; `x` retire l'objet sélectionné.
- `p` : ajouter un sort de la classe; dans la vue des sorts, `1` à `9` choisit le niveau d'emplacement, `]` dépense un emplacement et `[` le restaure.
- `f` : ajouter un don; `x` retire le don sélectionné dans la vue de progression.
- `e` : définir l'XP totale; le niveau suit le seuil 5e 2024 correspondant. `l` : définir le niveau directement (sans modifier l'XP); `+` : gagner un niveau. Les changements de niveau ouvrent les choix de classe débloqués; une ASI accepte `FOR+2` ou deux caractéristiques distinctes à `+1`, sans dépasser 20. Les prérequis de dons non reconnus par le compendium sont refusés. `b` : saisir ou modifier une sous-classe.
- `h` : saisir des dégâts (nombre négatif) ou des soins (nombre positif); les dégâts épuisent d'abord les PV temporaires.
- `t` : définir les PV temporaires; `g` : lancer un dé de vie; `u` : dépenser la ressource sélectionnée.
- `C` : ajouter une condition (ex. `stunned`, `fear` ou `Effrayé`); `x` dans la fiche retire la dernière condition. Les conditions ne sont pas effacées automatiquement lors d'un repos.
- `c` : lancer un sort de la liste; un sort avec emplacement disponible le dépense automatiquement. Les sorts mineurs n'en dépensent pas.
- `a` : noter manuellement une attaque, une action ou un événement; le journal de la fiche conserve les 100 dernières entrées.
- Dans l'onglet Journal : `o` édite l'histoire personnelle, `N` les notes d'aventure, `D` les défenses, `S` les sens, `P` les maîtrises/formations, `V` la vitesse personnalisée (`Entrée` vide rétablit celle de l'espèce), `E` ajoute une créature au format `nom | détails`, `M` modifie la créature sélectionnée et `x` la retire. Dans les champs histoire/notes, `Ctrl+Entrée` insère un saut de ligne; les flèches gauche/droite et `Retour arrière` permettent de modifier le texte.
- `[` / `]` dans la fiche : choisir une ressource; `v` : repos court; `r` ou `R` : repos long. Un repos court restaure les ressources marquées récupérables à court repos et les emplacements du Warlock; un repos long restaure les emplacements et ressources.
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

Cette première version fournit les flux de gestion courants mais n'offre pas encore une parité complète avec Fight Club 5. Les durées des conditions et les tours ne sont pas chronométrés; l'application ne calcule pas automatiquement les effets de sort, les jets d'attaque, l'initiative, la concentration ou les règles détaillées de combat. Les actions non gérées sont notées manuellement avec `a`.
