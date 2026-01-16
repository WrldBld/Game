//! Content types for the compendium system.
//!
//! These types provide a unified way to represent game content across
//! different TTRPG systems (D&D 5e, Pathfinder 2e, etc.).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Type of content source (for UI badges and filtering).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// Official publisher content (e.g., WotC for D&D)
    #[default]
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

/// Detailed source information for content items.
///
/// Provides source tracking for UI badges showing where content came from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSource {
    /// Short code (e.g., "PHB", "XGE", "TCE", "Homebrew")
    code: String,
    /// Full name (e.g., "Player's Handbook", "Xanathar's Guide to Everything")
    name: String,
    /// Source type for categorization
    source_type: SourceType,
    /// Page reference (optional)
    page: Option<u32>,
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
            "MPMM" => (
                "Mordenkainen Presents: Monsters of the Multiverse",
                SourceType::Official,
            ),
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

    // Read accessors
    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source_type(&self) -> &SourceType {
        &self.source_type
    }

    pub fn page(&self) -> Option<u32> {
        self.page
    }

    // Builder methods
    /// Add a page reference.
    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    pub fn with_source_type(mut self, source_type: SourceType) -> Self {
        self.source_type = source_type;
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
/// - D&D: Race -> CharacterOrigin, Class -> CharacterClass
/// - PF2e: Ancestry -> CharacterOrigin, Class -> CharacterClass
/// - Blades: Playbook -> CharacterClass
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_type_display() {
        assert_eq!(ContentType::CharacterOrigin.display_name(), "Origin");
        assert_eq!(ContentType::CharacterClass.display_name(), "Class");
        assert_eq!(
            ContentType::Custom("Playbook".to_string()).display_name(),
            "Playbook"
        );
    }

    #[test]
    fn content_type_slug() {
        assert_eq!(ContentType::CharacterOrigin.slug(), "origin");
        assert_eq!(ContentType::ClassFeature.slug(), "class_feature");
        assert_eq!(
            ContentType::Custom("Special Move".to_string()).slug(),
            "special_move"
        );
    }
}
