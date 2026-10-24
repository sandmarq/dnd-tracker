//! Types correspondant aux éléments XML du format de compendium Fight Club.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
/// Racine des catégories importées depuis un compendium XML Fight Club.
pub struct Compendium {
    #[serde(rename = "item", default)]
    /// Objets disponibles dans le catalogue.
    pub items: Vec<Item>,
    #[serde(rename = "spell", default)]
    /// Sorts disponibles dans le catalogue.
    pub spells: Vec<Spell>,
    #[serde(rename = "class", default)]
    /// Classes et progressions par niveau.
    pub classes: Vec<Class>,
    #[serde(rename = "race", default)]
    /// Espèces jouables et traits associés.
    pub races: Vec<Race>,
    #[serde(rename = "background", default)]
    /// Backgrounds jouables et avantages associés.
    pub backgrounds: Vec<Background>,
    #[serde(rename = "feat", default)]
    /// Dons et prérequis.
    pub feats: Vec<Feat>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
/// Objet d'équipement ou d'inventaire décrit dans le compendium.
pub struct Item {
    /// Nom d'affichage et clé de correspondance de l'objet.
    pub name: String,
    #[serde(rename = "type")]
    /// Catégorie d'objet indiquée dans le XML.
    pub item_type: Option<String>,
    /// Poids tel qu'écrit dans la source.
    pub weight: Option<String>,
    /// Classe d'armure de base éventuelle.
    pub ac: Option<String>,
    /// Dé de dégâts principal éventuel.
    pub dmg1: Option<String>,
    /// Dé de dégâts secondaire éventuel.
    pub dmg2: Option<String>,
    #[serde(rename = "dmgType")]
    /// Type de dégâts, par exemple perforant ou tranchant.
    pub dmg_type: Option<String>,
    // Certaines entrées du XML contiennent plusieurs propriétés du même nom.
    #[serde(default)]
    /// Propriétés répétables de l'objet.
    pub property: Vec<String>,
    #[serde(rename = "text", default)]
    /// Descriptions textuelles de l'objet.
    pub text: Vec<String>,
    #[serde(rename = "modifier", default)]
    /// Modificateurs encodés dans la source.
    pub modifier: Vec<Modifier>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
/// Sort avec métadonnées d'incantation, de niveau et classes associées.
pub struct Spell {
    /// Nom du sort.
    pub name: String,
    /// Niveau du sort ou mention de sort mineur.
    pub level: Option<String>,
    /// École de magie.
    pub school: Option<String>,
    /// Temps d'incantation.
    pub time: Option<String>,
    /// Portée.
    pub range: Option<String>,
    /// Composantes nécessaires.
    pub components: Option<String>,
    /// Durée.
    pub duration: Option<String>,
    /// Classes autorisées dans la chaîne du compendium.
    pub classes: Option<String>,
    #[serde(rename = "text", default)]
    /// Description et effets tels que fournis par le XML.
    pub text: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
/// Classe et progression par niveau telles que fournies par le XML.
pub struct Class {
    /// Nom de la classe.
    pub name: String,
    /// Taille du dé de vie.
    pub hd: Option<String>,
    /// Maîtrises de sauvegarde et compétences fournies dans le XML.
    pub proficiency: Option<String>,
    #[serde(rename = "spellAbility")]
    /// Caractéristique d'incantation si applicable.
    pub spell_ability: Option<String>,
    #[serde(rename = "numSkills")]
    /// Nombre de compétences à choisir à la création.
    pub num_skills: Option<String>,
    #[serde(rename = "autolevel", default)]
    /// Capacités, ressources et emplacements associés à chaque niveau.
    pub autolevels: Vec<Autolevel>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
/// Ensemble des capacités, ressources et emplacements d'un niveau de classe.
pub struct Autolevel {
    #[serde(rename = "@level", default)]
    /// Niveau de classe auquel s'appliquent ces éléments.
    pub level: String,
    #[serde(rename = "feature", default)]
    /// Capacités débloquées à ce niveau.
    pub features: Vec<Feature>,
    #[serde(rename = "counter", default)]
    /// Ressources et règles de récupération associées.
    pub counters: Vec<Counter>,
    /// Emplacements par niveau de sort, dans le format textuel source.
    pub slots: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
/// Ressource de classe récupérée selon une règle de repos.
pub struct Counter {
    /// Nom de la ressource.
    pub name: String,
    /// Maximum d'utilisations décrit par la source.
    pub value: String,
    /// Code du type de repos qui restaure la ressource.
    pub reset: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
/// Capacité ou trait textuel fourni par le compendium.
pub struct Feature {
    #[serde(default)]
    /// Titre de la capacité.
    pub name: String,
    #[serde(rename = "text", default)]
    /// Description de la capacité.
    pub text: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
/// Espèce jouable et traits raciaux associés.
pub struct Race {
    /// Nom de l'espèce.
    pub name: String,
    /// Taille des créatures de l'espèce.
    pub size: Option<String>,
    /// Vitesse de déplacement.
    pub speed: Option<String>,
    /// Modificateurs de caractéristiques encodés dans le XML.
    pub ability: Option<String>,
    #[serde(rename = "trait", default)]
    /// Traits raciaux.
    pub traits: Vec<Feature>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
/// Background jouable et traits associés, y compris les maîtrises et équipements.
pub struct Background {
    /// Nom du background.
    pub name: String,
    /// Compétences accordées sous forme de liste textuelle.
    pub proficiency: Option<String>,
    #[serde(rename = "trait", default)]
    /// Traits de background, incluant le don et l'équipement de départ.
    pub traits: Vec<Feature>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
/// Don et éventuelles conditions préalables.
pub struct Feat {
    /// Nom du don.
    pub name: String,
    /// Prérequis éventuel.
    pub prerequisite: Option<String>,
    #[serde(rename = "text", default)]
    /// Description des bénéfices.
    pub text: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
/// Modificateur textuel attaché à un objet.
pub struct Modifier {
    #[serde(rename = "@category")]
    /// Catégorie de modificateur XML.
    pub category: Option<String>,
    #[serde(rename = "$value")]
    /// Valeur textuelle du modificateur.
    pub text: Option<String>,
}

impl Compendium {
    /// Désérialise un fichier XML compilé au format Fight Club.
    ///
    /// Les erreurs de lecture et de désérialisation sont propagées à l'appelant.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let compendium: Compendium = quick_xml::de::from_reader(reader)?;
        Ok(compendium)
    }
}

#[cfg(test)]
mod tests {
    use super::{Background, Class, Item};

