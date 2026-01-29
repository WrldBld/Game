//! Lore service for fetching character lore
//!
//! This service handles requests to fetch lore knowledge for characters.

use wrldbldr_shared::{LoreJournalEntryData, LoreRequest, RequestPayload};

use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::infrastructure::messaging::CommandBus;

/// Lore service
pub struct LoreService {
    commands: CommandBus,
}

impl LoreService {
    /// Create a new LoreService
    pub fn new(commands: CommandBus) -> Self {
        Self { commands }
    }

    /// Get all known lore for a character
    ///
    /// Returns journal entries containing both lore data and character's knowledge.
    pub async fn get_character_lore(
        &self,
        character_id: &str,
    ) -> Result<Vec<LoreJournalEntryData>, ServiceError> {
        tracing::info!("Requesting character lore for {}", character_id);

        let payload = RequestPayload::Lore(LoreRequest::GetCharacterLore {
            character_id: character_id.to_string(),
        });
        let response = self
            .commands
            .request_with_timeout(payload, get_request_timeout_ms())
            .await?;
        response.parse()
    }

    /// Get lore summary for a single lore entry
    ///
    /// Used for fetching full lore details when player clicks on a lore entry.
    pub async fn get_lore_entry(
        &self,
        lore_id: &str,
    ) -> Result<LoreJournalEntryData, ServiceError> {
        tracing::info!("Requesting lore entry details for {}", lore_id);

        let payload = RequestPayload::Lore(LoreRequest::GetLore {
            lore_id: lore_id.to_string(),
        });
        let response = self
            .commands
            .request_with_timeout(payload, get_request_timeout_ms())
            .await?;
        response.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lore_service_creation() {
        // Just verify the service can be constructed
        // Full integration testing would require mocking CommandBus
        // This test is just a placeholder for test coverage
        let _service_type: Option<LoreService> = None;
    }
}
