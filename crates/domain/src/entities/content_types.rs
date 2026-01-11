//! Content types for the compendium system.
//!
//! These types provide a unified way to represent game content across
//! different TTRPG systems (D&D 5e, Pathfinder 2e, etc.).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Type of content source (for UI badges and filtering).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// Official publisher content (e.g., WotC for D&D)
    Official,
    /// Third-party published content (e.g., Kobold Press)
    ThirdParty,
    /// User-created homebrew content
    Homebrew,
    /// System Reference Document (free/open content)
    Srd,
}

impl SourceType {
    /// Get a display label for the source type.
    pub fn label(&self) -> &str {
        match self {
            SourceType::Official => "Official",
            SourceType::ThirdParty => "Third Party",
            SourceType::Homebrew => "Homebrew",
            SourceType::Srd => "SRD",
        }
    }
}

impl Default for SourceType {
    fn default() -> Self {
        SourceType::Official
    }
}

/// Detailed source information for content items.
///
/// Provides source tracking for UI badges showing where content came from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSource {
    /// Short code (e.g., "PHB", "XGE", "TCE", "Homebrew")
    pub code: String,
    /// Full name (e.g., "Player's Handbook", "Xanathar's Guide to Everything")
    pub name: String,
    /// Source type for categorization
    pub source_type: SourceType,
    /// Page reference (optional)
    pub page: Option<u32>,
}

impl ContentSource {
    /// Create a new ContentSource with required fields.
    pub fn new(code: impl Into<String>, name: impl Into<String>, source_type: SourceType) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            source_type,
            page: None,
        }
    }

    /// Create a source from just a code, inferring the full name and type.
    pub fn from_code(code: &str) -> Self {
        let (name, source_type) = match code.to_uppercase().as_str() {
            // Core D&D 5e books (Official)
            "PHB" => ("Player's Handbook", SourceType::Official),
            "DMG" => ("Dungeon Master's Guide", SourceType::Official),
            "MM" => ("Monster Manual", SourceType::Official),
            "XGE" | "XGTE" => ("Xanathar's Guide to Everything", SourceType::Official),
            "TCE" | "TCOE" => ("Tasha's Cauldron of Everything", SourceType::Official),
            "VGM" | "VGTM" => ("Volo's Guide to Monsters", SourceType::Official),
            "MTF" | "MTOF" => ("Mordenkainen's Tome of Foes", SourceType::Official),
            "SCAG" => ("Sword Coast Adventurer's Guide", SourceType::Official),
            "FTD" => ("Fizban's Treasury of Dragons", SourceType::Official),
            "MPMM" => ("Mordenkainen Presents: Monsters of the Multiverse", SourceType::Official),
            "BGG" | "BGGD" => ("Bigby Presents: Glory of the Giants", SourceType::Official),
            "BMT" => ("The Book of Many Things", SourceType::Official),

            // SRD
            "SRD" | "SRD5" => ("System Reference Document 5.1", SourceType::Srd),
            "BASIC" | "BASIC RULES" => ("Basic Rules", SourceType::Srd),

            // Third party (common examples)
            "KP" | "KOBOLD" => ("Kobold Press", SourceType::ThirdParty),
            "MC" | "MCDM" => ("MCDM Productions", SourceType::ThirdParty),

            // Homebrew
            "HB" | "HOMEBREW" => ("Homebrew", SourceType::Homebrew),

            // Unknown - default to official but use the code as name
            _ => (code, SourceType::Official),
        };

        Self {
            code: code.to_uppercase(),
            name: name.to_string(),
            source_type,
            page: None,
        }
    }

    /// Add a page reference.
    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Create a homebrew source.
    pub fn homebrew(name: impl Into<String>) -> Self {
        Self {
            code: "HB".to_string(),
            name: name.into(),
            source_type: SourceType::Homebrew,
            page: None,
        }
    }
}

