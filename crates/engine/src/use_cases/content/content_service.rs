// Content service - methods for future content provider features
#![allow(dead_code)]

//! Content service for managing game content.
//!
//! Provides unified access to game content (spells, feats, classes, races, etc.)
//! through the CompendiumProvider trait system.

use crate::infrastructure::content_sources::{Dnd5eContentProvider, FiveToolsImporter, ImportError};
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use wrldbldr_shared::game_systems::{
    CompendiumProvider, ContentFilter as DomainContentFilter, ContentItem, ContentType,
};

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
#[derive(Debug, Clone, Default)]
pub struct ContentServiceConfig {
    /// Path to 5etools data (optional).
    pub fivetools_path: Option<PathBuf>,
}

/// Service for managing game content through CompendiumProviders.
pub struct ContentService {
    /// Registered content providers by system ID.
    providers: DashMap<String, Arc<dyn CompendiumProvider>>,
    /// Configuration.
    config: ContentServiceConfig,
}

impl ContentService {
    /// Create a new content service.
    pub fn new(config: ContentServiceConfig) -> Self {
        Self {
            providers: DashMap::new(),
            config,
        }
    }

    /// Get the service configuration.
    pub fn config(&self) -> &ContentServiceConfig {
        &self.config
    }

    // === Provider Management ===

    /// Register a content provider for a game system.
    pub fn register_provider(
        &self,
        system_id: impl Into<String>,
        provider: Arc<dyn CompendiumProvider>,
    ) {
        self.providers.insert(system_id.into(), provider);
    }

    /// Get a registered content provider.
    pub fn get_provider(&self, system_id: &str) -> Option<Arc<dyn CompendiumProvider>> {
        self.providers.get(system_id).map(|p| Arc::clone(p.value()))
    }

    /// List all registered system IDs.
    pub fn registered_systems(&self) -> Vec<String> {
        self.providers.iter().map(|r| r.key().clone()).collect()
    }

    /// Get content types supported by a system.
    pub fn content_types_for_system(&self, system_id: &str) -> Vec<ContentType> {
        self.providers
            .get(system_id)
            .map(|p| p.value().content_types())
            .unwrap_or_default()
    }

    // === Content Access ===

    /// Get content of a specific type from a provider.
    pub fn get_content(
        &self,
        system_id: &str,
        content_type: &ContentType,
        filter: &DomainContentFilter,
    ) -> Result<Vec<ContentItem>, ContentError> {
        let provider = self
            .providers
            .get(system_id)
            .ok_or_else(|| ContentError::SystemNotFound(system_id.to_string()))?;

        provider
            .value()
            .load_content(content_type, filter)
            .map_err(|e| ContentError::ContentNotFound(e.to_string()))
    }

    /// Get a single content item by ID.
    pub fn get_content_by_id(
        &self,
        system_id: &str,
        content_type: &ContentType,
        id: &str,
    ) -> Result<Option<ContentItem>, ContentError> {
        let provider = self
            .providers
            .get(system_id)
            .ok_or_else(|| ContentError::SystemNotFound(system_id.to_string()))?;

        provider
            .value()
            .get_content_by_id(content_type, id)
            .map_err(|e| ContentError::ContentNotFound(e.to_string()))
    }

