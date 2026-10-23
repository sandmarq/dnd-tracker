//! Modèles sérialisables des fiches, de l'inventaire et des ressources de jeu.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Character {
    pub name: String,
    pub race: String,
    pub class: String,
    pub subclass: Option<String>,
    pub level: u8,
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
    pub max_hp: i32,
    pub current_hp: i32,
    pub temp_hp: i32,
    pub current_hit_dice: u8,
    pub resource_name: String,
    pub max_resource: u8,
    pub current_resource: u8,
    pub equipment: Vec<InventorySlot>,
    pub prepared_spells: Vec<String>,
    pub feats: Vec<String>,
    pub skill_proficiencies: Vec<String>,
    pub saving_throw_proficiencies: Vec<String>,
    pub resources: Vec<Resource>,
    pub spell_slots: [u8; 10],
    pub spell_slots_max: [u8; 10],
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct InventorySlot {
    pub item_name: String,
    pub quantity: u32,
    pub equipped: bool,
    pub attuned: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Resource {
    pub name: String,
    pub maximum: u8,
    pub current: u8,
    pub reset: String,
}

impl Default for Character {
    fn default() -> Self {
        Self {
            name: String::new(),
            race: String::new(),
            class: String::new(),
            subclass: None,
            level: 1,
            strength: 15,
            dexterity: 14,
            constitution: 13,
            intelligence: 12,
            wisdom: 10,
            charisma: 8,
            max_hp: 1,
            current_hp: 1,
            temp_hp: 0,
            current_hit_dice: 1,
            resource_name: String::new(),
            max_resource: 0,
            current_resource: 0,
            equipment: Vec::new(),
            prepared_spells: Vec::new(),
            feats: Vec::new(),
            skill_proficiencies: Vec::new(),
            saving_throw_proficiencies: Vec::new(),
            resources: Vec::new(),
            spell_slots: [0; 10],
            spell_slots_max: [0; 10],
        }
    }
}

impl Character {
    /// Applique l'arrondi inférieur utilisé pour les modificateurs de caractéristique.
    pub fn modifier(score: i32) -> i32 {
        (score - 10).div_euclid(2)
    }

    /// Retourne les six caractéristiques dans l'ordre canonique utilisé par l'interface.
    pub fn ability_scores(&self) -> [i32; 6] {
        [
            self.strength,
            self.dexterity,
            self.constitution,
            self.intelligence,
            self.wisdom,
            self.charisma,
        ]
    }

    pub fn set_ability_score(&mut self, index: usize, score: i32) {
        match index {
            0 => self.strength = score,
            1 => self.dexterity = score,
            2 => self.constitution = score,
            3 => self.intelligence = score,
            4 => self.wisdom = score,
            5 => self.charisma = score,
            _ => {}
        }
    }

    /// Calcule le bonus de maîtrise standard par tranche de niveaux.
    pub fn proficiency_bonus(&self) -> i32 {
        match self.level {
            1..=4 => 2,
            5..=8 => 3,
            9..=12 => 4,
            13..=16 => 5,
            _ => 6,
        }
    }
}
