//! About target types for lore linking
//!
//! Lore entries can be about characters, locations, regions, or items.
//! This module provides typed wrappers for these relationships.

use crate::{CharacterId, ItemId, LocationId, RegionId};
use serde::{Deserialize, Serialize};

/// Target that a lore entry is about
///
/// Used in ABOUT_* edges to link lore to entities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AboutTarget {
    /// Lore is about a character (PC or NPC)
    Character { character_id: CharacterId },
    /// Lore is about a location
    Location { location_id: LocationId },
    /// Lore is about a region (part of a location)
    Region { region_id: RegionId },
    /// Lore is about an item
    Item { item_id: ItemId },
}

impl AboutTarget {
    /// Convert to a string representation for edge type selection
    pub fn edge_type(&self) -> &'static str {
        match self {
            AboutTarget::Character { .. } => "ABOUT_CHARACTER",
            AboutTarget::Location { .. } => "ABOUT_LOCATION",
            AboutTarget::Region { .. } => "ABOUT_REGION",
            AboutTarget::Item { .. } => "ABOUT_ITEM",
        }
    }

    /// Get the ID as a string for Cypher queries
    pub fn id_str(&self) -> String {
        match self {
            AboutTarget::Character { character_id } => character_id.to_string(),
            AboutTarget::Location { location_id } => location_id.to_string(),
            AboutTarget::Region { region_id } => region_id.to_string(),
            AboutTarget::Item { item_id } => item_id.to_string(),
        }
    }

    /// Get the node label for the target
    pub fn node_label(&self) -> &'static str {
        match self {
            AboutTarget::Character { .. } => "Character",
            AboutTarget::Location { .. } => "Location",
            AboutTarget::Region { .. } => "Region",
            AboutTarget::Item { .. } => "Item",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_target_edge_type() {
        let target = AboutTarget::Character {
            character_id: CharacterId::new(),
        };
        assert_eq!(target.edge_type(), "ABOUT_CHARACTER");
    }

    #[test]
    fn about_target_id_str() {
        let char_id = CharacterId::new();
        let target = AboutTarget::Character { character_id };
        assert_eq!(target.id_str(), char_id.to_string());
    }
}