    /// Search content across all types for a system.
    pub fn search_content(
        &self,
        system_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ContentItem>, ContentError> {
        let provider = self
            .providers
            .get(system_id)
            .ok_or_else(|| ContentError::SystemNotFound(system_id.to_string()))?;

        let filter = DomainContentFilter::new()
            .with_search(query)
            .with_limit(limit);
        let mut results = Vec::new();

        for content_type in provider.value().content_types() {
            if let Ok(items) = provider.value().load_content(&content_type, &filter) {
                results.extend(items);
            }
        }

        results.truncate(limit);
        Ok(results)
    }

    // === 5etools Integration ===

    /// Register the D&D 5e content provider using 5etools data.
    pub fn register_dnd5e_provider(&self, data_path: impl Into<PathBuf>) {
        let provider = Arc::new(Dnd5eContentProvider::new(data_path));
        self.register_provider("dnd5e", provider);
        tracing::info!("Registered D&D 5e content provider");
    }

    /// Load content from 5etools data directory.
    ///
    /// Validates the path and registers the D&D 5e provider.
    pub async fn load_from_5etools(&self, path: &PathBuf) -> Result<usize, ContentError> {
        let importer = FiveToolsImporter::new(path);

        if !importer.validate_path().await {
            return Err(ContentError::Import(ImportError::IndexNotFound(
                path.join("data"),
            )));
        }

        // Register the content provider
        self.register_dnd5e_provider(path);

        // Count content by loading each type through the provider
        let mut total = 0;
        if let Some(provider) = self.get_provider("dnd5e") {
            let filter = DomainContentFilter::default();
            for ct in provider.content_types() {
                if let Ok(items) = provider.load_content(&ct, &filter) {
                    total += items.len();
                }
            }
        }

        tracing::info!("Loaded {} total items from 5etools", total);
        Ok(total)
    }

    // === Statistics ===

    /// Get statistics about loaded content.
    ///
    /// Uses count_content() for efficient counting without loading all data.
    pub fn stats(&self) -> ContentStats {
        let mut total_items = 0;

        for provider_ref in self.providers.iter() {
            let provider = provider_ref.value();
            for ct in provider.content_types() {
                if let Ok(count) = provider.count_content(&ct) {
                    total_items += count;
                }
            }
        }

        ContentStats {
            systems: self.providers.len(),
            total_items,
        }
    }
}

/// Statistics about loaded content.
#[derive(Debug, Clone)]
pub struct ContentStats {
    /// Number of registered game systems.
    pub systems: usize,
    /// Total number of content items across all providers.
    pub total_items: usize,
}

/// Create a shared content service.
#[allow(dead_code)]
pub fn create_content_service(config: ContentServiceConfig) -> Arc<ContentService> {
    Arc::new(ContentService::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wrldbldr_shared::game_systems::{
        CompendiumProvider, ContentError as DomainContentError, ContentItem,
    };

    struct MockProvider {
        items: Vec<ContentItem>,
    }

    impl MockProvider {
        fn new(items: Vec<ContentItem>) -> Self {
            Self { items }
        }
    }

    impl CompendiumProvider for MockProvider {
        fn content_types(&self) -> Vec<ContentType> {
            vec![ContentType::Spell]
        }

        fn load_content(
            &self,
            _content_type: &ContentType,
            filter: &DomainContentFilter,
        ) -> Result<Vec<ContentItem>, DomainContentError> {
            let mut results = self.items.clone();

            if let Some(ref search) = filter.search {
                results.retain(|item| item.name.to_lowercase().contains(&search.to_lowercase()));
            }

            if let Some(limit) = filter.limit {
                results.truncate(limit);
            }

            Ok(results)
        }

        fn get_content_by_id(
            &self,
            _content_type: &ContentType,
            id: &str,
        ) -> Result<Option<ContentItem>, DomainContentError> {
            Ok(self.items.iter().find(|item| item.id == id).cloned())
        }

        fn filter_schema(
            &self,
            _content_type: &ContentType,
        ) -> Option<wrldbldr_shared::game_systems::FilterSchema> {
            None
        }
    }

    fn create_test_item(id: &str, name: &str) -> ContentItem {
        ContentItem::new(id, ContentType::Spell, name, "Test")
            .with_description(format!("Test description for {}", name))
    }

    #[test]
    fn test_register_and_get_provider() {
        let service = ContentService::new(ContentServiceConfig::default());
        let provider = Arc::new(MockProvider::new(vec![]));

        service.register_provider("test", provider.clone());

        assert!(service.get_provider("test").is_some());
        assert!(service.get_provider("nonexistent").is_none());
    }

    #[test]
    fn test_registered_systems() {
        let service = ContentService::new(ContentServiceConfig::default());

        service.register_provider("system1", Arc::new(MockProvider::new(vec![])));
        service.register_provider("system2", Arc::new(MockProvider::new(vec![])));

        let systems = service.registered_systems();
        assert_eq!(systems.len(), 2);
        assert!(systems.contains(&"system1".to_string()));
        assert!(systems.contains(&"system2".to_string()));
    }

    #[test]
    fn test_get_content() {
        let service = ContentService::new(ContentServiceConfig::default());
        let items = vec![
            create_test_item("fireball", "Fireball"),
            create_test_item("magic_missile", "Magic Missile"),
        ];
        service.register_provider("test", Arc::new(MockProvider::new(items)));

        let filter = DomainContentFilter::default();
        let result = service
            .get_content("test", &ContentType::Spell, &filter)
            .unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_get_content_with_search() {
        let service = ContentService::new(ContentServiceConfig::default());
        let items = vec![
            create_test_item("fireball", "Fireball"),
            create_test_item("magic_missile", "Magic Missile"),
        ];
        service.register_provider("test", Arc::new(MockProvider::new(items)));

        let filter = DomainContentFilter::new().with_search("fire");
        let result = service
            .get_content("test", &ContentType::Spell, &filter)
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Fireball");
    }

    #[test]
    fn test_get_content_by_id() {
        let service = ContentService::new(ContentServiceConfig::default());
        let items = vec![create_test_item("fireball", "Fireball")];
        service.register_provider("test", Arc::new(MockProvider::new(items)));

        let result = service
            .get_content_by_id("test", &ContentType::Spell, "fireball")
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Fireball");

        let missing = service
            .get_content_by_id("test", &ContentType::Spell, "nonexistent")
            .unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_search_content() {
        let service = ContentService::new(ContentServiceConfig::default());
        let items = vec![
            create_test_item("fireball", "Fireball"),
            create_test_item("fire_bolt", "Fire Bolt"),
            create_test_item("magic_missile", "Magic Missile"),
        ];
        service.register_provider("test", Arc::new(MockProvider::new(items)));

        let result = service.search_content("test", "fire", 10).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_system_not_found() {
        let service = ContentService::new(ContentServiceConfig::default());
        let filter = DomainContentFilter::default();

        let result = service.get_content("nonexistent", &ContentType::Spell, &filter);
        assert!(matches!(result, Err(ContentError::SystemNotFound(_))));
    }

    #[test]
    fn test_stats() {
        let service = ContentService::new(ContentServiceConfig::default());
        let items = vec![
            create_test_item("spell1", "Spell 1"),
            create_test_item("spell2", "Spell 2"),
        ];
        service.register_provider("test", Arc::new(MockProvider::new(items)));

        let stats = service.stats();
        assert_eq!(stats.systems, 1);
        assert_eq!(stats.total_items, 2);
    }

    // Integration tests that require actual 5etools data
    // Run with: cargo test --package wrldbldr-engine content_service::tests::integration -- --ignored --nocapture
    mod integration {
        use super::*;
        use crate::infrastructure::content_sources::Dnd5eContentProvider;

        const FIVETOOLS_PATH: &str = "/Users/otto/repos/WrldBldr/5etools/5etools-src";

        fn skip_if_no_data() -> bool {
            !std::path::Path::new(FIVETOOLS_PATH).join("data").exists()
        }

        #[test]
        #[ignore = "requires 5etools data"]
        fn content_service_with_dnd5e_provider() {
            if skip_if_no_data() {
                println!("Skipping: 5etools data not found at {}", FIVETOOLS_PATH);
                return;
            }

            let service = ContentService::new(ContentServiceConfig::default());
            service.register_dnd5e_provider(FIVETOOLS_PATH);

            // Verify provider registered
            assert!(
                service.get_provider("dnd5e").is_some(),
                "D&D 5e provider should be registered"
            );
            assert!(
                service.registered_systems().contains(&"dnd5e".to_string()),
                "dnd5e should be in registered systems"
            );
        }

        // These tests need multi_thread runtime because CompendiumProvider uses block_in_place
        #[tokio::test(flavor = "multi_thread")]
        #[ignore = "requires 5etools data"]
        async fn content_service_get_content() {
            if skip_if_no_data() {
                println!("Skipping: 5etools data not found at {}", FIVETOOLS_PATH);
                return;
            }

            let service = ContentService::new(ContentServiceConfig::default());
            service.register_dnd5e_provider(FIVETOOLS_PATH);

            // Get backgrounds (spells work too, but backgrounds are simpler)
            let filter = DomainContentFilter::default();
            let backgrounds = service
                .get_content("dnd5e", &ContentType::CharacterBackground, &filter)
                .expect("Failed to get backgrounds");

            assert!(!backgrounds.is_empty(), "Should get backgrounds");
            println!("Got {} backgrounds from service", backgrounds.len());

            // Get spells
            let spells = service
                .get_content("dnd5e", &ContentType::Spell, &filter)
                .expect("Failed to get spells");

            assert!(!spells.is_empty(), "Should get spells");
            println!("Got {} spells from service", spells.len());

            // Get items
            let items = service
                .get_content("dnd5e", &ContentType::Item, &filter)
                .expect("Failed to get items");

            assert!(!items.is_empty(), "Should get items");
            println!("Got {} items from service", items.len());

            // Get class features
            let features = service
                .get_content("dnd5e", &ContentType::ClassFeature, &filter)
                .expect("Failed to get class features");

            assert!(!features.is_empty(), "Should get class features");
            println!("Got {} class features from service", features.len());
        }

        #[tokio::test(flavor = "multi_thread")]
        #[ignore = "requires 5etools data"]
        async fn content_service_get_content_by_id() {
            if skip_if_no_data() {
                println!("Skipping: 5etools data not found at {}", FIVETOOLS_PATH);
                return;
            }

            let service = ContentService::new(ContentServiceConfig::default());
            service.register_dnd5e_provider(FIVETOOLS_PATH);

            // First get list to find a valid ID
            let filter = DomainContentFilter::new().with_limit(1);
            let backgrounds = service
                .get_content("dnd5e", &ContentType::CharacterBackground, &filter)
                .expect("Failed to get backgrounds");

            if let Some(first_bg) = backgrounds.first() {
                println!("Looking up background by ID: {}", first_bg.id);

                let found = service
                    .get_content_by_id("dnd5e", &ContentType::CharacterBackground, &first_bg.id)
                    .expect("Failed to get by ID");

                assert!(found.is_some(), "Should find background by ID");
                assert_eq!(found.unwrap().name, first_bg.name);
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        #[ignore = "requires 5etools data"]
        async fn content_service_search() {
            if skip_if_no_data() {
                println!("Skipping: 5etools data not found at {}", FIVETOOLS_PATH);
                return;
            }

            let service = ContentService::new(ContentServiceConfig::default());
            service.register_dnd5e_provider(FIVETOOLS_PATH);

            // Search for "fireball" across all content
            let results = service
                .search_content("dnd5e", "fireball", 500)
                .expect("Failed to search");

            println!("Found {} items matching 'fireball'", results.len());
            assert!(
                !results.is_empty(),
                "Should find content matching 'fireball'"
            );

            // Should include Fireball spell
            let has_fireball = results
                .iter()
                .any(|r| r.name == "Fireball" && r.content_type == ContentType::Spell);
            assert!(has_fireball, "Should find Fireball spell");
        }

        #[tokio::test(flavor = "multi_thread")]
        #[ignore = "requires 5etools data"]
        async fn content_service_stats_with_real_data() {
            if skip_if_no_data() {
                println!("Skipping: 5etools data not found at {}", FIVETOOLS_PATH);
                return;
            }

            let service = ContentService::new(ContentServiceConfig::default());
            service.register_dnd5e_provider(FIVETOOLS_PATH);

            let stats = service.stats();

            println!(
                "Content stats: {} systems, {} total items",
                stats.systems, stats.total_items
            );
            assert_eq!(stats.systems, 1, "Should have 1 system registered");
            // At minimum, we have 936 spells + 149 backgrounds = 1085 items
            assert!(
                stats.total_items > 1000,
                "Should have many items (got {}, expected > 1000)",
                stats.total_items
            );
        }

        #[test]
        #[ignore = "requires 5etools data"]
        fn content_service_content_types() {
            if skip_if_no_data() {
                println!("Skipping: 5etools data not found at {}", FIVETOOLS_PATH);
                return;
            }

            let service = ContentService::new(ContentServiceConfig::default());
            service.register_dnd5e_provider(FIVETOOLS_PATH);

            let types = service.content_types_for_system("dnd5e");

            println!("D&D 5e content types: {:?}", types);
            assert!(types.contains(&ContentType::CharacterOrigin));
            assert!(types.contains(&ContentType::CharacterSuborigin));
            assert!(types.contains(&ContentType::CharacterClass));
            assert!(types.contains(&ContentType::CharacterSubclass));
            assert!(types.contains(&ContentType::CharacterBackground));
            assert!(types.contains(&ContentType::ClassFeature));
            assert!(types.contains(&ContentType::Ability));
            assert!(types.contains(&ContentType::Weapon));
            assert!(types.contains(&ContentType::Armor));
            assert!(types.contains(&ContentType::Item));
            assert!(types.contains(&ContentType::MagicItem));
            assert!(types.contains(&ContentType::Spell));
            assert!(types.contains(&ContentType::Feat));
        }
    }
}
