//! Visual State Service - Application service for visual state management
//!
//! This service provides use case implementations for fetching visual state details.
//! It abstracts away WebSocket client details from the presentation layer.

use uuid::Uuid;

use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::infrastructure::messaging::CommandBus;
use wrldbldr_shared::{
    requests::visual_state::{
        GeneratedVisualStateData, GenerateVisualStateRequest, GetVisualStateDetailsRequest,
        LocationStateData, RegionStateData, VisualStateRequest,
    },
    RequestPayload,
};

/// Visual state details result
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VisualStateDetailResult {
    /// Location state data (if requested and found)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location_state: Option<LocationStateData>,
    /// Region state data (if requested and found)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_state: Option<RegionStateData>,
}

/// Visual state service for managing visual states
///
/// This service provides methods for visual state-related operations
/// while depending only on `CommandBus`, not concrete
/// infrastructure implementations.
#[derive(Clone)]
pub struct VisualStateService {
    commands: CommandBus,
}

impl VisualStateService {
    /// Create a new VisualStateService with given command bus
    pub fn new(commands: CommandBus) -> Self {
        Self { commands }
    }

    /// Get details for a location and/or region visual state.
    ///
    /// Supports fetching a single state (location or region) or both states
    /// simultaneously. When both IDs are provided, two separate requests are
    /// issued and the results are combined.
    ///
    /// # Arguments
    /// * `location_state_id` - Optional location state ID to fetch
    /// * `region_state_id` - Optional region state ID to fetch
    ///
    /// # Returns
    /// A `VisualStateDetailResult` containing the requested state data
    ///
    /// # Errors
    /// Returns `ServiceError::ParseError` if neither ID is provided
    pub async fn get_details(
        &self,
        location_state_id: Option<Uuid>,
        region_state_id: Option<Uuid>,
    ) -> Result<VisualStateDetailResult, ServiceError> {
        // Reject requests where neither ID is provided
        if location_state_id.is_none() && region_state_id.is_none() {
            return Err(ServiceError::ParseError(
                "No state ID provided; either location_state_id or region_state_id must be specified".to_string()
            ));
        }

        // If both IDs are provided, issue two separate requests
        if location_state_id.is_some() && region_state_id.is_some() {
            // Issue location state request
            let location_request = GetVisualStateDetailsRequest {
                location_state_id,
                region_state_id: None,
            };
            let location_result = self
                .commands
                .request_with_timeout(
                    RequestPayload::VisualState(VisualStateRequest::GetDetails { request: location_request }),
                    get_request_timeout_ms(),
                )
                .await;

            // Issue region state request
            let region_request = GetVisualStateDetailsRequest {
                location_state_id: None,
                region_state_id,
            };
            let region_result = self
                .commands
                .request_with_timeout(
                    RequestPayload::VisualState(VisualStateRequest::GetDetails { request: region_request }),
                    get_request_timeout_ms(),
                )
                .await;

            // Parse results - fail fast if either request fails
            let location_state = match location_result {
                Ok(result) => Some(result.parse::<LocationStateData>()?),
                Err(e) => return Err(e.into()),
            };

            let region_state = match region_result {
                Ok(result) => Some(result.parse::<RegionStateData>()?),
                Err(e) => return Err(e.into()),
            };

            return Ok(VisualStateDetailResult {
                location_state,
                region_state,
            });
        }

        // Single ID request - preserve original behavior
        let request = GetVisualStateDetailsRequest {
            location_state_id,
            region_state_id,
        };

        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::VisualState(VisualStateRequest::GetDetails { request }),
                get_request_timeout_ms(),
            )
            .await?;

        // Server returns either LocationStateData or RegionStateData depending on
        // which ID was requested. Try parsing as each type.
        if location_state_id.is_some() {
            // Location state was requested - parse as LocationStateData
            let location_state: LocationStateData = result.parse()?;
            Ok(VisualStateDetailResult {
                location_state: Some(location_state),
                region_state: None,
            })
        } else {
            // Region state was requested - parse as RegionStateData
            let region_state: RegionStateData = result.parse()?;
            Ok(VisualStateDetailResult {
                location_state: None,
                region_state: Some(region_state),
            })
        }
    }

    /// Generate a new visual state with AI-generated assets
    ///
    /// # Arguments
    /// * `request` - The generation request containing state type, name, description,
    ///   prompt, and generation options
    ///
    /// # Returns
    /// A `GeneratedVisualStateData` containing the created state (location or region)
    /// and generation batch ID for tracking progress
    pub async fn generate_state(
        &self,
        request: GenerateVisualStateRequest,
    ) -> Result<GeneratedVisualStateData, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::VisualState(VisualStateRequest::Generate { request }),
                get_request_timeout_ms(),
            )
            .await?;

        // Parse response - server returns GeneratedVisualStateData
        let generated_data: GeneratedVisualStateData = result.parse()?;

        Ok(generated_data)
    }
}
