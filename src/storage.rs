//! Persistance locale des personnages dans un fichier JSON portable.

use crate::character::Character;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Choisit le dossier de données utilisateur selon les conventions de la plateforme.
pub fn character_file() -> Result<PathBuf, Box<dyn Error>> {
    Ok(data_directory()?.join("characters.json"))
}

/// Retourne le chemin de la base SQLite du compendium dans le dossier de données utilisateur.
pub fn compendium_file() -> Result<PathBuf, Box<dyn Error>> {
    Ok(data_directory()?.join("compendium.sqlite3"))
}

/// Choisit le dossier de données selon XDG, APPDATA ou le dossier personnel de la plateforme.
pub fn data_directory() -> Result<PathBuf, Box<dyn Error>> {
    let data_dir = if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(data_home)
    } else if let Some(app_data) = std::env::var_os("APPDATA") {
        PathBuf::from(app_data)
    } else {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .ok_or("HOME or USERPROFILE is not set")?;
        if cfg!(windows) {
            PathBuf::from(home).join("AppData/Roaming")
        } else {
            PathBuf::from(home).join(".local/share")
        }
    };
    Ok(data_dir.join("dnd-tracker"))
}

/// Une première exécution démarre avec une liste vide; les erreurs JSON restent visibles.
pub fn load(path: &Path) -> Result<Vec<Character>, Box<dyn Error>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

/// Sauvegarde les personnages en JSON en remplaçant atomiquement le fichier précédent.
pub fn save(path: &Path, characters: &[Character]) -> Result<(), Box<dyn Error>> {
    let parent = path
        .parent()
        .ok_or("character file has no parent directory")?;
    fs::create_dir_all(parent)?;
    // Écrire d'abord dans un fichier temporaire évite de corrompre la sauvegarde
    // si l'application est interrompue pendant la sérialisation.
    let temporary_path = path.with_extension("json.tmp");
    let contents = serde_json::to_string_pretty(characters)?;
    fs::write(&temporary_path, contents)?;
    replace_file(&temporary_path, path)?;
    Ok(())
}

/// Remplace la sauvegarde sans supprimer l'ancienne si l'opération échoue.
#[cfg(not(windows))]
fn replace_file(temporary_path: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(temporary_path, destination)
}

/// Utilise le remplacement natif de Windows pour écraser le fichier existant.
#[cfg(windows)]
fn replace_file(temporary_path: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source: Vec<u16> = temporary_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let target: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // Les deux pointeurs sont des chemins UTF-16 terminés par zéro et restent valides pendant l'appel.
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{load, save};
    use crate::character::Character;

    #[test]
    /// Vérifie que les fiches enregistrées sont relues sans perte de données.
    fn saves_and_loads_character_data() {
        let directory =
            std::env::temp_dir().join(format!("dnd-tracker-test-{}", std::process::id()));
        let path = directory.join("characters.json");
        let character = Character {
            name: "Test".into(),
            ..Character::default()
        };

        save(&path, &[character]).unwrap();
        save(
            &path,
            &[Character {
                name: "Updated".into(),
                ..Character::default()
            }],
        )
        .unwrap();
        let loaded = load(&path).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Updated");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
