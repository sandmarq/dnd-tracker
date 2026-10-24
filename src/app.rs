//! État, règles de fiche et vues interactives du gestionnaire de personnages.

use crate::character::{Character, InventorySlot, Resource};
use crate::compendium::{Background, Class, Compendium, Item, Spell};
use crate::storage;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rand::Rng;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use std::path::PathBuf;

/// Ordre stable des caractéristiques dans le modèle, les saisies et l'affichage.
const ABILITY_NAMES: [&str; 6] = ["FOR", "DEX", "CON", "INT", "SAG", "CHA"];
/// Tableau standard utilisé comme valeurs initiales à la création.
const STANDARD_ARRAY: [i32; 6] = [15, 14, 13, 12, 10, 8];
/// Option explicite permettant de créer un personnage sans background.
const NO_BACKGROUND_OPTION: &str = "Aucun (sans background)";
/// Seuils cumulés d'expérience de la progression 5e 2024, indexés par niveau.
const XP_LEVEL_THRESHOLDS: [u32; 20] = [
    0, 300, 900, 2_700, 6_500, 14_000, 23_000, 34_000, 48_000, 64_000, 85_000, 100_000, 120_000,
    140_000, 165_000, 195_000, 225_000, 265_000, 305_000, 355_000,
];
/// Liste des compétences et index de leur caractéristique associée.
const SKILLS: [(&str, usize); 18] = [
    ("Acrobatics", 1),
    ("Animal Handling", 4),
    ("Arcana", 3),
    ("Athletics", 0),
    ("Deception", 5),
    ("History", 3),
    ("Insight", 4),
    ("Intimidation", 5),
    ("Investigation", 3),
    ("Medicine", 4),
    ("Nature", 3),
    ("Perception", 4),
    ("Performance", 5),
    ("Persuasion", 5),
    ("Religion", 3),
    ("Sleight of Hand", 1),
    ("Stealth", 1),
    ("Survival", 4),
];

// L'ordre des onglets est également celui parcouru par la touche Tab.
/// Onglet affiché dans le panneau de détail du personnage.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Sheet,
    Equipment,
    Spells,
    Progression,
}

impl Tab {
    /// Retourne le prochain onglet dans l'ordre de navigation.
    fn next(self) -> Self {
        match self {
            Self::Sheet => Self::Equipment,
            Self::Equipment => Self::Spells,
            Self::Spells => Self::Progression,
            Self::Progression => Self::Sheet,
        }
    }
}

/// Catégorie de choix affichée dans une fenêtre de sélection.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PickKind {
    Race,
    Class,
    Background,
    Item,
    StartingItem,
    Spell,
    CastSpell,
    Feat,
}

/// Type de saisie texte en cours et validation associée.
#[derive(Clone, Copy)]
enum InputKind {
    CharacterName,
    Score(usize),
    BackgroundAbilityChoices,
    EquipmentChoices,
    EditScore(usize),
    HitPointAdjustment,
    TemporaryHitPoints,
    Experience,
    Level,
    Condition,
    CombatAction,
    Subclass,
    SkillChoices,
}

/// Mode d'interaction exclusif actif dans l'interface.
#[derive(Clone, Copy)]
enum Mode {
    Normal,
    Input(InputKind),
    Picker(PickKind),
    ConfirmDelete,
}

/// État provisoire du personnage tant que l'assistant de création n'est pas terminé.
struct CreateWizard {
    /// Nom saisi pour la nouvelle fiche.
    name: String,
    /// Espèce sélectionnée.
    race: Option<String>,
    /// Classe sélectionnée.
    class: Option<String>,
    /// Background sélectionné, ou aucun.
    background: Option<String>,
    /// Scores avant application des bonus du background.
    scores: [i32; 6],
    /// Compétences choisies au titre de la classe.
    skills: Vec<String>,
    /// Objets ajoutés depuis les paquets et le sélecteur manuel.
    equipment: Vec<String>,
    /// Sorts mineurs et sorts de niveau 1 sélectionnés.
    spells: Vec<String>,
    /// Texte des paquets A/B retenus pour la fiche.
    equipment_choices: Vec<String>,
    /// Objets du paquet non résolus automatiquement.
    equipment_warnings: Vec<String>,
}

/// État applicatif et règles d'interaction de l'interface terminal.
pub struct App {
    /// Données de référence disponibles pour les choix et calculs.
    pub compendium: Compendium,
    /// Fiches chargées pour la session.
    characters: Vec<Character>,
    /// Index de la fiche actuellement affichée.
    selected: usize,
    /// Index de ligne sélectionné dans le panneau détaillé.
    detail_index: usize,
    /// Décalage vertical du panneau de détails.
    detail_scroll: u16,
    /// Niveau d'emplacement utilisé par les commandes de sort.
    spell_slot_level: usize,
    /// Index de ressource de classe actif.
    resource_index: usize,
    /// Onglet de fiche affiché.
    tab: Tab,
    /// État d'interaction courant.
    mode: Mode,
    /// Indique si la boucle d'interface est active.
    running: bool,
    /// Brouillon de création, présent uniquement pendant l'assistant.
    wizard: Option<CreateWizard>,
    /// Texte de la fenêtre de saisie courante.
    input: String,
    /// Texte de filtre du sélecteur.
    picker_query: String,
    /// Index de l'option surlignée.
    picker_index: usize,
    /// Message d'état affiché en pied d'écran.
    status: String,
    /// Emplacement du fichier JSON de personnages.
    storage_path: PathBuf,
}

impl App {
    /// Charge les personnages sauvegardés et initialise l'interface avec le compendium fourni.
    pub fn new(
        compendium: Compendium,
        storage_path: PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let characters = storage::load(&storage_path)?;
        Ok(Self {
            compendium,
            characters,
            selected: 0,
            detail_index: 0,
            detail_scroll: 0,
            spell_slot_level: 1,
            resource_index: 0,
            tab: Tab::Sheet,
            mode: Mode::Normal,
            running: true,
            wizard: None,
            input: String::new(),
            picker_query: String::new(),
            picker_index: 0,
            status: String::from("Prêt. Appuyez sur n pour créer un personnage."),
            storage_path,
        })
    }

    /// Indique si la boucle de l'interface doit continuer à tourner.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Remplace le texte de statut affiché en bas de l'écran.
    pub fn set_status(&mut self, message: String) {
        self.status = message;
    }

