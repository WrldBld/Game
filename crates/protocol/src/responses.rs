//! Response types for WebSocket request/response pattern
//!
//! This module defines the response types returned from WebSocket requests,
//! as well as broadcast types for entity changes.

use serde::{Deserialize, Serialize};

// Re-export shared vocabulary types from domain::types
pub use wrldbldr_domain::types::{ChangeType, EntityType};

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
    /// Unknown response type for forward compatibility
    ///
    /// When deserializing an unknown variant, this variant is used instead of
    /// failing. Allows older clients to gracefully handle new response types.
    #[serde(other)]
    Unknown,
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

    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
}

// NOTE: ErrorCode::to_http_status() was removed as it was unused and violates
// hexagonal architecture (HTTP is adapter-layer concern, not protocol).
// If needed, implement in engine-adapters HTTP layer.

// =============================================================================
// Request Error (Client-Side)
// =============================================================================

/// Client-side request errors
///
/// These are errors that occur on the client side when making requests,
/// distinct from server-side errors returned in `ResponseResult::Error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestError {
    /// Request was cancelled (e.g., channel closed)
    Cancelled,
    /// Request timed out waiting for response
    Timeout,
    /// Failed to send request over WebSocket
    SendFailed(String),
    /// Not connected to server
    NotConnected,
    /// Failed to serialize request
    SerializationError(String),
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestError::Cancelled => write!(f, "Request was cancelled"),
            RequestError::Timeout => write!(f, "Request timed out"),
            RequestError::SendFailed(msg) => write!(f, "Failed to send request: {}", msg),
            RequestError::NotConnected => write!(f, "Not connected to server"),
            RequestError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for RequestError {}

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

// EntityType and ChangeType are re-exported from domain::types at the top of this file

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
    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
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

/// Convert from domain WorldRole to protocol WorldRole
impl From<wrldbldr_domain::WorldRole> for WorldRole {
    fn from(role: wrldbldr_domain::WorldRole) -> Self {
        match role {
            wrldbldr_domain::WorldRole::Dm => WorldRole::Dm,
            wrldbldr_domain::WorldRole::Player => WorldRole::Player,
            wrldbldr_domain::WorldRole::Spectator => WorldRole::Spectator,
        }
    }
}

/// Convert from protocol WorldRole to domain WorldRole
/// Note: Unknown maps to Spectator as the safe default
impl From<WorldRole> for wrldbldr_domain::WorldRole {
    fn from(role: WorldRole) -> Self {
        match role {
            WorldRole::Dm => wrldbldr_domain::WorldRole::Dm,
            WorldRole::Player => wrldbldr_domain::WorldRole::Player,
            WorldRole::Spectator | WorldRole::Unknown => wrldbldr_domain::WorldRole::Spectator,
        }
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
    DmAlreadyConnected { existing_user_id: String },
    /// Player role requires a PC selection
    PlayerRequiresPc,
    /// Spectator role requires a target PC
    SpectatorRequiresTarget,
    /// World does not exist
    WorldNotFound,
    /// User is not authorized to join this world
    Unauthorized,
    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
}
