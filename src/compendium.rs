//! Types correspondant aux éléments XML du format de compendium Fight Club.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Compendium {
    #[serde(rename = "item", default)]
    pub items: Vec<Item>,
    #[serde(rename = "spell", default)]
    pub spells: Vec<Spell>,
    #[serde(rename = "class", default)]
    pub classes: Vec<Class>,
    #[serde(rename = "race", default)]
    pub races: Vec<Race>,
    #[serde(rename = "feat", default)]
    pub feats: Vec<Feat>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Item {
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub weight: Option<String>,
    pub ac: Option<String>,
    pub dmg1: Option<String>,
    pub dmg2: Option<String>,
    #[serde(rename = "dmgType")]
    pub dmg_type: Option<String>,
    // Certaines entrées du XML contiennent plusieurs propriétés du même nom.
    #[serde(default)]
    pub property: Vec<String>,
    #[serde(rename = "text", default)]
    pub text: Vec<String>,
    #[serde(rename = "modifier", default)]
    pub modifier: Vec<Modifier>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Spell {
    pub name: String,
    pub level: Option<String>,
    pub school: Option<String>,
    pub time: Option<String>,
    pub range: Option<String>,
    pub components: Option<String>,
    pub duration: Option<String>,
    pub classes: Option<String>,
    #[serde(rename = "text", default)]
    pub text: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Class {
    pub name: String,
    pub hd: Option<String>,
    pub proficiency: Option<String>,
    #[serde(rename = "spellAbility")]
    pub spell_ability: Option<String>,
    #[serde(rename = "numSkills")]
    pub num_skills: Option<String>,
    #[serde(rename = "autolevel", default)]
    pub autolevels: Vec<Autolevel>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Autolevel {
    #[serde(rename = "@level", default)]
    pub level: String,
    #[serde(rename = "feature", default)]
    pub features: Vec<Feature>,
    #[serde(rename = "counter", default)]
    pub counters: Vec<Counter>,
    pub slots: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Counter {
    pub name: String,
    pub value: String,
    pub reset: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Feature {
    #[serde(default)]
    pub name: String,
    #[serde(rename = "text", default)]
    pub text: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Race {
    pub name: String,
    pub size: Option<String>,
    pub speed: Option<String>,
    pub ability: Option<String>,
    #[serde(rename = "trait", default)]
    pub traits: Vec<Feature>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Feat {
    pub name: String,
    pub prerequisite: Option<String>,
    #[serde(rename = "text", default)]
    pub text: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Modifier {
    #[serde(rename = "@category")]
    pub category: Option<String>,
    #[serde(rename = "$value")]
    pub text: Option<String>,
}

impl Compendium {
    /// Désérialise un compendium XML compilé au format Fight Club.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let compendium: Compendium = quick_xml::de::from_reader(reader)?;
        Ok(compendium)
    }
}

#[cfg(test)]
mod tests {
    use super::{Class, Item};

    #[test]
    fn class_accepts_autolevel_elements_interrupted_by_other_tags() {
        let xml = r#"<class><name>Test</name><autolevel level="1"/><trait/><autolevel level="2"/></class>"#;
        let class: Class = quick_xml::de::from_str(xml).unwrap();

        assert_eq!(class.autolevels.len(), 2);
        assert_eq!(class.autolevels[0].level, "1");
        assert_eq!(class.autolevels[1].level, "2");
    }

    #[test]
    fn item_accepts_multiple_property_elements() {
        let xml = r#"<item><name>Test</name><property>Heavy</property><property>Two-Handed</property></item>"#;
        let item: Item = quick_xml::de::from_str(xml).unwrap();

        assert_eq!(item.property, ["Heavy", "Two-Handed"]);
    }
}
