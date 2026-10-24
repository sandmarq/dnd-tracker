# Localisation

D&D Tracker affiche les libellés intégrés en français par défaut et fournit un
catalogue anglais. Les fichiers JSON suivent un code de langue de type BCP 47,
par exemple `fr`, `en` ou `es-mx`. Si un catalogue régional, comme `es-mx`,
n'existe pas, l'application essaie son catalogue de base `es`. Une entrée
traduite remplace une phrase française dans l'interface; toute entrée absente
retombe explicitement sur le français.

## Choisir une langue

La barre d'aide en bas de l'écran affiche la langue active. Appuie sur `F2` pour
basculer pendant la session entre les catalogues intégrés français et anglais;
ce raccourci fonctionne aussi pendant les dialogues et les sélecteurs. Les
langues personnalisées se chargent au lancement avec `DND_LANGUAGE` :

```sh
DND_LANGUAGE=en cargo run
```

Sous PowerShell :

```powershell
$env:DND_LANGUAGE = "en"
cargo run
```

Les valeurs non reconnues, les catalogues absents et les fichiers JSON invalides
font échouer le démarrage avec un message explicite. Sans variable, l'application
utilise le français.

## Ajouter un catalogue

Crée un fichier `{code}.json` dans le dossier utilisateur `locales/` :

| Plateforme | Dossier |
| --- | --- |
| Linux/macOS | `$XDG_DATA_HOME/dnd-tracker/locales/`, sinon `~/.local/share/dnd-tracker/locales/` |
| Windows | `%APPDATA%\dnd-tracker\locales\` |

Exemple pour `es` :

```json
{
  "language": "es",
  "name": "Español",
  "translations": {
    "Fiche": "Ficha",
    "Inventaire": "Inventario",
    "Niveau ": "Nivel "
  }
}
```

`language` doit correspondre au nom du fichier sans extension. `name` est le
nom de langue affiché dans le titre de l'application. `translations` est une
table de phrases françaises vers leur traduction. Les fragments plus longs sont
appliqués avant les fragments plus courts afin de garder les phrases complètes
prioritaires. Les espaces et la ponctuation dans les clés font partie du texte
source.

Le catalogue anglais intégré sous [`locales/en.json`](../locales/en.json) est
l'exemple complet et peut servir de base. Le catalogue français de référence se
trouve sous [`locales/fr.json`](../locales/fr.json). Un fichier utilisateur du
même nom surcharge le catalogue intégré correspondant.

## Portée et maintenance

Les libellés, dialogues, aide contextuelle et messages d'état sont traduits.
Les noms, descriptions de sorts, capacités et objets du compendium restent dans
la langue fournie par le fichier XML. Les noms de personnages et notes libres
restent des données utilisateur; évite donc de traduire des fragments génériques
qui pourraient apparaître dans ces valeurs.

Pour mettre à jour les traductions :

1. Ajouter ou modifier le texte de l'interface dans l'application.
2. Ajouter les mêmes fragments français au catalogue anglais et aux catalogues
   concernés.
3. Garder les valeurs dynamiques dans les parties non traduites de la phrase;
   ne pas traduire les noms ou les notes fournis par le joueur.
4. Valider les fichiers JSON et exécuter `cargo test`.

Les tests vérifient le français par défaut, les traductions anglaises, le
chargement d'un catalogue utilisateur, le repli sur le français et le rejet
d'une langue ou d'un code malformé.