    /// Demande à la boucle principale de fermer l'interface.
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Achemine une touche selon le mode courant de l'interface.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            self.mode = Mode::Normal;
            self.wizard = None;
            self.picker_query.clear();
            return;
        }

        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Input(kind) => self.handle_input_key(key, kind),
            Mode::Picker(kind) => self.handle_picker_key(key, kind),
            Mode::ConfirmDelete => self.handle_delete_key(key),
        }
    }

    /// Traite les commandes disponibles hors dialogue ou sélecteur.
    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.quit(),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Tab => {
                self.tab = self.tab.next();
                self.detail_index = 0;
                self.detail_scroll = 0;
            }
            KeyCode::PageUp => self.detail_scroll = self.detail_scroll.saturating_sub(10),
            KeyCode::PageDown => self.detail_scroll = self.detail_scroll.saturating_add(10),
            KeyCode::Char('n') => self.start_creation(),
            KeyCode::Char('d') if self.selected_character().is_some() => {
                self.mode = Mode::ConfirmDelete;
            }
            KeyCode::Char('i') if self.selected_character().is_some() => {
                self.open_picker(PickKind::Item);
            }
            KeyCode::Char('p') if self.selected_character().is_some() => {
                self.open_picker(PickKind::Spell);
            }
            KeyCode::Char('f') if self.selected_character().is_some() => {
                self.open_picker(PickKind::Feat);
            }
            KeyCode::Char('e') if self.selected_character().is_some() => {
                self.start_input(InputKind::Experience, "XP totale");
            }
            KeyCode::Char('l') if self.selected_character().is_some() => {
                self.start_input(InputKind::Level, "Niveau (1–20)");
            }
            KeyCode::Char('C') if self.selected_character().is_some() => {
                self.start_input(InputKind::Condition, "Ajouter une condition");
            }
            KeyCode::Char('a') if self.selected_character().is_some() => {
                self.start_input(InputKind::CombatAction, "Action, attaque ou note de combat");
            }
            KeyCode::Char('c') if self.selected_character().is_some() => {
                self.open_picker(PickKind::CastSpell);
            }
            KeyCode::Char('x') if self.selected_character().is_some() && self.tab == Tab::Sheet => {
                self.remove_last_condition();
            }
            KeyCode::Char('x') if self.selected_character().is_some() => self.remove_selected(),
            KeyCode::Char(' ') if self.tab == Tab::Equipment => self.toggle_equipped(),
            KeyCode::Char('h') if self.selected_character().is_some() => {
                self.start_input(InputKind::HitPointAdjustment, "Dégâts/soins (ex. -5 ou 8)");
            }
            KeyCode::Char('t') if self.selected_character().is_some() => {
                self.start_input(InputKind::TemporaryHitPoints, "PV temporaires");
            }
            KeyCode::Char('u') if self.selected_character().is_some() => self.spend_resource(),
            KeyCode::Char('R') if self.selected_character().is_some() => self.long_rest(),
            KeyCode::Char('v') if self.selected_character().is_some() => self.short_rest(),
            KeyCode::Char('g') if self.selected_character().is_some() => self.spend_hit_die(),
            KeyCode::Char('+') | KeyCode::Char('=') if self.selected_character().is_some() => {
                self.level_up();
            }
            KeyCode::Char('b') if self.selected_character().is_some() => {
                self.start_input(InputKind::Subclass, "Sous-classe");
            }
            KeyCode::Char('[') if self.tab == Tab::Spells => self.adjust_spell_slot(-1),
            KeyCode::Char(']') if self.tab == Tab::Spells => self.adjust_spell_slot(1),
            KeyCode::Char('[') if self.tab == Tab::Sheet => self.adjust_resource_selection(-1),
            KeyCode::Char(']') if self.tab == Tab::Sheet => self.adjust_resource_selection(1),
            KeyCode::Char(ch @ '1'..='9') if self.tab == Tab::Spells => {
                self.spell_slot_level = ch.to_digit(10).unwrap() as usize;
            }
            KeyCode::Char(ch @ '1'..='6')
                if self.selected_character().is_some() && self.tab == Tab::Sheet =>
            {
                self.start_input(
                    InputKind::EditScore((ch as usize) - ('1' as usize)),
                    "Nouvelle valeur",
                );
            }
            KeyCode::Char('r') if self.selected_character().is_some() => self.long_rest(),
            _ => {}
        }
    }

    /// Crée un état provisoire et démarre l'assistant de création.
    fn start_creation(&mut self) {
        self.wizard = Some(CreateWizard {
            name: String::new(),
            race: None,
            class: None,
            background: None,
            scores: STANDARD_ARRAY,
            skills: Vec::new(),
            equipment: Vec::new(),
            spells: Vec::new(),
            equipment_choices: Vec::new(),
            equipment_warnings: Vec::new(),
        });
        self.start_input(InputKind::CharacterName, "Nom du personnage");
    }

    /// Enchaîne vers les bonus du background ou vers la sélection d'équipement.
    fn after_skills_selected(&mut self) {
        let background_name = self
            .wizard
            .as_ref()
            .and_then(|wizard| wizard.background.as_deref())
            .unwrap_or("");
        if self
            .find_background(background_name)
            .is_some_and(|background| background_ability_indices(background).len() == 3)
        {
            self.start_input(
                InputKind::BackgroundAbilityChoices,
                "Bonus de caractéristiques du background",
            );
            return;
        }
        self.begin_equipment_selection();
    }

    /// Affiche les choix de paquets A/B s'ils sont décrits dans le compendium.
    fn begin_equipment_selection(&mut self) {
        let packages = self.equipment_package_options();
        if !packages.is_empty() {
            self.start_input(
                InputKind::EquipmentChoices,
                "Paquets d'équipement de départ (A ou B)",
            );
        } else {
            self.open_picker(PickKind::StartingItem);
        }
    }

    /// Ouvre le sélecteur des sorts de niveau 0 et 1 pour la création.
    fn open_creation_spells(&mut self) {
        if self.picker_options(PickKind::Spell).is_empty() {
            self.finish_creation();
        } else {
            self.open_picker(PickKind::Spell);
        }
    }

    /// Inverse la sélection de l'option actuellement surlignée.
    fn toggle_current_creation_choice(&mut self, kind: PickKind) {
        let Some(value) = self.picker_options(kind).get(self.picker_index).cloned() else {
            return;
        };
        self.toggle_creation_choice(kind, &value);
    }

    /// Ajoute ou retire une option d'équipement ou de sort du brouillon.
    fn toggle_creation_choice(&mut self, kind: PickKind, value: &str) {
        let Some(wizard) = self.wizard.as_mut() else {
            return;
        };
        let selected = match kind {
            PickKind::StartingItem => &mut wizard.equipment,
            PickKind::Spell => &mut wizard.spells,
            _ => return,
        };
        if selected.iter().any(|entry| entry == value) {
            selected.retain(|entry| entry != value);
        } else {
            selected.push(value.to_owned());
        }
    }

    /// Extrait les choix de paquets d'équipement depuis les traits du background et de la classe.
    fn equipment_package_options(&self) -> Vec<(String, (String, String))> {
        let Some(wizard) = self.wizard.as_ref() else {
            return Vec::new();
        };
        let mut packages = Vec::new();
        if let Some(background) = wizard
            .background
            .as_deref()
            .and_then(|name| self.find_background(name))
        {
            if let Some(text) = background
                .traits
                .iter()
                .find(|feature| feature.name.eq_ignore_ascii_case("Starting Equipment"))
                .and_then(|feature| feature.text.first())
            {
                if let Some(options) = extract_equipment_options(text) {
                    packages.push(("Background".to_owned(), options));
                }
            }
        }
        if let Some(class) = wizard
            .class
            .as_deref()
            .and_then(|name| self.find_class(name))
        {
            let text = class
                .autolevels
                .iter()
                .filter(|autolevel| autolevel.level == "1")
                .flat_map(|autolevel| &autolevel.features)
                .flat_map(|feature| &feature.text)
                .find(|text| {
                    text.to_ascii_lowercase()
                        .contains("start with the following equipment")
                });
            if let Some(options) = text.and_then(|text| extract_equipment_options(text)) {
                packages.push(("Classe".to_owned(), options));
            }
        }
        packages
    }

    /// Associe les noms d'un paquet aux objets du compendium et signale les entrées inconnues.
    fn items_in_package(&self, package: &str) -> (Vec<String>, Vec<String>) {
        let mut items = Vec::new();
        let mut warnings = Vec::new();
        for part in package
            .split([',', ';'])
            .flat_map(|part| part.split(" and "))
            .map(str::trim)
        {
            let (quantity, name) = parse_equipment_quantity(part);
            let name = name.trim_end_matches(['.', ' ']);
            let name = name
                .strip_prefix("a ")
                .or_else(|| name.strip_prefix("an "))
                .unwrap_or(name);
            if name.is_empty()
                || ["GP", "SP", "CP", "PP"]
                    .iter()
                    .any(|currency| name.eq_ignore_ascii_case(currency))
            {
                continue;
            }
            let normalized = name.to_lowercase();
            let singular = normalized.strip_suffix('s').unwrap_or(&normalized);
            let item = self
                .compendium
                .items
                .iter()
                .find(|item| {
                    item.name.eq_ignore_ascii_case(name) || item.name.eq_ignore_ascii_case(singular)
                })
                .or_else(|| {
                    self.compendium
                        .items
                        .iter()
                        .find(|item| item.name.to_lowercase().starts_with(singular))
                });
            if let Some(item) = item {
                items.extend(std::iter::repeat_n(item.name.clone(), quantity));
            } else {
                warnings.push(name.to_owned());
            }
        }
        (items, warnings)
    }

    /// Déplace la sélection selon l'onglet ou la liste des personnages affichés.
    fn move_selection(&mut self, direction: i8) {
        if self.tab == Tab::Equipment || self.tab == Tab::Spells || self.tab == Tab::Progression {
            let length = self
                .selected_character()
                .map(|character| match self.tab {
                    Tab::Equipment => character.equipment.len(),
                    Tab::Spells => character.prepared_spells.len(),
                    Tab::Progression => character.feats.len(),
                    _ => 0,
                })
                .unwrap_or(0);
            if direction < 0 {
                self.detail_index = self.detail_index.saturating_sub(1);
            } else if self.detail_index + 1 < length {
                self.detail_index += 1;
            }
            return;
        }
        if direction < 0 {
            self.selected = self.selected.saturating_sub(1);
        } else if self.selected + 1 < self.characters.len() {
            self.selected += 1;
        }
        self.detail_index = 0;
        self.detail_scroll = 0;
        self.resource_index = 0;
    }

    /// Prépare une fenêtre de saisie et en définit le libellé de statut.
    fn start_input(&mut self, kind: InputKind, prompt: &str) {
        self.input.clear();
        self.status = prompt.to_owned();
        self.mode = Mode::Input(kind);
    }

    /// Réinitialise la recherche puis ouvre le sélecteur demandé.
    fn open_picker(&mut self, kind: PickKind) {
        self.picker_query.clear();
        self.picker_index = 0;
        self.mode = Mode::Picker(kind);
    }

    /// Modifie le texte saisi ou valide la valeur au clavier.
    fn handle_input_key(&mut self, key: KeyEvent, kind: InputKind) {
        match key.code {
            KeyCode::Enter => self.commit_input(kind),
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.push(ch);
            }
            _ => {}
        }
    }

    /// Valide la saisie active et applique ses règles métier.
    fn commit_input(&mut self, kind: InputKind) {
        match kind {
            InputKind::CharacterName => {
                if self.input.trim().is_empty() {
                    self.status = "Le nom ne peut pas être vide.".into();
                    return;
                }
                if let Some(wizard) = self.wizard.as_mut() {
                    wizard.name = self.input.trim().to_owned();
                }
                self.open_picker(PickKind::Race);
            }
            InputKind::Score(index) => {
                let Ok(score) = self.input.trim().parse::<i32>() else {
                    self.status = "Entrez une valeur numérique entre 1 et 30.".into();
                    return;
                };
                if !(1..=30).contains(&score) {
                    self.status = "La valeur doit être comprise entre 1 et 30.".into();
                    return;
                }
                let Some(wizard) = self.wizard.as_mut() else {
                    self.mode = Mode::Normal;
                    return;
                };
                wizard.scores[index] = score;
                if index + 1 < 6 {
                    self.start_input(
                        InputKind::Score(index + 1),
                        &format!("Caractéristique {}", ABILITY_NAMES[index + 1]),
                    );
                    self.input = STANDARD_ARRAY[index + 1].to_string();
                } else {
                    let class_name = self
                        .wizard
                        .as_ref()
                        .and_then(|wizard| wizard.class.as_deref())
                        .unwrap_or("");
                    let skill_count = self
                        .find_class(class_name)
                        .and_then(|class| class.num_skills.as_deref())
                        .and_then(|count| count.parse::<usize>().ok())
                        .unwrap_or(0);
                    if skill_count > 0 {
                        self.start_input(
                            InputKind::SkillChoices,
                            "Maîtrises de compétences (séparées par des virgules)",
                        );
                    } else {
                        self.after_skills_selected();
                    }
                }
            }
            InputKind::BackgroundAbilityChoices => {
                let background_name = self
                    .wizard
                    .as_ref()
                    .and_then(|wizard| wizard.background.as_deref())
                    .unwrap_or("");
                let allowed = self
                    .find_background(background_name)
                    .map(background_ability_indices)
                    .unwrap_or_default();
                let choices = parse_background_ability_choices(&self.input, &allowed);
                let Some(choices) = choices else {
                    self.status = "Choisissez les bonus parmi les caractéristiques du background : deux (+2 et +1), ou les trois (+1).".into();
                    return;
                };
                let Some(wizard) = self.wizard.as_mut() else {
                    self.mode = Mode::Normal;
                    return;
                };
                if choices
                    .iter()
                    .any(|(index, increase)| wizard.scores[*index] + increase > 20)
                {
                    self.status =
                        "Un bonus ne peut pas faire dépasser 20 à une caractéristique.".into();
                    return;
                }
                for (index, increase) in choices {
                    wizard.scores[index] += increase;
                }
                self.begin_equipment_selection();
            }
            InputKind::EquipmentChoices => {
                let packages = self.equipment_package_options();
                let selected: Vec<String> = self
                    .input
                    .split(',')
                    .map(|choice| choice.trim().to_ascii_uppercase())
                    .collect();
                if selected.len() != packages.len()
                    || selected.iter().any(|choice| choice != "A" && choice != "B")
                {
                    self.status = format!(
                        "Choisissez A ou B pour chaque paquet ({} réponse(s), séparées par des virgules).",
                        packages.len()
                    );
                    return;
                }
                let selected_packages: Vec<(String, Vec<String>, Vec<String>)> = packages
                    .iter()
                    .zip(selected.iter())
                    .map(|((label, options), choice)| {
                        let package = if choice == "A" {
                            &options.0
                        } else {
                            &options.1
                        };
                        let (items, warnings) = self.items_in_package(package);
                        (format!("{label} — {choice} : {package}"), items, warnings)
                    })
                    .collect();
                if let Some(wizard) = self.wizard.as_mut() {
                    for (description, items, warnings) in selected_packages {
                        wizard.equipment_choices.push(description);
                        wizard.equipment.extend(items);
                        wizard.equipment_warnings.extend(warnings);
                    }
                }
                self.open_picker(PickKind::StartingItem);
            }
            InputKind::EditScore(index) => {
                let Ok(score) = self.input.trim().parse::<i32>() else {
                    self.status = "Entrez une valeur numérique entre 1 et 30.".into();
                    return;
                };
                if !(1..=30).contains(&score) {
                    self.status = "La valeur doit être comprise entre 1 et 30.".into();
                    return;
                }
                let hit_die = self
                    .selected_character()
                    .map(|character| self.class_hit_die(&character.class))
                    .unwrap_or(8);
                if let Some(character) = self.selected_character_mut() {
                    character.set_ability_score(index, score);
                    let old_max = character.max_hp;
                    let new_max =
                        calculate_max_hp(character.level, character.constitution, hit_die);
                    character.max_hp = new_max;
                    character.current_hp =
                        (character.current_hp + new_max - old_max).clamp(0, new_max);
                }
                self.persist("Caractéristique mise à jour.");
                self.mode = Mode::Normal;
            }
            InputKind::HitPointAdjustment => {
                let Ok(adjustment) = self.input.trim().parse::<i32>() else {
                    self.status = "Entrez un nombre signé, par exemple -5 ou 8.".into();
                    return;
                };
                if let Some(character) = self.selected_character_mut() {
                    if adjustment < 0 {
                        let damage = adjustment.saturating_neg();
                        let absorbed = damage.min(character.temp_hp);
                        character.temp_hp -= absorbed;
                        character.current_hp = character
                            .current_hp
                            .saturating_sub(damage - absorbed)
                            .max(0);
                    } else {
                        character.current_hp = character
                            .current_hp
                            .saturating_add(adjustment)
                            .min(character.max_hp);
                    }
                    append_combat_log(
                        character,
                        &format!("Dégâts/soins appliqués : {adjustment} PV."),
                    );
                }
                self.persist("PV mis à jour.");
                self.mode = Mode::Normal;
            }
            InputKind::TemporaryHitPoints => {
                let Ok(value) = self.input.trim().parse::<i32>() else {
                    self.status = "Entrez un nombre entier positif ou nul.".into();
                    return;
                };
                if value < 0 {
                    self.status = "Les PV temporaires ne peuvent pas être négatifs.".into();
                    return;
                }
                if let Some(character) = self.selected_character_mut() {
                    character.temp_hp = value;
                }
                self.persist("PV temporaires mis à jour.");
                self.mode = Mode::Normal;
            }
            InputKind::Experience => {
                let Ok(experience) = self.input.trim().parse::<u32>() else {
                    self.status = "Entrez un total d'XP positif ou nul.".into();
                    return;
                };
                let level = level_for_experience(experience);
                let Some(index) = self.selected_character_index() else {
                    self.mode = Mode::Normal;
                    return;
                };
                self.characters[index].experience = experience;
                self.set_character_level(index, level);
                append_combat_log(
                    &mut self.characters[index],
                    &format!("XP définis à {experience} (niveau {level})."),
                );
                self.persist(&format!(
                    "XP mis à jour : {experience}; niveau correspondant : {level}."
                ));
                self.mode = Mode::Normal;
            }
            InputKind::Level => {
                let Ok(level) = self.input.trim().parse::<u8>() else {
                    self.status = "Le niveau doit être un nombre entre 1 et 20.".into();
                    return;
                };
                if !(1..=20).contains(&level) {
                    self.status = "Le niveau doit être compris entre 1 et 20.".into();
                    return;
                }
                let Some(index) = self.selected_character_index() else {
                    self.mode = Mode::Normal;
                    return;
                };
                self.set_character_level(index, level);
                append_combat_log(
                    &mut self.characters[index],
                    &format!("Niveau réglé manuellement à {level}."),
                );
                self.persist(&format!("Niveau défini à {level}."));
                self.mode = Mode::Normal;
            }
            InputKind::Condition => {
                let condition = normalize_condition(self.input.trim());
                let condition = condition.as_str();
                if condition.is_empty() {
                    self.status = "La condition ne peut pas être vide.".into();
                    return;
                }
                if let Some(character) = self.selected_character_mut() {
                    if character
                        .conditions
                        .iter()
                        .any(|existing| existing.eq_ignore_ascii_case(condition))
                    {
                        self.status = "Cette condition est déjà active.".into();
                        return;
                    }
                    character.conditions.push(condition.to_owned());
                    append_combat_log(character, &format!("Condition ajoutée : {condition}"));
                }
                self.persist("Condition ajoutée; elle reste active jusqu'à son retrait manuel.");
                self.mode = Mode::Normal;
            }
            InputKind::CombatAction => {
                let action = self.input.trim().to_owned();
                if action.is_empty() {
                    self.status = "La description de l'action ne peut pas être vide.".into();
                    return;
                }
                if let Some(character) = self.selected_character_mut() {
                    append_combat_log(character, &action);
                }
                self.persist("Action ajoutée au journal de combat.");
                self.mode = Mode::Normal;
            }
            InputKind::Subclass => {
                let value = self.input.trim().to_owned();
                if let Some(character) = self.selected_character_mut() {
                    character.subclass = (!value.is_empty()).then_some(value);
                }
                self.persist("Sous-classe mise à jour.");
                self.mode = Mode::Normal;
            }
            InputKind::SkillChoices => {
                let class_name = self
                    .wizard
                    .as_ref()
                    .and_then(|wizard| wizard.class.as_deref())
                    .unwrap_or("");
                let Some(class) = self.find_class(class_name) else {
                    self.status = "Classe absente du compendium.".into();
                    return;
                };
                let required = class
                    .num_skills
                    .as_deref()
                    .and_then(|count| count.parse::<usize>().ok())
                    .unwrap_or(0);
                let selected: Vec<String> = self
                    .input
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .collect();
                let (_, mut allowed) = class_proficiencies(class);
                if let Some(background) = self
                    .wizard
                    .as_ref()
                    .and_then(|wizard| wizard.background.as_deref())
                    .and_then(|name| self.find_background(name))
                {
                    let background_skills = background_skill_proficiencies(background);
                    allowed.retain(|option| {
                        !background_skills
                            .iter()
                            .any(|skill| skill.eq_ignore_ascii_case(option))
                    });
                }
                let normalized: Vec<String> =
                    selected.iter().map(|skill| skill.to_lowercase()).collect();
                if selected.len() != required
                    || normalized
                        .iter()
                        .collect::<std::collections::HashSet<_>>()
                        .len()
                        != selected.len()
                {
                    self.status =
                        format!("Choisissez exactement {required} compétences différentes.");
                    return;
                }
                if selected.iter().any(|skill| {
                    !allowed
                        .iter()
                        .any(|option| option.eq_ignore_ascii_case(skill))
                }) {
                    self.status = format!("Choix invalide. Options : {}", allowed.join(", "));
                    return;
                }
                if let Some(wizard) = self.wizard.as_mut() {
                    wizard.skills = selected;
                }
                self.after_skills_selected();
            }
        }
    }

    /// Navigue dans un sélecteur ou confirme l'option courante.
    fn handle_picker_key(&mut self, key: KeyEvent, kind: PickKind) {
        match key.code {
            KeyCode::Enter => {
                if self.wizard.is_some() && kind == PickKind::StartingItem {
                    self.open_creation_spells();
                } else if self.wizard.is_some() && kind == PickKind::Spell {
                    self.finish_creation();
                } else if kind == PickKind::CastSpell {
                    if let Some(value) = self.picker_options(kind).get(self.picker_index).cloned() {
                        self.cast_spell(&value);
                    }
                } else {
                    let options = self.picker_options(kind);
                    if let Some(value) = options.get(self.picker_index).cloned() {
                        self.choose_option(kind, value);
                    }
                }
            }
            KeyCode::Char(' ') if self.wizard.is_some() && kind == PickKind::StartingItem => {
                self.toggle_current_creation_choice(kind);
            }
            KeyCode::Char(' ') if self.wizard.is_some() && kind == PickKind::Spell => {
                self.toggle_current_creation_choice(kind);
            }
            KeyCode::Up => self.picker_index = self.picker_index.saturating_sub(1),
            KeyCode::Down => {
                let count = self.picker_options(kind).len();
                if self.picker_index + 1 < count {
                    self.picker_index += 1;
                }
            }
            KeyCode::Backspace => {
                self.picker_query.pop();
                self.picker_index = 0;
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.picker_query.push(ch);
                self.picker_index = 0;
            }
            _ => {}
        }
    }

    /// Construit la liste filtrée d'options pour la catégorie sélectionnée.
    fn picker_options(&self, kind: PickKind) -> Vec<String> {
        let mut options: Vec<String> = match kind {
            PickKind::Race => self
                .compendium
                .races
                .iter()
                .map(|entry| entry.name.clone())
                .collect(),
            PickKind::Class => self
                .compendium
                .classes
                .iter()
                .map(|entry| entry.name.clone())
                .collect(),
            PickKind::Background => self
                .compendium
                .backgrounds
                .iter()
                .map(|entry| entry.name.clone())
                .chain(std::iter::once(NO_BACKGROUND_OPTION.to_owned()))
                .collect(),
            PickKind::Item => self
                .compendium
                .items
                .iter()
                .map(|entry| entry.name.clone())
                .collect(),
            PickKind::StartingItem => self
                .compendium
                .items
                .iter()
                .map(|entry| entry.name.clone())
                .collect(),
            PickKind::Spell => {
                let class = self
                    .wizard
                    .as_ref()
                    .and_then(|wizard| wizard.class.as_deref())
                    .or_else(|| self.selected_character().map(|entry| entry.class.as_str()));
                self.compendium
                    .spells
                    .iter()
                    .filter(|spell| {
                        class.is_none_or(|class_name| {
                            spell
                                .classes
                                .as_deref()
                                .is_some_and(|classes| classes.contains(class_name))
                        }) && (self.wizard.is_none()
                            || spell.level.as_deref().is_some_and(|level| {
                                level.eq_ignore_ascii_case("cantrip")
                                    || level.parse::<u8>().is_ok_and(|level| level <= 1)
                            }))
                    })
                    .map(|spell| spell.name.clone())
                    .collect()
            }
            PickKind::CastSpell => self
                .selected_character()
                .map(|character| character.prepared_spells.clone())
                .unwrap_or_default(),
            PickKind::Feat => self
                .compendium
                .feats
                .iter()
                .map(|feat| feat.name.clone())
                .collect(),
        };
        if !self.picker_query.is_empty() {
            let query = self.picker_query.to_lowercase();
            options.retain(|option| option.to_lowercase().contains(&query));
        }
        options
    }

    /// Applique le choix et enchaîne vers l'étape suivante ou sauvegarde le personnage.
    fn choose_option(&mut self, kind: PickKind, value: String) {
        match kind {
            PickKind::Race => {
                if let Some(wizard) = self.wizard.as_mut() {
                    wizard.race = Some(value);
                }
                self.open_picker(PickKind::Class);
            }
            PickKind::Class => {
                if let Some(wizard) = self.wizard.as_mut() {
                    wizard.class = Some(value);
                }
                self.open_picker(PickKind::Background);
            }
            PickKind::Background => {
                if let Some(wizard) = self.wizard.as_mut() {
                    wizard.background = (value != NO_BACKGROUND_OPTION).then_some(value);
                }
                self.start_input(InputKind::Score(0), "Caractéristique FOR");
                self.input = STANDARD_ARRAY[0].to_string();
            }
            PickKind::Item => {
                if let Some(character) = self.selected_character_mut() {
                    if let Some(existing) = character
                        .equipment
                        .iter_mut()
                        .find(|slot| slot.item_name == value)
                    {
                        existing.quantity += 1;
                    } else {
                        character.equipment.push(InventorySlot {
                            item_name: value,
                            quantity: 1,
                            equipped: false,
                            attuned: false,
                        });
                    }
                    self.detail_index = character.equipment.len().saturating_sub(1);
                }
                self.persist("Objet ajouté à l'inventaire.");
                self.mode = Mode::Normal;
            }
            PickKind::StartingItem => {
                self.toggle_creation_choice(PickKind::StartingItem, &value);
            }
            PickKind::Spell => {
                if let Some(character) = self.selected_character_mut() {
                    if !character.prepared_spells.contains(&value) {
                        character.prepared_spells.push(value);
                    }
                    self.detail_index = character.prepared_spells.len().saturating_sub(1);
                }
                self.persist("Sort ajouté à la liste.");
                self.mode = Mode::Normal;
            }
            PickKind::CastSpell => self.cast_spell(&value),
            PickKind::Feat => {
                if let Some(character) = self.selected_character_mut() {
                    if !character.feats.contains(&value) {
                        character.feats.push(value);
                    }
                    self.detail_index = character.feats.len().saturating_sub(1);
                }
                self.persist("Don ajouté à la fiche.");
                self.mode = Mode::Normal;
            }
        }
    }

    /// Convertit le brouillon en fiche complète et la sauvegarde.
    fn finish_creation(&mut self) {
        let Some(wizard) = self.wizard.take() else {
            self.mode = Mode::Normal;
            return;
        };
        let background = wizard.background.unwrap_or_default();
        let equipment_warning = wizard.equipment_warnings.join(", ");
        let mut equipment_choices = wizard.equipment_choices;
        if !equipment_warning.is_empty() {
            equipment_choices.push(format!(
                "Objets du paquet à ajouter manuellement : {equipment_warning}"
            ));
        }
        let mut skill_proficiencies = wizard.skills;
        if let Some(entry) = self.find_background(&background) {
            for skill in background_skill_proficiencies(entry) {
                if !skill_proficiencies
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&skill))
                {
                    skill_proficiencies.push(skill);
                }
            }
        }
        let feats: Vec<String> = self
            .find_background(&background)
            .and_then(background_feat)
            .map(|feat| {
                let base_name = feat.split(" (").next().unwrap_or(&feat);
                self.compendium
                    .feats
                    .iter()
                    .find(|entry| {
                        entry.name.eq_ignore_ascii_case(&feat)
                            || entry.name.eq_ignore_ascii_case(base_name)
                    })
                    .map(|entry| entry.name.clone())
                    .unwrap_or(feat)
            })
            .into_iter()
            .collect();
        let mut equipment: Vec<InventorySlot> = Vec::new();
        for item_name in wizard.equipment {
            if let Some(existing) = equipment
                .iter_mut()
                .find(|slot| slot.item_name == item_name)
            {
                existing.quantity = existing.quantity.saturating_add(1);
            } else {
                equipment.push(InventorySlot {
                    item_name,
                    quantity: 1,
                    equipped: false,
                    attuned: false,
                });
            }
        }
        let mut character = Character {
            name: wizard.name,
            race: wizard.race.unwrap_or_default(),
            class: wizard.class.unwrap_or_default(),
            background,
            strength: wizard.scores[0],
            dexterity: wizard.scores[1],
            constitution: wizard.scores[2],
            intelligence: wizard.scores[3],
            wisdom: wizard.scores[4],
            charisma: wizard.scores[5],
            equipment,
            starting_equipment_choices: equipment_choices,
            prepared_spells: wizard.spells,
            feats,
            skill_proficiencies,
            ..Character::default()
        };
        if let Some(class) = self.find_class(&character.class) {
            character.saving_throw_proficiencies = class_proficiencies(class).0;
        }
        let hit_die = self.class_hit_die(&character.class);
        character.max_hp = (hit_die + Character::modifier(character.constitution)).max(1);
        character.current_hp = character.max_hp;
        character.current_hit_dice = 1;
        if let Some(class) = self.find_class(&character.class).cloned() {
            Self::update_progression(&mut character, &class, true);
        }
        self.characters.push(character);
        self.selected = self.characters.len() - 1;
        self.tab = Tab::Sheet;
        self.mode = Mode::Normal;
        self.persist("Personnage créé et sauvegardé.");
        if !equipment_warning.is_empty() && self.status == "Personnage créé et sauvegardé." {
            self.status = format!(
                "Personnage créé. Objets du paquet non trouvés dans le compendium, à ajouter manuellement : {equipment_warning}"
            );
        }
    }

    /// Traite la confirmation ou l'annulation de suppression d'une fiche.
    fn handle_delete_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') => {
                if self.selected < self.characters.len() {
                    self.characters.remove(self.selected);
                    self.selected = self.selected.min(self.characters.len().saturating_sub(1));
                    self.persist("Personnage supprimé.");
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
    }

    /// Retire l'entrée sélectionnée de l'inventaire, des sorts ou des dons.
    fn remove_selected(&mut self) {
        let mut changed = false;
        let tab = self.tab;
        let index = self.detail_index;
        if let Some(character) = self.selected_character_mut() {
            match tab {
                Tab::Equipment if index < character.equipment.len() => {
                    if character.equipment[index].quantity > 1 {
                        character.equipment[index].quantity -= 1;
                    } else {
                        character.equipment.remove(index);
                    }
                    changed = true;
                }
                Tab::Spells if index < character.prepared_spells.len() => {
                    character.prepared_spells.remove(index);
                    changed = true;
                }
                Tab::Progression if index < character.feats.len() => {
                    character.feats.remove(index);
                    changed = true;
                }
                _ => {}
            }
            self.detail_index = index.min(match tab {
                Tab::Equipment => character.equipment.len().saturating_sub(1),
                Tab::Spells => character.prepared_spells.len().saturating_sub(1),
                Tab::Progression => character.feats.len().saturating_sub(1),
                _ => 0,
            });
        }
        if changed {
            self.persist("Élément retiré.");
        }
    }

    /// Inverse l'état équipé de l'objet sélectionné.
    fn toggle_equipped(&mut self) {
        let mut changed = false;
        let index = self.detail_index;
        if let Some(character) = self.selected_character_mut() {
            if let Some(slot) = character.equipment.get_mut(index) {
                slot.equipped = !slot.equipped;
                changed = true;
            }
        }
        if changed {
            self.persist("Équipement mis à jour.");
        }
    }

    /// Augmente d'un niveau en actualisant PV, progression et journal de combat.
    fn level_up(&mut self) {
        let Some(index) = self.selected_character_index() else {
            return;
        };
        if self.characters[index].level >= 20 {
            self.status = "Le personnage est déjà niveau 20.".into();
            return;
        }
        let level = self.characters[index].level + 1;
        self.set_character_level(index, level);
        append_combat_log(
            &mut self.characters[index],
            &format!("Niveau gagné : {level}."),
        );
        self.persist("Niveau gagné; PV et ressources de classe actualisés.");
    }

    /// Applique un niveau explicite en recalculant les valeurs dépendantes sans effacer les dépenses.
    fn set_character_level(&mut self, index: usize, level: u8) {
        let class = self.find_class(&self.characters[index].class).cloned();
        let character = &mut self.characters[index];
        let old_max_hp = character.max_hp;
        let spent_hit_dice = character.level.saturating_sub(character.current_hit_dice);
        character.level = level;
        let hit_die = class
            .as_ref()
            .and_then(|class| class.hd.as_deref())
            .and_then(|die| die.parse::<i32>().ok())
            .unwrap_or(8);
        character.max_hp = calculate_max_hp(level, character.constitution, hit_die);
        character.current_hp =
            (character.current_hp + character.max_hp - old_max_hp).clamp(0, character.max_hp);
        character.current_hit_dice = level.saturating_sub(spent_hit_dice.min(level));

        let previous_resources = std::mem::take(&mut character.resources);
        let previous_slots = character.spell_slots;
        character.spell_slots = [0; 10];
        character.spell_slots_max = [0; 10];
        if let Some(class) = class {
            Self::update_progression(character, &class, true);
        }
        for resource in &mut character.resources {
            if let Some(previous) = previous_resources
                .iter()
                .find(|previous| previous.name == resource.name)
            {
                let gained_maximum = resource.maximum.saturating_sub(previous.maximum);
                resource.current = previous
                    .current
                    .saturating_add(gained_maximum)
                    .min(resource.maximum);
            }
        }
        for (index, spent) in previous_slots.iter().enumerate() {
            character.spell_slots[index] = (*spent).min(character.spell_slots_max[index]);
        }
        Self::sync_legacy_resource(character);
    }

    /// Retire la condition ajoutée le plus récemment.
    fn remove_last_condition(&mut self) {
        let Some(character) = self.selected_character_mut() else {
            return;
        };
        if let Some(condition) = character.conditions.pop() {
            append_combat_log(character, &format!("Condition retirée : {condition}"));
            self.persist(&format!("Condition « {condition} » retirée."));
        } else {
            self.status = "Aucune condition active à retirer.".into();
        }
    }

    /// Lance un sort préparé et dépense un emplacement valide, puis journalise l'action.
    fn cast_spell(&mut self, name: &str) {
        let Some(spell) = self
            .compendium
            .spells
            .iter()
            .find(|spell| spell.name == name)
        else {
            self.status = format!("Sort « {name} » absent du compendium.");
            self.mode = Mode::Normal;
            return;
        };
        let spell_level = spell
            .level
            .as_deref()
            .and_then(|level| level.parse::<usize>().ok())
            .unwrap_or(0);
        let slot_level = if spell_level == 0 {
            0
        } else {
            self.spell_slot_level.max(spell_level).min(9)
        };
        let Some(character) = self.selected_character_mut() else {
            self.mode = Mode::Normal;
            return;
        };
        if spell_level > 0
            && (character.spell_slots_max[slot_level] == 0
                || character.spell_slots[slot_level] >= character.spell_slots_max[slot_level])
        {
            self.status = format!("Aucun emplacement de niveau {slot_level} disponible.");
            self.mode = Mode::Normal;
            return;
        }
        if spell_level > 0 {
            character.spell_slots[slot_level] += 1;
        }
        let entry = if spell_level == 0 {
            format!("Sort lancé : {name} (sort mineur)")
        } else {
            format!("Sort lancé : {name} (emplacement niveau {slot_level})")
        };
        append_combat_log(character, &entry);
        self.persist(&entry);
        self.mode = Mode::Normal;
    }

    /// Dépense la ressource sélectionnée si elle a encore des utilisations.
    fn spend_resource(&mut self) {
        let index = self.resource_index;
        let Some(character) = self.selected_character_mut() else {
            return;
        };
        if let Some(resource) = character.resources.get_mut(index) {
            if resource.current == 0 {
                self.status = format!("Aucune utilisation restante de {}.", resource.name);
                return;
            }
            resource.current -= 1;
            let name = resource.name.clone();
            append_combat_log(character, &format!("Ressource dépensée : {name}."));
            Self::sync_legacy_resource(character);
            self.persist("Ressource dépensée.");
        } else if character.current_resource > 0 {
            character.current_resource -= 1;
            self.persist("Ressource dépensée.");
        } else {
            self.status = "Aucune ressource disponible à dépenser.".into();
        }
    }

    /// Déplace la sélection dans la liste des ressources de classe.
    fn adjust_resource_selection(&mut self, amount: i8) {
        let count = self
            .selected_character()
            .map(|character| character.resources.len())
            .unwrap_or(0);
        if count == 0 {
            return;
        }
        self.resource_index = if amount < 0 {
            self.resource_index.saturating_sub(1)
        } else {
            (self.resource_index + 1).min(count - 1)
        };
    }

    /// Lance un dé de vie et applique les soins après le bonus de Constitution.
    fn spend_hit_die(&mut self) {
        let die = self
            .selected_character()
            .map(|character| self.class_hit_die(&character.class))
            .unwrap_or(8);
        let Some(character) = self.selected_character_mut() else {
            return;
        };
        if character.current_hit_dice == 0 {
            self.status = "Aucun dé de vie disponible.".into();
            return;
        }
        let roll = rand::thread_rng().gen_range(1..=die);
        let healing = (roll + Character::modifier(character.constitution)).max(0);
        character.current_hit_dice -= 1;
        character.current_hp = (character.current_hp + healing).min(character.max_hp);
        append_combat_log(
            character,
            &format!("Dé de vie dépensé : {roll} + CON, {healing} PV récupérés."),
        );
        self.persist(&format!(
            "Dé de vie dépensé : {roll} + CON, {healing} PV récupérés."
        ));
    }

    /// Restaure les PV, les ressources et les emplacements après un repos long.
    fn long_rest(&mut self) {
        if let Some(character) = self.selected_character_mut() {
            character.current_hp = character.max_hp;
            character.temp_hp = 0;
            character.current_hit_dice =
                (character.current_hit_dice + (character.level / 2).max(1)).min(character.level);
            for resource in &mut character.resources {
                resource.current = resource.maximum;
            }
            Self::sync_legacy_resource(character);
            character.spell_slots = [0; 10];
            append_combat_log(
                character,
                "Repos long : PV, ressources et emplacements restaurés.",
            );
            self.persist("Repos long : PV et ressources restaurés.");
        }
    }

    /// Restaure les ressources configurées pour un repos court.
    fn short_rest(&mut self) {
        if let Some(character) = self.selected_character_mut() {
            for resource in &mut character.resources {
                if resource.reset.eq_ignore_ascii_case("S") {
                    resource.current = resource.maximum;
                }
            }
            Self::sync_legacy_resource(character);
            if character.class.contains("Warlock") {
                character.spell_slots = [0; 10];
            }
            append_combat_log(
                character,
                "Repos court : ressources à récupération courte restaurées.",
            );
            self.persist("Repos court : ressources récupérables restaurées. Touche g pour dépenser un dé de vie.");
        }
    }

    /// Dépense ou restaure un emplacement du niveau de sort sélectionné.
    fn adjust_spell_slot(&mut self, amount: i8) {
        let level = self.spell_slot_level.clamp(1, 9);
        if let Some(character) = self.selected_character_mut() {
            let slot = &mut character.spell_slots[level];
            let previous = *slot;
            *slot = if amount > 0 {
                slot.saturating_add(1).min(character.spell_slots_max[level])
            } else {
                slot.saturating_sub(1)
            };
            if *slot != previous {
                let action = if amount > 0 {
                    format!("Emplacement de niveau {level} dépensé.")
                } else {
                    format!("Emplacement de niveau {level} restauré.")
                };
                append_combat_log(character, &action);
                self.persist("Emplacements de sort mis à jour.");
            }
        }
    }

    /// Synchronise ressources de classe et emplacements avec les paliers XML déjà atteints.
    fn update_progression(character: &mut Character, class: &Class, initialize: bool) {
        // Les compteurs et emplacements évoluent par paliers dans les autolevels XML.
        for autolevel in &class.autolevels {
            let Ok(level) = autolevel.level.parse::<u8>() else {
                continue;
            };
            if level > character.level {
                continue;
            }
            for counter in &autolevel.counters {
                let Ok(maximum) = counter.value.parse::<u8>() else {
                    continue;
                };
                if let Some(resource) = character
                    .resources
                    .iter_mut()
                    .find(|resource| resource.name == counter.name)
                {
                    if maximum > resource.maximum {
                        let increase = maximum - resource.maximum;
                        resource.maximum = maximum;
                        resource.current = if initialize {
                            maximum
                        } else {
                            resource.current.saturating_add(increase).min(maximum)
                        };
                    }
                    resource.reset = counter.reset.clone();
                } else {
                    character.resources.push(Resource {
                        name: counter.name.clone(),
                        maximum,
                        current: maximum,
                        reset: counter.reset.clone(),
                    });
                }
            }
            if let Some(slots) = autolevel.slots.as_deref() {
                for (spell_level, count) in slots.split(',').skip(1).enumerate() {
                    let Ok(maximum) = count.trim().parse::<u8>() else {
                        continue;
                    };
                    let index = spell_level + 1;
                    if index < character.spell_slots_max.len() {
                        character.spell_slots_max[index] = maximum;
                        character.spell_slots[index] = character.spell_slots[index].min(maximum);
                    }
                }
            }
        }
        Self::sync_legacy_resource(character);
    }

    /// Maintient les anciens champs de ressource en accord avec la première ressource détaillée.
    fn sync_legacy_resource(character: &mut Character) {
        if let Some(resource) = character.resources.first() {
            character.resource_name = resource.name.clone();
            character.max_resource = resource.maximum;
            character.current_resource = resource.current;
        }
    }

    /// Renvoie le personnage actuellement sélectionné.
    fn selected_character(&self) -> Option<&Character> {
        self.characters.get(self.selected)
    }

    /// Renvoie le personnage sélectionné pour modification.
    fn selected_character_mut(&mut self) -> Option<&mut Character> {
        self.characters.get_mut(self.selected)
    }

    /// Retourne l'index sélectionné uniquement s'il référence une fiche existante.
    fn selected_character_index(&self) -> Option<usize> {
        (self.selected < self.characters.len()).then_some(self.selected)
    }

    /// Recherche une classe par son nom de compendium.
    fn find_class(&self, name: &str) -> Option<&Class> {
        self.compendium
            .classes
            .iter()
            .find(|class| class.name == name)
    }

    /// Recherche un background par son nom de compendium.
    fn find_background(&self, name: &str) -> Option<&Background> {
        self.compendium
            .backgrounds
            .iter()
            .find(|background| background.name == name)
    }

    /// Lit le dé de vie de classe; utilise D8 si le compendium ne donne aucune valeur.
    fn class_hit_die(&self, name: &str) -> i32 {
        self.find_class(name)
            .and_then(|class| class.hd.as_deref())
            .and_then(|hit_die| hit_die.parse().ok())
            .unwrap_or(8)
    }

    /// Persiste toutes les fiches et affiche explicitement une erreur en cas d'échec.
    fn persist(&mut self, success: &str) {
        match storage::save(&self.storage_path, &self.characters) {
            Ok(()) => self.status = success.to_owned(),
            Err(error) => self.status = format!("Erreur de sauvegarde : {error}"),
        }
    }

    /// Dessine le cadre, la fiche et le dialogue correspondant au mode actif.
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(3),
            ])
            .split(area);

        let title = Paragraph::new("D&D PERSONNAGES  |  Compendium 5e 2024")
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, layout[0]);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(27), Constraint::Percentage(73)])
            .split(layout[1]);
        self.render_character_list(frame, body[0]);
        self.render_detail(frame, body[1]);

        let footer = Paragraph::new(self.status.as_str())
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("État"));
        frame.render_widget(footer, layout[2]);

        match self.mode {
            Mode::Input(kind) => self.render_input(frame, kind, area),
            Mode::Picker(kind) => self.render_picker(frame, kind, area),
            Mode::ConfirmDelete => self.render_confirm_delete(frame, area),
            Mode::Normal => {}
        }
    }

    /// Affiche la liste des personnages avec la sélection courante.
    fn render_character_list(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = if self.characters.is_empty() {
            vec![ListItem::new("Aucun personnage\nAppuyez sur n")]
        } else {
            self.characters
                .iter()
                .map(|character| {
                    ListItem::new(format!(
                        "{}\n{} niv. {}",
                        character.name, character.class, character.level
                    ))
                })
                .collect()
        };
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Personnages"))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("› ");
        let mut state = ListState::default();
        if !self.characters.is_empty() {
            state.select(Some(self.selected));
        }
        frame.render_stateful_widget(list, area, &mut state);
    }

    /// Dessine le panneau de détail et ses onglets.
    fn render_detail(&self, frame: &mut Frame, area: Rect) {
        let Some(character) = self.selected_character() else {
            let text = Text::from(vec![
                Line::from("Bienvenue dans votre gestionnaire de personnages."),
                Line::from(
                    "Créez une fiche avec n. Les données sont sauvegardées automatiquement.",
                ),
                Line::from(""),
                Line::from(format!("Personnages : {}", self.characters.len())),
                Line::from(format!(
                    "Classes dans le compendium : {}",
                    self.compendium.classes.len()
                )),
                Line::from(format!(
                    "Races : {}  |  Backgrounds : {}  |  Sorts : {}  |  Objets : {}",
                    self.compendium.races.len(),
                    self.compendium.backgrounds.len(),
                    self.compendium.spells.len(),
                    self.compendium.items.len()
                )),
            ]);
            frame.render_widget(
                Paragraph::new(text).wrap(Wrap { trim: false }).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Tableau de bord"),
                ),
                area,
            );
            return;
        };
        let (title, lines) = match self.tab {
            Tab::Sheet => ("Fiche", self.sheet_lines(character)),
            Tab::Equipment => ("Équipement", self.equipment_lines(character)),
            Tab::Spells => ("Sorts et emplacements", self.spell_lines(character)),
            Tab::Progression => ("Progression", self.progression_lines(character)),
        };
        let tabs = Line::from(vec![
            Span::styled(" Fiche ", tab_style(self.tab == Tab::Sheet)),
            Span::styled(" Équipement ", tab_style(self.tab == Tab::Equipment)),
            Span::styled(" Sorts ", tab_style(self.tab == Tab::Spells)),
            Span::styled(" Progression ", tab_style(self.tab == Tab::Progression)),
        ]);
        let text = Text::from(lines);
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(inner);
        frame.render_widget(Paragraph::new(tabs), chunks[0]);
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .scroll((self.detail_scroll, 0)),
            chunks[1],
        );
    }

    /// Construit les lignes de fiche calculées et les éléments de suivi de combat.
    fn sheet_lines(&self, character: &Character) -> Vec<Line<'static>> {
        let race = self
            .compendium
            .races
            .iter()
            .find(|race| race.name == character.race);
        let speed = race.and_then(|race| race.speed.as_deref()).unwrap_or("?");
        let armor_class = self.armor_class(character);
        let ability = character.ability_scores();
        let mut lines = vec![
            Line::from(Span::styled(
                if character.background.is_empty() {
                    format!("{} — {}", character.name, character.race)
                } else {
                    format!(
                        "{} — {} — {}",
                        character.name, character.race, character.background
                    )
                },
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(format!(
                "{}{}  |  Niveau {}  |  Vitesse {} pi",
                character.class,
                character
                    .subclass
                    .as_deref()
                    .map(|s| format!(" ({s})"))
                    .unwrap_or_default(),
                character.level,
                speed
            )),
            Line::from(""),
            Line::from(format!(
                "PV {}/{}  (+{} temp.)     CA {}     Initiative {:+}     Maîtrise +{}",
                character.current_hp,
                character.max_hp,
                character.temp_hp,
                armor_class,
                Character::modifier(character.dexterity),
                character.proficiency_bonus()
            )),
            Line::from(format!(
                "XP {} (prochain niveau : {})     Dés de vie {}/{}     Perception passive {}",
                character.experience,
                next_level_xp(character.level).map_or("niveau max".into(), |xp| format!("{xp} XP")),
                character.current_hit_dice,
                character.level,
                10 + Character::modifier(character.wisdom)
                    + if character
                        .skill_proficiencies
                        .iter()
                        .any(|skill| skill.eq_ignore_ascii_case("Perception"))
                    {
                        character.proficiency_bonus()
                    } else {
                        0
                    }
            )),
            Line::from(""),
            Line::from("Caractéristiques (touche 1–6 pour modifier) :"),
            Line::from(
                ABILITY_NAMES
                    .iter()
                    .enumerate()
                    .map(|(index, name)| {
                        format!(
                            "{name} {:>2} ({:+})",
                            ability[index],
                            Character::modifier(ability[index])
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("    "),
            ),
            Line::from(""),
            Line::from(if character.resources.is_empty() {
                format!(
                    "Ressources : {} {}/{}",
                    character.resource_name, character.current_resource, character.max_resource
                )
            } else {
                format!(
                    "Ressources ([ / ] choisir, u dépenser) : {}",
                    character
                        .resources
                        .iter()
                        .enumerate()
                        .map(|(index, resource)| format!(
                            "{}{} {}/{}",
                            if index == self.resource_index {
                                "›"
                            } else {
                                ""
                            },
                            resource.name,
                            resource.current,
                            resource.maximum
                        ))
                        .collect::<Vec<_>>()
                        .join("  |  ")
                )
            }),
            Line::from(""),
            Line::from(if character.conditions.is_empty() {
                "Conditions : aucune".to_owned()
            } else {
                format!(
                    "Conditions actives (C ajouter, x retirer la dernière) : {}",
                    character.conditions.join(", ")
                )
            }),
            Line::from("Actions récentes :"),
        ];
        if character.combat_log.is_empty() {
            lines.push(Line::from("Aucune action enregistrée."));
        } else {
            lines.extend(
                character
                    .combat_log
                    .iter()
                    .rev()
                    .take(8)
                    .map(|action| Line::from(format!("• {action}"))),
            );
        }
        lines.push(Line::from("Commandes : e XP | l niveau | C condition | c lancer un sort | a noter une action | h dégâts/soins | r repos long | v repos court | g dé de vie"));
        let saves = character
            .ability_scores()
            .iter()
            .enumerate()
            .map(|(index, score)| {
                let bonus = Character::modifier(*score)
                    + if character
                        .saving_throw_proficiencies
                        .iter()
                        .any(|save| save.eq_ignore_ascii_case(ABILITY_NAMES[index]))
                    {
                        character.proficiency_bonus()
                    } else {
                        0
                    };
                format!("{} {:+}", ABILITY_NAMES[index], bonus)
            })
            .collect::<Vec<_>>()
            .join("  ");
        lines.push(Line::from(format!("Jets de sauvegarde : {saves}")));
        lines.push(Line::from("Compétences :"));
        for chunk in SKILLS.chunks(3) {
            lines.push(Line::from(
                chunk
                    .iter()
                    .map(|(name, ability_index)| {
                        let score = ability[*ability_index];
                        let proficient = character
                            .skill_proficiencies
                            .iter()
                            .any(|skill| skill.eq_ignore_ascii_case(name));
                        let bonus = Character::modifier(score)
                            + if proficient {
                                character.proficiency_bonus()
                            } else {
                                0
                            };
                        format!("{} {:+}{}", name, bonus, if proficient { "*" } else { "" })
                    })
                    .collect::<Vec<_>>()
                    .join("    "),
            ));
        }
        if let Some(class) = self.find_class(&character.class) {
            if let Some(ability_name) = class.spell_ability.as_deref() {
                if let Some(index) = ability_index(ability_name) {
                    let modifier = Character::modifier(ability[index]);
                    lines.push(Line::from(format!(
                        "Incantation ({ability_name}) : DD {} | attaque {:+}",
                        8 + character.proficiency_bonus() + modifier,
                        character.proficiency_bonus() + modifier
                    )));
                }
            }
        }
        if let Some(race) = race {
            for trait_entry in &race.traits {
                lines.push(Line::from(format!("Trait — {}", trait_entry.name)));
                if let Some(text) = trait_entry.text.first() {
                    lines.push(Line::from(format!("  {}", text.replace('\n', " "))));
                }
            }
        }
        if let Some(background) = self.find_background(&character.background) {
            for feature in &background.traits {
                if feature.name.eq_ignore_ascii_case("Description") {
                    continue;
                }
                lines.push(Line::from(format!("Background — {}", feature.name)));
                if let Some(text) = feature.text.first() {
                    lines.push(Line::from(format!("  {}", text.replace('\n', " "))));
                }
            }
        }
        lines
    }

    /// Construit l'inventaire et ses caractéristiques issues du compendium.
    fn equipment_lines(&self, character: &Character) -> Vec<Line<'static>> {
        let mut lines = vec![Line::from(
            "i ajouter depuis le compendium | espace équiper/déséquiper | x retirer",
        )];
        lines.extend(
            character
                .starting_equipment_choices
                .iter()
                .map(|choice| Line::from(format!("Départ : {choice}"))),
        );
        if character.equipment.is_empty() {
            lines.push(Line::from("Inventaire vide."));
        }
        for (index, slot) in character.equipment.iter().enumerate() {
            let marker = if index == self.detail_index {
                "›"
            } else {
                " "
            };
            let equipped = if slot.equipped { " [équipé]" } else { "" };
            if let Some(item) = self
                .compendium
                .items
                .iter()
                .find(|item| item.name == slot.item_name)
            {
                let stats = item_summary(item);
                lines.push(Line::from(format!(
                    "{marker} {} ×{}{}  {}",
                    slot.item_name, slot.quantity, equipped, stats
                )));
            } else {
                lines.push(Line::from(format!(
                    "{marker} {} ×{}{}",
                    slot.item_name, slot.quantity, equipped
                )));
            }
        }
        lines
    }

    /// Affiche les sorts préparés et les emplacements par niveau.
    fn spell_lines(&self, character: &Character) -> Vec<Line<'static>> {
        let mut lines = vec![
            Line::from(
                "p ajouter un sort | x retirer | 1–9 niveau d'emplacement | [ restaurer / ] dépenser",
            ),
            Line::from(format!(
                "Emplacements dépensés/total (niveau {} sélectionné) : {}",
                self.spell_slot_level,
                (1..=9)
                    .map(|level| format!(
                        "{level}:{}/{}",
                        character.spell_slots[level], character.spell_slots_max[level]
                    ))
                    .collect::<Vec<_>>()
                    .join("  ")
            )),
        ];
        if character.prepared_spells.is_empty() {
            lines.push(Line::from("Aucun sort enregistré."));
        }
        for (index, name) in character.prepared_spells.iter().enumerate() {
            let marker = if index == self.detail_index {
                "›"
            } else {
                " "
            };
            if let Some(spell) = self
                .compendium
                .spells
                .iter()
                .find(|spell| &spell.name == name)
            {
                lines.push(Line::from(format!(
                    "{marker} niv. {} {} ({})",
                    spell.level.as_deref().unwrap_or("?"),
                    spell.name,
                    spell.school.as_deref().unwrap_or("école inconnue")
                )));
                if let Some(details) = spell_detail(spell) {
                    lines.push(Line::from(format!("   {details}")));
                }
                for description in spell.text.iter().take(1) {
                    lines.push(Line::from(format!("   {}", description.replace('\n', " "))));
                }
            } else {
                lines.push(Line::from(format!("{marker} {name}")));
            }
        }
        lines
    }

    /// Affiche le niveau, les dons et les capacités de classe débloquées.
    fn progression_lines(&self, character: &Character) -> Vec<Line<'static>> {
        let mut lines = vec![
            Line::from(
                "e XP | l niveau | + niveau | b sous-classe | f ajouter un don | x retirer le don sélectionné",
            ),
            Line::from(format!(
                "Niveau {} / 20 — XP {} — prochain seuil : {}",
                character.level,
                character.experience,
                next_level_xp(character.level)
                    .map_or("niveau maximum".into(), |xp| format!("{xp} XP"))
            )),
        ];
        if character.feats.is_empty() {
            lines.push(Line::from("Dons : aucun"));
        } else {
            lines.push(Line::from("Dons :"));
            for (index, name) in character.feats.iter().enumerate() {
                let marker = if index == self.detail_index {
                    "›"
                } else {
                    " "
                };
                lines.push(Line::from(format!("{marker} {name}")));
                if let Some(feat) = self.compendium.feats.iter().find(|feat| &feat.name == name) {
                    if let Some(text) = feat.text.first() {
                        lines.push(Line::from(format!("   {}", text.replace('\n', " "))));
                    }
                }
            }
        }
        if let Some(class) = self.find_class(&character.class) {
            for autolevel in &class.autolevels {
                let Ok(level) = autolevel.level.parse::<u8>() else {
                    continue;
                };
                if level > character.level {
                    continue;
                }
                for feature in &autolevel.features {
                    lines.push(Line::from(format!("Niv. {level} — {}", feature.name)));
                    if let Some(description) = feature.text.first() {
                        lines.push(Line::from(format!("  {}", description.replace('\n', " "))));
                    }
                }
            }
        } else {
            lines.push(Line::from("Classe absente du compendium."));
        }
        lines
    }

    /// Calcule la CA à partir des armures et boucliers équipés.
    fn armor_class(&self, character: &Character) -> i32 {
        let mut armor = 0;
        let mut shield = 0;
        for slot in character.equipment.iter().filter(|slot| slot.equipped) {
            if let Some(item) = self
                .compendium
                .items
                .iter()
                .find(|item| item.name == slot.item_name)
            {
                let value = item
                    .ac
                    .as_deref()
                    .and_then(|value| value.parse::<i32>().ok())
                    .unwrap_or(0);
                if item.name.to_lowercase().contains("shield")
                    || item.name.to_lowercase().contains("bouclier")
                {
                    shield = shield.max(value);
                } else {
                    armor = armor.max(value);
                }
            }
        }
        // La valeur CA du XML est la base de l'armure; les boucliers s'y ajoutent.
        let dexterity = Character::modifier(character.dexterity);
        if armor == 0 {
            10 + dexterity + shield
        } else {
            let armor_name = character
                .equipment
                .iter()
                .filter(|slot| slot.equipped)
                .find(|slot| {
                    self.compendium.items.iter().any(|item| {
                        item.name == slot.item_name
                            && item
                                .ac
                                .as_deref()
                                .and_then(|ac| ac.parse::<i32>().ok())
                                .is_some_and(|ac| ac == armor)
                    })
                })
                .map(|slot| slot.item_name.to_lowercase())
                .unwrap_or_default();
            let dexterity_cap = if ["ring mail", "chain mail", "splint", "plate"]
                .iter()
                .any(|word| armor_name.contains(word))
            {
                0
            } else if [
                "hide",
                "chain shirt",
                "scale mail",
                "breastplate",
                "half plate",
            ]
            .iter()
            .any(|word| armor_name.contains(word))
            {
                2
            } else {
                dexterity
            };
            armor + dexterity.min(dexterity_cap) + shield
        }
    }

    /// Dessine une fenêtre de saisie avec l'aide adaptée au champ actif.
    fn render_input(&self, frame: &mut Frame, kind: InputKind, area: Rect) {
        let prompt = match kind {
            InputKind::CharacterName => "Nom du personnage",
            InputKind::Score(index) => match index {
                0 => "Caractéristique FOR",
                1 => "Caractéristique DEX",
                2 => "Caractéristique CON",
                3 => "Caractéristique INT",
                4 => "Caractéristique SAG",
                _ => "Caractéristique CHA",
            },
            InputKind::BackgroundAbilityChoices => "Bonus de caractéristiques du background",
            InputKind::EquipmentChoices => "Paquets d'équipement de départ (A ou B)",
            InputKind::EditScore(index) => match index {
                0 => "Modifier FOR",
                1 => "Modifier DEX",
                2 => "Modifier CON",
                3 => "Modifier INT",
                4 => "Modifier SAG",
                _ => "Modifier CHA",
            },
            InputKind::HitPointAdjustment => "Dégâts/soins (valeur signée)",
            InputKind::TemporaryHitPoints => "Points de vie temporaires",
            InputKind::Experience => "XP totale",
            InputKind::Level => "Niveau (1–20)",
            InputKind::Condition => "Ajouter une condition",
            InputKind::CombatAction => "Action, attaque ou note de combat",
            InputKind::Subclass => "Sous-classe (vide pour effacer)",
            InputKind::SkillChoices => "Maîtrises de compétences (séparées par des virgules)",
        };
        let help = match kind {
            InputKind::SkillChoices => self
                .wizard
                .as_ref()
                .and_then(|wizard| wizard.class.as_deref())
                .and_then(|class_name| self.find_class(class_name))
                .map(|class| {
                    let required = class.num_skills.as_deref().unwrap_or("0");
                    let (_, options) = class_proficiencies(class);
                    let background_skills = self
                        .wizard
                        .as_ref()
                        .and_then(|wizard| wizard.background.as_deref())
                        .and_then(|name| self.find_background(name))
                        .map(background_skill_proficiencies)
                        .unwrap_or_default();
                    let options: Vec<_> = options
                        .into_iter()
                        .filter(|option| {
                            !background_skills
                                .iter()
                                .any(|skill| skill.eq_ignore_ascii_case(option))
                        })
                        .collect();
                    format!("\nChoisissez {required} : {}", options.join(", "))
                })
                .unwrap_or_default(),
            InputKind::BackgroundAbilityChoices => self
                .wizard
                .as_ref()
                .and_then(|wizard| wizard.background.as_deref())
                .and_then(|name| self.find_background(name))
                .map(|background| {
                    let names = background_ability_indices(background)
                        .iter()
                        .map(|index| ABILITY_NAMES[*index])
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(
                        "\nCaractéristiques disponibles : {names}\nSaisissez FOR+2, SAG+1 ou les trois (FOR,SAG,CHA) pour +1 chacune."
                    )
                })
                .unwrap_or_default(),
            InputKind::EquipmentChoices => {
                let descriptions = self
                    .equipment_package_options()
                    .iter()
                    .map(|(label, options)| {
                        format!("\n{label} : A) {}  |  B) {}", options.0, options.1)
                    })
                    .collect::<String>();
                format!(
                    "{descriptions}\nRépondez par A ou B pour chaque ligne, dans l'ordre, séparées par des virgules."
                )
            }
            _ => String::new(),
        };
        let popup = centered_rect(88, if help.is_empty() { 20 } else { 60 }, area);
        frame.render_widget(Clear, popup);
        let content = format!(
            "{prompt}{help}\n\n> {}\n\nEntrée : valider  |  Échap : annuler",
            self.input,
        );
        frame.render_widget(
            Paragraph::new(content)
                .block(Block::default().borders(Borders::ALL).title("Saisie"))
                .wrap(Wrap { trim: false }),
            popup,
        );
    }

    /// Dessine une liste filtrable et indique les options multisélectionnées.
    fn render_picker(&self, frame: &mut Frame, kind: PickKind, area: Rect) {
        let title = match kind {
            PickKind::Race => "Choisir une espèce",
            PickKind::Class => "Choisir une classe",
            PickKind::Background => "Choisir un background",
            PickKind::Item => "Ajouter un objet",
            PickKind::StartingItem => "Ajuster l'équipement de départ",
            PickKind::Spell => "Ajouter un sort de classe",
            PickKind::CastSpell => "Lancer un sort préparé",
            PickKind::Feat => "Ajouter un don",
        };
        let popup = centered_rect(78, 80, area);
        frame.render_widget(Clear, popup);
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(2),
                Constraint::Length(2),
            ])
            .split(inner);
        frame.render_widget(
            Paragraph::new(format!("Recherche : {}", self.picker_query))
                .block(Block::default().borders(Borders::ALL)),
            inner_chunks[0],
        );
        let options = self.picker_options(kind);
        let start = self.picker_index.saturating_sub(8);
        let visible: Vec<ListItem> = options
            .iter()
            .skip(start)
            .take(16)
            .map(|option| {
                let marker = if self.creation_choice_selected(kind, option) {
                    "[x] "
                } else if self.wizard.is_some()
                    && matches!(kind, PickKind::StartingItem | PickKind::Spell)
                {
                    "[ ] "
                } else {
                    ""
                };
                let description = if kind == PickKind::Background {
                    self.find_background(option)
                        .map(background_picker_summary)
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                ListItem::new(format!("{marker}{option}{description}"))
            })
            .collect();
        let relative_index = self.picker_index - start;
        let mut state = ListState::default();
        if !visible.is_empty() {
            state.select(Some(relative_index.min(visible.len() - 1)));
        }
        frame.render_stateful_widget(
            List::new(visible)
                .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
                .highlight_symbol("› ")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("{} résultat(s)", options.len())),
                ),
            inner_chunks[1],
            &mut state,
        );
        frame.render_widget(
            Paragraph::new(if self.wizard.is_some()
                && matches!(kind, PickKind::StartingItem | PickKind::Spell)
            {
                "↑/↓ choisir | taper pour filtrer | Espace sélectionner | Entrée continuer | Échap annuler"
            } else {
                "↑/↓ choisir | taper pour filtrer | Entrée valider | Échap annuler"
            }),
            inner_chunks[2],
        );
    }

    /// Indique si un objet ou un sort est déjà sélectionné pendant la création.
    fn creation_choice_selected(&self, kind: PickKind, value: &str) -> bool {
        self.wizard.as_ref().is_some_and(|wizard| match kind {
            PickKind::StartingItem => wizard.equipment.iter().any(|item| item == value),
            PickKind::Spell => wizard.spells.iter().any(|spell| spell == value),
            _ => false,
        })
    }

    /// Dessine la confirmation avant suppression d'un personnage.
    fn render_confirm_delete(&self, frame: &mut Frame, area: Rect) {
        let popup = centered_rect(50, 20, area);
        frame.render_widget(Clear, popup);
        let name = self
            .selected_character()
            .map(|character| character.name.as_str())
            .unwrap_or("");
        frame.render_widget(
            Paragraph::new(format!("Supprimer « {name} » ?\n\ny confirmer | n annuler"))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Confirmation")),
            popup,
        );
    }
}

