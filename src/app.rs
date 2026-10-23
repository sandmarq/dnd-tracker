//! État, règles de fiche et vues interactives du gestionnaire de personnages.

use crate::character::{Character, InventorySlot, Resource};
use crate::compendium::{Class, Compendium, Item, Spell};
use crate::storage;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rand::Rng;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use std::path::PathBuf;

const ABILITY_NAMES: [&str; 6] = ["FOR", "DEX", "CON", "INT", "SAG", "CHA"];
const STANDARD_ARRAY: [i32; 6] = [15, 14, 13, 12, 10, 8];
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
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Sheet,
    Equipment,
    Spells,
    Progression,
}

impl Tab {
    fn next(self) -> Self {
        match self {
            Self::Sheet => Self::Equipment,
            Self::Equipment => Self::Spells,
            Self::Spells => Self::Progression,
            Self::Progression => Self::Sheet,
        }
    }
}

#[derive(Clone, Copy)]
enum PickKind {
    Race,
    Class,
    Item,
    Spell,
    Feat,
}

#[derive(Clone, Copy)]
enum InputKind {
    CharacterName,
    Score(usize),
    EditScore(usize),
    HitPointAdjustment,
    TemporaryHitPoints,
    Subclass,
    SkillChoices,
}

#[derive(Clone, Copy)]
enum Mode {
    Normal,
    Input(InputKind),
    Picker(PickKind),
    ConfirmDelete,
}

struct CreateWizard {
    name: String,
    race: Option<String>,
    class: Option<String>,
    scores: [i32; 6],
    skills: Vec<String>,
}

pub struct App {
    pub compendium: Compendium,
    characters: Vec<Character>,
    selected: usize,
    detail_index: usize,
    detail_scroll: u16,
    spell_slot_level: usize,
    resource_index: usize,
    tab: Tab,
    mode: Mode,
    running: bool,
    wizard: Option<CreateWizard>,
    input: String,
    picker_query: String,
    picker_index: usize,
    status: String,
    storage_path: PathBuf,
}

impl App {
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

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

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

    fn start_creation(&mut self) {
        self.wizard = Some(CreateWizard {
            name: String::new(),
            race: None,
            class: None,
            scores: STANDARD_ARRAY,
            skills: Vec::new(),
        });
        self.start_input(InputKind::CharacterName, "Nom du personnage");
    }

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

    fn start_input(&mut self, kind: InputKind, prompt: &str) {
        self.input.clear();
        self.status = prompt.to_owned();
        self.mode = Mode::Input(kind);
    }

