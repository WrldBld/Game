//! Use case error types for hexagonal architecture
//!
//! Each use case has its own error type with:
//! - Meaningful variants with domain context
//! - Display implementation for user-facing messages
//! - ErrorCode implementation for error code extraction
//!
//! # Design Rationale
//!
//! 1. Separate error types per use case: Clearer ownership, avoids "god error enum"
//! 2. ErrorCode trait: Provides error codes; adapters handle protocol conversion
//! 3. No protocol dependencies: App layer is protocol-agnostic
//!
//! # Error Code Conventions
//!
//! - Use SCREAMING_SNAKE_CASE
//! - Start with entity name when relevant (PC_NOT_FOUND, REGION_NOT_FOUND)
//! - Be specific (CONNECTION_LOCKED vs generic ERROR)
//! - Match existing protocol error codes where possible

use thiserror::Error;
use wrldbldr_domain::{CharacterId, ItemId, LocationId, PlayerCharacterId, RegionId};

// Re-export ErrorCode, ConnectionError, and MovementError from within the crate
pub use crate::outbound::{ConnectionError, ErrorCode, MovementError};

// =============================================================================
// Staging Errors
// =============================================================================

/// Errors that can occur during staging operations
#[derive(Debug, Error)]
pub enum StagingError {
    /// Pending staging request not found
    #[error("Pending staging not found: {0}")]
    PendingNotFound(String),

    /// Target region not found
    #[error("Region not found: {0}")]
    RegionNotFound(RegionId),

    /// Character (NPC) not found
    #[error("Character not found: {0}")]
    CharacterNotFound(CharacterId),

    /// Staging approval operation failed
    #[error("Staging approval failed: {0}")]
    ApprovalFailed(String),

    /// LLM regeneration failed
    #[error("Regeneration failed: {0}")]
    RegenerationFailed(String),

    /// Pre-staging operation failed
    #[error("Pre-staging failed: {0}")]
    PreStagingFailed(String),

    /// Database operation failed
    #[error("Database error: {0}")]
    Database(String),
}