/// Calcule les PV maximums selon la moyenne fixe du dé de vie à chaque niveau.
fn calculate_max_hp(level: u8, constitution: i32, hit_die: i32) -> i32 {
    // PV maximum au niveau 1, puis moyenne fixe du dé de vie à chaque niveau.
    (hit_die + Character::modifier(constitution)).max(1)
        + (1..level)
            .map(|_| (hit_die / 2 + 1 + Character::modifier(constitution)).max(1))
            .sum::<i32>()
}

/// Retourne le niveau correspondant au total d'expérience selon les seuils 2024.
fn level_for_experience(experience: u32) -> u8 {
    XP_LEVEL_THRESHOLDS
        .iter()
        .rposition(|threshold| experience >= *threshold)
        .map_or(1, |index| index as u8 + 1)
}

/// Renvoie le prochain seuil d'XP, ou `None` au niveau maximum.
fn next_level_xp(level: u8) -> Option<u32> {
    XP_LEVEL_THRESHOLDS.get(level as usize).copied()
}

/// Ajoute une entrée au journal et limite sa taille aux 100 événements les plus récents.
fn append_combat_log(character: &mut Character, entry: &str) {
    character.combat_log.push(entry.to_owned());
    if character.combat_log.len() > 100 {
        character.combat_log.remove(0);
    }
}

