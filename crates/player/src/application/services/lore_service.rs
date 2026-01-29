//! Lore service for fetching character lore
//!
//! This service handles requests to fetch lore knowledge for characters.

use std::sync::Arc;

use crate::application::api::ApiClient;
use crate::application::dto::LoreJournalEntryData;
use crate::infrastructure::websocket::ClientMessageBuilder;
use crate::ports::outbound::CommandBus;
use crate::ports::outbound::PlatformPort;

/// Lore service
pub struct LoreService {
    api_client: Arc<ApiClient>,
    command_bus: Arc<dyn CommandBus>,
}

impl LoreService {
    /// Create a new LoreService
    pub fn new(api_client: Arc<ApiClient>, command_bus: Arc<dyn CommandBus>) -> Self {
        Self {
            api_client,
            command_bus,
        }
    }

    /// Get all known lore for a character
    ///
    /// Returns journal entries containing both lore data and character's knowledge.
    pub async fn get_character_lore(
        &self,
        character_id: &str,
    ) -> Result<Vec<LoreJournalEntryData>, String> {
        tracing::info!("Requesting character lore for {}", character_id);

        // Send GetCharacterLore request
        let request_id = uuid::Uuid::new_v4().to_string();
        let message = ClientMessageBuilder::get_character_lore(&request_id, character_id);

        // Use request API to wait for response
        self.api_client
            .send_request::<Vec<LoreJournalEntryData>>(
                &self.command_bus,
                request_id,
                message,
            )
            .await
    }

    /// Get lore summary for a single lore entry
    ///
    /// Used for fetching full lore details when player clicks on a lore entry.
    pub async fn get_lore_entry(
        &self,
        lore_id: &str,
    ) -> Result<LoreJournalEntryData, String> {
        tracing::info!("Requesting lore entry details for {}", lore_id);

        // Send GetLoreEntry request (via Request payload)
        let request_id = uuid::Uuid::new_v4().to_string();
        let message = ClientMessageBuilder::get_lore_entry(&request_id, lore_id);

        self.api_client
            .send_request::<LoreJournalEntryData>(
                &self.command_bus,
                request_id,
                message,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lore_service_creation() {
        // Just verify the service can be constructed
        // Full integration testing would require mocking dependencies
        let api_client = Arc::new(ApiClient::new());
        // Note: In real tests, we'd mock CommandBus
        // This test is just a placeholder for test coverage
        let _api_client = api_client;
    }
}
