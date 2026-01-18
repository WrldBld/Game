//! Entity and asset type enumerations

use serde::{Deserialize, Serialize};

use crate::error::DomainError;

/// Unified entity type enum for the entire WrldBldr system
///
/// This enum represents all types of entities in the game. It serves two purposes:
/// 1. Asset management: Character, Location, Item can have gallery assets
/// 2. Change broadcasts: All entity types can be created/updated/deleted
///
/// Use `has_assets()` to check if an entity type can have gallery assets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityType {
    // === Asset-bearing entities ===
    /// Character entity (NPCs and PCs) - can have assets
    #[serde(alias = "Character")]
    Character,
    /// Location entity - can have assets
    #[serde(alias = "Location")]
    Location,
    /// Item entity - can have assets
    #[serde(alias = "Item")]
    Item,

    // === World structure entities ===
    /// World container entity
    World,
    /// Region within a world
    Region,
    /// Scene (narrative unit)
    Scene,
    /// Act (larger narrative unit)
    Act,

    // === Game mechanics entities ===
    /// Skill definition
    Skill,
    /// Challenge (skill check, combat, etc.)
    Challenge,
    /// Interaction definition
    Interaction,

    // === Narrative entities ===
    /// Narrative event (triggered story beat)
    NarrativeEvent,
    /// Event chain (linked events)
    EventChain,
    /// Story event (timeline entry)
    StoryEvent,

    // === Character-related entities ===
    /// Player character (PC)
    PlayerCharacter,
    /// Relationship between characters
    Relationship,
    /// Observation trigger
    Observation,

    // === Motivation system entities ===
    /// Character goal
    Goal,
    /// Character want (motivation)
    Want,
    /// Actantial view (narrative role)
    ActantialView,

    // === Time ===
    /// Game time state
    GameTime,

    // === Forward compatibility ===
    /// Unknown entity type (for forward compatibility)
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Character => write!(f, "Character"),
            Self::Location => write!(f, "Location"),
            Self::Item => write!(f, "Item"),
            Self::World => write!(f, "World"),
            Self::Region => write!(f, "Region"),
            Self::Scene => write!(f, "Scene"),
            Self::Act => write!(f, "Act"),
            Self::Skill => write!(f, "Skill"),
            Self::Challenge => write!(f, "Challenge"),
            Self::Interaction => write!(f, "Interaction"),
            Self::NarrativeEvent => write!(f, "NarrativeEvent"),
            Self::EventChain => write!(f, "EventChain"),
            Self::StoryEvent => write!(f, "StoryEvent"),
            Self::PlayerCharacter => write!(f, "PlayerCharacter"),
            Self::Relationship => write!(f, "Relationship"),
            Self::Observation => write!(f, "Observation"),
            Self::Goal => write!(f, "Goal"),
            Self::Want => write!(f, "Want"),
            Self::ActantialView => write!(f, "ActantialView"),
            Self::GameTime => write!(f, "GameTime"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl EntityType {
    /// Returns true if this entity type can have gallery assets
    ///
    /// Only Character, Location, and Item entities can have assets
    /// (portraits, sprites, backdrops, icons, etc.)
    pub fn has_assets(&self) -> bool {
        matches!(self, Self::Character | Self::Location | Self::Item)
    }

    /// Get the lowercase string representation for file paths
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Location => "location",
            Self::Item => "item",
            Self::World => "world",
            Self::Region => "region",
            Self::Scene => "scene",
            Self::Act => "act",
            Self::Skill => "skill",
            Self::Challenge => "challenge",
            Self::Interaction => "interaction",
            Self::NarrativeEvent => "narrative_event",
            Self::EventChain => "event_chain",
            Self::StoryEvent => "story_event",
            Self::PlayerCharacter => "player_character",
            Self::Relationship => "relationship",
            Self::Observation => "observation",
            Self::Goal => "goal",
            Self::Want => "want",
            Self::ActantialView => "actantial_view",
            Self::GameTime => "game_time",
            Self::Unknown => "unknown",
        }
    }
}