/// Convertit les noms de conditions courants en libellés français canoniques.
fn normalize_condition(condition: &str) -> String {
    [
        ("aveuglé", "Aveuglé"),
        ("blinded", "Aveuglé"),
        ("charmé", "Charmé"),
        ("charmed", "Charmé"),
        ("assourdi", "Assourdi"),
        ("deafened", "Assourdi"),
        ("effrayé", "Effrayé"),
        ("frightened", "Effrayé"),
        ("fear", "Effrayé"),
        ("agrippé", "Agrippé"),
        ("grappled", "Agrippé"),
        ("neutralisé", "Neutralisé"),
        ("incapacitated", "Neutralisé"),
        ("invisible", "Invisible"),
        ("paralysé", "Paralysé"),
        ("paralyzed", "Paralysé"),
        ("pétrifié", "Pétrifié"),
        ("petrified", "Pétrifié"),
        ("empoisonné", "Empoisonné"),
        ("poisoned", "Empoisonné"),
        ("à terre", "À terre"),
        ("prone", "À terre"),
        ("entravé", "Entravé"),
        ("restrained", "Entravé"),
        ("étourdi", "Étourdi"),
        ("stunned", "Étourdi"),
        ("inconscient", "Inconscient"),
        ("unconscious", "Inconscient"),
        ("épuisement", "Épuisement"),
        ("exhaustion", "Épuisement"),
    ]
    .iter()
    .find(|(known, _)| known.eq_ignore_ascii_case(condition))
    .map(|(_, canonical)| (*canonical).to_owned())
    .unwrap_or_else(|| condition.to_owned())
}