    #[test]
    /// Vérifie que les niveaux de classe restent lisibles malgré des balises intercalées.
    fn class_accepts_autolevel_elements_interrupted_by_other_tags() {
        let xml = r#"<class><name>Test</name><autolevel level="1"/><trait/><autolevel level="2"/></class>"#;
        let class: Class = quick_xml::de::from_str(xml).unwrap();

        assert_eq!(class.autolevels.len(), 2);
        assert_eq!(class.autolevels[0].level, "1");
        assert_eq!(class.autolevels[1].level, "2");
    }

    #[test]
    /// Vérifie la désérialisation de propriétés d'objet répétées.
    fn item_accepts_multiple_property_elements() {
        let xml = r#"<item><name>Test</name><property>Heavy</property><property>Two-Handed</property></item>"#;
        let item: Item = quick_xml::de::from_str(xml).unwrap();

        assert_eq!(item.property, ["Heavy", "Two-Handed"]);
    }

    #[test]
    /// Vérifie la désérialisation des maîtrises et traits d'un background.
    fn background_accepts_proficiencies_and_traits() {
        let xml = r#"<background><name>Sage</name><proficiency>Insight, Religion</proficiency><trait><name>Feat: Alert</name><text>Benefit</text></trait></background>"#;
        let background: Background = quick_xml::de::from_str(xml).unwrap();

        assert_eq!(background.name, "Sage");
        assert_eq!(background.proficiency.as_deref(), Some("Insight, Religion"));
        assert_eq!(background.traits[0].name, "Feat: Alert");
    }
}