    fn open_picker(&mut self, kind: PickKind) {
        self.picker_query.clear();
        self.picker_index = 0;
        self.mode = Mode::Picker(kind);
    }

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
                        self.finish_creation();
                    }
                }
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
                let (_, allowed) = class_proficiencies(class);
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
                self.finish_creation();
            }
        }
    }

    fn handle_picker_key(&mut self, key: KeyEvent, kind: PickKind) {
        match key.code {
            KeyCode::Enter => {
                let options = self.picker_options(kind);
                if let Some(value) = options.get(self.picker_index).cloned() {
                    self.choose_option(kind, value);
                }
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
            PickKind::Item => self
                .compendium
                .items
                .iter()
                .map(|entry| entry.name.clone())
                .collect(),
            PickKind::Spell => {
                let class = self
                    .selected_character()
                    .map(|entry| entry.class.as_str())
                    .or_else(|| {
                        self.wizard
                            .as_ref()
                            .and_then(|wizard| wizard.class.as_deref())
                    });
                self.compendium
                    .spells
                    .iter()
                    .filter(|spell| {
                        class.is_none_or(|class_name| {
                            spell
                                .classes
                                .as_deref()
                                .is_some_and(|classes| classes.contains(class_name))
                        })
                    })
                    .map(|spell| spell.name.clone())
                    .collect()
            }
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

    fn finish_creation(&mut self) {
        let Some(wizard) = self.wizard.take() else {
            self.mode = Mode::Normal;
            return;
        };
        let mut character = Character {
            name: wizard.name,
            race: wizard.race.unwrap_or_default(),
            class: wizard.class.unwrap_or_default(),
            strength: wizard.scores[0],
            dexterity: wizard.scores[1],
            constitution: wizard.scores[2],
            intelligence: wizard.scores[3],
            wisdom: wizard.scores[4],
            charisma: wizard.scores[5],
            skill_proficiencies: wizard.skills,
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
    }

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

    fn level_up(&mut self) {
        let Some(index) = self.selected_character_index() else {
            return;
        };
        if self.characters[index].level >= 20 {
            self.status = "Le personnage est déjà niveau 20.".into();
            return;
        }
        self.characters[index].level += 1;
        let die = self.class_hit_die(&self.characters[index].class);
        let gain = (die / 2 + 1 + Character::modifier(self.characters[index].constitution)).max(1);
        self.characters[index].max_hp += gain;
        self.characters[index].current_hp += gain;
        self.characters[index].current_hit_dice = self.characters[index].level;
        if let Some(class) = self.find_class(&self.characters[index].class).cloned() {
            Self::update_progression(&mut self.characters[index], &class, false);
        }
        self.persist("Niveau gagné; PV et ressources de classe actualisés.");
    }

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
            Self::sync_legacy_resource(character);
            self.persist("Ressource dépensée.");
        } else if character.current_resource > 0 {
            character.current_resource -= 1;
            self.persist("Ressource dépensée.");
        } else {
            self.status = "Aucune ressource disponible à dépenser.".into();
        }
    }

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
        self.persist(&format!(
            "Dé de vie dépensé : {roll} + CON, {healing} PV récupérés."
        ));
    }

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
            self.persist("Repos long : PV et ressources restaurés.");
        }
    }

    fn short_rest(&mut self) {
        if let Some(character) = self.selected_character_mut() {
            for resource in &mut character.resources {
                if resource.reset.eq_ignore_ascii_case("S") {
                    resource.current = resource.maximum;
                }
            }
            Self::sync_legacy_resource(character);
            self.persist("Repos court : ressources récupérables restaurées. Touche g pour dépenser un dé de vie.");
        }
    }

    fn adjust_spell_slot(&mut self, amount: i8) {
        let level = self.spell_slot_level.clamp(1, 9);
        if let Some(character) = self.selected_character_mut() {
            let slot = &mut character.spell_slots[level];
            *slot = if amount > 0 {
                slot.saturating_add(1).min(character.spell_slots_max[level])
            } else {
                slot.saturating_sub(1)
            };
            self.persist("Emplacements de sort mis à jour.");
        }
    }

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

    fn sync_legacy_resource(character: &mut Character) {
        if let Some(resource) = character.resources.first() {
            character.resource_name = resource.name.clone();
            character.max_resource = resource.maximum;
            character.current_resource = resource.current;
        }
    }

    fn selected_character(&self) -> Option<&Character> {
        self.characters.get(self.selected)
    }

    fn selected_character_mut(&mut self) -> Option<&mut Character> {
        self.characters.get_mut(self.selected)
    }

    fn selected_character_index(&self) -> Option<usize> {
        (self.selected < self.characters.len()).then_some(self.selected)
    }

    fn find_class(&self, name: &str) -> Option<&Class> {
        self.compendium
            .classes
            .iter()
            .find(|class| class.name == name)
    }

    fn class_hit_die(&self, name: &str) -> i32 {
        self.find_class(name)
            .and_then(|class| class.hd.as_deref())
            .and_then(|hit_die| hit_die.parse().ok())
            .unwrap_or(8)
    }

    fn persist(&mut self, success: &str) {
        match storage::save(&self.storage_path, &self.characters) {
            Ok(()) => self.status = success.to_owned(),
            Err(error) => self.status = format!("Erreur de sauvegarde : {error}"),
        }
    }

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
                    "Races : {}  |  Sorts : {}  |  Objets : {}",
                    self.compendium.races.len(),
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
            Line::from(Span::styled(format!("{} — {}", character.name, character.race), Style::default().add_modifier(Modifier::BOLD))),
            Line::from(format!("{}{}  |  Niveau {}  |  Vitesse {} pi",
                character.class,
                character.subclass.as_deref().map(|s| format!(" ({s})")).unwrap_or_default(),
                character.level, speed)),
            Line::from(""),
            Line::from(format!("PV {}/{}  (+{} temp.)     CA {}     Initiative {:+}     Maîtrise +{}",
                character.current_hp, character.max_hp, character.temp_hp, armor_class,
                Character::modifier(character.dexterity), character.proficiency_bonus())),
            Line::from(format!("Dés de vie {}/{}     Perception passive {}",
                character.current_hit_dice, character.level,
                10 + Character::modifier(character.wisdom)
                    + if character.skill_proficiencies.iter().any(|skill| skill.eq_ignore_ascii_case("Perception")) {
                        character.proficiency_bonus()
                    } else { 0 })),
            Line::from(""),
            Line::from("Caractéristiques (touche 1–6 pour modifier) :"),
            Line::from(ABILITY_NAMES.iter().enumerate().map(|(index, name)| {
                format!("{name} {:>2} ({:+})", ability[index], Character::modifier(ability[index]))
            }).collect::<Vec<_>>().join("    ")),
            Line::from(""),
            Line::from(if character.resources.is_empty() {
                format!("Ressources : {} {}/{}", character.resource_name, character.current_resource, character.max_resource)
            } else {
                format!("Ressources ([ / ] choisir, u dépenser) : {}", character.resources.iter().enumerate().map(|(index, resource)|
                    format!("{}{} {}/{}", if index == self.resource_index { "›" } else { "" },
                        resource.name, resource.current, resource.maximum)).collect::<Vec<_>>().join("  |  "))
            }),
            Line::from(""),
            Line::from("Commandes : n créer | h dégâts/soins | t PV temp. | u dépenser | r repos long | v repos court | g lancer un dé de vie | + niveau"),
        ];
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
        lines
    }

    fn equipment_lines(&self, character: &Character) -> Vec<Line<'static>> {
        let mut lines = vec![Line::from(
            "i ajouter depuis le compendium | espace équiper/déséquiper | x retirer",
        )];
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

    fn progression_lines(&self, character: &Character) -> Vec<Line<'static>> {
        let mut lines = vec![
            Line::from(
                "+ niveau | b sous-classe | f ajouter un don | x retirer le don sélectionné",
            ),
            Line::from(format!("Niveau {} / 20", character.level)),
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
            InputKind::Subclass => "Sous-classe (vide pour effacer)",
            InputKind::SkillChoices => "Maîtrises de compétences (séparées par des virgules)",
        };
        let skill_help = if matches!(kind, InputKind::SkillChoices) {
            self.wizard
                .as_ref()
                .and_then(|wizard| wizard.class.as_deref())
                .and_then(|class_name| self.find_class(class_name))
                .map(|class| {
                    let required = class.num_skills.as_deref().unwrap_or("0");
                    let (_, options) = class_proficiencies(class);
                    format!("\nChoisissez {required} : {}", options.join(", "))
                })
                .unwrap_or_default()
        } else {
            String::new()
        };
        let popup = centered_rect(75, if skill_help.is_empty() { 20 } else { 35 }, area);
        frame.render_widget(Clear, popup);
        let content = format!(
            "{prompt}{skill_help}\n\n> {}\n\nEntrée : valider  |  Échap : annuler",
            self.input
        );
        frame.render_widget(
            Paragraph::new(content)
                .block(Block::default().borders(Borders::ALL).title("Saisie"))
                .wrap(Wrap { trim: false }),
            popup,
        );
    }

    fn render_picker(&self, frame: &mut Frame, kind: PickKind, area: Rect) {
        let title = match kind {
            PickKind::Race => "Choisir une espèce",
            PickKind::Class => "Choisir une classe",
            PickKind::Item => "Ajouter un objet",
            PickKind::Spell => "Ajouter un sort de classe",
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
            .map(|option| ListItem::new(option.as_str()))
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
            Paragraph::new("↑/↓ choisir | taper pour filtrer | Entrée valider | Échap annuler"),
            inner_chunks[2],
        );
    }

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