/// Sépare les deux sauvegardes des compétences proposées par la classe.
fn class_proficiencies(class: &Class) -> (Vec<String>, Vec<String>) {
    let proficiencies: Vec<String> = class
        .proficiency
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect();
    // Dans ce format, les deux premières maîtrises sont les sauvegardes;
    // les suivantes indiquent les compétences offertes par la classe.
    let saving_throws = proficiencies
        .iter()
        .take(2)
        .filter_map(|ability| ability_index(ability).map(|index| ABILITY_NAMES[index].to_owned()))
        .collect();
    let skill_options = proficiencies
        .into_iter()
        .skip(2)
        .filter(|skill| {
            SKILLS
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case(skill))
        })
        .collect();
    (saving_throws, skill_options)
}

/// Extrait les maîtrises de compétences valides accordées par un background.
fn background_skill_proficiencies(background: &Background) -> Vec<String> {
    background
        .proficiency
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|proficiency| {
            SKILLS
                .iter()
                .any(|(skill, _)| skill.eq_ignore_ascii_case(proficiency))
        })
        .map(str::to_owned)
        .collect()
}

/// Lit les trois caractéristiques éligibles au bonus du background.
fn background_ability_indices(background: &Background) -> Vec<usize> {
    let Some(feature) = background.traits.iter().find(|feature| {
        feature
            .name
            .to_ascii_lowercase()
            .starts_with("ability scores:")
    }) else {
        return Vec::new();
    };
    let choices: Vec<usize> = feature
        .name
        .split_once(':')
        .map(|(_, abilities)| abilities.split(',').filter_map(ability_index).collect())
        .unwrap_or_default();
    if choices.len() == 3 {
        choices
    } else {
        Vec::new()
    }
}