/// Generic content types across all game systems.
///
/// Each game system maps its specific content to these abstract types:
/// - D&D: Race → CharacterOrigin, Class → CharacterClass
/// - PF2e: Ancestry → CharacterOrigin, Class → CharacterClass
/// - Blades: Playbook → CharacterClass
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    // Character creation options
    /// Race, Ancestry, Heritage, Kin, Species
    CharacterOrigin,
    /// Class, Playbook, Occupation, Role
    CharacterClass,
    /// Background, Origin Story
    CharacterBackground,
    /// Subrace, Lineage, Variant
    CharacterSuborigin,
    /// Subclass, Specialization, Path
    CharacterSubclass,

    // Abilities and features
    /// Spell, Power, Psionics
    Spell,
    /// Feat, Talent, Edge
    Feat,
    /// Special abilities, Stunts, Moves
    Ability,
    /// Class features, Advances, Talents
    ClassFeature,

    // Equipment
    /// Weapons
    Weapon,
    /// Armor and shields
    Armor,
    /// General items, gear, equipment
    Item,
    /// Magic items, artifacts
    MagicItem,

    // System-specific custom types
    /// Custom content type for system-specific needs
    Custom(String),
}

impl ContentType {
    /// Get a human-readable display name for this content type.
    pub fn display_name(&self) -> &str {
        match self {
            ContentType::CharacterOrigin => "Origin",
            ContentType::CharacterClass => "Class",
            ContentType::CharacterBackground => "Background",
            ContentType::CharacterSuborigin => "Subrace",
            ContentType::CharacterSubclass => "Subclass",
            ContentType::Spell => "Spell",
            ContentType::Feat => "Feat",
            ContentType::Ability => "Ability",
            ContentType::ClassFeature => "Class Feature",
            ContentType::Weapon => "Weapon",
            ContentType::Armor => "Armor",
            ContentType::Item => "Item",
            ContentType::MagicItem => "Magic Item",
            ContentType::Custom(name) => name.as_str(),
        }
    }

    /// Get a machine-readable slug for this content type.
    pub fn slug(&self) -> String {
        match self {
            ContentType::CharacterOrigin => "origin".to_string(),
            ContentType::CharacterClass => "class".to_string(),
            ContentType::CharacterBackground => "background".to_string(),
            ContentType::CharacterSuborigin => "suborigin".to_string(),
            ContentType::CharacterSubclass => "subclass".to_string(),
            ContentType::Spell => "spell".to_string(),
            ContentType::Feat => "feat".to_string(),
            ContentType::Ability => "ability".to_string(),
            ContentType::ClassFeature => "class_feature".to_string(),
            ContentType::Weapon => "weapon".to_string(),
            ContentType::Armor => "armor".to_string(),
            ContentType::Item => "item".to_string(),
            ContentType::MagicItem => "magic_item".to_string(),
            ContentType::Custom(name) => name.to_lowercase().replace(' ', "_"),
        }
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// A polymorphic content item from a game system's compendium.
///
/// This provides a unified representation for content across systems while
/// allowing system-specific data in the `data` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItem {
    /// Unique identifier (e.g., "dnd5e_phb_human", "pf2e_crb_elf")
    pub id: String,

    /// The type of content this represents
    pub content_type: ContentType,

    /// Display name (e.g., "Human", "Elf")
    pub name: String,

    /// Source information (book, page, type)
    pub source: ContentSource,

    /// Human-readable description
    pub description: String,

    /// System-specific data as JSON
    ///
    /// For D&D races, this might include: size, speed, darkvision, ability_bonuses
    /// For PF2e ancestries: size, speed, hp, ability_boosts, ability_flaw
    pub data: serde_json::Value,

    /// Searchable tags for filtering
    pub tags: Vec<String>,
}

impl ContentItem {
    /// Create a new ContentItem with required fields.
    ///
    /// The source parameter can be a source code string (e.g., "PHB") or a ContentSource.
    pub fn new(
        id: impl Into<String>,
        content_type: ContentType,
        name: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        let source_code = source.into();
        Self {
            id: id.into(),
            content_type,
            name: name.into(),
            source: ContentSource::from_code(&source_code),
            description: String::new(),
            data: serde_json::Value::Null,
            tags: Vec::new(),
        }
    }

