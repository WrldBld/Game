//! WebSocket handler context extraction and authorization
//!
//! Provides unified context extraction and authorization checks,
//! eliminating boilerplate across handlers.
//!
//! # Usage
//!
//! ```rust,ignore
//! // In a handler function:
//! let ctx = HandlerContext::extract(state, client_id).await?;
//!
//! // For DM-only operations:
//! let dm_ctx = ctx.require_dm()?;
//!
//! // For player-only operations:
//! let player_ctx = ctx.require_player()?;
//! ```
//!
//! # Architecture
//!
//! This module is part of the adapter layer and provides conversion to
//! use case context types defined in `engine-ports`. The separation ensures:
//! - Adapters handle protocol/transport concerns (ServerMessage errors)
//! - Use cases receive clean domain context (UseCaseContext)

use uuid::Uuid;
use wrldbldr_domain::{CharacterId, ItemId, LocationId, PlayerCharacterId, RegionId, WorldId, ChallengeId, NarrativeEventId, SceneId};
use wrldbldr_protocol::ServerMessage;

use crate::infrastructure::state::AppState;

// =============================================================================
// Context Types
// =============================================================================

/// Extracted context for WebSocket handlers
///
/// Contains all connection-related information needed by handlers.
/// Can be converted to `UseCaseContext` for passing to use cases.
#[derive(Debug, Clone)]
pub struct HandlerContext {
    /// Client ID string for the connection
    pub connection_id: String,
    /// World ID as domain type
    pub world_id: WorldId,
    /// World ID as raw UUID (for compatibility with existing code)
    pub world_id_uuid: Uuid,
    /// User identifier
    pub user_id: String,
    /// Whether this connection is a DM
    pub is_dm: bool,
    /// Player character ID (if this is a player connection)
    pub pc_id: Option<PlayerCharacterId>,
}

/// Context for DM-only operations
///
/// Returned by `HandlerContext::require_dm()` to provide compile-time
/// guarantee that DM authorization has been verified.
#[derive(Debug, Clone)]
pub struct DmContext {
    /// Client ID string for the connection
    pub connection_id: String,
    /// World ID as domain type
    pub world_id: WorldId,
    /// World ID as raw UUID
    pub world_id_uuid: Uuid,
    /// DM's user identifier
    pub user_id: String,
}

/// Context for player-only operations
///
/// Returned by `HandlerContext::require_player()` to provide compile-time
/// guarantee that:
/// 1. User is not a DM
/// 2. User has a selected PC (pc_id is guaranteed to be Some)
#[derive(Debug, Clone)]
pub struct PlayerContext {
    /// Client ID string for the connection
    pub connection_id: String,
    /// World ID as domain type
    pub world_id: WorldId,
    /// World ID as raw UUID
    pub world_id_uuid: Uuid,
    /// Player's user identifier
    pub user_id: String,
    /// Player character ID (guaranteed to exist)
    pub pc_id: PlayerCharacterId,
}

// =============================================================================
// HandlerContext Implementation
// =============================================================================

impl HandlerContext {
    /// Extract context from connection state
    ///
    /// Returns error ServerMessage if:
    /// - Client is not connected
    /// - Client is not in a world
    pub async fn extract(state: &AppState, client_id: Uuid) -> Result<Self, ServerMessage> {
        let client_id_str = client_id.to_string();
        let connection = state
            .world_connection_manager
            .get_connection_by_client_id(&client_id_str)
            .await
            .ok_or_else(|| error_response("NOT_CONNECTED", "Client is not connected"))?;

        let world_id_uuid = connection
            .world_id
            .ok_or_else(|| error_response("NO_WORLD", "Not connected to a world"))?;

        Ok(Self {
            connection_id: client_id_str,
            world_id: WorldId::from_uuid(world_id_uuid),
            world_id_uuid,
            user_id: connection.user_id.clone(),
            is_dm: connection.is_dm(),
            pc_id: connection.pc_id.map(PlayerCharacterId::from_uuid),
        })
    }

    /// Require DM authorization, returning DmContext
    ///
    /// Use this for operations that only DMs can perform.
    pub fn require_dm(self) -> Result<DmContext, ServerMessage> {
        if self.is_dm {
            Ok(DmContext {
                connection_id: self.connection_id,
                world_id: self.world_id,
                world_id_uuid: self.world_id_uuid,
                user_id: self.user_id,
            })
        } else {
            Err(error_response(
                "NOT_AUTHORIZED",
                "Only the DM can perform this action",
            ))
        }
    }

    /// Require player authorization (not DM, has PC), returning PlayerContext
    ///
    /// Use this for operations that only players (non-DMs with selected PCs) can perform.
    pub fn require_player(self) -> Result<PlayerContext, ServerMessage> {
        match (self.is_dm, self.pc_id) {
            (false, Some(pc_id)) => Ok(PlayerContext {
                connection_id: self.connection_id,
                world_id: self.world_id,
                world_id_uuid: self.world_id_uuid,
                user_id: self.user_id,
                pc_id,
            }),
            (true, _) => Err(error_response(
                "NOT_AUTHORIZED",
                "DMs cannot perform player actions",
            )),
            (false, None) => Err(error_response(
                "NO_PC_SELECTED",
                "No player character selected",
            )),
        }
    }

