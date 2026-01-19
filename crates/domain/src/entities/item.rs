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
///
/// # ADR-008 Tier 4: Simple Data Struct
///
/// This is a data-carrying struct with no invariants to protect. All fields are public
/// because there's no invalid state that can be constructed - any combination of values
/// is valid.
///
/// # Container Coupling
///
/// When setting `can_contain_items` to true, ensure `container_limit` is also set appropriately
/// (Some(limit) for finite containers, None for unlimited). This coupling is not enforced
/// by the type system - it's a data modeling concern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub world_id: WorldId,
    pub name: ItemName,
    pub description: Option<String>,
    /// Type of item (e.g., "Weapon", "Consumable", "Key", "Quest")
    pub item_type: Option<String>,
    /// Whether only one of this item can exist
    pub is_unique: bool,
    /// Item-specific properties (JSON - acceptable per ADR)
    pub properties: Option<String>,
    /// Whether this item can contain other items (bag, chest, etc.)
    pub can_contain_items: bool,
    /// Maximum number of items this container can hold (None = unlimited)
    pub container_limit: Option<u32>,
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
