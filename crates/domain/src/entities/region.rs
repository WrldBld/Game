//! Region entity - Sub-locations within a Location
//!
//! Regions represent distinct areas within a location, each with their own
//! backdrop image for scene display. Think of them as "screens" in a JRPG.
//!
//! # Neo4j Relationships
//! - `(Location)-[:HAS_REGION]->(Region)` - Containment
//! - `(Region)-[:CONNECTED_TO_REGION]->(Region)` - Internal navigation
//! - `(Region)-[:EXITS_TO_LOCATION]->(Location)` - Exit to another location
//! - `(Character)-[:WORKS_AT_REGION]->(Region)` - NPC works here
//! - `(Character)-[:FREQUENTS_REGION]->(Region)` - NPC hangs out here
//! - `(Character)-[:HOME_REGION]->(Region)` - NPC lives here
//! - `(Character)-[:AVOIDS_REGION]->(Region)` - NPC avoids this place

use serde::{Deserialize, Serialize};
use wrldbldr_domain::{LocationId, RegionId};

use crate::value_objects::{AssetPath, Atmosphere, RegionName};

/// A region within a location - represents a distinct "screen" or area
///
/// Regions are the leaf nodes of the location hierarchy. Players navigate
/// between regions, and scenes are derived from the current region's backdrop
/// plus any NPCs present.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    id: RegionId,
    location_id: LocationId,
    name: RegionName,
    description: String,

    // Scene display (visual novel view)
    /// Path to backdrop image for this region's scene
    backdrop_asset: Option<AssetPath>,
    /// Sensory/emotional description of the region's atmosphere
    atmosphere: Option<Atmosphere>,

    // Position on parent location's map (clickable area)
    /// Bounds defining where this region is on the parent location's map
    map_bounds: Option<MapBounds>,

    /// Whether players can spawn here when creating a new PC
    is_spawn_point: bool,
    /// Display order within the location
    order: u32,
}

impl Region {
    /// Create a new region within a location
    pub fn new(location_id: LocationId, name: RegionName) -> Self {
        Self {
            id: RegionId::new(),
            location_id,
            name,
            description: String::new(),
            backdrop_asset: None,
            atmosphere: None,
            map_bounds: None,
            is_spawn_point: false,
            order: 0,
        }
    }

    /// Create a region with a specific ID (for reconstitution from storage)
    pub fn from_parts(
        id: RegionId,
        location_id: LocationId,
        name: RegionName,
        description: String,
        backdrop_asset: Option<AssetPath>,
        atmosphere: Option<Atmosphere>,
        map_bounds: Option<MapBounds>,
        is_spawn_point: bool,
        order: u32,
    ) -> Self {
        Self {
            id,
            location_id,
            name,
            description,
            backdrop_asset,
            atmosphere,
            map_bounds,
            is_spawn_point,
            order,
        }
    }

    // Read-only accessors

    pub fn id(&self) -> RegionId {
        self.id
    }

    pub fn location_id(&self) -> LocationId {
        self.location_id
    }

    pub fn name(&self) -> &RegionName {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn backdrop_asset(&self) -> Option<&AssetPath> {
        self.backdrop_asset.as_ref()
    }

    pub fn atmosphere(&self) -> Option<&Atmosphere> {
        self.atmosphere.as_ref()
    }

    pub fn map_bounds(&self) -> Option<&MapBounds> {
        self.map_bounds.as_ref()
    }

    pub fn is_spawn_point(&self) -> bool {
        self.is_spawn_point
    }

    pub fn order(&self) -> u32 {
        self.order
    }

    // Builder methods

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_backdrop(mut self, asset_path: AssetPath) -> Self {
        self.backdrop_asset = Some(asset_path);
        self
    }

    pub fn with_atmosphere(mut self, atmosphere: Atmosphere) -> Self {
        self.atmosphere = Some(atmosphere);
        self
    }

    pub fn with_map_bounds(mut self, bounds: MapBounds) -> Self {
        self.map_bounds = Some(bounds);
        self
    }

    pub fn as_spawn_point(mut self) -> Self {
        self.is_spawn_point = true;
        self
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }

    /// Check if a pixel position is within this region's map bounds
    pub fn contains_point(&self, x: u32, y: u32) -> bool {
        if let Some(bounds) = &self.map_bounds {
            bounds.contains(x, y)
        } else {
            false
        }
    }
}

/// Bounds defining a rectangular area on a map image
///
/// Simple data struct with public fields (ADR-008: no invariants to protect).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapBounds {
    /// X coordinate of the region's top-left corner
    pub x: u32,
    /// Y coordinate of the region's top-left corner
    pub y: u32,
    /// Width of the region
    pub width: u32,
    /// Height of the region
    pub height: u32,
}

