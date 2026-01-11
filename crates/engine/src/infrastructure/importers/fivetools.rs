//! 5etools data importer.
//!
//! Imports spell, feat, and class feature data from 5etools JSON files
//! and converts them to our domain types.

use super::fivetools_types::*;
use std::path::PathBuf;
use thiserror::Error;
use tokio::fs;
use wrldbldr_domain::{
    CastingTime, CastingTimeUnit, DurationUnit, Feat, FeatBenefit, MaterialComponent, Prerequisite,
    Spell, SpellComponents, SpellDuration, SpellLevel, SpellRange,
};

/// Errors that can occur during import.
#[derive(Debug, Error)]
pub enum ImportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Index file not found at {0}")]
    IndexNotFound(PathBuf),
    #[error("Data file not found: {0}")]
    DataFileNotFound(PathBuf),
}

/// Importer for 5etools data.
pub struct FiveToolsImporter {
    data_path: PathBuf,
}

impl FiveToolsImporter {
    /// Create a new importer pointing to the 5etools data directory.
    ///
    /// The path should point to the root of the extracted 5etools folder,
    /// e.g., `/path/to/5etools-v2.22.0`.
    pub fn new(data_path: impl Into<PathBuf>) -> Self {
        Self {
            data_path: data_path.into(),
        }
    }

    /// Import all spells from 5etools data.
    pub async fn import_spells(&self) -> Result<Vec<Spell>, ImportError> {
        let spells_dir = self.data_path.join("data/spells");
        let index_path = spells_dir.join("index.json");

        if !index_path.exists() {
            return Err(ImportError::IndexNotFound(index_path));
        }

        let index_content = fs::read_to_string(&index_path).await?;
        let index: FiveToolsIndex = serde_json::from_str(&index_content)?;

        let mut all_spells = Vec::new();

        for (_source, filename) in index {
            let file_path = spells_dir.join(&filename);
            if !file_path.exists() {
                continue; // Skip missing files
            }

            let content = fs::read_to_string(&file_path).await?;
            let spell_file: FiveToolsSpellFile = serde_json::from_str(&content)?;

            for raw_spell in spell_file.spell {
                if let Some(spell) = self.convert_spell(raw_spell) {
                    all_spells.push(spell);
                }
            }
        }

        Ok(all_spells)
    }

    /// Import spells from a single source file.
    pub async fn import_spells_from_file(&self, filename: &str) -> Result<Vec<Spell>, ImportError> {
        let file_path = self.data_path.join("data/spells").join(filename);

        if !file_path.exists() {
            return Err(ImportError::DataFileNotFound(file_path));
        }

        let content = fs::read_to_string(&file_path).await?;
        let spell_file: FiveToolsSpellFile = serde_json::from_str(&content)?;

        let spells = spell_file
            .spell
            .into_iter()
            .filter_map(|raw| self.convert_spell(raw))
            .collect();

        Ok(spells)
    }

    /// Import all feats from 5etools data.
    pub async fn import_feats(&self) -> Result<Vec<Feat>, ImportError> {
        let feats_path = self.data_path.join("data/feats.json");

        if !feats_path.exists() {
            return Err(ImportError::DataFileNotFound(feats_path));
        }

        let content = fs::read_to_string(&feats_path).await?;
        let feat_file: FiveToolsFeatFile = serde_json::from_str(&content)?;

        let feats = feat_file
            .feat
            .into_iter()
            .filter_map(|raw| self.convert_feat(raw))
            .collect();

        Ok(feats)
    }

    /// Check if 5etools data exists at the configured path.
    pub async fn validate_path(&self) -> bool {
        let data_dir = self.data_path.join("data");
        data_dir.exists()
    }

    /// Get the list of available spell source files.
    pub async fn list_spell_sources(&self) -> Result<Vec<String>, ImportError> {
        let index_path = self.data_path.join("data/spells/index.json");

        if !index_path.exists() {
            return Err(ImportError::IndexNotFound(index_path));
        }

        let content = fs::read_to_string(&index_path).await?;
        let index: FiveToolsIndex = serde_json::from_str(&content)?;

        Ok(index.into_keys().collect())
    }

    // === Conversion Methods ===