    /// Create a new ContentItem with a full ContentSource.
    pub fn with_source(
        id: impl Into<String>,
        content_type: ContentType,
        name: impl Into<String>,
        source: ContentSource,
    ) -> Self {
        Self {
            id: id.into(),
            content_type,
            name: name.into(),
            source,
            description: String::new(),
            data: serde_json::Value::Null,
            tags: Vec::new(),
        }
    }

    /// Add a description to the content item.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add system-specific data to the content item.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    /// Add tags to the content item.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Check if this item matches a search query.
    pub fn matches_search(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.name.to_lowercase().contains(&query_lower)
            || self.description.to_lowercase().contains(&query_lower)
            || self.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
    }

    /// Check if this item has a specific tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        let tag_lower = tag.to_lowercase();
        self.tags.iter().any(|t| t.to_lowercase() == tag_lower)
    }
}

/// Filter for querying content items.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentFilter {
    /// Filter by content type
    pub content_type: Option<ContentType>,

    /// Filter by source (e.g., "PHB", "XGE")
    pub source: Option<String>,

    /// Text search across name, description, and tags
    pub search: Option<String>,

    /// Filter by specific tags
    pub tags: Option<Vec<String>>,

    /// Maximum number of results
    pub limit: Option<usize>,

    /// Offset for pagination
    pub offset: Option<usize>,
}

impl ContentFilter {
    /// Create a new empty filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by content type.
    pub fn with_type(mut self, content_type: ContentType) -> Self {
        self.content_type = Some(content_type);
        self
    }

    /// Filter by source.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Filter by search query.
    pub fn with_search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }

    /// Filter by tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Limit results.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Offset for pagination.
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Check if a content item matches this filter.
    pub fn matches(&self, item: &ContentItem) -> bool {
        // Check content type
        if let Some(ref ct) = self.content_type {
            if &item.content_type != ct {
                return false;
            }
        }

        // Check source
        if let Some(ref source) = self.source {
            if !item.source.code.eq_ignore_ascii_case(source) {
                return false;
            }
        }

        // Check search
        if let Some(ref search) = self.search {
            if !item.matches_search(search) {
                return false;
            }
        }

        // Check tags
        if let Some(ref tags) = self.tags {
            if !tags.iter().all(|t| item.has_tag(t)) {
                return false;
            }
        }

        true
    }

    /// Apply this filter to a collection of items.
    pub fn apply<'a>(&self, items: impl Iterator<Item = &'a ContentItem>) -> Vec<&'a ContentItem> {
        let filtered: Vec<_> = items.filter(|item| self.matches(item)).collect();

        let offset = self.offset.unwrap_or(0);
        let limited: Vec<_> = filtered.into_iter().skip(offset).collect();

        if let Some(limit) = self.limit {
            limited.into_iter().take(limit).collect()
        } else {
            limited
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_type_display() {
        assert_eq!(ContentType::CharacterOrigin.display_name(), "Origin");
        assert_eq!(ContentType::CharacterClass.display_name(), "Class");
        assert_eq!(ContentType::Custom("Playbook".to_string()).display_name(), "Playbook");
    }

    #[test]
    fn content_type_slug() {
        assert_eq!(ContentType::CharacterOrigin.slug(), "origin");
        assert_eq!(ContentType::ClassFeature.slug(), "class_feature");
        assert_eq!(ContentType::Custom("Special Move".to_string()).slug(), "special_move");
    }

    #[test]
    fn content_item_builder() {
        let item = ContentItem::new("test_id", ContentType::CharacterOrigin, "Human", "PHB")
            .with_description("A versatile race")
            .with_tags(vec!["core".to_string(), "player".to_string()]);

        assert_eq!(item.id, "test_id");
        assert_eq!(item.name, "Human");
        assert!(item.has_tag("core"));
        assert!(item.matches_search("versatile"));
    }

    #[test]
    fn content_filter_matches() {
        let item = ContentItem::new("human", ContentType::CharacterOrigin, "Human", "PHB")
            .with_tags(vec!["core".to_string()]);

        let filter = ContentFilter::new()
            .with_type(ContentType::CharacterOrigin)
            .with_source("PHB");

        assert!(filter.matches(&item));

        let wrong_type_filter = ContentFilter::new().with_type(ContentType::Spell);
        assert!(!wrong_type_filter.matches(&item));
    }
}
