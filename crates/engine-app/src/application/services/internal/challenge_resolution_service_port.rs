//! Challenge resolution service port - Interface for challenge resolution operations
//!
//! This port abstracts challenge resolution business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.
//!
//! # Architecture Note
//!
//! This port handles the flow of challenge resolution including dice rolls,
//! outcome determination, and queueing for DM approval. It does NOT handle
//! broadcasting - that is the responsibility of the use case layer.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use wrldbldr_domain::{ChallengeId, PlayerCharacterId};

/// A dice roll submitted for challenge resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiceRoll {
    /// The raw roll value (before modifiers)
    pub roll: i32,
    /// Optional breakdown string (e.g., "1d20+5 = 15 + 5 = 20")
    pub breakdown: Option<String>,
    /// Individual dice results (for formula-based rolls)
    pub individual_rolls: Option<Vec<i32>>,
}

impl DiceRoll {
    /// Create a simple dice roll with just the value
    pub fn simple(roll: i32) -> Self {
        Self {
            roll,
            breakdown: None,
            individual_rolls: None,
        }
    }

    /// Create a dice roll with full formula details
    pub fn with_formula(roll: i32, breakdown: String, individual_rolls: Vec<i32>) -> Self {
        Self {
            roll,
            breakdown: Some(breakdown),
            individual_rolls: Some(individual_rolls),
        }
    }
}

/// Result of submitting a roll for challenge resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollResult {
    /// Resolution ID for tracking this pending approval
    pub resolution_id: String,
    /// Challenge ID
    pub challenge_id: String,
    /// Challenge name
    pub challenge_name: String,
    /// Challenge description
    pub challenge_description: Option<String>,
    /// Skill name (if applicable)
    pub skill_name: Option<String>,
    /// Character ID who rolled
    pub character_id: String,
    /// Character name who rolled
    pub character_name: String,
    /// The raw roll value
    pub roll: i32,
    /// Skill modifier applied
    pub modifier: i32,
    /// Total result (roll + modifier)
    pub total: i32,
    /// Outcome type (success, failure, critical, etc.)
    pub outcome_type: String,
    /// Outcome description text
    pub outcome_description: String,
    /// Roll breakdown string (e.g., "1d20+5 = 15 + 5 = 20")
    pub roll_breakdown: Option<String>,
    /// Individual dice results
    pub individual_rolls: Option<Vec<i32>>,
}

/// A pending challenge resolution awaiting DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingResolution {
    /// Resolution ID
    pub resolution_id: String,
    /// Challenge ID
    pub challenge_id: String,
    /// Challenge name
    pub challenge_name: String,
    /// Player character ID
    pub pc_id: String,
    /// Player character name
    pub pc_name: String,
    /// Roll result
    pub roll: i32,
    /// Applied modifier
    pub modifier: i32,
    /// Total (roll + modifier)
    pub total: i32,
    /// Determined outcome type
    pub outcome_type: String,
    /// Timestamp when the roll was submitted
    pub submitted_at: String,
}

/// Port for challenge resolution service operations
///
/// This trait defines the application use cases for challenge resolution,
/// including starting resolutions, submitting rolls, and querying pending
/// resolutions.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ChallengeResolutionServicePort: Send + Sync {
    /// Start a challenge resolution for a player character
    ///
    /// Creates a pending resolution and returns a resolution ID for tracking.
    /// The player can then submit their roll using `submit_roll`.
    async fn start_resolution(
        &self,
        challenge_id: ChallengeId,
        pc_id: PlayerCharacterId,
    ) -> Result<String>;

    /// Submit a dice roll for an ongoing challenge resolution
    ///
    /// Evaluates the roll against the challenge difficulty, determines the
    /// outcome, and queues it for DM approval.
    async fn submit_roll(&self, resolution_id: String, roll: DiceRoll) -> Result<RollResult>;

    /// Get the pending resolution for a player character (if any)
    ///
    /// Used to check if a PC has an active challenge they need to resolve.
    async fn get_pending_resolution(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Option<PendingResolution>>;
}