    fn convert_spell(&self, raw: FiveToolsSpell) -> Option<Spell> {
        let id = format!(
            "5e_{}_{}",
            raw.source.to_lowercase(),
            raw.name.to_lowercase().replace(' ', "_").replace('\'', "")
        );

        let level = if raw.level == 0 {
            SpellLevel::Cantrip
        } else {
            SpellLevel::Level(raw.level)
        };

        let school = Some(self.convert_school(&raw.school));
        let casting_time = self.convert_casting_time(&raw.time);
        let range = self.convert_range(&raw.range);
        let components = self.convert_components(&raw.components);
        let duration = self.convert_duration(&raw.duration);
        let description = self.entries_to_string(&raw.entries);
        let higher_levels = raw
            .entries_higher_level
            .map(|e| self.entries_to_string(&e));

        let classes = raw
            .classes
            .map(|c| {
                c.from_class_list
                    .unwrap_or_default()
                    .into_iter()
                    .map(|ce| ce.name.to_lowercase())
                    .collect()
            })
            .unwrap_or_default();

        let source = format!(
            "{} p.{}",
            raw.source,
            raw.page.map(|p| p.to_string()).unwrap_or_default()
        );

        let concentration = raw
            .duration
            .first()
            .map(|d| d.concentration)
            .unwrap_or(false);
        let ritual = raw.meta.map(|m| m.ritual).unwrap_or(false);

        let mut tags = raw.misc_tags.unwrap_or_default();
        if let Some(damage) = raw.damage_inflict {
            tags.extend(damage);
        }
        if let Some(conditions) = raw.condition_inflict {
            tags.extend(conditions);
        }

        Some(Spell {
            id,
            system_id: "dnd5e".to_string(),
            name: raw.name,
            level,
            school,
            casting_time,
            range,
            components,
            duration,
            description,
            higher_levels,
            classes,
            source,
            tags,
            ritual,
            concentration,
        })
    }

    fn convert_school(&self, code: &str) -> String {
        match code.to_uppercase().as_str() {
            "A" => "Abjuration",
            "C" => "Conjuration",
            "D" => "Divination",
            "E" => "Enchantment",
            "V" => "Evocation",
            "I" => "Illusion",
            "N" => "Necromancy",
            "T" => "Transmutation",
            "P" => "Psionic",
            _ => code,
        }
        .to_string()
    }

    fn convert_casting_time(&self, times: &[FiveToolsTime]) -> CastingTime {
        let time = times.first();

        match time {
            Some(t) => {
                let unit = match t.unit.to_lowercase().as_str() {
                    "action" => CastingTimeUnit::Action,
                    "bonus" => CastingTimeUnit::BonusAction,
                    "reaction" => CastingTimeUnit::Reaction,
                    "minute" => CastingTimeUnit::Minute,
                    "hour" => CastingTimeUnit::Hour,
                    _ => CastingTimeUnit::Special,
                };

                CastingTime {
                    amount: t.number.unwrap_or(1),
                    unit,
                    condition: t.condition.clone(),
                }
            }
            None => CastingTime::action(),
        }
    }

    fn convert_range(&self, range: &Option<FiveToolsRange>) -> SpellRange {
        match range {
            Some(r) => match r.range_type.to_lowercase().as_str() {
                "point" => {
                    if let Some(dist) = &r.distance {
                        match dist.distance_type.to_lowercase().as_str() {
                            "self" => SpellRange::SelfOnly { area: None },
                            "touch" => SpellRange::Touch,
                            "feet" => SpellRange::Feet {
                                distance: dist.amount.unwrap_or(0),
                            },
                            "miles" => SpellRange::Miles {
                                distance: dist.amount.unwrap_or(0),
                            },
                            "sight" => SpellRange::Sight,
                            "unlimited" => SpellRange::Unlimited,
                            _ => SpellRange::Special {
                                description: format!("{:?}", dist),
                            },
                        }
                    } else {
                        SpellRange::Touch
                    }
                }
                "radius" | "sphere" | "cone" | "line" | "cube" | "hemisphere" => {
                    let area = r.distance.as_ref().map(|d| {
                        format!(
                            "{}-foot {}",
                            d.amount.unwrap_or(0),
                            r.range_type.to_lowercase()
                        )
                    });
                    SpellRange::SelfOnly { area }
                }
                "special" => SpellRange::Special {
                    description: "See spell description".to_string(),
                },
                _ => SpellRange::Touch,
            },
            None => SpellRange::Touch,
        }
    }

