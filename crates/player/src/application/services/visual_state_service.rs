//! Visual State Service - Application service for visual state management
//!
//! This service provides use case implementations for fetching visual state details.
//! It abstracts away WebSocket client details from the presentation layer.

use uuid::Uuid;

use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::infrastructure::messaging::CommandBus;
use wrldbldr_shared::{
    requests::visual_state::{GetVisualStateDetailsRequest, LocationStateData, RegionStateData, VisualStateRequest},
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

    /// Get details for location and/or region visual states
    ///
    /// # Arguments
    /// * `location_state_id` - Optional location state ID to fetch
    /// * `region_state_id` - Optional region state ID to fetch
    ///
    /// # Returns
    /// A `VisualStateDetailResult` containing the requested state data
    pub async fn get_details(
        &self,
        location_state_id: Option<Uuid>,
        region_state_id: Option<Uuid>,
    ) -> Result<VisualStateDetailResult, ServiceError> {
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

        // Parse response - server returns a VisualStateCatalogData structure
        let catalog_data: wrldbldr_shared::requests::visual_state::VisualStateCatalogData =
            result.parse()?;

        // Extract the requested states from the catalog
        let location_state = location_state_id.and_then(|id| {
            catalog_data
                .location_states
                .into_iter()
                .find(|s| s.id == id)
        });

        let region_state = region_state_id.and_then(|id| {
            catalog_data
                .region_states
                .into_iter()
                .find(|s| s.id == id)
        });

        Ok(VisualStateDetailResult {
            location_state,
            region_state,
        })
    }
}
