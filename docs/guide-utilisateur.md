# Guide utilisateur — D&D Tracker

D&D Tracker est une application interactive en terminal (TUI) écrite en Rust pour
créer et suivre des personnages de D&D 5e, avec un compendium Fight Club comme
source de données. Les libellés de l'interface sont en français; certains noms
et textes de capacités peuvent rester dans la langue du compendium importé.

Ce guide décrit les fonctions disponibles dans l'application actuelle. Le
gestionnaire suit les informations de personnage et plusieurs ressources de jeu,
mais ne remplace pas un moteur complet de règles ou un outil de table virtuelle.

## Sommaire

- [Prérequis](#prérequis)
- [Installer et lancer](#installer-et-lancer)
- [Importer un compendium](#importer-un-compendium)
- [Comprendre l'interface](#comprendre-linterface)
- [Créer un personnage](#créer-un-personnage)
- [Sections de la fiche](#sections-de-la-fiche)
- [Progression et niveaux](#progression-et-niveaux)
- [Suivi des combats et repos](#suivi-des-combats-et-repos)
- [Raccourcis clavier](#raccourcis-clavier)
- [Sauvegarde et données](#sauvegarde-et-données)
- [Limites actuelles](#limites-actuelles)
- [Développement et structure du projet](#développement-et-structure-du-projet)
- [Licence et données externes](#licence-et-données-externes)

## Prérequis

- Rust et Cargo, canal stable.
- Un compendium XML compilé au format Fight Club.
- `xsltproc` pour compiler le compendium source sous Linux ou macOS; sous
  Windows, le dépôt source fournit un script PowerShell.

## Installer et lancer

Depuis le dépôt du projet, importer d'abord un compendium (voir la section
suivante), puis lancer l'application :

```sh
cargo run
```

Pour créer un binaire optimisé :

```sh
cargo build --release
```

Le binaire est produit dans `target/release/`. Pour afficher l'aide en ligne de
commande :

```sh
cargo run -- --help
```

## Importer un compendium

Le dépôt [FightClub5eXML](https://github.com/vidalvanbergen/FightClub5eXML)
contient les sources XML. Pour compiler la collection officielle 5e 2024 :

```sh
git clone https://github.com/vidalvanbergen/FightClub5eXML.git
cd FightClub5eXML
./build-collections.sh WotC_5.5e_only.xml
```

Le fichier compilé est normalement créé sous
`FightClub5eXML/Compendiums/WotC_5.5e_only.xml`. Le dépôt source fournit son
propre script PowerShell pour Windows.

Importer ou remplacer le compendium actif :

```sh
cargo run -- --import /chemin/vers/WotC_5.5e_only.xml
```

Un chemin seul est également accepté :

```sh
cargo run -- /chemin/vers/compendium.xml
```

La variable `DND_COMPENDIUM` permet de choisir la source XML sans fournir un
argument à chaque lancement. Après l'import initial, `cargo run` charge le
compendium depuis son cache SQLite. Si le XML source du cache est toujours
disponible et a changé, le cache est actualisé. Sans cache ni chemin explicite,
l'application recherche notamment `Compendiums/WotC_5.5e_only.xml`,
`data/compendium.xml` et `WotC_5.5e+Legacy_5e.xml` dans les répertoires prévus.

L'import remplace le cache dans une transaction : si la lecture ou l'import
échoue, le cache précédent est conservé.

## Comprendre l'interface

L'écran principal comporte une liste de personnages, un panneau de fiche et une
barre d'état en bas. Appuyer sur `Tab` parcourt les cinq onglets :

1. **Fiche** — valeurs générales, caractéristiques, compétences, sauvegardes,
   sens, capacités raciales et d'historique, états et activité récente.
2. **Inventaire** — objets, quantités et état équipé.
3. **Sorts** — sorts suivis et emplacements de sorts.
4. **Progression et capacités** — XP, niveau, dons et capacités de classe.
5. **Journal et éléments supplémentaires** — histoire, notes de campagne,
   détails personnalisés et créatures ou compagnons.

La barre d'aide inférieure change selon l'onglet ou le dialogue ouvert. Les
raccourcis y apparaissent sous une forme telle que `(I)nventaire`; la touche mise
en évidence est celle à presser. `↑` et `↓` déplacent la sélection ou changent
de personnage selon l'onglet. `PgUp` et `PgDn` font défiler le panneau de
détails. `Échap` annule un dialogue; `q` quitte l'application.

Les sélecteurs peuvent être filtrés en tapant du texte. Dans les sélecteurs
multisélection de l'assistant de création, `Espace` coche ou décoche un choix et
`Entrée` passe à l'étape suivante.

## Créer un personnage

Appuyer sur `n` puis suivre les étapes affichées :

1. Saisir le nom, choisir l'espèce, la classe et l'historique.
2. Répartir les caractéristiques et sélectionner les compétences demandées par
   la classe.
3. Lorsque l'historique fournit des choix de caractéristiques, répartir ses
   bonus parmi les caractéristiques permises.
4. Choisir les paquets d'équipement A/B s'ils sont décrits dans le compendium,
   puis ajuster l'équipement.
5. Choisir les sorts initiaux proposés pour la classe.

Les maîtrises de compétences et le don détectés dans l'historique sont ajoutés
automatiquement. Les choix dont le contenu n'est pas reconnu dans le compendium
peuvent nécessiter un ajout ou une note manuelle après la création.

## Sections de la fiche

### Fiche

Les caractéristiques FOR, DEX, CON, INT, SAG et CHA affichent leur valeur et
leur modificateur. Les jets de sauvegarde, les bonus des compétences (avec `*`
pour une maîtrise), le bonus de maîtrise, l'initiative, la CA, les PV, les PV
temporaires, les dés de vie, l'XP et la perception passive sont affichés
lorsqu'ils sont calculables.

Les capacités de l'espèce et les traits de l'historique disponibles dans le
compendium sont présentés dans la fiche. Les sens, défenses et maîtrises
supplémentaires sont des champs libres modifiables dans l'onglet Journal. La
vitesse vient de l'espèce, sauf si une vitesse personnalisée est saisie.

### Inventaire

`i` ouvre le catalogue du compendium pour ajouter un objet. Chaque entrée
conserve une quantité; `Espace` équipe ou déséquipe l'objet, et `x` retire une
unité ou l'entrée sélectionnée. Les informations fournies par le compendium,
comme les dégâts, les propriétés et la CA d'une armure, sont affichées si elles
existent. La CA affichée tient compte des armures et boucliers équipés
reconnus par le compendium.

### Sorts

`p` ajoute un sort du compendium à la liste suivie et `x` le retire. Les
emplacements par niveau montrent les utilisations dépensées sur le total
disponible. `1` à `9` choisit le niveau d'emplacement actif; `]` dépense un
emplacement et `[` en restaure un.

`c` ouvre la liste des sorts suivis pour en lancer un. Le lancement d'un sort de
niveau dépense automatiquement un emplacement disponible correspondant au
niveau du sort ou au niveau d'emplacement sélectionné, si celui-ci est
supérieur. Un sort mineur ne consomme pas d'emplacement. Le lancement et les
dépenses sont inscrits au journal de combat.

### Progression et capacités

La progression affiche le niveau, l'XP, les dons connus et les capacités de
classe jusqu'au niveau courant. Les choix de sous-classe et de capacités de
classe du compendium sont filtrés selon la sous-classe choisie lorsque les
données le permettent.

### Journal et éléments supplémentaires

Les commandes du journal sont affichées au bas de l'écran. Elles permettent de
modifier :

- **Histoire personnelle** : texte multiligne, indépendant des notes de partie.
- **Notes d'aventure** : carnet multiligne modifiable au fil des séances.
- **Défenses** : notes sur les résistances, immunités, vulnérabilités ou autres
  défenses.
- **Sens spéciaux** : par exemple, vision dans le noir ou vision véritable.
- **Maîtrises et formations** : langues, outils, armures et autres formations
  qui ne sont pas déjà calculés à partir de la classe.
- **Vitesse personnalisée** : texte libre; effacer le champ rétablit la vitesse
  fournie par l'espèce.
- **Créatures et compagnons** : nom et détails libres pour un familier, une
  monture ou toute autre créature suivie.

Dans les champs histoire et notes, `Ctrl+Entrée` insère une nouvelle ligne.
Les flèches gauche et droite déplacent le curseur; `Retour arrière` efface le
caractère précédent. `E` ajoute une créature au format `nom | détails`, `M`
modifie la créature sélectionnée et `x` la retire.

## Progression et niveaux

- `e` saisit l'XP. Le niveau cible est déterminé par les seuils 5e 2024 de
  l'application.
- `l` définit le niveau sans modifier l'XP.
- `+` augmente le niveau d'une unité.

Les seuils d'XP utilisés par l'application sont :

| Niveau | XP cumulés | Niveau | XP cumulés |
| ---: | ---: | ---: | ---: |
| 1 | 0 | 11 | 85 000 |
| 2 | 300 | 12 | 100 000 |
| 3 | 900 | 13 | 120 000 |
| 4 | 2 700 | 14 | 140 000 |
| 5 | 6 500 | 15 | 165 000 |
| 6 | 14 000 | 16 | 195 000 |
| 7 | 23 000 | 17 | 225 000 |
| 8 | 34 000 | 18 | 265 000 |
| 9 | 48 000 | 19 | 305 000 |
| 10 | 64 000 | 20 | 355 000 |

Lors d'une hausse, l'application examine la progression de classe présente dans
le compendium et ouvre les choix de sous-classe, d'amélioration de
caractéristiques ou de don associés aux niveaux gagnés. Une amélioration de
caractéristiques accepte `FOR+2` ou deux caractéristiques distinctes comme
`FOR+1,DEX+1`; le total ne peut dépasser 20. Au niveau 19, si la classe demande
un don épique, le sélecteur est limité aux dons épiques.

Les dons avec prérequis interprétables sont filtrés en fonction du niveau, des
caractéristiques et de l'accès à l'incantation lorsque l'information est
disponible. Les prérequis non interprétés sont refusés plutôt que supposés
valides. Les options dépendent de la qualité et de la langue du compendium.

Une baisse de niveau retire les améliorations et dons enregistrés comme choix
de progression aux niveaux perdus, retire une sous-classe si son niveau de
déblocage n'est plus atteint, puis recalcule les ressources de classe et
emplacements. Les dons d'historique ou ajoutés manuellement ne sont pas
automatiquement supprimés.

## Suivi des combats et repos

- `h` applique une valeur signée aux PV : une valeur négative inflige des dégâts
  (les PV temporaires sont consommés en premier), une valeur positive soigne
  sans dépasser les PV maximum.
- `t` définit les PV temporaires.
- `C` ajoute une condition; des noms anglais courants comme `stunned` et `fear`
  sont normalisés en français. Dans l'onglet Fiche, `x` retire la dernière
  condition. Les conditions sont conservées lors des repos et doivent être
  retirées manuellement.
- `g` dépense un dé de vie disponible, lance le dé de la classe et applique le
  modificateur de Constitution aux soins.
- `u` dépense une utilisation de la ressource de classe sélectionnée. `[` et
  `]` changent la ressource sélectionnée dans la fiche.
- `v` effectue un repos court : restaure les ressources marquées comme
  récupérables à court repos; pour une classe dont le nom contient « Warlock »,
  restaure également les emplacements suivis.
- `r` ou `R` effectue un repos long : restaure les PV, retire les PV temporaires,
  rend une partie des dés de vie, et restaure les ressources et emplacements.
- `a` ajoute une note manuelle au journal de combat.

Le journal conserve jusqu'à 100 entrées récentes. Les sorts lancés, ressources
dépensées, changements de PV et repos y sont également notés.

## Raccourcis clavier

Les commandes disponibles dépendent de l'onglet. La barre d'aide affichée en
bas de l'écran est la référence la plus directe. Principaux raccourcis :

| Touche | Action |
| --- | --- |
| `Tab` | Passer à l'onglet suivant |
| `↑` / `↓` | Changer de personnage ou déplacer la sélection |
| `n` | Créer un personnage |
| `d` | Demander la suppression du personnage sélectionné |
| `i` | Ajouter un objet depuis le compendium |
| `p` | Ajouter un sort suivi |
| `c` | Lancer un sort suivi |
| `f` | Ajouter un don |
| `e` | Modifier l'XP |
| `l` | Modifier le niveau |
| `+` | Gagner un niveau |
| `1` à `6` | Modifier une caractéristique dans l'onglet Fiche |
| `h` / `t` | Modifier les PV / PV temporaires |
| `g` | Dépenser un dé de vie |
| `C` / `x` | Ajouter / retirer une condition dans la fiche |
| `a` | Ajouter une action ou note de combat |
| `v` / `r` | Repos court / long |
| `b` | Modifier la sous-classe |
| `Espace` | Équiper/déséquiper un objet; cocher les sélections de création |
| `x` | Retirer l'entrée sélectionnée dans l'inventaire, les sorts, dons ou créatures |
| `PgUp` / `PgDn` | Faire défiler les détails |
| `Échap` | Annuler le dialogue actif |
| `q` | Quitter |

## Sauvegarde et données

Les fiches sont sauvegardées automatiquement en JSON dans `characters.json`.
Le compendium importé est stocké séparément dans `compendium.sqlite3`.

Les deux fichiers résident dans le dossier de données de l'application :

- Linux/macOS : `$XDG_DATA_HOME/dnd-tracker/` si défini, sinon
  `~/.local/share/dnd-tracker/`.
- Windows : `%APPDATA%\dnd-tracker\`.

Les nouvelles propriétés de fiche ont des valeurs par défaut lors de la lecture
des anciennes sauvegardes. Il reste recommandé de copier `characters.json` avant
une mise à jour importante ou une modification manuelle.

## Limites actuelles

D&D Tracker ne simule pas encore tous les effets des règles. Il n'automatise pas
les jets d'attaque, dégâts, initiative de rencontre, concentration, durée des
conditions, tours de combat ni effets de sorts. Les notes de défenses, sens,
maîtrises supplémentaires et créatures sont des champs de suivi libres. Les
détails et choix automatisés dépendent des données contenues dans le compendium.
Utilisez les règles officielles de votre table pour arbitrer les situations non
gérées.

## Développement et structure du projet

Le code est organisé autour de ces modules :

- `src/main.rs` : arguments de lancement, import ou chargement du compendium et
  initialisation/restauration du terminal.
- `src/app.rs` : état de l'interface, raccourcis clavier, rendu des onglets,
  assistants et règles de suivi.
- `src/character.rs` : structures sérialisables du personnage, des ressources,
  de l'inventaire, des progressions et des créatures supplémentaires.
- `src/compendium.rs` : structures des données XML Fight Club.
- `src/compendium_store.rs` : cache SQLite, métadonnées de source et mises à
  jour transactionnelles.
- `src/storage.rs` : chemins des données utilisateur et sauvegardes JSON.

Les modifications de fiche sont enregistrées automatiquement. Les sources XML
ne sont pas incluses dans le dépôt : le compendium externe est importé dans le
cache SQLite local. Les tests couvrent notamment la désérialisation, les caches,
la création, la progression, l'interface et le suivi des repos.

### Vérifications

Depuis la racine du projet :

```sh
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

## Licence et données externes

Le code du projet est sous licence MIT (voir [LICENSE](../LICENSE)). La licence
du code ne couvre pas les contenus d'un compendium XML externe. Les fichiers
FightClub5eXML sont obtenus séparément et restent soumis aux licences et
conditions de leurs sources respectives. Le projet est indépendant et n'est
affilié ni à Wizards of the Coast ni à FightClub5eXML.
