//! Location aggregate - Physical or conceptual places in the world
//!
//! # Graph-First Design (Phase 0.C)
//!
//! The following relationships are stored as Neo4j edges, NOT embedded fields:
//! - Parent/child: `(parent)-[:CONTAINS_LOCATION]->(child)`
//! - Navigation: `(from)-[:CONNECTED_TO]->(to)`
//! - Regions: `(location)-[:HAS_REGION]->(region)`
//! - Grid map: `(location)-[:HAS_TACTICAL_MAP]->(map)`
//!
//! # Rustic DDD Design
//!
//! This aggregate follows Rustic DDD principles:
//! - **Private fields**: All fields are encapsulated
//! - **Newtypes**: `LocationName` and `Description` for validated strings
//! - **Valid by construction**: `new()` takes pre-validated types
//! - **Builder pattern**: Fluent API for optional fields

use serde::{Deserialize, Serialize};

use crate::value_objects::{AssetPath, Atmosphere, Description, LocationName, PresenceTtlHours};
use wrldbldr_domain::{LocationId, RegionId, WorldId};

// Re-export from entities for now (MapBounds, LocationType)
pub use crate::entities::{ConnectionType, LocationConnection, LocationType, MapBounds};

/// A location in the world
///
/// # Invariants
///
/// - `name` is always non-empty and <= 200 characters (enforced by `LocationName`)
/// - `description` is always <= 5000 characters (enforced by `Description`)
///
/// # Example
///
/// ```
/// use wrldbldr_domain::{WorldId, LocationId};
/// use wrldbldr_domain::aggregates::location::{Location, LocationType};
/// use wrldbldr_domain::value_objects::{LocationName, Description};
///
/// let world_id = WorldId::new();
/// let name = LocationName::new("The Prancing Pony").unwrap();
/// let location = Location::new(world_id, name, LocationType::Interior);
///
/// assert_eq!(location.name().as_str(), "The Prancing Pony");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    // Identity
    id: LocationId,
    world_id: WorldId,

    // Core attributes (newtypes)
    name: LocationName,
    description: Description,
    location_type: LocationType,

    // Visual assets
    /// Path to the default backdrop image asset (used if entering without specific region)
    backdrop_asset: Option<AssetPath>,
    /// Path to the top-down map image for navigation between regions
    map_asset: Option<AssetPath>,

    // Position on parent location's map (if this location is nested)
    /// Bounds defining where this location appears on its parent's map
    parent_map_bounds: Option<MapBounds>,

    // Default entry point
    /// Default region to place players when arriving without a specific region target
    default_region_id: Option<RegionId>,

    /// Sensory/emotional description of the location's atmosphere
    atmosphere: Option<Atmosphere>,

    // Staging settings
    /// Default staging duration in game hours (default: 3 hours)
    presence_cache_ttl_hours: PresenceTtlHours,
    /// Whether to use LLM for staging decisions (default: true)
    use_llm_presence: bool,
}

impl Location {
    // =========================================================================
    // Constructor
    // =========================================================================

    /// Create a new location with the given world, name, and type.
    ///
    /// The `name` parameter must be a pre-validated `LocationName` - validation
    /// happens when creating the `LocationName`, not here.
    ///
    /// # Example
    ///
    /// ```
    /// use wrldbldr_domain::{WorldId, LocationId};
    /// use wrldbldr_domain::aggregates::location::{Location, LocationType};
    /// use wrldbldr_domain::value_objects::LocationName;
    ///
    /// let world_id = WorldId::new();
    /// let name = LocationName::new("Rivendell").unwrap();
    /// let location = Location::new(world_id, name, LocationType::Exterior);
    ///
    /// assert_eq!(location.name().as_str(), "Rivendell");
    /// ```
    pub fn new(world_id: WorldId, name: LocationName, location_type: LocationType) -> Self {
        Self {
            id: LocationId::new(),
            world_id,
            name,
            description: Description::empty(),
            location_type,
            backdrop_asset: None,
            map_asset: None,
            parent_map_bounds: None,
            default_region_id: None,
            atmosphere: None,
            presence_cache_ttl_hours: PresenceTtlHours::default(),
            use_llm_presence: true,
        }
    }