fn calculate_max_hp(level: u8, constitution: i32, hit_die: i32) -> i32 {
    // PV maximum au niveau 1, puis moyenne fixe du dé de vie à chaque niveau.
    (hit_die + Character::modifier(constitution)).max(1)
        + (1..level)
            .map(|_| (hit_die / 2 + 1 + Character::modifier(constitution)).max(1))
            .sum::<i32>()
}

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

fn tab_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    }
}

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
    use crate::compendium::{Autolevel, Class, Compendium, Counter};

    fn test_compendium() -> Compendium {
        Compendium {
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
            ..Compendium::default()
        }
    }

    #[test]
    fn character_creation_and_level_up_apply_compendium_hit_die_and_resources() {
        let path =
            std::env::temp_dir().join(format!("dnd-tracker-app-test-{}.json", std::process::id()));
        let mut app = App::new(test_compendium(), path.clone()).unwrap();
        app.start_creation();
        app.input = "Ada".into();
        app.commit_input(InputKind::CharacterName);
        app.choose_option(super::PickKind::Race, "Elf".into());
        app.choose_option(super::PickKind::Class, "Wizard".into());
        for index in 0..6 {
            app.input = [15, 14, 13, 12, 10, 8][index].to_string();
            app.commit_input(InputKind::Score(index));
        }
        app.input = "Arcana, History".into();
        app.commit_input(InputKind::SkillChoices);

        assert_eq!(app.characters.len(), 1);
        assert_eq!(app.characters[0].max_hp, 7);
        assert_eq!(app.characters[0].resources[0].current, 1);
        assert_eq!(app.characters[0].skill_proficiencies, ["Arcana", "History"]);
        assert_eq!(app.characters[0].saving_throw_proficiencies, ["INT", "SAG"]);
        assert_eq!(app.characters[0].spell_slots_max[1], 2);
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
        assert_eq!(crate::storage::load(&path).unwrap()[0].name, "Ada");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn max_hit_points_follow_hit_die_and_constitution_modifier() {
        assert_eq!(super::calculate_max_hp(1, 8, 10), 9);
        assert_eq!(super::calculate_max_hp(3, 14, 8), 24);
    }
}
