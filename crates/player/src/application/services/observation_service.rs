//! Observation Service - Application service for NPC observations
//!
//! US-OBS-004/005: Fetch and manage PC observations of NPCs.

use serde::{Deserialize, Serialize};

use crate::application::ports::outbound::{ApiError, ApiPort};

/// Summary of an NPC observation from the API
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
pub struct ObservationService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> ObservationService<A> {
    /// Create a new ObservationService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    /// Get all observations for a player character
    pub async fn list_observations(
        &self,
        pc_id: &str,
    ) -> Result<Vec<ObservationSummary>, ApiError> {
        let path = format!("/api/player-characters/{}/observations", pc_id);
        self.api.get(&path).await
    }
}

impl<A: ApiPort + Clone> Clone for ObservationService<A> {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}