impl ErrorCode for StagingError {
    fn code(&self) -> &'static str {
        match self {
            Self::PendingNotFound(_) => "STAGING_NOT_FOUND",
            Self::RegionNotFound(_) => "REGION_NOT_FOUND",
            Self::CharacterNotFound(_) => "CHARACTER_NOT_FOUND",
            Self::ApprovalFailed(_) => "STAGING_APPROVAL_FAILED",
            Self::RegenerationFailed(_) => "REGENERATION_FAILED",
            Self::PreStagingFailed(_) => "PRE_STAGING_FAILED",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Inventory Errors
// =============================================================================

/// Errors that can occur during inventory operations
#[derive(Debug, Error)]
pub enum InventoryError {
    /// Player character not found
    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    /// Item not found in database
    #[error("Item not found: {0}")]
    ItemNotFound(ItemId),

    /// Item not in PC's inventory
    #[error("Item not in inventory")]
    NotInInventory,

    /// Item already owned by another character
    #[error("Item already owned by another character")]
    AlreadyOwned,

    /// Not enough quantity to perform operation
    #[error("Insufficient quantity: need {needed}, have {available}")]
    InsufficientQuantity { needed: u32, available: u32 },

    /// Item cannot be equipped (not equippable type)
    #[error("Item cannot be equipped")]
    NotEquippable,

    /// Item is already equipped
    #[error("Item is already equipped")]
    AlreadyEquipped,

    /// Item is not equipped
    #[error("Item is not equipped")]
    NotEquipped,

    /// PC is not in a region (cannot drop items)
    #[error("PC is not in a region")]
    NoCurrentRegion,

    /// Database operation failed
    #[error("Database error: {0}")]
    Database(String),
}

impl ErrorCode for InventoryError {
    fn code(&self) -> &'static str {
        match self {
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::ItemNotFound(_) => "ITEM_NOT_FOUND",
            Self::NotInInventory => "NOT_IN_INVENTORY",
            Self::AlreadyOwned => "ITEM_ALREADY_OWNED",
            Self::InsufficientQuantity { .. } => "INSUFFICIENT_QUANTITY",
            Self::NotEquippable => "NOT_EQUIPPABLE",
            Self::AlreadyEquipped => "ALREADY_EQUIPPED",
            Self::NotEquipped => "NOT_EQUIPPED",
            Self::NoCurrentRegion => "NO_CURRENT_REGION",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Challenge Errors
// =============================================================================

/// Errors that can occur during challenge operations
#[derive(Debug, Error)]
pub enum ChallengeError {
    /// Challenge not found
    #[error("Challenge not found: {0}")]
    ChallengeNotFound(String),

    /// Player character not found
    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    /// Target character (NPC) not found
    #[error("Target character not found: {0}")]
    TargetNotFound(CharacterId),

    /// Roll already submitted for this challenge attempt
    #[error("Roll already submitted for this challenge")]
    RollAlreadySubmitted,

    /// Roll value is invalid
    #[error("Invalid roll value: {0}")]
    InvalidRoll(String),

    /// Challenge outcome is pending DM approval
    #[error("Challenge outcome pending approval")]
    OutcomePending,

    /// User not authorized to perform this action
    #[error("Not authorized to approve this outcome")]
    NotAuthorized,

    /// Challenge resolution failed
    #[error("Challenge resolution failed: {0}")]
    ResolutionFailed(String),

    /// Database operation failed
    #[error("Database error: {0}")]
    Database(String),
}

impl ErrorCode for ChallengeError {
    fn code(&self) -> &'static str {
        match self {
            Self::ChallengeNotFound(_) => "CHALLENGE_NOT_FOUND",
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::TargetNotFound(_) => "TARGET_NOT_FOUND",
            Self::RollAlreadySubmitted => "ROLL_ALREADY_SUBMITTED",
            Self::InvalidRoll(_) => "INVALID_ROLL",
            Self::OutcomePending => "OUTCOME_PENDING",
            Self::NotAuthorized => "NOT_AUTHORIZED",
            Self::ResolutionFailed(_) => "RESOLUTION_FAILED",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Observation Errors
// =============================================================================

/// Errors that can occur during observation operations
#[derive(Debug, Error)]
pub enum ObservationError {
    /// Player character not found
    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    /// NPC not found
    #[error("NPC not found: {0}")]
    NpcNotFound(CharacterId),

    /// NPC is not in the current region
    #[error("NPC not in current region")]
    NpcNotInRegion,

    /// Region not found
    #[error("Region not found: {0}")]
    RegionNotFound(RegionId),

    /// Location not found
    #[error("Location not found: {0}")]
    LocationNotFound(LocationId),

    /// Event generation (LLM) failed
    #[error("Event generation failed: {0}")]
    EventGenerationFailed(String),

    /// Database operation failed
    #[error("Database error: {0}")]
    Database(String),
}

impl ErrorCode for ObservationError {
    fn code(&self) -> &'static str {
        match self {
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::NpcNotFound(_) => "NPC_NOT_FOUND",
            Self::NpcNotInRegion => "NPC_NOT_IN_REGION",
            Self::RegionNotFound(_) => "REGION_NOT_FOUND",
            Self::LocationNotFound(_) => "LOCATION_NOT_FOUND",
            Self::EventGenerationFailed(_) => "EVENT_GENERATION_FAILED",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Scene Errors
// =============================================================================

/// Errors that can occur during scene operations
#[derive(Debug, Error)]
pub enum SceneError {
    /// Scene not found
    #[error("Scene not found: {0}")]
    SceneNotFound(String),

    /// Player character not found
    #[error("Player character not found: {0}")]
    PcNotFound(PlayerCharacterId),

    /// Region not found
    #[error("Region not found: {0}")]
    RegionNotFound(RegionId),

    /// Scene change request is pending DM approval
    #[error("Scene change request pending approval")]
    RequestPending,

    /// Directorial context is invalid
    #[error("Invalid directorial context: {0}")]
    InvalidContext(String),

    /// User not authorized
    #[error("Not authorized to approve scene changes")]
    NotAuthorized,

    /// Scene resolution failed
    #[error("Scene resolution failed: {0}")]
    ResolutionFailed(String),

    /// Database operation failed
    #[error("Database error: {0}")]
    Database(String),
}

impl ErrorCode for SceneError {
    fn code(&self) -> &'static str {
        match self {
            Self::SceneNotFound(_) => "SCENE_NOT_FOUND",
            Self::PcNotFound(_) => "PC_NOT_FOUND",
            Self::RegionNotFound(_) => "REGION_NOT_FOUND",
            Self::RequestPending => "REQUEST_PENDING",
            Self::InvalidContext(_) => "INVALID_CONTEXT",
            Self::NotAuthorized => "NOT_AUTHORIZED",
            Self::ResolutionFailed(_) => "RESOLUTION_FAILED",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Narrative Event Errors
// =============================================================================

/// Errors that can occur during narrative event operations
#[derive(Debug, Error)]
pub enum NarrativeEventError {
    /// User not authorized (not DM)
    #[error("Not authorized: {0}")]
    Unauthorized(String),

    /// Narrative event approval failed
    #[error("Approval failed: {0}")]
    ApprovalFailed(String),
}

impl ErrorCode for NarrativeEventError {
    fn code(&self) -> &'static str {
        match self {
            Self::Unauthorized(_) => "NOT_AUTHORIZED",
            Self::ApprovalFailed(_) => "NARRATIVE_EVENT_ERROR",
        }
    }
}

// =============================================================================
// Action Errors
// =============================================================================

/// Errors that can occur during player action operations
#[derive(Debug, Error)]
pub enum ActionError {
    /// No player character selected
    #[error("No player character selected")]
    NoPcSelected,

    /// Missing required target for action
    #[error("Missing target for action")]
    MissingTarget,

    /// Invalid action type
    #[error("Invalid action type: {0}")]
    InvalidActionType(String),

    /// Movement operation failed
    #[error("Movement failed: {0}")]
    MovementFailed(String),

    /// Movement was blocked (locked door, etc.)
    #[error("Movement blocked: {0}")]
    MovementBlocked(String),

    /// Failed to enqueue action
    #[error("Action queue failed: {0}")]
    QueueFailed(String),

    /// Action requires DM approval
    #[error("Action requires DM approval")]
    RequiresApproval,

    /// Database operation failed
    #[error("Database error: {0}")]
    Database(String),
}

impl ErrorCode for ActionError {
    fn code(&self) -> &'static str {
        match self {
            Self::NoPcSelected => "NO_PC_SELECTED",
            Self::MissingTarget => "MISSING_TARGET",
            Self::InvalidActionType(_) => "INVALID_ACTION_TYPE",
            Self::MovementFailed(_) => "MOVEMENT_FAILED",
            Self::MovementBlocked(_) => "MOVEMENT_BLOCKED",
            Self::QueueFailed(_) => "QUEUE_FAILED",
            Self::RequiresApproval => "REQUIRES_APPROVAL",
            Self::Database(_) => "DATABASE_ERROR",
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movement_error_codes() {
        let err = MovementError::PcNotFound(PlayerCharacterId::from_uuid(uuid::Uuid::nil()));
        assert_eq!(err.code(), "PC_NOT_FOUND");
        assert!(err.to_string().contains("Player character not found"));
    }

    #[test]
    fn test_staging_error_codes() {
        let err = StagingError::PendingNotFound("test-123".to_string());
        assert_eq!(err.code(), "STAGING_NOT_FOUND");
        assert!(err.to_string().contains("Pending staging not found"));
    }

    #[test]
    fn test_inventory_error_with_context() {
        let err = InventoryError::InsufficientQuantity {
            needed: 5,
            available: 2,
        };
        assert_eq!(err.code(), "INSUFFICIENT_QUANTITY");
        assert!(err.to_string().contains("need 5"));
        assert!(err.to_string().contains("have 2"));
    }
}