    fn convert_components(&self, components: &Option<FiveToolsComponents>) -> SpellComponents {
        match components {
            Some(c) => {
                let material = c.m.as_ref().map(|m| match m {
                    FiveToolsMaterial::Simple(s) => MaterialComponent {
                        description: s.clone(),
                        consumed: false,
                        cost: None,
                    },
                    FiveToolsMaterial::Detailed(d) => {
                        let consumed = match &d.consume {
                            Some(FiveToolsConsume::Bool(b)) => *b,
                            Some(FiveToolsConsume::String(_)) => true,
                            None => false,
                        };
                        MaterialComponent {
                            description: d.text.clone(),
                            consumed,
                            cost: d.cost,
                        }
                    }
                });

                SpellComponents {
                    verbal: c.v,
                    somatic: c.s,
                    material,
                }
            }
            None => SpellComponents::default(),
        }
    }

    fn convert_duration(&self, durations: &[FiveToolsDuration]) -> SpellDuration {
        let dur = durations.first();

        match dur {
            Some(d) => match d.duration_type.to_lowercase().as_str() {
                "instant" => SpellDuration::Instantaneous,
                "timed" => {
                    if let Some(amount) = &d.duration {
                        let unit = match amount.duration_type.to_lowercase().as_str() {
                            "round" => DurationUnit::Round,
                            "minute" => DurationUnit::Minute,
                            "hour" => DurationUnit::Hour,
                            "day" => DurationUnit::Day,
                            _ => DurationUnit::Minute,
                        };

                        SpellDuration::Timed {
                            amount: amount.amount.unwrap_or(1),
                            unit,
                            concentration: d.concentration,
                        }
                    } else {
                        SpellDuration::Instantaneous
                    }
                }
                "permanent" => SpellDuration::UntilDispelled {
                    trigger: d.ends.as_ref().map(|e| e.join(", ")),
                },
                "special" => SpellDuration::Special {
                    description: "See spell description".to_string(),
                },
                _ => SpellDuration::Instantaneous,
            },
            None => SpellDuration::Instantaneous,
        }
    }

    fn convert_feat(&self, raw: FiveToolsFeat) -> Option<Feat> {
        let id = format!(
            "5e_{}_{}",
            raw.source.to_lowercase(),
            raw.name.to_lowercase().replace(' ', "_").replace('\'', "")
        );

        let description = self.entries_to_string(&raw.entries);

        let prerequisites = raw
            .prerequisite
            .map(|prereqs| {
                prereqs
                    .into_iter()
                    .flat_map(|p| self.convert_prerequisites(p))
                    .collect()
            })
            .unwrap_or_default();

        let benefits = raw
            .ability
            .map(|abilities| {
                abilities
                    .into_iter()
                    .flat_map(|a| self.convert_ability_bonus(a))
                    .collect()
            })
            .unwrap_or_default();

        let source = format!(
            "{} p.{}",
            raw.source,
            raw.page.map(|p| p.to_string()).unwrap_or_default()
        );

        Some(Feat {
            id,
            system_id: "dnd5e".to_string(),
            name: raw.name,
            description,
            prerequisites,
            benefits,
            source,
            category: raw.category,
            repeatable: false,
            tags: Vec::new(),
        })
    }

    fn convert_prerequisites(&self, prereq: FiveToolsPrerequisite) -> Vec<Prerequisite> {
        let mut result = Vec::new();

        // Level prerequisite
        if let Some(level) = prereq.level {
            match level {
                FiveToolsLevelPrereq::Simple(l) => {
                    result.push(Prerequisite::MinLevel { level: l });
                }
                FiveToolsLevelPrereq::ClassLevel { class, level } => {
                    result.push(Prerequisite::HasClass {
                        class_id: class.name.to_lowercase(),
                        class_name: Some(class.name),
                        min_level: Some(level),
                    });
                }
            }
        }

        // Race prerequisite
        if let Some(races) = prereq.race {
            for race in races {
                result.push(Prerequisite::Race { race: race.name });
            }
        }

        // Ability prerequisite
        if let Some(abilities) = prereq.ability {
            for ability_map in abilities {
                for (stat, value) in ability_map {
                    result.push(Prerequisite::MinStat { stat, value });
                }
            }
        }

        // Spellcasting prerequisite
        if prereq.spellcasting.unwrap_or(false) || prereq.spellcasting2020.unwrap_or(false) {
            result.push(Prerequisite::Spellcaster {
                min_spell_level: None,
            });
        }

        // Other/custom prerequisite
        if let Some(other) = prereq.other {
            result.push(Prerequisite::Custom { description: other });
        }

        result
    }

