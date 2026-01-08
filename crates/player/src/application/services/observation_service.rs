//! Observation Service - Application service for NPC observations
//!
//! US-OBS-004/005: Fetch and manage PC observations of NPCs via WebSocket.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::ports::outbound::GameConnectionPort;
use wrldbldr_protocol::{ObservationRequest, RequestPayload};

/// Summary of an NPC observation from the engine
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservationSummary {
    pub npc_id: String,
    pub npc_name: String,
    pub npc_portrait: Option<String>,
    pub location_name: String,
    pub region_name: String,
    pub game_time: String,
    pub observation_type: String,
    pub observation_type_icon: String,
    pub notes: Option<String>,
}

/// Observation service for managing NPC observations
///
/// This service provides methods for observation-related operations
/// using WebSocket request/response pattern via the `GameConnectionPort`.
#[derive(Clone)]
pub struct ObservationService {
    connection: Arc<dyn GameConnectionPort>,
}

impl ObservationService {
    /// Create a new ObservationService with the given connection
    pub fn new(connection: Arc<dyn GameConnectionPort>) -> Self {
        Self { connection }
    }

    /// Get all observations for a player character
    pub async fn list_observations(
        &self,
        pc_id: &str,
    ) -> Result<Vec<ObservationSummary>, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::Observation(ObservationRequest::ListObservations {
                    pc_id: pc_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }
}