impl MapBounds {
    /// Create new map bounds
    ///
    /// Returns `None` if width or height is zero (invalid bounds).
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        Some(Self {
            x,
            y,
            width,
            height,
        })
    }

    /// Check if a pixel position is within these bounds
    ///
    /// Uses saturating arithmetic to prevent integer overflow.
    /// Returns false for zero-size bounds.
    pub fn contains(&self, px: u32, py: u32) -> bool {
        // Zero-size bounds contain nothing
        if self.width == 0 || self.height == 0 {
            return false;
        }
        px >= self.x
            && px < self.x.saturating_add(self.width)
            && py >= self.y
            && py < self.y.saturating_add(self.height)
    }
}

/// A connection between two regions
///
/// Stored as a `CONNECTED_TO_REGION` edge in Neo4j with properties.
/// Simple data struct with public fields (ADR-008: no invariants to protect).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionConnection {
    pub from_region: RegionId,
    pub to_region: RegionId,
    /// Description of the path/transition (e.g., "A door leads to...")
    pub description: Option<String>,
    /// Whether this connection works both ways
    pub bidirectional: bool,
    /// Whether this connection is currently locked
    pub is_locked: bool,
    /// Description of what's needed to unlock (if locked)
    pub lock_description: Option<String>,
}

impl RegionConnection {
    /// Create a connection from parts (for reconstitution from storage)
    pub fn from_parts(
        from_region: RegionId,
        to_region: RegionId,
        description: Option<String>,
        bidirectional: bool,
        is_locked: bool,
        lock_description: Option<String>,
    ) -> Self {
        Self {
            from_region,
            to_region,
            description,
            bidirectional,
            is_locked,
            lock_description,
        }
    }
}

/// An exit from a region to another location
///
/// Stored as an `EXITS_TO_LOCATION` edge in Neo4j with properties.
/// Used when leaving a building/area to go to a parent or sibling location.
/// Simple data struct with public fields (ADR-008: no invariants to protect).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionExit {
    pub from_region: RegionId,
    pub to_location: LocationId,
    /// Which region in the target location the player arrives at
    pub arrival_region_id: RegionId,
    /// Description of the exit (e.g., "Step outside into the market")
    pub description: Option<String>,
    /// Whether this exit works both ways (can enter from that location)
    pub bidirectional: bool,
}

