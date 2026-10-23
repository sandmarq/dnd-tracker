//! Cache SQLite local du compendium actif et import atomique des fichiers XML.

use crate::compendium::Compendium;
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i64 = 1;

#[derive(Debug)]
pub struct Metadata {
    pub source_path: String,
    pub sha256: String,
}

pub fn fingerprint(path: &Path) -> Result<String, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn load(database_path: &Path) -> Result<Option<(Compendium, Metadata)>, Box<dyn Error>> {
    if !database_path.is_file() {
        return Ok(None);
    }
    let connection = open(database_path)?;
    let schema_version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if schema_version != SCHEMA_VERSION {
        return Err(format!(
            "Version du cache SQLite non prise en charge ({schema_version}); réimportez un compendium avec --import."
        )
        .into());
    }

    let metadata = read_metadata(&connection)?;
    let compendium = Compendium {
        items: load_category(&connection, "items")?,
        spells: load_category(&connection, "spells")?,
        classes: load_category(&connection, "classes")?,
        races: load_category(&connection, "races")?,
        feats: load_category(&connection, "feats")?,
    };
    Ok(Some((compendium, metadata)))
}

pub fn source_fingerprint(database_path: &Path) -> Result<Option<String>, Box<dyn Error>> {
    if !database_path.is_file() {
        return Ok(None);
    }
    let connection = open(database_path)?;
    let schema_version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if schema_version != SCHEMA_VERSION {
        return Err(format!(
            "Version du cache SQLite non prise en charge ({schema_version}); réimportez un compendium avec --import."
        )
        .into());
    }
    let hash = connection
        .query_row(
            "SELECT value FROM compendium_metadata WHERE key = 'sha256'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(hash)
}

pub fn replace(
    database_path: &Path,
    source_path: &Path,
    source_sha256: &str,
    compendium: &Compendium,
) -> Result<(), Box<dyn Error>> {
    let parent = database_path
        .parent()
        .ok_or("SQLite database path has no parent directory")?;
    std::fs::create_dir_all(parent)?;
    let source_path = absolute_path(source_path)?;
    let mut connection = open(database_path)?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS compendium_metadata (
             key TEXT PRIMARY KEY NOT NULL,
             value TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS compendium_entries (
             category TEXT NOT NULL,
             position INTEGER NOT NULL,
             name TEXT NOT NULL,
             payload TEXT NOT NULL,
             PRIMARY KEY (category, position)
         );
         CREATE INDEX IF NOT EXISTS compendium_entries_name
             ON compendium_entries (category, name COLLATE NOCASE);
        ",
    )?;

    transaction.execute("DELETE FROM compendium_entries", [])?;
    transaction.execute("DELETE FROM compendium_metadata", [])?;
    transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;

    insert_category(&transaction, "items", &compendium.items, |entry| {
        &entry.name
    })?;
    insert_category(&transaction, "spells", &compendium.spells, |entry| {
        &entry.name
    })?;
    insert_category(&transaction, "classes", &compendium.classes, |entry| {
        &entry.name
    })?;
    insert_category(&transaction, "races", &compendium.races, |entry| {
        &entry.name
    })?;
    insert_category(&transaction, "feats", &compendium.feats, |entry| {
        &entry.name
    })?;

    let imported_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .to_string();
    for (key, value) in [
        ("source_path", source_path.to_string_lossy().into_owned()),
        ("sha256", source_sha256.to_owned()),
        ("imported_at", imported_at),
    ] {
        transaction.execute(
            "INSERT INTO compendium_metadata (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

fn open(path: &Path) -> Result<Connection, Box<dyn Error>> {
    let connection = Connection::open(path)?;
    connection.busy_timeout(Duration::from_secs(5))?;
    Ok(connection)
}

fn absolute_path(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn read_metadata(connection: &Connection) -> Result<Metadata, Box<dyn Error>> {
    let source_path = connection.query_row(
        "SELECT value FROM compendium_metadata WHERE key = 'source_path'",
        [],
        |row| row.get(0),
    )?;
    let sha256 = connection.query_row(
        "SELECT value FROM compendium_metadata WHERE key = 'sha256'",
        [],
        |row| row.get(0),
    )?;
    Ok(Metadata {
        source_path,
        sha256,
    })
}

fn load_category<T: DeserializeOwned>(
    connection: &Connection,
    category: &str,
) -> Result<Vec<T>, Box<dyn Error>> {
    let mut statement = connection.prepare(
        "SELECT payload FROM compendium_entries
         WHERE category = ?1 ORDER BY position",
    )?;
    let records = statement.query_map([category], |row| row.get::<_, String>(0))?;
    records
        .map(|record| {
            let json = record?;
            Ok(serde_json::from_str(&json)?)
        })
        .collect()
}

fn insert_category<T: Serialize>(
    transaction: &Transaction<'_>,
    category: &str,
    entries: &[T],
    name: impl Fn(&T) -> &str,
) -> Result<(), Box<dyn Error>> {
    let mut statement = transaction.prepare(
        "INSERT INTO compendium_entries (category, position, name, payload)
         VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (position, entry) in entries.iter().enumerate() {
        let payload = serde_json::to_string(entry)?;
        statement.execute(params![category, position as i64, name(entry), payload])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{fingerprint, load, replace, source_fingerprint};
    use crate::compendium::{Class, Compendium, Item};

    #[test]
    fn replaces_and_reloads_compendium_with_source_metadata() {
        let directory =
            std::env::temp_dir().join(format!("dnd-compendium-store-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let source = directory.join("source.xml");
        let database = directory.join("compendium.sqlite3");
        std::fs::write(&source, "<compendium/>").unwrap();
        let source_sha256 = fingerprint(&source).unwrap();
        let compendium = Compendium {
            items: vec![Item {
                name: "Longsword".into(),
                property: vec!["Versatile".into()],
                ..Item::default()
            }],
            classes: vec![Class {
                name: "Fighter".into(),
                hd: Some("10".into()),
                ..Class::default()
            }],
            ..Compendium::default()
        };

        replace(&database, &source, &source_sha256, &compendium).unwrap();
        let (loaded, metadata) = load(&database).unwrap().unwrap();

        assert_eq!(loaded.items[0].name, "Longsword");
        assert_eq!(loaded.items[0].property, ["Versatile"]);
        assert_eq!(loaded.classes[0].hd.as_deref(), Some("10"));
        assert_eq!(metadata.sha256, source_sha256);
        assert_eq!(
            source_fingerprint(&database).unwrap().as_deref(),
            Some(source_sha256.as_str())
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn failed_replacement_keeps_the_previous_compendium() {
        let directory =
            std::env::temp_dir().join(format!("dnd-compendium-rollback-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let source = directory.join("source.xml");
        let database = directory.join("compendium.sqlite3");
        std::fs::write(&source, "<compendium/>").unwrap();
        let original = Compendium {
            items: vec![Item {
                name: "Original".into(),
                ..Item::default()
            }],
            ..Compendium::default()
        };
        replace(&database, &source, "original-hash", &original).unwrap();

        let connection = rusqlite::Connection::open(&database).unwrap();
        connection
            .execute_batch(
                "CREATE TRIGGER reject_replacement
             BEFORE INSERT ON compendium_entries
             BEGIN SELECT RAISE(ABORT, 'simulated import failure'); END;",
            )
            .unwrap();
        drop(connection);

        let replacement = Compendium {
            items: vec![Item {
                name: "Replacement".into(),
                ..Item::default()
            }],
            ..Compendium::default()
        };
        assert!(replace(&database, &source, "replacement-hash", &replacement).is_err());

        let (loaded, metadata) = load(&database).unwrap().unwrap();
        assert_eq!(loaded.items[0].name, "Original");
        assert_eq!(metadata.sha256, "original-hash");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