    fn convert_ability_bonus(&self, bonus: FiveToolsAbilityBonus) -> Vec<FeatBenefit> {
        let mut result = Vec::new();

        // Fixed bonuses
        for (stat, value) in bonus.bonuses {
            if stat != "choose" {
                result.push(FeatBenefit::StatIncrease {
                    stat: stat.to_uppercase(),
                    value,
                });
            }
        }

        // Choice bonuses
        if let Some(choice) = bonus.choose {
            let options: Vec<String> = choice.from.into_iter().map(|s| s.to_uppercase()).collect();
            let count = choice.count.unwrap_or(1);
            let value = choice.amount.unwrap_or(1);

            result.push(FeatBenefit::StatChoice {
                options,
                value,
                count,
            });
        }

        result
    }

    fn entries_to_string(&self, entries: &[serde_json::Value]) -> String {
        entries
            .iter()
            .filter_map(|e| self.entry_to_string(e))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn entry_to_string(&self, entry: &serde_json::Value) -> Option<String> {
        match entry {
            serde_json::Value::String(s) => Some(self.clean_formatting(s)),
            serde_json::Value::Object(obj) => {
                if let Some(entries) = obj.get("entries") {
                    if let Some(arr) = entries.as_array() {
                        let name = obj
                            .get("name")
                            .and_then(|n| n.as_str())
                            .map(|n| format!("**{}**\n", n))
                            .unwrap_or_default();
                        let content = arr
                            .iter()
                            .filter_map(|e| self.entry_to_string(e))
                            .collect::<Vec<_>>()
                            .join("\n");
                        Some(format!("{}{}", name, content))
                    } else {
                        None
                    }
                } else if let Some(text) = obj.get("text") {
                    text.as_str().map(|s| self.clean_formatting(s))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn clean_formatting(&self, text: &str) -> String {
        // Remove 5etools formatting tags like {@damage 1d6}, {@spell fireball}, etc.
        let mut result = text.to_string();

        // Pattern: {@tag content} or {@tag content|display}
        let re = regex_lite::Regex::new(r"\{@\w+\s+([^|}]+)(?:\|[^}]*)?\}").unwrap();
        result = re.replace_all(&result, "$1").to_string();

        // Pattern: {@tag content|display} - use display
        let re2 = regex_lite::Regex::new(r"\{@\w+\s+[^|]+\|([^}]+)\}").unwrap();
        result = re2.replace_all(&result, "$1").to_string();

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn school_conversion() {
        let importer = FiveToolsImporter::new("/test");
        assert_eq!(importer.convert_school("A"), "Abjuration");
        assert_eq!(importer.convert_school("V"), "Evocation");
        assert_eq!(importer.convert_school("N"), "Necromancy");
    }

    #[test]
    fn casting_time_conversion() {
        let importer = FiveToolsImporter::new("/test");

        let times = vec![FiveToolsTime {
            number: Some(1),
            unit: "action".to_string(),
            condition: None,
        }];
        let result = importer.convert_casting_time(&times);
        assert_eq!(result.unit, CastingTimeUnit::Action);
        assert_eq!(result.amount, 1);

        let times = vec![FiveToolsTime {
            number: Some(10),
            unit: "minute".to_string(),
            condition: None,
        }];
        let result = importer.convert_casting_time(&times);
        assert_eq!(result.unit, CastingTimeUnit::Minute);
        assert_eq!(result.amount, 10);
    }

    #[test]
    fn clean_formatting_removes_tags() {
        let importer = FiveToolsImporter::new("/test");

        assert_eq!(
            importer.clean_formatting("Deal {@damage 2d6} fire damage"),
            "Deal 2d6 fire damage"
        );
        assert_eq!(
            importer.clean_formatting("Cast {@spell fireball}"),
            "Cast fireball"
        );
        assert_eq!(
            importer.clean_formatting("See {@creature goblin|mm}"),
            "See goblin"
        );
    }
}