/// Valide une répartition +2/+1 ou trois bonus +1 parmi les caractéristiques éligibles.
fn parse_background_ability_choices(input: &str, allowed: &[usize]) -> Option<Vec<(usize, i32)>> {
    if allowed.len() != 3 {
        return None;
    }
    let choices: Vec<_> = input
        .split(',')
        .map(str::trim)
        .filter(|choice| !choice.is_empty())
        .collect();
    let mut parsed = Vec::new();
    for choice in choices {
        if let Some((ability, increase)) = choice.split_once('+') {
            let index = ability_index(ability)?;
            let increase = increase.trim().parse::<i32>().ok()?;
            parsed.push((index, increase));
        } else {
            parsed.push((ability_index(choice)?, 1));
        }
    }
    let indices: std::collections::HashSet<_> = parsed.iter().map(|(index, _)| index).collect();
    if indices.len() != parsed.len() || parsed.iter().any(|(index, _)| !allowed.contains(index)) {
        return None;
    }
    let bonuses: Vec<_> = parsed.iter().map(|(_, increase)| *increase).collect();
    let valid_three = parsed.len() == 3 && bonuses.iter().all(|increase| *increase == 1);
    let valid_two = parsed.len() == 2
        && bonuses.iter().filter(|increase| **increase == 2).count() == 1
        && bonuses.iter().filter(|increase| **increase == 1).count() == 1;
    (valid_three || valid_two).then_some(parsed)
}