impl RegionExit {
    /// Create a region exit from parts (for reconstitution from storage)
    pub fn from_parts(
        from_region: RegionId,
        to_location: LocationId,
        arrival_region_id: RegionId,
        description: Option<String>,
        bidirectional: bool,
    ) -> Self {
        Self {
            from_region,
            to_location,
            arrival_region_id,
            description,
            bidirectional,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Issue 6.1: Integer Overflow in MapBounds.contains()
    // ==========================================================================

    #[test]
    fn test_map_bounds_contains_normal_case() {
        let bounds = MapBounds::new(10, 20, 100, 50).unwrap();
        // Inside bounds
        assert!(bounds.contains(10, 20)); // Top-left corner
        assert!(bounds.contains(109, 69)); // Just inside bottom-right
        assert!(bounds.contains(50, 40)); // Middle

        // Outside bounds
        assert!(!bounds.contains(9, 20)); // Left of bounds
        assert!(!bounds.contains(10, 19)); // Above bounds
        assert!(!bounds.contains(110, 20)); // Right of bounds
        assert!(!bounds.contains(10, 70)); // Below bounds
    }

    #[test]
    fn test_map_bounds_contains_near_max_values() {
        // Test with values near u32::MAX to verify saturating_add works
        let bounds = MapBounds {
            x: u32::MAX - 10,
            y: u32::MAX - 10,
            width: 100,
            height: 100,
        };

        // Point at the origin of bounds should be contained
        assert!(bounds.contains(u32::MAX - 10, u32::MAX - 10));

        // Point at MAX-1 should be contained (within saturated range)
        // Note: saturating_add(u32::MAX - 10, 100) = u32::MAX
        // So the check is px < u32::MAX, meaning u32::MAX-1 is the last valid point
        assert!(bounds.contains(u32::MAX - 1, u32::MAX - 1));

        // Point at exactly MAX is NOT contained because px < MAX.saturating_add(width)
        // becomes px < MAX when it saturates, and MAX < MAX is false
        assert!(!bounds.contains(u32::MAX, u32::MAX));

        // Point before bounds should NOT be contained
        assert!(!bounds.contains(u32::MAX - 11, u32::MAX - 10));
        assert!(!bounds.contains(u32::MAX - 10, u32::MAX - 11));
    }

    #[test]
    fn test_map_bounds_overflow_protection() {
        // Create bounds at MAX position with width that would overflow
        let bounds = MapBounds {
            x: u32::MAX,
            y: u32::MAX,
            width: 10,
            height: 10,
        };

        // This should NOT panic due to overflow - saturating_add prevents it
        // Point at MAX is contained since x >= MAX and x < MAX.saturating_add(10) = MAX
        // Wait, saturating_add(MAX, 10) = MAX, so x < MAX is false when x = MAX
        // Actually the point AT u32::MAX should be at the boundary
        assert!(!bounds.contains(u32::MAX, u32::MAX)); // MAX is not < MAX
    }

    // ==========================================================================
    // Issue 6.4: Validate Zero-Size MapBounds
    // ==========================================================================

    #[test]
    fn test_map_bounds_new_rejects_zero_width() {
        assert!(MapBounds::new(10, 20, 0, 50).is_none());
    }

    #[test]
    fn test_map_bounds_new_rejects_zero_height() {
        assert!(MapBounds::new(10, 20, 100, 0).is_none());
    }

    #[test]
    fn test_map_bounds_new_rejects_both_zero() {
        assert!(MapBounds::new(10, 20, 0, 0).is_none());
    }

    #[test]
    fn test_map_bounds_new_accepts_valid_bounds() {
        let bounds = MapBounds::new(10, 20, 100, 50);
        assert!(bounds.is_some());
        let bounds = bounds.unwrap();
        assert_eq!(bounds.x, 10);
        assert_eq!(bounds.y, 20);
        assert_eq!(bounds.width, 100);
        assert_eq!(bounds.height, 50);
    }

    #[test]
    fn test_map_bounds_contains_returns_false_for_zero_size() {
        // Even with zero-size bounds, contains should handle gracefully
        let zero_width = MapBounds {
            x: 10,
            y: 20,
            width: 0,
            height: 50,
        };
        assert!(!zero_width.contains(10, 20));

        let zero_height = MapBounds {
            x: 10,
            y: 20,
            width: 100,
            height: 0,
        };
        assert!(!zero_height.contains(10, 20));

        let zero_both = MapBounds {
            x: 10,
            y: 20,
            width: 0,
            height: 0,
        };
        assert!(!zero_both.contains(10, 20));
    }

    // ==========================================================================
    // RegionConnection tests
    // ==========================================================================

    #[test]
    fn test_region_connection_field_access() {
        let from_region = RegionId::new();
        let to_region = RegionId::new();
        let connection = RegionConnection {
            from_region,
            to_region,
            description: Some("A narrow passage".to_string()),
            bidirectional: false,
            is_locked: true,
            lock_description: Some("Requires a key".to_string()),
        };

        assert_eq!(connection.from_region, from_region);
        assert_eq!(connection.to_region, to_region);
        assert!(!connection.bidirectional);
        assert_eq!(connection.description.as_deref(), Some("A narrow passage"));
        assert!(connection.is_locked);
        assert_eq!(
            connection.lock_description.as_deref(),
            Some("Requires a key")
        );
    }
}