impl std::str::FromStr for EntityType {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "character" => Ok(Self::Character),
            "location" => Ok(Self::Location),
            "item" => Ok(Self::Item),
            "world" => Ok(Self::World),
            "region" => Ok(Self::Region),
            "scene" => Ok(Self::Scene),
            "act" => Ok(Self::Act),
            "skill" => Ok(Self::Skill),
            "challenge" => Ok(Self::Challenge),
            "interaction" => Ok(Self::Interaction),
            "narrative_event" | "narrativeevent" => Ok(Self::NarrativeEvent),
            "event_chain" | "eventchain" => Ok(Self::EventChain),
            "story_event" | "storyevent" => Ok(Self::StoryEvent),
            "player_character" | "playercharacter" => Ok(Self::PlayerCharacter),
            "relationship" => Ok(Self::Relationship),
            "observation" => Ok(Self::Observation),
            "goal" => Ok(Self::Goal),
            "want" => Ok(Self::Want),
            "actantial_view" | "actantialview" => Ok(Self::ActantialView),
            "game_time" | "gametime" => Ok(Self::GameTime),
            "unknown" => Ok(Self::Unknown),
            _ => Err(DomainError::parse(format!("Unknown entity type: {}", s))),
        }
    }
}

/// Types of entity changes for broadcast notifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChangeType {
    /// Entity was created
    Created,
    /// Entity was updated
    Updated,
    /// Entity was deleted
    Deleted,
    /// Unknown change type (for forward compatibility)
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Updated => write!(f, "updated"),
            Self::Deleted => write!(f, "deleted"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Type of asset (determines which slot it occupies)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetType {
    /// Character face portrait (256x256)
    Portrait,
    /// Character full-body sprite (512x512)
    Sprite,
    /// Scene/location backdrop (1920x1080)
    Backdrop,
    /// Grid map tilesheet (512x512)
    Tilesheet,
    /// Item icon (64x64)
    ItemIcon,
    /// Grid of character expressions (768x768)
    EmotionSheet,
    /// Backdrop for clickable map region (1280x720)
    RegionBackdrop,
    /// Unknown asset type (for forward compatibility)
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Portrait => write!(f, "Portrait"),
            Self::Sprite => write!(f, "Sprite"),
            Self::Backdrop => write!(f, "Backdrop"),
            Self::Tilesheet => write!(f, "Tilesheet"),
            Self::ItemIcon => write!(f, "ItemIcon"),
            Self::EmotionSheet => write!(f, "EmotionSheet"),
            Self::RegionBackdrop => write!(f, "RegionBackdrop"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::str::FromStr for AssetType {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "portrait" => Self::Portrait,
            "sprite" => Self::Sprite,
            "backdrop" => Self::Backdrop,
            "tilesheet" => Self::Tilesheet,
            "itemicon" | "item_icon" => Self::ItemIcon,
            "emotionsheet" | "emotion_sheet" => Self::EmotionSheet,
            "regionbackdrop" | "region_backdrop" => Self::RegionBackdrop,
            _ => Self::Unknown,
        })
    }
}

impl AssetType {
    /// Get the lowercase string representation for file paths
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Portrait => "portrait",
            Self::Sprite => "sprite",
            Self::Backdrop => "backdrop",
            Self::Tilesheet => "tilesheet",
            Self::ItemIcon => "item_icon",
            Self::EmotionSheet => "emotion_sheet",
            Self::RegionBackdrop => "region_backdrop",
            Self::Unknown => "unknown",
        }
    }

    /// Get default dimensions for this asset type
    pub fn default_dimensions(&self) -> (u32, u32) {
        match self {
            Self::Portrait => (256, 256),
            Self::Sprite => (512, 512),
            Self::Backdrop => (1920, 1080),
            Self::Tilesheet => (512, 512),
            Self::ItemIcon => (64, 64),
            Self::EmotionSheet => (768, 768),
            Self::RegionBackdrop => (1280, 720),
            Self::Unknown => (256, 256),
        }
    }
}
