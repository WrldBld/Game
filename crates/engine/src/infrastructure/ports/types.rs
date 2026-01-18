// Port traits define the full contract - many methods are for future use
#![allow(dead_code)]

//! Helper types for port operations.

use chrono::{DateTime, Utc};
use uuid::Uuid;
use wrldbldr_domain::*;

// WorldRole is re-exported from wrldbldr_domain (imported via `use wrldbldr_domain::*`)
// This ensures the domain owns the canonical definition of this concept.
pub use wrldbldr_domain::WorldRole;

// =============================================================================
// Session/Connection Types
// =============================================================================

/// Information about a connected client (for use case queries).
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Unique ID for this connection
    pub connection_id: Uuid,
    /// User identifier (may be anonymous)
    pub user_id: String,
    /// The world this connection is associated with (if joined)
    pub world_id: Option<WorldId>,
    /// The role in the world
    pub role: WorldRole,
    /// Player character ID (if role is Player)
    pub pc_id: Option<PlayerCharacterId>,
}

// =============================================================================
// Directorial Context Types (DM guidance for scenes)
// =============================================================================

/// NPC motivation guidance from the DM.
#[derive(Debug, Clone, PartialEq)]
pub struct NpcMotivation {
    /// Character ID as string (for flexibility with external IDs)
    pub character_id: String,
    /// Free-form emotional guidance (e.g., "Conflicted about revealing secrets")
    pub emotional_guidance: String,
    /// What the NPC is trying to achieve right now
    pub immediate_goal: String,
    /// Hidden agenda the NPC may have
    pub secret_agenda: Option<String>,
}

/// DM directorial context for guiding AI responses.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectorialContext {
    /// Scene notes and setting description
    pub scene_notes: String,
    /// Desired tone for responses
    pub tone: String,
    /// Per-NPC motivation overrides
    pub npc_motivations: Vec<NpcMotivation>,
    /// Topics the AI should avoid
    pub forbidden_topics: Vec<String>,
}

// =============================================================================
// Session Result Types (for use case return values)
// =============================================================================

/// A user connected to a world (domain representation).
#[derive(Debug, Clone)]
pub struct ConnectedUserInfo {
    pub user_id: String,
    pub username: Option<String>,
    pub role: WorldRole,
    pub pc_id: Option<PlayerCharacterId>,
    pub connection_count: u32,
}

/// Payload for user joined notification.
#[derive(Debug, Clone)]
pub struct UserJoinedInfo {
    pub user_id: String,
    pub role: WorldRole,
    pub pc: Option<serde_json::Value>,
}

/// NPC disposition information for a specific PC (domain representation).
#[derive(Debug, Clone)]
pub struct NpcDispositionInfo {
    pub npc_id: String,
    pub npc_name: String,
    pub disposition: String,
    pub relationship: String,
    pub sentiment: f32,
    pub last_reason: Option<String>,
}

// =============================================================================
// Infrastructure Types
// =============================================================================

/// NPC-Region relationship for staging suggestions
#[derive(Debug, Clone)]
pub struct NpcRegionRelationship {
    pub region_id: RegionId,
    pub relationship_type: NpcRegionRelationType,
    pub shift: Option<String>,     // For WORKS_AT: "day", "night", "always"
    pub frequency: Option<String>, // For FREQUENTS: "always", "often", "sometimes", "rarely"
    pub time_of_day: Option<String>, // For FREQUENTS: "morning", "afternoon", "evening", "night"
    pub reason: Option<String>,    // For AVOIDS: why they avoid it
}

/// Type of NPC-Region relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcRegionRelationType {
    HomeRegion,
    WorksAt,
    Frequents,
    Avoids,
}

impl std::fmt::Display for NpcRegionRelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HomeRegion => write!(f, "HOME_REGION"),
            Self::WorksAt => write!(f, "WORKS_AT_REGION"),
            Self::Frequents => write!(f, "FREQUENTS_REGION"),
            Self::Avoids => write!(f, "AVOIDS_REGION"),
        }
    }
}

impl std::str::FromStr for NpcRegionRelationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "HOME_REGION" | "HOME" | "HOMEREGION" => Ok(Self::HomeRegion),
            "WORKS_AT_REGION" | "WORKS_AT" | "WORK" | "WORKSAT" => Ok(Self::WorksAt),
            "FREQUENTS_REGION" | "FREQUENTS" | "FREQUENTSREGION" => Ok(Self::Frequents),
            "AVOIDS_REGION" | "AVOIDS" | "AVOIDSREGION" => Ok(Self::Avoids),
            _ => Err(format!("Unknown NPC region relationship type: '{}'", s)),
        }
    }
}

/// NPC with their region relationship info (for staging suggestions)
#[derive(Debug, Clone)]
pub struct NpcWithRegionInfo {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub relationship_type: NpcRegionRelationType,
    pub shift: Option<String>,
    pub frequency: Option<String>,
    pub time_of_day: Option<String>,
    pub reason: Option<String>,
    /// NPC's default mood (used when staging doesn't override)
    pub default_mood: MoodState,
}

/// Want details with relationship metadata and resolved target.
#[derive(Debug, Clone)]
pub struct WantDetails {
    pub character_id: CharacterId,
    pub want: Want,
    pub priority: u32,
    pub target: Option<WantTarget>,
}

/// Goal details with usage statistics.
#[derive(Debug, Clone)]
pub struct GoalDetails {
    pub goal: Goal,
    pub usage_count: u32,
}

/// Reference to a want target by type.
#[derive(Debug, Clone)]
pub enum WantTargetRef {
    Character(CharacterId),
    Item(ItemId),
    Goal(GoalId),
}

/// Actantial view record for a specific want and target.
#[derive(Debug, Clone)]
pub struct ActantialViewRecord {
    pub want_id: WantId,
    pub target: ActantialTarget,
    pub target_name: String,
    pub role: ActantialRole,
    pub reason: String,
}

/// A dialogue turn record from the database.
///
/// Used for building conversation history in LLM prompts.
/// Contains speaker name and text, ready for ConversationTurn conversion.
#[derive(Debug, Clone)]
pub struct ConversationTurnRecord {
    /// Speaker name (PC name or NPC name)
    pub speaker: String,
    /// The dialogue text
    pub text: String,
    /// Order in the conversation (for sorting)
    pub order: i64,
}

// =============================================================================
// Staging Storage Data Types
// =============================================================================

/// Pending staging request tracking (request_id -> region/location).
///
/// Used to track staging approval requests while waiting for DM response.
#[derive(Debug, Clone)]
pub struct PendingStagingRequest {
    pub region_id: RegionId,
    pub location_id: LocationId,
    pub world_id: WorldId,
    pub created_at: DateTime<Utc>,
}

// Note: PendingStagingStore trait was removed.
// The concrete implementation is PendingStagingStoreImpl in api/websocket/mod.rs.

// =============================================================================
// Time Suggestion Data Types
// =============================================================================

/// A time suggestion generated for DM approval.
///
/// Contains the suggested time advancement and context about the action.
#[derive(Debug, Clone)]
pub struct TimeSuggestion {
    pub id: Uuid,
    pub world_id: WorldId,
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    pub action_type: String,
    pub action_description: String,
    pub suggested_minutes: u32,
    pub current_time: GameTime,
    pub resulting_time: GameTime,
    pub period_change: Option<(TimeOfDay, TimeOfDay)>,
}

// Note: TimeSuggestionStore trait was removed.
// The concrete implementation is TimeSuggestionStoreImpl in api/websocket/mod.rs.
