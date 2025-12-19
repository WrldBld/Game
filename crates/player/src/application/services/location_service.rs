//! Location Service - Application service for location management
//!
//! This service provides use case implementations for listing, creating,
//! updating, and fetching locations. It abstracts away the HTTP client
//! details from the presentation layer.

use serde::{Deserialize, Serialize};

use crate::application::ports::outbound::{ApiError, ApiPort};

/// Location summary for list views
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocationSummary {
    pub id: String,
    pub name: String,
    pub location_type: Option<String>,
}

/// Full location data for create/edit forms via API
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocationFormData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atmosphere: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notable_features: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden_secrets: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_location_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backdrop_asset: Option<String>,
    #[serde(default)]
    pub backdrop_regions: Vec<serde_json::Value>,
    /// Default TTL in hours for staging cache in this location (default: 4 hours)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_cache_ttl_hours: Option<i32>,
}

/// Location connection data
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConnectionData {
    pub from_location_id: String,
    pub to_location_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_bidirectional")]
    pub bidirectional: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub travel_time: Option<u32>,
}

fn default_bidirectional() -> bool {
    true
}

/// Region data with map bounds (for mini-map)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegionData {
    pub id: String,
    pub location_id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub backdrop_asset: Option<String>,
    pub atmosphere: Option<String>,
    pub map_bounds: Option<MapBoundsData>,
    #[serde(default)]
    pub is_spawn_point: bool,
    #[serde(default)]
    pub order: u32,
}

/// Map bounds for positioning regions
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapBoundsData {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Location service for managing locations
///
/// This service provides methods for location-related operations
/// while depending only on the `ApiPort` trait, not concrete
/// infrastructure implementations.
pub struct LocationService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> LocationService<A> {
    /// Create a new LocationService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    /// List all locations in a world
    pub async fn list_locations(&self, world_id: &str) -> Result<Vec<LocationSummary>, ApiError> {
        let path = format!("/api/worlds/{}/locations", world_id);
        self.api.get(&path).await
    }

    /// Get a single location by ID
    pub async fn get_location(
        &self,
        _world_id: &str, // Not used in API endpoint, but kept for API compatibility
        location_id: &str,
    ) -> Result<LocationFormData, ApiError> {
        let path = format!("/api/locations/{}", location_id);
        self.api.get(&path).await
    }

    /// Create a new location
    pub async fn create_location(
        &self,
        world_id: &str,
        location: &LocationFormData,
    ) -> Result<LocationFormData, ApiError> {
        let path = format!("/api/worlds/{}/locations", world_id);
        self.api.post(&path, location).await
    }

    /// Update an existing location
    pub async fn update_location(
        &self,
        location_id: &str,
        location: &LocationFormData,
    ) -> Result<LocationFormData, ApiError> {
        let path = format!("/api/locations/{}", location_id);
        self.api.put(&path, location).await
    }

    /// Delete a location
    pub async fn delete_location(&self, location_id: &str) -> Result<(), ApiError> {
        let path = format!("/api/locations/{}", location_id);
        self.api.delete(&path).await
    }

    /// Get connections from a location
    pub async fn get_connections(
        &self,
        location_id: &str,
    ) -> Result<Vec<ConnectionData>, ApiError> {
        let path = format!("/api/locations/{}/connections", location_id);
        self.api.get(&path).await
    }

    /// Create a connection between locations
    pub async fn create_connection(&self, connection: &ConnectionData) -> Result<(), ApiError> {
        self.api
            .post_no_response("/api/connections", connection)
            .await
    }

    /// Get all regions for a location (with map bounds)
    pub async fn get_regions(&self, location_id: &str) -> Result<Vec<RegionData>, ApiError> {
        let path = format!("/api/locations/{}/regions", location_id);
        self.api.get(&path).await
    }
}

impl<A: ApiPort + Clone> Clone for LocationService<A> {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}
