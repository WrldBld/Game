//! Content service for managing game content.
//!
//! Stores and provides access to spells, feats, class features, and other
//! game content across different game systems.

use crate::infrastructure::importers::{FiveToolsImporter, ImportError};
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use wrldbldr_domain::{Feat, Spell, WorldId};

/// Errors that can occur in the content service.
#[derive(Debug, Error)]
pub enum ContentError {
    #[error("Import error: {0}")]
    Import(#[from] ImportError),
    #[error("System not found: {0}")]
    SystemNotFound(String),
    #[error("Content not found: {0}")]
    ContentNotFound(String),
}

/// Configuration for the content service.
#[derive(Debug, Clone)]
pub struct ContentServiceConfig {
    /// Path to 5etools data (optional).
    pub fivetools_path: Option<PathBuf>,
    /// Whether to preload content on startup.
    pub preload: bool,
}

impl Default for ContentServiceConfig {
    fn default() -> Self {
        Self {
            fivetools_path: None,
            preload: false,
        }
    }
}

/// Filter criteria for content queries.
#[derive(Debug, Clone, Default)]
pub struct ContentFilter {
    /// Minimum spell level.
    pub level_min: Option<u8>,
    /// Maximum spell level.
    pub level_max: Option<u8>,
    /// School of magic (for spells).
    pub school: Option<String>,
    /// Class that can use this content.
    pub class: Option<String>,
    /// Text search in name/description.
    pub search: Option<String>,
    /// Source book filter.
    pub source: Option<String>,
    /// Maximum results to return.
    pub limit: Option<usize>,
}

impl ContentFilter {
    /// Check if a spell matches this filter.
    pub fn matches_spell(&self, spell: &Spell) -> bool {
        // Level filter
        let level = spell.level.as_number();
        if let Some(min) = self.level_min {
            if level < min {
                return false;
            }
        }
        if let Some(max) = self.level_max {
            if level > max {
                return false;
            }
        }

        // School filter
        if let Some(ref school) = self.school {
            if let Some(ref spell_school) = spell.school {
                if !spell_school.eq_ignore_ascii_case(school) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Class filter
        if let Some(ref class) = self.class {
            let class_lower = class.to_lowercase();
            if !spell.classes.iter().any(|c| c.to_lowercase() == class_lower) {
                return false;
            }
        }

        // Search filter
        if let Some(ref search) = self.search {
            let search_lower = search.to_lowercase();
            if !spell.name.to_lowercase().contains(&search_lower)
                && !spell.description.to_lowercase().contains(&search_lower)
            {
                return false;
            }
        }

        // Source filter
        if let Some(ref source) = self.source {
            if !spell.source.to_lowercase().contains(&source.to_lowercase()) {
                return false;
            }
        }

        true
    }

    /// Check if a feat matches this filter.
    pub fn matches_feat(&self, feat: &Feat) -> bool {
        // Search filter
        if let Some(ref search) = self.search {
            let search_lower = search.to_lowercase();
            if !feat.name.to_lowercase().contains(&search_lower)
                && !feat.description.to_lowercase().contains(&search_lower)
            {
                return false;
            }
        }

        // Source filter
        if let Some(ref source) = self.source {
            if !feat.source.to_lowercase().contains(&source.to_lowercase()) {
                return false;
            }
        }

        true
    }
}

/// Service for managing game content (spells, feats, features).
pub struct ContentService {
    /// Spells by system ID.
    spells: DashMap<String, Vec<Spell>>,
    /// Feats by system ID.
    feats: DashMap<String, Vec<Feat>>,
    /// Custom spells by world ID.
    custom_spells: DashMap<WorldId, Vec<Spell>>,
    /// Custom feats by world ID.
    custom_feats: DashMap<WorldId, Vec<Feat>>,
    /// Configuration.
    config: ContentServiceConfig,
}

impl ContentService {
    /// Create a new content service.
    pub fn new(config: ContentServiceConfig) -> Self {
        Self {
            spells: DashMap::new(),
            feats: DashMap::new(),
            custom_spells: DashMap::new(),
            custom_feats: DashMap::new(),
            config,
        }
    }

    /// Initialize the service, optionally loading content from configured sources.
    pub async fn initialize(&self) -> Result<(), ContentError> {
        if self.config.preload {
            if let Some(ref path) = self.config.fivetools_path {
                self.load_from_5etools(path).await?;
            }
        }
        Ok(())
    }

    /// Load content from 5etools data directory.
    pub async fn load_from_5etools(&self, path: &PathBuf) -> Result<usize, ContentError> {
        let importer = FiveToolsImporter::new(path);

        if !importer.validate_path().await {
            return Err(ContentError::Import(ImportError::IndexNotFound(
                path.join("data"),
            )));
        }

        let mut total = 0;

        // Import spells
        match importer.import_spells().await {
            Ok(spells) => {
                total += spells.len();
                self.spells.insert("dnd5e".to_string(), spells);
            }
            Err(e) => {
                tracing::warn!("Failed to import spells: {}", e);
            }
        }

        // Import feats
        match importer.import_feats().await {
            Ok(feats) => {
                total += feats.len();
                self.feats.insert("dnd5e".to_string(), feats);
            }
            Err(e) => {
                tracing::warn!("Failed to import feats: {}", e);
            }
        }

        tracing::info!("Loaded {} items from 5etools", total);
        Ok(total)
    }

    // === Spell Methods ===

    /// Get all spells for a system.
    pub fn get_spells(&self, system_id: &str) -> Vec<Spell> {
        self.spells
            .get(system_id)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    /// Get spells matching a filter.
    pub fn get_spells_filtered(&self, system_id: &str, filter: &ContentFilter) -> Vec<Spell> {
        let spells = self.spells.get(system_id);
        let matching: Vec<Spell> = spells
            .map(|r| {
                r.value()
                    .iter()
                    .filter(|s| filter.matches_spell(s))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        match filter.limit {
            Some(limit) => matching.into_iter().take(limit).collect(),
            None => matching,
        }
    }

    /// Get a spell by ID.
    pub fn get_spell(&self, system_id: &str, spell_id: &str) -> Option<Spell> {
        self.spells.get(system_id).and_then(|spells| {
            spells
                .value()
                .iter()
                .find(|s| s.id == spell_id)
                .cloned()
        })
    }

    /// Search spells by name.
    pub fn search_spells(&self, system_id: &str, query: &str, limit: usize) -> Vec<Spell> {
        let filter = ContentFilter {
            search: Some(query.to_string()),
            limit: Some(limit),
            ..Default::default()
        };
        self.get_spells_filtered(system_id, &filter)
    }

    /// Get spell count for a system.
    pub fn spell_count(&self, system_id: &str) -> usize {
        self.spells
            .get(system_id)
            .map(|r| r.value().len())
            .unwrap_or(0)
    }

    // === Feat Methods ===

    /// Get all feats for a system.
    pub fn get_feats(&self, system_id: &str) -> Vec<Feat> {
        self.feats
            .get(system_id)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    /// Get feats matching a filter.
    pub fn get_feats_filtered(&self, system_id: &str, filter: &ContentFilter) -> Vec<Feat> {
        let feats = self.feats.get(system_id);
        let matching: Vec<Feat> = feats
            .map(|r| {
                r.value()
                    .iter()
                    .filter(|f| filter.matches_feat(f))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        match filter.limit {
            Some(limit) => matching.into_iter().take(limit).collect(),
            None => matching,
        }
    }

    /// Get a feat by ID.
    pub fn get_feat(&self, system_id: &str, feat_id: &str) -> Option<Feat> {
        self.feats.get(system_id).and_then(|feats| {
            feats.value().iter().find(|f| f.id == feat_id).cloned()
        })
    }

    /// Search feats by name.
    pub fn search_feats(&self, system_id: &str, query: &str, limit: usize) -> Vec<Feat> {
        let filter = ContentFilter {
            search: Some(query.to_string()),
            limit: Some(limit),
            ..Default::default()
        };
        self.get_feats_filtered(system_id, &filter)
    }

    /// Get feat count for a system.
    pub fn feat_count(&self, system_id: &str) -> usize {
        self.feats
            .get(system_id)
            .map(|r| r.value().len())
            .unwrap_or(0)
    }

    // === Custom Content Methods ===

    /// Add a custom spell for a world.
    pub fn add_custom_spell(&self, world_id: WorldId, spell: Spell) {
        self.custom_spells
            .entry(world_id)
            .or_default()
            .push(spell);
    }

    /// Get custom spells for a world.
    pub fn get_custom_spells(&self, world_id: WorldId) -> Vec<Spell> {
        self.custom_spells
            .get(&world_id)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    /// Add a custom feat for a world.
    pub fn add_custom_feat(&self, world_id: WorldId, feat: Feat) {
        self.custom_feats
            .entry(world_id)
            .or_default()
            .push(feat);
    }

    /// Get custom feats for a world.
    pub fn get_custom_feats(&self, world_id: WorldId) -> Vec<Feat> {
        self.custom_feats
            .get(&world_id)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    // === Statistics ===

    /// Get statistics about loaded content.
    pub fn stats(&self) -> ContentStats {
        ContentStats {
            systems: self.spells.len(),
            total_spells: self.spells.iter().map(|r| r.value().len()).sum(),
            total_feats: self.feats.iter().map(|r| r.value().len()).sum(),
            worlds_with_custom: self.custom_spells.len() + self.custom_feats.len(),
        }
    }
}

/// Statistics about loaded content.
#[derive(Debug, Clone)]
pub struct ContentStats {
    /// Number of game systems with content.
    pub systems: usize,
    /// Total number of spells.
    pub total_spells: usize,
    /// Total number of feats.
    pub total_feats: usize,
    /// Number of worlds with custom content.
    pub worlds_with_custom: usize,
}

/// Create a shared content service.
pub fn create_content_service(config: ContentServiceConfig) -> Arc<ContentService> {
    Arc::new(ContentService::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_domain::{CastingTime, SpellComponents, SpellDuration, SpellLevel, SpellRange};

    fn create_test_spell(name: &str, level: u8, school: &str, classes: Vec<&str>) -> Spell {
        Spell {
            id: format!("test_{}", name.to_lowercase().replace(' ', "_")),
            system_id: "dnd5e".to_string(),
            name: name.to_string(),
            level: if level == 0 {
                SpellLevel::Cantrip
            } else {
                SpellLevel::Level(level)
            },
            school: Some(school.to_string()),
            casting_time: CastingTime::action(),
            range: SpellRange::feet(60),
            components: SpellComponents::verbal_somatic(),
            duration: SpellDuration::instantaneous(),
            description: format!("Test description for {}", name),
            higher_levels: None,
            classes: classes.into_iter().map(String::from).collect(),
            source: "Test".to_string(),
            tags: vec![],
            ritual: false,
            concentration: false,
        }
    }

    #[test]
    fn filter_by_level() {
        let filter = ContentFilter {
            level_min: Some(1),
            level_max: Some(3),
            ..Default::default()
        };

        let cantrip = create_test_spell("Fire Bolt", 0, "Evocation", vec!["wizard"]);
        let level1 = create_test_spell("Magic Missile", 1, "Evocation", vec!["wizard"]);
        let level3 = create_test_spell("Fireball", 3, "Evocation", vec!["wizard"]);
        let level5 = create_test_spell("Cone of Cold", 5, "Evocation", vec!["wizard"]);

        assert!(!filter.matches_spell(&cantrip));
        assert!(filter.matches_spell(&level1));
        assert!(filter.matches_spell(&level3));
        assert!(!filter.matches_spell(&level5));
    }

    #[test]
    fn filter_by_school() {
        let filter = ContentFilter {
            school: Some("Evocation".to_string()),
            ..Default::default()
        };

        let evocation = create_test_spell("Fireball", 3, "Evocation", vec!["wizard"]);
        let necromancy = create_test_spell("Animate Dead", 3, "Necromancy", vec!["wizard"]);

        assert!(filter.matches_spell(&evocation));
        assert!(!filter.matches_spell(&necromancy));
    }

    #[test]
    fn filter_by_class() {
        let filter = ContentFilter {
            class: Some("cleric".to_string()),
            ..Default::default()
        };

        let wizard_only = create_test_spell("Fireball", 3, "Evocation", vec!["wizard"]);
        let cleric = create_test_spell("Cure Wounds", 1, "Evocation", vec!["cleric", "bard"]);

        assert!(!filter.matches_spell(&wizard_only));
        assert!(filter.matches_spell(&cleric));
    }

    #[test]
    fn filter_by_search() {
        let filter = ContentFilter {
            search: Some("fire".to_string()),
            ..Default::default()
        };

        let fireball = create_test_spell("Fireball", 3, "Evocation", vec!["wizard"]);
        let magic_missile =
            create_test_spell("Magic Missile", 1, "Evocation", vec!["wizard"]);

        assert!(filter.matches_spell(&fireball));
        assert!(!filter.matches_spell(&magic_missile));
    }

    #[test]
    fn content_service_stats() {
        let service = ContentService::new(ContentServiceConfig::default());
        let stats = service.stats();

        assert_eq!(stats.systems, 0);
        assert_eq!(stats.total_spells, 0);
        assert_eq!(stats.total_feats, 0);
    }
}