    // =========================================================================
    // Identity Accessors (read-only)
    // =========================================================================

    /// Returns the location's unique identifier.
    #[inline]
    pub fn id(&self) -> LocationId {
        self.id
    }

    /// Returns the ID of the world this location belongs to.
    #[inline]
    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    /// Returns the location's name.
    #[inline]
    pub fn name(&self) -> &LocationName {
        &self.name
    }

    /// Returns the location's description.
    #[inline]
    pub fn description(&self) -> &Description {
        &self.description
    }

    /// Returns the location's type.
    #[inline]
    pub fn location_type(&self) -> LocationType {
        self.location_type
    }

    // =========================================================================
    // Asset Accessors
    // =========================================================================

    /// Returns the path to the location's backdrop asset, if any.
    #[inline]
    pub fn backdrop_asset(&self) -> Option<&AssetPath> {
        self.backdrop_asset.as_ref()
    }

    /// Returns the path to the location's map asset, if any.
    #[inline]
    pub fn map_asset(&self) -> Option<&AssetPath> {
        self.map_asset.as_ref()
    }

    // =========================================================================
    // Map/Navigation Accessors
    // =========================================================================

    /// Returns the bounds of this location on its parent's map, if any.
    #[inline]
    pub fn parent_map_bounds(&self) -> Option<&MapBounds> {
        self.parent_map_bounds.as_ref()
    }

    /// Returns the default region ID for this location, if set.
    #[inline]
    pub fn default_region_id(&self) -> Option<RegionId> {
        self.default_region_id
    }

    /// Returns the atmosphere description, if any.
    #[inline]
    pub fn atmosphere(&self) -> Option<&Atmosphere> {
        self.atmosphere.as_ref()
    }

    // =========================================================================
    // Staging Settings Accessors
    // =========================================================================

    /// Returns the presence cache TTL in hours.
    #[inline]
    pub fn presence_cache_ttl_hours(&self) -> i32 {
        self.presence_cache_ttl_hours.value()
    }

    /// Returns whether LLM is used for presence decisions.
    #[inline]
    pub fn use_llm_presence(&self) -> bool {
        self.use_llm_presence
    }

    // =========================================================================
    // Builder Methods (for construction)
    // =========================================================================

    /// Set the location's description.
    pub fn with_description(mut self, description: Description) -> Self {
        self.description = description;
        self
    }

    /// Set the location's backdrop asset path.
    pub fn with_backdrop(mut self, asset_path: AssetPath) -> Self {
        self.backdrop_asset = Some(asset_path);
        self
    }

    /// Set the location's map asset path.
    pub fn with_map(mut self, asset_path: AssetPath) -> Self {
        self.map_asset = Some(asset_path);
        self
    }

    /// Set the location's bounds on its parent's map.
    pub fn with_parent_map_bounds(mut self, bounds: MapBounds) -> Self {
        self.parent_map_bounds = Some(bounds);
        self
    }

    /// Set the location's default region.
    pub fn with_default_region(mut self, region_id: RegionId) -> Self {
        self.default_region_id = Some(region_id);
        self
    }

    /// Set the location's atmosphere description.
    pub fn with_atmosphere(mut self, atmosphere: Atmosphere) -> Self {
        self.atmosphere = Some(atmosphere);
        self
    }

    /// Set the presence cache TTL in hours.
    pub fn with_presence_ttl(mut self, hours: i32) -> Self {
        self.presence_cache_ttl_hours = PresenceTtlHours::clamped(hours);
        self
    }

    /// Set the presence cache TTL using validated newtype.
    pub fn with_presence_ttl_validated(mut self, ttl: PresenceTtlHours) -> Self {
        self.presence_cache_ttl_hours = ttl;
        self
    }

    /// Set whether to use LLM for presence decisions.
    pub fn with_llm_presence(mut self, enabled: bool) -> Self {
        self.use_llm_presence = enabled;
        self
    }

    /// Set the location's ID (used when loading from storage).
    pub fn with_id(mut self, id: LocationId) -> Self {
        self.id = id;
        self
    }

