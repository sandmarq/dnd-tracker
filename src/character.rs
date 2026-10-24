//! Modèles sérialisables des fiches, de l'inventaire et des ressources de jeu.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Character {
    /// Nom affiché dans la liste des personnages.
    pub name: String,
    /// Espèce issue du compendium.
    pub race: String,
    /// Classe principale utilisée pour la progression et les ressources.
    pub class: String,
    /// Background choisi à la création.
    pub background: String,
    /// Sous-classe saisie ou choisie après la création.
    pub subclass: Option<String>,
    /// Niveau courant, compris entre 1 et 20.
    pub level: u8,
    /// Total d'expérience; les seuils 2024 déterminent le niveau automatique.
    pub experience: u32,
    /// Scores de caractéristiques utilisés pour calculer les modificateurs.
    pub strength: i32,
    /// Dextérité actuelle.
    pub dexterity: i32,
    /// Constitution actuelle.
    pub constitution: i32,
    /// Intelligence actuelle.
    pub intelligence: i32,
    /// Sagesse actuelle.
    pub wisdom: i32,
    /// Charisme actuel.
    pub charisma: i32,
    /// Points de vie maximum et courants, ainsi que les points temporaires.
    pub max_hp: i32,
    /// Points de vie courants.
    pub current_hp: i32,
    /// Points de vie temporaires, absorbant les dégâts avant les PV courants.
    pub temp_hp: i32,
    /// Nombre de dés de vie encore disponibles pour les repos courts.
    pub current_hit_dice: u8,
    /// Ressources de classe; les champs simples qui précèdent sont conservés pour compatibilité.
    pub resource_name: String,
    /// Maximum de la ressource historique.
    pub max_resource: u8,
    /// Valeur courante de la ressource historique.
    pub current_resource: u8,
    /// Objets possédés et leur état d'équipement.
    pub equipment: Vec<InventorySlot>,
    /// Récapitulatif des paquets de départ choisis lors de la création.
    pub starting_equipment_choices: Vec<String>,
    /// Sorts suivis dans la fiche et proposés au lancement.
    pub prepared_spells: Vec<String>,
    /// Dons connus, incluant le don du background.
    pub feats: Vec<String>,
    /// Maîtrises de compétences acquises par classe ou background.
    pub skill_proficiencies: Vec<String>,
    /// Maîtrises de jets de sauvegarde accordées par la classe.
    pub saving_throw_proficiencies: Vec<String>,
    /// Ressources de classe avec maximum, usages courants et type de repos.
    pub resources: Vec<Resource>,
    /// Emplacements dépensés par niveau de sort (indices 1 à 9).
    pub spell_slots: [u8; 10],
    /// Capacité maximale d'emplacements par niveau de sort.
    pub spell_slots_max: [u8; 10],
    /// Conditions actives saisies par le joueur; leur durée est gérée manuellement.
    pub conditions: Vec<String>,
    /// Journal limité des sorts, ressources, repos et actions de combat.
    pub combat_log: Vec<String>,
}

/// Une pile d'objets d'inventaire et ses indicateurs d'équipement.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct InventorySlot {
    /// Nom d'objet correspondant au compendium si disponible.
    pub item_name: String,
    /// Nombre d'unités regroupées dans cette entrée.
    pub quantity: u32,
    /// Indique si l'objet contribue aux calculs d'équipement.
    pub equipped: bool,
    /// Indique si l'objet magique est lié au personnage.
    pub attuned: bool,
}

/// Une ressource de classe et ses règles de récupération.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Resource {
    /// Nom affiché dans la fiche.
    pub name: String,
    /// Nombre maximal d'utilisations.
    pub maximum: u8,
    /// Nombre d'utilisations restantes.
    pub current: u8,
    /// Code de repos associé dans le compendium (court, long ou autre).
    pub reset: String,
}

impl Default for Character {
    /// Crée une fiche niveau 1 compatible avec la désérialisation de sauvegardes anciennes.
    fn default() -> Self {
        Self {
            name: String::new(),
            race: String::new(),
            class: String::new(),
            background: String::new(),
            subclass: None,
            level: 1,
            experience: 0,
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
            starting_equipment_choices: Vec::new(),
            prepared_spells: Vec::new(),
            feats: Vec::new(),
            skill_proficiencies: Vec::new(),
            saving_throw_proficiencies: Vec::new(),
            resources: Vec::new(),
            spell_slots: [0; 10],
            spell_slots_max: [0; 10],
            conditions: Vec::new(),
            combat_log: Vec::new(),
        }
    }
}

impl Character {
    /// Calcule le modificateur D&D, en arrondissant les scores impairs vers le bas.
    pub fn modifier(score: i32) -> i32 {
        (score - 10).div_euclid(2)
    }

    /// Retourne les caractéristiques dans l'ordre utilisé par les sélecteurs de l'interface.
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

    /// Remplace un score par son index canonique; ignore les indices hors de 0 à 5.
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

    /// Retourne le bonus de maîtrise standard selon le niveau du personnage.
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
