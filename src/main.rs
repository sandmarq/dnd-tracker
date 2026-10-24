//! Point d'entrée de l'application TUI et gestion du cycle de vie du terminal.

mod app;
mod character;
mod compendium;
mod compendium_store;
mod localization;
mod storage;

use app::App;
use compendium::Compendium;
use compendium_store::Metadata;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::error::Error;
use std::io::{self, stdout};
use std::path::PathBuf;
use std::time::Duration;

/// Initialise le compendium, prépare le terminal et exécute l'application TUI.
fn main() -> Result<(), Box<dyn Error>> {
    let Some(xml_path) = resolve_xml_argument()? else {
        return Ok(());
    };
    let database_path = storage::compendium_file()?;
    let (compendium, startup_message) = match xml_path {
        Some(xml_path) => import_or_load(&xml_path, &database_path)?,
        None => load_active_compendium(&database_path)?,
    };
    let storage_path = storage::character_file()?;
    let mut app = App::new(compendium, storage_path.clone()).map_err(|error| {
        format!(
            "Impossible de lire les personnages sauvegardés ({}): {error}",
            storage_path.display()
        )
    })?;
    app.set_status(startup_message);

    enable_raw_mode()?;
    let mut output = stdout();
    execute!(output, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(output);
    let mut terminal = Terminal::new(backend)?;

    let app_result = run_app(&mut terminal, &mut app);
    let restore_result = restore_terminal(&mut terminal);
    app_result?;
    restore_result?;
    Ok(())
}

/// Résout l'option d'import, le chemin XML éventuel et le mode aide.
fn resolve_xml_argument() -> Result<Option<Option<PathBuf>>, Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let first_argument = arguments.next();
    if first_argument.as_deref() == Some(std::ffi::OsStr::new("--help"))
        || first_argument.as_deref() == Some(std::ffi::OsStr::new("-h"))
    {
        println!(
            "Usage: dnd-tracker [--import] [CHEMIN_DU_COMPENDIUM.xml]\n\
             Sans argument, le compendium actif est chargé depuis SQLite.\n\
             --import ou un chemin importe/remplace le compendium actif.\n\
             DND_COMPENDIUM peut aussi sélectionner un fichier XML.\n\
             DND_LANGUAGE choisit la langue de l'interface (fr par défaut, en intégré)."
        );
        return Ok(None);
    }
    let import_flag = first_argument.as_deref() == Some(std::ffi::OsStr::new("--import"));
    let supplied_path = if import_flag {
        arguments.next()
    } else {
        first_argument
    };
    if arguments.next().is_some() {
        return Err("Usage: dnd-tracker [--import] [CHEMIN_DU_COMPENDIUM.xml]".into());
    }
    if import_flag && supplied_path.is_none() {
        return Err("--import doit être suivi du chemin du compendium XML.".into());
    }
    if let Some(path) = supplied_path {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(Some(Some(path)));
        }
        return Err(format!("Compendium XML introuvable : {}", path.display()).into());
    }
    if let Some(path) = std::env::var_os("DND_COMPENDIUM") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(Some(Some(path)));
        }
        return Err(format!(
            "Le fichier défini par DND_COMPENDIUM est introuvable : {}",
            path.display()
        )
        .into());
    }
    Ok(Some(None))
}

/// Recherche un compendium XML aux emplacements conventionnels du projet et du binaire.
fn default_xml_path() -> Result<Option<PathBuf>, Box<dyn Error>> {
    let file_name = "WotC_5.5e_only.xml";
    let current_dir = std::env::current_dir()?;
    // Priorité au fichier dans le dossier de lancement, puis aux répertoires
    // proches du binaire pour que l'application reste portable.
    let mut candidates = vec![
        current_dir.join("Compendiums").join(file_name),
        current_dir.join("data").join("compendium.xml"),
        // Reconnaît les noms de compendium déjà utilisés par cette installation.
        current_dir.join("WotC_5.5e+Legacy_5e.xml"),
    ];
    if let Ok(executable) = std::env::current_exe() {
        let mut directory = executable.parent();
        while let Some(current) = directory {
            candidates.push(current.join("Compendiums").join(file_name));
            candidates.push(current.join("data").join("compendium.xml"));
            candidates.push(current.join("WotC_5.5e+Legacy_5e.xml"));
            directory = current.parent();
        }
    }
    candidates.dedup();
    Ok(candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .cloned())
}

/// Charge le cache actif, l'actualise si sa source a changé ou effectue le premier import.
fn load_active_compendium(
    database_path: &std::path::Path,
) -> Result<(Compendium, String), Box<dyn Error>> {
    if let Some((compendium, metadata)) = compendium_store::load(database_path)? {
        if PathBuf::from(&metadata.source_path).is_file() {
            let current_hash =
                compendium_store::fingerprint(PathBuf::from(&metadata.source_path).as_path())?;
            if current_hash != metadata.sha256 {
                return import_or_load(
                    PathBuf::from(metadata.source_path).as_path(),
                    database_path,
                );
            }
        }
        return Ok((compendium, cache_message(&metadata)));
    }

    let xml_path = default_xml_path()?.ok_or_else(|| {
        "Aucun compendium en cache. Importez un XML avec \
         `cargo run -- --import /chemin/vers/compendium.xml`."
            .to_string()
    })?;
    import_or_load(&xml_path, database_path)
}

/// Importe le XML uniquement si son empreinte diffère de celle déjà mise en cache.
fn import_or_load(
    xml_path: &std::path::Path,
    database_path: &std::path::Path,
) -> Result<(Compendium, String), Box<dyn Error>> {
    let source_hash = compendium_store::fingerprint(xml_path).map_err(|error| {
        format!(
            "Impossible de lire le compendium XML ({}): {error}",
            xml_path.display()
        )
    })?;
    if compendium_store::source_fingerprint(database_path)?.as_deref() == Some(&source_hash) {
        if let Some((compendium, metadata)) = compendium_store::load(database_path)? {
            return Ok((compendium, cache_message(&metadata)));
        }
    }

    let compendium = Compendium::load_from_file(xml_path).map_err(|error| {
        format!(
            "Impossible de charger le compendium XML ({}): {error}",
            xml_path.display()
        )
    })?;
    if compendium_store::fingerprint(xml_path)? != source_hash {
        return Err(format!(
            "Le fichier XML a changé pendant l'import; le cache précédent est conservé. Réessayez avec {}.",
            xml_path.display()
        )
        .into());
    }
    compendium_store::replace(database_path, xml_path, &source_hash, &compendium)
        .map_err(|error| format!("Échec de remplacement du cache SQLite : {error}"))?;
    let message = format!(
        "Compendium importé dans SQLite : {} objets, {} sorts, {} classes, {} espèces, {} backgrounds et {} dons.",
        compendium.items.len(),
        compendium.spells.len(),
        compendium.classes.len(),
        compendium.races.len(),
        compendium.backgrounds.len(),
        compendium.feats.len()
    );
    Ok((compendium, message))
}

/// Produit le message utilisateur décrivant l'origine du compendium chargé.
fn cache_message(metadata: &Metadata) -> String {
    format!(
        "Compendium chargé depuis le cache SQLite (source : {}). Utilisez --import pour le remplacer.",
        metadata.source_path
    )
}

/// Dessine l'interface et distribue les événements clavier jusqu'à la fermeture.
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    while app.is_running() {
        terminal.draw(|frame| app.render(frame))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }
    }
    Ok(())
}

/// Quitte le mode terminal alternatif et restaure l'entrée terminal standard.
fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