    // =========================================================================
    // Mutation Methods
    // =========================================================================

    /// Set the location's name.
    pub fn set_name(&mut self, name: LocationName) {
        self.name = name;
    }

    /// Set the location's description.
    pub fn set_description(&mut self, description: Description) {
        self.description = description;
    }

    /// Set the location's backdrop asset path.
    pub fn set_backdrop(&mut self, path: Option<AssetPath>) {
        self.backdrop_asset = path;
    }

    /// Set the location's map asset path.
    pub fn set_map(&mut self, path: Option<AssetPath>) {
        self.map_asset = path;
    }

    /// Set the location's atmosphere description.
    pub fn set_atmosphere(&mut self, atmosphere: Option<Atmosphere>) {
        self.atmosphere = atmosphere;
    }

    /// Set the default region ID.
    pub fn set_default_region(&mut self, region_id: Option<RegionId>) {
        self.default_region_id = region_id;
    }

    /// Set the parent map bounds.
    pub fn set_parent_map_bounds(&mut self, bounds: Option<MapBounds>) {
        self.parent_map_bounds = bounds;
    }

    /// Set the presence cache TTL.
    pub fn set_presence_ttl(&mut self, hours: i32) {
        self.presence_cache_ttl_hours = PresenceTtlHours::clamped(hours);
    }

    /// Set the presence cache TTL using validated newtype.
    pub fn set_presence_ttl_validated(&mut self, ttl: PresenceTtlHours) {
        self.presence_cache_ttl_hours = ttl;
    }

    /// Set whether to use LLM for presence decisions.
    pub fn set_llm_presence(&mut self, enabled: bool) {
        self.use_llm_presence = enabled;
    }

    // =========================================================================
    // Domain Methods
    // =========================================================================