    /// Get PC ID or return error
    ///
    /// Use this when you need a PC ID but don't need full player context validation.
    pub fn require_pc_id(&self) -> Result<PlayerCharacterId, ServerMessage> {
        self.pc_id.ok_or_else(|| {
            error_response("NO_PC_SELECTED", "No player character selected")
        })
    }
}

// =============================================================================
// Error Response Helpers
// =============================================================================

/// Create a ServerMessage::Error response
pub fn error_response(code: &str, message: &str) -> ServerMessage {
    ServerMessage::Error {
        code: code.to_string(),
        message: message.to_string(),
    }
}

/// Create a "not found" error response
pub fn not_found_error(entity: &str, id: &str) -> ServerMessage {
    error_response(
        &format!("{}_NOT_FOUND", entity.to_uppercase()),
        &format!("{} not found: {}", entity, id),
    )
}

/// Create an "invalid ID" error response
pub fn invalid_id_error(entity: &str, id: &str) -> ServerMessage {
    error_response(
        &format!("INVALID_{}_ID", entity.to_uppercase()),
        &format!("Invalid {} ID: {}", entity, id),
    )
}

// =============================================================================
// ID Parsing Helpers
// =============================================================================

/// Parse a string as UUID, returning ServerMessage error on failure
pub fn parse_uuid(id: &str, entity: &str) -> Result<Uuid, ServerMessage> {
    Uuid::parse_str(id).map_err(|_| invalid_id_error(entity, id))
}

/// Parse a string as WorldId
pub fn parse_world_id(id: &str) -> Result<WorldId, ServerMessage> {
    parse_uuid(id, "world").map(WorldId::from_uuid)
}

/// Parse a string as PlayerCharacterId
pub fn parse_player_character_id(id: &str) -> Result<PlayerCharacterId, ServerMessage> {
    parse_uuid(id, "PC").map(PlayerCharacterId::from_uuid)
}

/// Parse a string as RegionId
pub fn parse_region_id(id: &str) -> Result<RegionId, ServerMessage> {
    parse_uuid(id, "region").map(RegionId::from_uuid)
}

/// Parse a string as LocationId
pub fn parse_location_id(id: &str) -> Result<LocationId, ServerMessage> {
    parse_uuid(id, "location").map(LocationId::from_uuid)
}

/// Parse a string as CharacterId (NPC)
pub fn parse_character_id(id: &str) -> Result<CharacterId, ServerMessage> {
    parse_uuid(id, "character").map(CharacterId::from_uuid)
}

/// Parse a string as ItemId
pub fn parse_item_id(id: &str) -> Result<ItemId, ServerMessage> {
    parse_uuid(id, "item").map(ItemId::from_uuid)
}

/// Parse a string as ChallengeId
pub fn parse_challenge_id(id: &str) -> Result<ChallengeId, ServerMessage> {
    parse_uuid(id, "challenge").map(ChallengeId::from_uuid)
}

/// Parse a string as NarrativeEventId
pub fn parse_narrative_event_id(id: &str) -> Result<NarrativeEventId, ServerMessage> {
    parse_uuid(id, "narrative_event").map(NarrativeEventId::from_uuid)
}

/// Parse a string as SceneId
pub fn parse_scene_id(id: &str) -> Result<SceneId, ServerMessage> {
    parse_uuid(id, "scene").map(SceneId::from_uuid)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_uuid() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_uuid(id, "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid_uuid() {
        let id = "not-a-uuid";
        let result = parse_uuid(id, "test");
        assert!(result.is_err());
        
        if let Err(ServerMessage::Error { code, .. }) = result {
            assert_eq!(code, "INVALID_TEST_ID");
        } else {
            panic!("Expected ServerMessage::Error");
        }
    }

    #[test]
    fn test_error_response_format() {
        let err = error_response("TEST_CODE", "Test message");
        if let ServerMessage::Error { code, message } = err {
            assert_eq!(code, "TEST_CODE");
            assert_eq!(message, "Test message");
        } else {
            panic!("Expected ServerMessage::Error");
        }
    }

    #[test]
    fn test_not_found_error_format() {
        let err = not_found_error("region", "123");
        if let ServerMessage::Error { code, message } = err {
            assert_eq!(code, "REGION_NOT_FOUND");
            assert!(message.contains("region"));
            assert!(message.contains("123"));
        } else {
            panic!("Expected ServerMessage::Error");
        }
    }

    #[test]
    fn test_parse_world_id() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_world_id(id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_player_character_id() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_player_character_id(id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_region_id() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_region_id(id);
        assert!(result.is_ok());
    }
}