/// Retourne le nom du don mentionné dans les traits du background.
fn background_feat(background: &Background) -> Option<String> {
    background
        .traits
        .iter()
        .find(|feature| feature.name.to_ascii_lowercase().starts_with("feat:"))
        .and_then(|feature| feature.name.split_once(':').map(|(_, name)| name.trim()))
        .map(str::to_owned)
}

/// Résume les maîtrises, caractéristiques et don dans la liste des backgrounds.
fn background_picker_summary(background: &Background) -> String {
    let skills = background_skill_proficiencies(background).join(", ");
    let feat = background_feat(background).unwrap_or_default();
    let abilities = background_ability_indices(background)
        .iter()
        .map(|index| ABILITY_NAMES[*index])
        .collect::<Vec<_>>()
        .join(", ");
    let mut details = Vec::new();
    if !skills.is_empty() {
        details.push(format!("compétences : {skills}"));
    }
    if !abilities.is_empty() {
        details.push(format!("caractéristiques : {abilities}"));
    }
    if !feat.is_empty() {
        details.push(format!("don : {feat}"));
    }
    if details.is_empty() {
        String::new()
    } else {
        format!(" — {}", details.join(" | "))
    }
}

/// Extrait les textes des options A et B d'une description d'équipement XML.
fn extract_equipment_options(text: &str) -> Option<(String, String)> {
    let a_start = text.find("(A)")? + "(A)".len();
    let b_start = text.find("(B)")?;
    if b_start <= a_start {
        return None;
    }
    let b_end = b_start + "(B)".len();
    let end = text[b_end..]
        .find("Source:")
        .map(|offset| b_end + offset)
        .unwrap_or(text.len());
    let clean = |value: &str| {
        value
            .trim()
            .trim_start_matches([':', ' ', '\t'])
            .trim_end_matches(['.', ';', ' '])
            .to_owned()
    };
    let option_a_raw = clean(&text[a_start..b_start]);
    let option_a_without_or = option_a_raw
        .strip_suffix("or")
        .filter(|prefix| prefix.chars().last().is_some_and(char::is_whitespace))
        .unwrap_or(&option_a_raw);
    let option_a = option_a_without_or.trim().to_owned();
    let option_b = clean(&text[b_end..end]);
    if option_a.is_empty() || option_b.is_empty() {
        None
    } else {
        Some((option_a, option_b))
    }
}