    /// Check if a pixel position is within this location's parent map bounds.
    pub fn contains_point_on_parent_map(&self, x: u32, y: u32) -> bool {
        if let Some(bounds) = &self.parent_map_bounds {
            bounds.contains(x, y)
        } else {
            false
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_objects::Atmosphere;

    fn create_test_location() -> Location {
        let world_id = WorldId::new();
        let name = LocationName::new("Test Location").unwrap();
        Location::new(world_id, name, LocationType::Interior)
    }

    mod constructor {
        use super::*;

        #[test]
        fn new_creates_location_with_correct_defaults() {
            let world_id = WorldId::new();
            let name = LocationName::new("The Prancing Pony").unwrap();
            let location = Location::new(world_id, name, LocationType::Interior);

            assert_eq!(location.name().as_str(), "The Prancing Pony");
            assert_eq!(location.world_id(), world_id);
            assert!(matches!(location.location_type(), LocationType::Interior));
            assert!(location.description().is_empty());
            assert!(location.backdrop_asset().is_none());
            assert!(location.map_asset().is_none());
            assert!(location.parent_map_bounds().is_none());
            assert!(location.default_region_id().is_none());
            assert!(location.atmosphere().is_none());
            assert_eq!(location.presence_cache_ttl_hours(), 3);
            assert!(location.use_llm_presence());
        }

        #[test]
        fn builder_methods_work() {
            use crate::value_objects::AssetPath;

            let world_id = WorldId::new();
            let name = LocationName::new("Moria").unwrap();
            let desc = Description::new("An ancient dwarven kingdom").unwrap();
            let region_id = RegionId::new();
            let bounds = MapBounds::new(10, 20, 100, 50).unwrap();
            let backdrop = AssetPath::new("backdrops/moria.png").unwrap();
            let map = AssetPath::new("maps/moria.png").unwrap();

            let atm = Atmosphere::new("Dark and echoing").unwrap();
            let location = Location::new(world_id, name, LocationType::Interior)
                .with_description(desc)
                .with_backdrop(backdrop)
                .with_map(map)
                .with_parent_map_bounds(bounds)
                .with_default_region(region_id)
                .with_atmosphere(atm)
                .with_presence_ttl(6)
                .with_llm_presence(false);

            assert_eq!(
                location.description().as_str(),
                "An ancient dwarven kingdom"
            );
            assert_eq!(
                location.backdrop_asset().map(|p| p.as_str()),
                Some("backdrops/moria.png")
            );
            assert_eq!(
                location.map_asset().map(|p| p.as_str()),
                Some("maps/moria.png")
            );
            assert!(location.parent_map_bounds().is_some());
            assert_eq!(location.default_region_id(), Some(region_id));
            assert_eq!(
                location.atmosphere().map(|a| a.as_str()),
                Some("Dark and echoing")
            );
            assert_eq!(location.presence_cache_ttl_hours(), 6);
            assert!(!location.use_llm_presence());
        }
    }

    mod mutation {
        use super::*;

        #[test]
        fn set_description_works() {
            let mut location = create_test_location();
            let desc = Description::new("A cozy tavern").unwrap();
            location.set_description(desc);
            assert_eq!(location.description().as_str(), "A cozy tavern");
        }

        #[test]
        fn set_backdrop_works() {
            use crate::value_objects::AssetPath;

            let mut location = create_test_location();
            location.set_backdrop(Some(AssetPath::new("backdrops/tavern.png").unwrap()));
            assert_eq!(
                location.backdrop_asset().map(|p| p.as_str()),
                Some("backdrops/tavern.png")
            );

            location.set_backdrop(None);
            assert!(location.backdrop_asset().is_none());
        }

        #[test]
        fn set_map_works() {
            use crate::value_objects::AssetPath;

            let mut location = create_test_location();
            location.set_map(Some(AssetPath::new("maps/tavern.png").unwrap()));
            assert_eq!(
                location.map_asset().map(|p| p.as_str()),
                Some("maps/tavern.png")
            );

            location.set_map(None);
            assert!(location.map_asset().is_none());
        }

        #[test]
        fn set_atmosphere_works() {
            let mut location = create_test_location();
            let atm = Atmosphere::new("Warm and inviting").unwrap();
            location.set_atmosphere(Some(atm));
            assert_eq!(
                location.atmosphere().map(|a| a.as_str()),
                Some("Warm and inviting")
            );

            location.set_atmosphere(None);
            assert!(location.atmosphere().is_none());
        }

        #[test]
        fn set_default_region_works() {
            let mut location = create_test_location();
            let region_id = RegionId::new();

            location.set_default_region(Some(region_id));
            assert_eq!(location.default_region_id(), Some(region_id));

            location.set_default_region(None);
            assert!(location.default_region_id().is_none());
        }

        #[test]
        fn set_parent_map_bounds_works() {
            let mut location = create_test_location();
            let bounds = MapBounds::new(10, 20, 100, 50).unwrap();

            location.set_parent_map_bounds(Some(bounds));
            assert!(location.parent_map_bounds().is_some());

            location.set_parent_map_bounds(None);
            assert!(location.parent_map_bounds().is_none());
        }

        #[test]
        fn set_presence_ttl_works() {
            let mut location = create_test_location();
            location.set_presence_ttl(12);
            assert_eq!(location.presence_cache_ttl_hours(), 12);
        }

        #[test]
        fn set_llm_presence_works() {
            let mut location = create_test_location();
            assert!(location.use_llm_presence());

            location.set_llm_presence(false);
            assert!(!location.use_llm_presence());
        }
    }

    mod domain_methods {
        use super::*;

        #[test]
        fn contains_point_on_parent_map_without_bounds_returns_false() {
            let location = create_test_location();
            assert!(!location.contains_point_on_parent_map(50, 50));
        }

        #[test]
        fn contains_point_on_parent_map_with_bounds_works() {
            let world_id = WorldId::new();
            let name = LocationName::new("Test").unwrap();
            let bounds = MapBounds::new(10, 20, 100, 50).unwrap();
            let location = Location::new(world_id, name, LocationType::Interior)
                .with_parent_map_bounds(bounds);

            // Inside bounds
            assert!(location.contains_point_on_parent_map(50, 40));
            assert!(location.contains_point_on_parent_map(10, 20)); // top-left

            // Outside bounds
            assert!(!location.contains_point_on_parent_map(5, 40)); // left of bounds
            assert!(!location.contains_point_on_parent_map(200, 40)); // right of bounds
        }
    }
}
