//! Chargement et application des catalogues de traduction JSON.

use crate::storage;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const BUILTIN_ENGLISH: &str = include_str!("../locales/en.json");
const BUILTIN_FRENCH: &str = include_str!("../locales/fr.json");

#[derive(Debug, Deserialize)]
struct Catalog {
    language: String,
    name: String,
    translations: BTreeMap<String, String>,
}

/// Catalogue actif; les entrées inconnues restent dans leur langue source (français).
pub struct Localization {
    language: String,
    name: String,
    translations: Vec<(String, String)>,
}

impl Localization {
    /// Charge la langue demandée par `DND_LANGUAGE`, ou le français par défaut.
    pub fn from_environment() -> Result<Self, Box<dyn Error>> {
        let requested = match std::env::var_os("DND_LANGUAGE") {
            Some(value) => value
                .into_string()
                .map_err(|_| "DND_LANGUAGE must contain valid Unicode")?,
            None => "fr".to_owned(),
        };
        Self::from_language(&requested)
    }

    /// Charge une langue intégrée ou installée dans le dossier utilisateur.
    pub fn from_language(language: &str) -> Result<Self, Box<dyn Error>> {
        Self::load(language, &locale_directory()?)
    }

    /// Charge un catalogue intégré ou utilisateur avec validation du code BCP 47 simple.
    fn load(language: &str, directory: &Path) -> Result<Self, Box<dyn Error>> {
        let normalized = language.replace('_', "-").to_ascii_lowercase();
        let segments: Vec<_> = normalized.split('-').collect();
        if !(2..=8).contains(&segments[0].len())
            || !segments[0].bytes().all(|byte| byte.is_ascii_alphabetic())
            || segments.iter().any(|segment| {
                segment.is_empty()
                    || segment.len() > 8
                    || !segment.bytes().all(|byte| byte.is_ascii_alphanumeric())
            })
        {
            return Err(format!("Code de langue invalide : {language}").into());
        }
        let base_language = segments[0];
        let requested_path = directory.join(format!("{normalized}.json"));
        let base_path = directory.join(format!("{base_language}.json"));
        let (catalog_language, external_path, source) = if requested_path.is_file() {
            (
                normalized.as_str(),
                requested_path.clone(),
                fs::read_to_string(&requested_path)?,
            )
        } else if base_path.is_file() {
            (
                base_language,
                base_path.clone(),
                fs::read_to_string(&base_path)?,
            )
        } else if base_language == "en" {
            (base_language, requested_path, BUILTIN_ENGLISH.to_owned())
        } else if base_language == "fr" {
            (base_language, requested_path, BUILTIN_FRENCH.to_owned())
        } else {
            return Err(format!(
                "Catalogue de langue « {normalized} » introuvable. Ajoutez {}/{}.json.",
                directory.display(),
                normalized
            )
            .into());
        };

        let catalog: Catalog = serde_json::from_str(&source)?;
        if catalog.language.replace('_', "-").to_ascii_lowercase() != catalog_language {
            return Err(format!(
                "Le code de langue déclaré ({}) ne correspond pas au catalogue {catalog_language}.",
                catalog.language
            )
            .into());
        }
        if catalog.name.trim().is_empty() {
            return Err(format!(
                "Le catalogue {} doit avoir un nom non vide.",
                external_path.display()
            )
            .into());
        }
        let mut translations: Vec<_> = catalog.translations.into_iter().collect();
        if translations
            .iter()
            .any(|(source, target)| source.is_empty() || target.is_empty())
        {
            return Err(format!(
                "Le catalogue {} contient une entrée vide.",
                external_path.display()
            )
            .into());
        }
        translations.sort_by_key(|(source, _)| std::cmp::Reverse(source.len()));
        Ok(Self {
            language: catalog_language.to_owned(),
            name: catalog.name,
            translations,
        })
    }

    /// Retourne le code de langue du catalogue chargé.
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Retourne le nom lisible de la langue active.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Traduit les fragments français catalogués et préserve les valeurs dynamiques.
    pub fn translate(&self, source: &str) -> String {
        let mut translated = source.to_owned();
        for (original, target) in &self.translations {
            if translated.contains(original) {
                translated = translated.replace(original, target);
            }
        }
        translated
    }
}

/// Retourne le dossier utilisateur où l'on peut installer des catalogues supplémentaires.
fn locale_directory() -> Result<PathBuf, Box<dyn Error>> {
    Ok(storage::data_directory()?.join("locales"))
}

#[cfg(test)]
mod tests {
    use super::Localization;
    use std::path::Path;

    #[test]
    fn english_catalog_translates_labels_and_preserves_dynamic_values() {
        let english = Localization::load("en", Path::new("missing-locales")).unwrap();
        assert_eq!(english.name(), "English");
        assert_eq!(
            english.translate("Niveau 3  |  Vitesse 9 m"),
            "Level 3  |  Speed 9 m"
        );
        assert_eq!(
            english.translate("(I)Inventaire : ajouter objet"),
            "(I)Inventory: add item"
        );
        assert_eq!(english.translate("FOR+2,SAG+1"), "STR+2,WIS+1");
        assert_eq!(english.translate("Athlétisme"), "Athletics");
        assert_eq!(english.translate("1 résultat"), "1 result");
        assert_eq!(english.translate("2 résultats"), "2 results");
        assert_eq!(english.translate("Warrior of Mercy"), "Warrior of Mercy");
    }

    #[test]
    fn french_catalog_keeps_source_text() {
        let french = Localization::load("fr", Path::new("missing-locales")).unwrap();
        assert_eq!(french.language(), "fr");
        assert_eq!(french.name(), "Français");
        assert_eq!(french.translate("Inventaire"), "Inventaire");
    }

    #[test]
    fn user_catalogs_translate_known_phrases_and_fall_back_for_missing_entries() {
        let directory =
            std::env::temp_dir().join(format!("dnd-tracker-locales-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let catalog_path = directory.join("es.json");
        std::fs::write(
            &catalog_path,
            r#"{"language":"es","name":"Español","translations":{"Niveau ":"Nivel "}}"#,
        )
        .unwrap();

        let spanish = Localization::load("es", &directory).unwrap();

        assert_eq!(spanish.name(), "Español");
        assert_eq!(spanish.translate("Niveau 3"), "Nivel 3");
        assert_eq!(
            Localization::load("es-MX", &directory).unwrap().name(),
            "Español"
        );
        assert_eq!(spanish.translate("Inventaire"), "Inventaire");
        std::fs::remove_file(catalog_path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn unknown_locale_and_malformed_locale_code_are_reported() {
        assert!(Localization::load("xx", Path::new("missing-locales")).is_err());
        assert!(Localization::load("../en", Path::new("missing-locales")).is_err());
    }
}