/// Sépare une quantité initiale d'un nom d'objet dans un paquet d'équipement.
fn parse_equipment_quantity(part: &str) -> (usize, &str) {
    let Some((quantity, item_name)) = part.trim().split_once(' ') else {
        return (1, part.trim());
    };
    match quantity.parse::<usize>() {
        Ok(quantity) => (quantity.max(1), item_name.trim()),
        Err(_) => (1, part.trim()),
    }
}

/// Traduit un nom ou une abréviation de caractéristique en index canonique.
fn ability_index(name: &str) -> Option<usize> {
    match name.trim().to_lowercase().as_str() {
        "strength" | "str" | "for" => Some(0),
        "dexterity" | "dex" => Some(1),
        "constitution" | "con" => Some(2),
        "intelligence" | "int" => Some(3),
        "wisdom" | "wis" | "sag" => Some(4),
        "charisma" | "cha" => Some(5),
        _ => None,
    }
}

/// Choisit le style visuel actif ou inactif d'un onglet.
fn tab_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    }
}

/// Calcule un rectangle centré à partir de pourcentages de la zone disponible.
fn centered_rect(width_percent: u16, height_percent: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}

/// Produit un résumé compact des propriétés utiles d'un objet.
fn item_summary(item: &Item) -> String {
    let mut details = Vec::new();
    if let Some(damage) = &item.dmg1 {
        details.push(format!(
            "dégâts {damage} {}",
            item.dmg_type.as_deref().unwrap_or("")
        ));
    }
    if let Some(ac) = &item.ac {
        details.push(format!("CA {ac}"));
    }
    if !item.property.is_empty() {
        details.push(item.property.join(", "));
    }
    if let Some(modifier) = item.modifier.first() {
        let value = modifier.text.as_deref().unwrap_or_default();
        if !value.is_empty() {
            details.push(format!(
                "{}{}",
                modifier
                    .category
                    .as_deref()
                    .map(|category| format!("{category}: "))
                    .unwrap_or_default(),
                value
            ));
        }
    }
    if let Some(description) = item.text.first() {
        let compact = description.replace('\n', " ");
        let preview: String = compact.chars().take(100).collect();
        details.push(preview);
    }
    details.join(" | ")
}

/// Assemble les métadonnées d'incantation présentes pour l'affichage d'un sort.
fn spell_detail(spell: &Spell) -> Option<String> {
    let fields = [
        spell
            .time
            .as_deref()
            .map(|value| format!("Incantation : {value}")),
        spell
            .range
            .as_deref()
            .map(|value| format!("Portée : {value}")),
        spell
            .duration
            .as_deref()
            .map(|value| format!("Durée : {value}")),
        spell
            .components
            .as_deref()
            .map(|value| format!("Composantes : {value}")),
    ];
    let result = fields.into_iter().flatten().collect::<Vec<_>>().join(" | ");
    (!result.is_empty()).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::{App, InputKind};
    use crate::character::{Character, Resource};
    use crate::compendium::{
        Autolevel, Background, Class, Compendium, Counter, Feat, Feature, Item, Spell,
    };

    /// Construit un compendium réduit réutilisable dans les tests de l'interface.
    fn test_compendium() -> Compendium {
        Compendium {
            items: vec![Item {
                name: "Dagger".into(),
                ..Item::default()
            }],
            spells: vec![Spell {
                name: "Magic Missile".into(),
                level: Some("1".into()),
                classes: Some("Wizard".into()),
                ..Spell::default()
            }],
            classes: vec![Class {
                name: "Wizard".into(),
                hd: Some("6".into()),
                proficiency: Some("Intelligence, Wisdom, Arcana, History, Insight".into()),
                num_skills: Some("2".into()),
                spell_ability: Some("Intelligence".into()),
                autolevels: vec![
                    Autolevel {
                        level: "1".into(),
                        counters: vec![Counter {
                            name: "Arcane Recovery".into(),
                            value: "1".into(),
                            reset: "S".into(),
                        }],
                        slots: Some("3,2".into()),
                        ..Autolevel::default()
                    },
                    Autolevel {
                        level: "2".into(),
                        counters: vec![Counter {
                            name: "Arcane Recovery".into(),
                            value: "2".into(),
                            reset: "S".into(),
                        }],
                        slots: Some("3,3".into()),
                        ..Autolevel::default()
                    },
                ],
            }],
            backgrounds: vec![Background {
                name: "Sage".into(),
                proficiency: Some("Insight, Religion".into()),
                traits: vec![
                    Feature {
                        name: "Ability Scores: Intelligence, Wisdom, Charisma".into(),
                        ..Feature::default()
                    },
                    Feature {
                        name: "Feat: Alert".into(),
                        ..Feature::default()
                    },
                    Feature {
                        name: "Starting Equipment".into(),
                        text: vec!["Choose A or B: (A) Dagger and 10 GP; or (B) 20 GP".into()],
                    },
                ],
            }],
            feats: vec![Feat {
                name: "Alert".into(),
                ..Feat::default()
            }],
            ..Compendium::default()
        }
    }

    #[test]
    /// Vérifie la création, les règles de classe et la progression d'une fiche.
    fn character_creation_and_level_up_apply_compendium_hit_die_and_resources() {
        let path =
            std::env::temp_dir().join(format!("dnd-tracker-app-test-{}.json", std::process::id()));
        let mut app = App::new(test_compendium(), path.clone()).unwrap();
        app.start_creation();
        app.input = "Ada".into();
        app.commit_input(InputKind::CharacterName);
        app.choose_option(super::PickKind::Race, "Elf".into());
        app.choose_option(super::PickKind::Class, "Wizard".into());
        app.choose_option(super::PickKind::Background, "Sage".into());
        for index in 0..6 {
            app.input = [15, 14, 13, 12, 10, 8][index].to_string();
            app.commit_input(InputKind::Score(index));
        }
        app.input = "Arcana, History".into();
        app.commit_input(InputKind::SkillChoices);
        app.input = "INT+2,WIS+1".into();
        app.commit_input(InputKind::BackgroundAbilityChoices);
        app.input = "A".into();
        app.commit_input(InputKind::EquipmentChoices);
        assert_eq!(app.wizard.as_ref().unwrap().equipment, ["Dagger"]);
        app.handle_picker_key(
            crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Enter,
                crossterm::event::KeyModifiers::NONE,
            ),
            super::PickKind::StartingItem,
        );
        app.handle_picker_key(
            crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char(' '),
                crossterm::event::KeyModifiers::NONE,
            ),
            super::PickKind::Spell,
        );
        app.handle_picker_key(
            crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Enter,
                crossterm::event::KeyModifiers::NONE,
            ),
            super::PickKind::Spell,
        );

        assert_eq!(app.characters.len(), 1);
        assert_eq!(app.characters[0].max_hp, 7);
        assert_eq!(app.characters[0].resources[0].current, 1);
        assert_eq!(
            app.characters[0].skill_proficiencies,
            ["Arcana", "History", "Insight", "Religion"]
        );
        assert_eq!(app.characters[0].saving_throw_proficiencies, ["INT", "SAG"]);
        assert_eq!(app.characters[0].spell_slots_max[1], 2);
        assert_eq!(app.characters[0].background, "Sage");
        assert_eq!(app.characters[0].intelligence, 14);
        assert_eq!(app.characters[0].wisdom, 11);
        assert_eq!(app.characters[0].feats, ["Alert"]);
        assert_eq!(app.characters[0].equipment[0].item_name, "Dagger");
        assert_eq!(app.characters[0].prepared_spells, ["Magic Missile"]);
        assert_eq!(app.characters[0].starting_equipment_choices.len(), 1);
        app.characters[0].temp_hp = 3;
        app.start_input(InputKind::HitPointAdjustment, "");
        app.input = "-5".into();
        app.commit_input(InputKind::HitPointAdjustment);
        assert_eq!(app.characters[0].temp_hp, 0);
        assert_eq!(app.characters[0].current_hp, 5);
        app.characters[0].resources[0].current = 0;
        app.level_up();

        assert_eq!(app.characters[0].level, 2);
        assert_eq!(app.characters[0].max_hp, 12);
        assert_eq!(app.characters[0].resources[0].maximum, 2);
        assert_eq!(app.characters[0].resources[0].current, 1);
        assert_eq!(app.characters[0].spell_slots_max[1], 3);
        assert_eq!(app.characters[0].current_hit_dice, 2);
        app.characters[0].current_hit_dice = 1;
        app.input = "900".into();
        app.commit_input(InputKind::Experience);
        assert_eq!(app.characters[0].experience, 900);
        assert_eq!(app.characters[0].level, 3);
        assert_eq!(app.characters[0].current_hit_dice, 2);
        app.input = "1".into();
        app.commit_input(InputKind::Level);
        assert_eq!(app.characters[0].level, 1);
        assert_eq!(app.characters[0].current_hit_dice, 0);
        assert_eq!(crate::storage::load(&path).unwrap()[0].name, "Ada");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    /// Vérifie les dépenses de sort, conditions, repos et journal de combat.
    fn combat_actions_conditions_spells_and_rests_are_tracked() {
        let path = std::env::temp_dir().join(format!(
            "dnd-tracker-combat-test-{}.json",
            std::process::id()
        ));
        let mut app = App::new(test_compendium(), path.clone()).unwrap();
        let mut character = Character {
            name: "Combat".into(),
            class: "Wizard".into(),
            max_hp: 12,
            current_hp: 4,
            prepared_spells: vec!["Magic Missile".into()],
            spell_slots_max: [0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
            spell_slots: [0; 10],
            resources: vec![Resource {
                name: "Arcane Recovery".into(),
                maximum: 1,
                current: 0,
                reset: "S".into(),
            }],
            ..Character::default()
        };
        character.conditions.push("Étourdi".into());
        app.characters.push(character);
        app.cast_spell("Magic Missile");
        app.cast_spell("Magic Missile");

        assert_eq!(app.characters[0].spell_slots[1], 1);
        assert!(app.characters[0].combat_log[0].contains("Magic Missile"));
        assert_eq!(app.status, "Aucun emplacement de niveau 1 disponible.");
        app.input = "attaque à l'épée".into();
        app.commit_input(InputKind::CombatAction);
        app.short_rest();
        assert_eq!(app.characters[0].resources[0].current, 1);
        assert_eq!(app.characters[0].spell_slots[1], 1);
        app.long_rest();
        assert_eq!(app.characters[0].current_hp, 12);
        assert_eq!(app.characters[0].spell_slots[1], 0);
        assert_eq!(app.characters[0].conditions, ["Étourdi"]);

        app.input = "fear".into();
        app.commit_input(InputKind::Condition);
        assert_eq!(app.characters[0].conditions[1], "Effrayé");
        app.remove_last_condition();
        assert_eq!(app.characters[0].conditions, ["Étourdi"]);
        assert!(app.characters[0]
            .combat_log
            .iter()
            .any(|entry| entry.contains("attaque à l'épée")));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    /// Vérifie le calcul des PV et les seuils d'expérience du système 2024.
    fn max_hit_points_follow_hit_die_and_constitution_modifier() {
        assert_eq!(super::calculate_max_hp(1, 8, 10), 9);
        assert_eq!(super::calculate_max_hp(3, 14, 8), 24);
        assert_eq!(super::level_for_experience(299), 1);
        assert_eq!(super::level_for_experience(300), 2);
        assert_eq!(super::level_for_experience(355_000), 20);
    }
}
