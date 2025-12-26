//! Response types for WebSocket request/response pattern
//!
//! This module defines the response types returned from WebSocket requests,
//! as well as broadcast types for entity changes.

use serde::{Deserialize, Serialize};

// =============================================================================
// Response Result
// =============================================================================

/// Result of a request operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ResponseResult {
    /// Operation succeeded
    Success {
        /// Optional data payload (varies by request type)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },
    /// Operation failed
    Error {
        /// Error classification code
        code: ErrorCode,
        /// Human-readable error message
        message: String,
        /// Additional error details (optional)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        details: Option<serde_json::Value>,
    },
}

impl ResponseResult {
    /// Create a success response with data
    pub fn success<T: Serialize>(data: T) -> Self {
        ResponseResult::Success {
            data: Some(serde_json::to_value(data).unwrap_or_default()),
        }
    }

    /// Create a success response without data
    pub fn success_empty() -> Self {
        ResponseResult::Success { data: None }
    }

    /// Create an error response
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        ResponseResult::Error {
            code,
            message: message.into(),
            details: None,
        }
    }

    /// Create an error response with details
    pub fn error_with_details<T: Serialize>(
        code: ErrorCode,
        message: impl Into<String>,
        details: T,
    ) -> Self {
        ResponseResult::Error {
            code,
            message: message.into(),
            details: Some(serde_json::to_value(details).unwrap_or_default()),
        }
    }

    /// Check if this is a success response
    pub fn is_success(&self) -> bool {
        matches!(self, ResponseResult::Success { .. })
    }

    /// Check if this is an error response
    pub fn is_error(&self) -> bool {
        matches!(self, ResponseResult::Error { .. })
    }
}

// =============================================================================
// Error Codes
// =============================================================================

/// Error classification codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    // === Client Errors (4xx) ===
    /// Request was malformed or invalid
    BadRequest,
    /// Authentication required or failed
    Unauthorized,
    /// User lacks permission for this operation
    Forbidden,
    /// Requested resource not found
    NotFound,
    /// Operation conflicts with current state
    Conflict,
    /// Request data failed validation
    ValidationError,
    /// Rate limit exceeded
    RateLimitExceeded,

    // === Server Errors (5xx) ===
    /// Internal server error
    InternalError,
    /// Required service is unavailable
    ServiceUnavailable,
    /// Operation timed out
    Timeout,
}

impl ErrorCode {
    /// Convert to HTTP status code equivalent
    pub fn to_http_status(&self) -> u16 {
        match self {
            ErrorCode::BadRequest => 400,
            ErrorCode::Unauthorized => 401,
            ErrorCode::Forbidden => 403,
            ErrorCode::NotFound => 404,
            ErrorCode::Conflict => 409,
            ErrorCode::ValidationError => 422,
            ErrorCode::RateLimitExceeded => 429,
            ErrorCode::InternalError => 500,
            ErrorCode::ServiceUnavailable => 503,
            ErrorCode::Timeout => 504,
        }
    }
}

// =============================================================================
// Entity Changed Broadcast
// =============================================================================

/// Entity change notification for broadcasts
///
/// Sent to all connected clients when an entity is created, updated, or deleted.
/// Clients can use this to invalidate caches and update UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityChangedData {
    /// Type of entity that changed
    pub entity_type: EntityType,
    /// ID of the entity
    pub entity_id: String,
    /// Type of change
    pub change_type: ChangeType,
    /// Entity data (for create/update; None for delete)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// World this entity belongs to
    pub world_id: String,
}

impl EntityChangedData {
    /// Create a "created" notification
    pub fn created<T: Serialize>(
        entity_type: EntityType,
        entity_id: impl Into<String>,
        world_id: impl Into<String>,
        data: &T,
    ) -> Self {
        Self {
            entity_type,
            entity_id: entity_id.into(),
            change_type: ChangeType::Created,
            data: Some(serde_json::to_value(data).unwrap_or_default()),
            world_id: world_id.into(),
        }
    }

    /// Create an "updated" notification
    pub fn updated<T: Serialize>(
        entity_type: EntityType,
        entity_id: impl Into<String>,
        world_id: impl Into<String>,
        data: &T,
    ) -> Self {
        Self {
            entity_type,
            entity_id: entity_id.into(),
            change_type: ChangeType::Updated,
            data: Some(serde_json::to_value(data).unwrap_or_default()),
            world_id: world_id.into(),
        }
    }

    /// Create a "deleted" notification
    pub fn deleted(
        entity_type: EntityType,
        entity_id: impl Into<String>,
        world_id: impl Into<String>,
    ) -> Self {
        Self {
            entity_type,
            entity_id: entity_id.into(),
            change_type: ChangeType::Deleted,
            data: None,
            world_id: world_id.into(),
        }
    }
}

/// Types of entities that can change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    World,
    Character,
    Location,
    Region,
    Scene,
    Act,
    Interaction,
    Skill,
    Challenge,
    NarrativeEvent,
    EventChain,
    StoryEvent,
    PlayerCharacter,
    Relationship,
    Observation,
    Goal,
    Want,
    ActantialView,
    GameTime,
}

/// Types of changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Created,
    Updated,
    Deleted,
}

// =============================================================================
// World Role
// =============================================================================

/// Role of a user in a world
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldRole {
    /// Dungeon Master - full control
    Dm,
    /// Player - controls a PC
    Player,
    /// Spectator - read-only view
    Spectator,
}

impl WorldRole {
    /// Check if this role can modify data
    pub fn can_modify(&self) -> bool {
        matches!(self, WorldRole::Dm | WorldRole::Player)
    }

    /// Check if this role is DM
    pub fn is_dm(&self) -> bool {
        matches!(self, WorldRole::Dm)
    }

    /// Check if this role is a spectator
    pub fn is_spectator(&self) -> bool {
        matches!(self, WorldRole::Spectator)
    }
}

// =============================================================================
// Connected User
// =============================================================================

/// Information about a user connected to a world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedUser {
    /// Unique user identifier
    pub user_id: String,
    /// Display name (if available)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// User's role in this world
    pub role: WorldRole,
    /// Player character ID (for Player role)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pc_id: Option<String>,
    /// Number of active connections (for DM with multiple screens)
    pub connection_count: u32,
}

// =============================================================================
// Join Error
// =============================================================================

/// Errors that can occur when joining a world
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JoinError {
    /// Another DM is already connected to this world
    DmAlreadyConnected {
        existing_user_id: String,
    },
    /// Player role requires a PC selection
    PlayerRequiresPc,
    /// Spectator role requires a target PC
    SpectatorRequiresTarget,
    /// World does not exist
    WorldNotFound,
    /// User is not authorized to join this world
    Unauthorized,
}
