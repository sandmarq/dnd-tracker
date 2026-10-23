//! Point d'entrée de l'application TUI et gestion du cycle de vie du terminal.

mod app;
mod character;
mod compendium;
mod storage;

use app::App;
use compendium::Compendium;
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

fn main() -> Result<(), Box<dyn Error>> {
    let Some(xml_path) = find_compendium_path()? else {
        return Ok(());
    };
    let compendium = Compendium::load_from_file(&xml_path).map_err(|error| {
        format!(
            "Impossible de charger le compendium XML ({}) : {error}",
            xml_path.display()
        )
    })?;
    let storage_path = storage::character_file()?;
    let mut app = App::new(compendium, storage_path.clone()).map_err(|error| {
        format!(
            "Impossible de lire les personnages sauvegardés ({}): {error}",
            storage_path.display()
        )
    })?;

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

fn find_compendium_path() -> Result<Option<PathBuf>, Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let first_argument = arguments.next();
    if first_argument.as_deref() == Some(std::ffi::OsStr::new("--help"))
        || first_argument.as_deref() == Some(std::ffi::OsStr::new("-h"))
    {
        println!(
            "Usage: dnd-tracker [CHEMIN_DU_COMPENDIUM.xml]\n\
             Ou définir DND_COMPENDIUM pour choisir le compendium XML."
        );
        return Ok(None);
    }
    if arguments.next().is_some() {
        return Err("Usage: dnd-tracker [CHEMIN_DU_COMPENDIUM.xml]".into());
    }
    if let Some(path) = first_argument {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(Some(path));
        }
        return Err(format!("Compendium XML introuvable : {}", path.display()).into());
    }
    if let Some(path) = std::env::var_os("DND_COMPENDIUM") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(Some(path));
        }
        return Err(format!(
            "Le fichier défini par DND_COMPENDIUM est introuvable : {}",
            path.display()
        )
        .into());
    }

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
    candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .cloned()
        .map(Some)
        .ok_or_else(|| {
            format!(
                "Compendium introuvable. Compilez la collection WotC_5.5e_only.xml de FightClub5eXML et placez le résultat dans Compendiums/{file_name}, ou passez son chemin en argument / via DND_COMPENDIUM. Un fichier WotC_5.5e+Legacy_5e.xml dans le dossier du projet est aussi détecté automatiquement."
            ).into()
        })
}

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

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
