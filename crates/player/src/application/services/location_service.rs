//! Location Service - Application service for location management
//!
//! This service provides use case implementations for listing, creating,
//! updating, and fetching locations. It abstracts away the WebSocket client
//! details from the presentation layer.

use serde::{Deserialize, Serialize};

use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::infrastructure::messaging::CommandBus;
use wrldbldr_shared::{LocationRequest, RegionListItemData, RegionRequest, RequestPayload};

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
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub location_type: Option<String>,
    /// Setting/atmosphere of the location
    #[serde(default)]
    pub atmosphere: Option<String>,
    #[serde(default)]
    pub notable_features: Option<String>,
    #[serde(default)]
    pub hidden_secrets: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_location_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backdrop_asset: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backdrop_regions: Vec<serde_json::Value>,
    /// Default TTL in hours for staging cache in this location (default: 4 hours)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence_cache_ttl_hours: Option<i32>,
    /// Whether to use LLM for staging decisions in this location (default: true)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_llm_presence: Option<bool>,
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

// From impls for protocol conversion at the boundary
impl LocationFormData {
    fn to_create_data(&self) -> wrldbldr_shared::requests::CreateLocationData {
        wrldbldr_shared::requests::CreateLocationData {
            name: self.name.clone(),
            description: self.description.clone(),
            setting: self.atmosphere.clone(),
        }
    }

    fn to_update_data(&self) -> wrldbldr_shared::requests::UpdateLocationData {
        wrldbldr_shared::requests::UpdateLocationData {
            name: Some(self.name.clone()),
            description: self.description.clone(),
            setting: self.atmosphere.clone(),
            presence_cache_ttl_hours: self.presence_cache_ttl_hours,
            use_llm_presence: self.use_llm_presence,
        }
    }
}

impl ConnectionData {
    fn to_create_data(&self) -> wrldbldr_shared::requests::CreateLocationConnectionData {
        wrldbldr_shared::requests::CreateLocationConnectionData {
            from_id: self.from_location_id.clone(),
            to_id: self.to_location_id.clone(),
            bidirectional: Some(self.bidirectional),
        }
    }
}

// RegionListItemData and MapBoundsData are imported from wrldbldr_shared

/// Location service for managing locations
///
/// This service provides methods for location-related operations
/// while depending only on the `CommandBus`, not concrete
/// infrastructure implementations.
#[derive(Clone)]
pub struct LocationService {
    commands: CommandBus,
}

impl LocationService {
    /// Create a new LocationService with the given command bus
    pub fn new(commands: CommandBus) -> Self {
        Self { commands }
    }

    /// List all locations in a world
    pub async fn list_locations(
        &self,
        world_id: &str,
    ) -> Result<Vec<LocationSummary>, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Location(LocationRequest::ListLocations {
                    world_id: world_id.to_string(),
                    limit: None,
                    offset: None,
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }

    /// Get a single location by ID
    pub async fn get_location(&self, location_id: &str) -> Result<LocationFormData, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Location(LocationRequest::GetLocation {
                    location_id: location_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }

    /// Create a new location
    pub async fn create_location(
        &self,
        world_id: &str,
        location: &LocationFormData,
    ) -> Result<LocationFormData, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Location(LocationRequest::CreateLocation {
                    world_id: world_id.to_string(),
                    data: location.to_create_data(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }

    /// Update an existing location
    pub async fn update_location(
        &self,
        location_id: &str,
        location: &LocationFormData,
    ) -> Result<LocationFormData, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Location(LocationRequest::UpdateLocation {
                    location_id: location_id.to_string(),
                    data: location.to_update_data(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }

    /// Delete a location
    pub async fn delete_location(&self, location_id: &str) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Location(LocationRequest::DeleteLocation {
                    location_id: location_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse_empty()
    }

    /// Get connections from a location
    pub async fn get_connections(
        &self,
        location_id: &str,
    ) -> Result<Vec<ConnectionData>, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Location(LocationRequest::GetLocationConnections {
                    location_id: location_id.to_string(),
                    limit: None,
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }

    /// Create a connection between locations
    pub async fn create_connection(&self, connection: &ConnectionData) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Location(LocationRequest::CreateLocationConnection {
                    data: connection.to_create_data(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse_empty()
    }

    /// Get all regions for a location (with map bounds)
    pub async fn get_regions(
        &self,
        location_id: &str,
    ) -> Result<Vec<RegionListItemData>, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Region(RegionRequest::ListRegions {
                    location_id: location_id.to_string(),
                    limit: None,
                    offset: None,
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }
}
