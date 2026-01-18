//! Item entity - Objects that can be possessed or interacted with
//!
//! # Graph-First Design (Phase 0.C)
//!
//! Items exist as nodes in the world. Possession is modeled as an edge:
//!
//! ```cypher
//! (world:World)-[:CONTAINS_ITEM]->(item:Item)
//! (character:Character)-[:POSSESSES {quantity: 1, equipped: true}]->(item:Item)
//! (playerCharacter:PlayerCharacter)-[:POSSESSES {quantity: 1, equipped: true}]->(item:Item)
//! ```
//!
//! Container items can hold other items:
//!
//! ```cypher
//! (container:Item {can_contain_items: true})-[:CONTAINS {quantity: 1, added_at: datetime()}]->(item:Item)
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use wrldbldr_domain::{ItemId, ItemName, WorldId};

/// An object that can be possessed or interacted with
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    id: ItemId,
    world_id: WorldId,
    name: ItemName,
    description: Option<String>,
    /// Type of item (e.g., "Weapon", "Consumable", "Key", "Quest")
    item_type: Option<String>,
    /// Whether only one of this item can exist
    is_unique: bool,
    /// Item-specific properties (JSON - acceptable per ADR)
    properties: Option<String>,
    /// Whether this item can contain other items (bag, chest, etc.)
    can_contain_items: bool,
    /// Maximum number of items this container can hold (None = unlimited)
    container_limit: Option<u32>,
}

impl Item {
    pub fn new(world_id: WorldId, name: ItemName) -> Self {
        Self {
            id: ItemId::new(),
            world_id,
            name,
            description: None,
            item_type: None,
            is_unique: false,
            properties: None,
            can_contain_items: false,
            container_limit: None,
        }
    }

    // Read accessors
    pub fn id(&self) -> ItemId {
        self.id
    }

    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn name(&self) -> &ItemName {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn item_type(&self) -> Option<&str> {
        self.item_type.as_deref()
    }

    pub fn is_unique(&self) -> bool {
        self.is_unique
    }

    pub fn properties(&self) -> Option<&str> {
        self.properties.as_deref()
    }

    pub fn can_contain_items(&self) -> bool {
        self.can_contain_items
    }

    pub fn container_limit(&self) -> Option<u32> {
        self.container_limit
    }

    // Builder methods
    pub fn with_id(mut self, id: ItemId) -> Self {
        self.id = id;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_type(mut self, item_type: impl Into<String>) -> Self {
        self.item_type = Some(item_type.into());
        self
    }

    pub fn unique(mut self) -> Self {
        self.is_unique = true;
        self
    }

    pub fn with_properties(mut self, properties: impl Into<String>) -> Self {
        self.properties = Some(properties.into());
        self
    }

    /// Make this item a container that can hold other items
    pub fn as_container(mut self) -> Self {
        self.can_contain_items = true;
        self
    }

    /// Set the maximum number of items this container can hold
    pub fn with_container_limit(mut self, limit: u32) -> Self {
        self.can_contain_items = true;
        self.container_limit = Some(limit);
        self
    }
}

/// Data for the POSSESSES edge between Character/PlayerCharacter and Item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    /// The item being possessed
    pub item: Item,
    /// How many of this item the character has
    pub quantity: u32,
    /// Whether the item is currently equipped/held
    pub equipped: bool,
    /// When the item was acquired
    pub acquired_at: DateTime<Utc>,
    /// How the item was acquired
    pub acquisition_method: Option<AcquisitionMethod>,
}

/// How an item was acquired
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcquisitionMethod {
    Found,
    Purchased,
    Gifted,
    Looted,
    Crafted,
    Inherited,
    /// Unknown method for forward compatibility
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for AcquisitionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Found => write!(f, "Found"),
            Self::Purchased => write!(f, "Purchased"),
            Self::Gifted => write!(f, "Gifted"),
            Self::Looted => write!(f, "Looted"),
            Self::Crafted => write!(f, "Crafted"),
            Self::Inherited => write!(f, "Inherited"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::str::FromStr for AcquisitionMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Found" => Ok(Self::Found),
            "Purchased" => Ok(Self::Purchased),
            "Gifted" => Ok(Self::Gifted),
            "Looted" => Ok(Self::Looted),
            "Crafted" => Ok(Self::Crafted),
            "Inherited" => Ok(Self::Inherited),
            _ => Ok(Self::Unknown),
        }
    }
}

/// How often a character frequents a location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrequencyLevel {
    Rarely,
    Sometimes,
    Often,
    Always,
    /// Unknown frequency for forward compatibility
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for FrequencyLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rarely => write!(f, "Rarely"),
            Self::Sometimes => write!(f, "Sometimes"),
            Self::Often => write!(f, "Often"),
            Self::Always => write!(f, "Always"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::str::FromStr for FrequencyLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Rarely" => Ok(Self::Rarely),
            "Sometimes" => Ok(Self::Sometimes),
            "Often" => Ok(Self::Often),
            "Always" => Ok(Self::Always),
            _ => Ok(Self::Unknown),
        }
    }
}
