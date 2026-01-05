//! Shared type definitions
//!
//! Common types used across the protocol that don't fit in other modules.

use serde::{Deserialize, Serialize};

// Re-export shared vocabulary types from domain::types
// These are stable types used in wire format without modification.
pub use wrldbldr_domain::types::CampbellArchetype;
pub use wrldbldr_domain::types::MonomythStage;

// =============================================================================
// Session & Participant Types
// =============================================================================

/// Role of a participant in a game session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantRole {
    DungeonMaster,
    Player,
    Spectator,
    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
}

// =============================================================================
// Approval Types
// =============================================================================

/// Proposed tool call information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProposedToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub arguments: serde_json::Value,
}

/// DM's decision on an approval request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decision")]
pub enum ApprovalDecision {
    /// Accept all proposed tools with default recipients
    Accept,
    /// Accept with item recipient selection
    AcceptWithRecipients {
        /// For give_item tools: maps tool_id -> recipient PC IDs
        /// Empty list means "don't give this item"
        item_recipients: std::collections::HashMap<String, Vec<String>>,
    },
    /// Accept with modifications to dialogue and/or tool selection
    AcceptWithModification {
        modified_dialogue: String,
        approved_tools: Vec<String>,
        rejected_tools: Vec<String>,
        /// For give_item tools: maps tool_id -> recipient PC IDs
        /// Empty list means "don't give this item"
        #[serde(default)]
        item_recipients: std::collections::HashMap<String, Vec<String>>,
    },
    Reject {
        feedback: String,
    },
    TakeOver {
        dm_response: String,
    },
    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
}

// =============================================================================
// Suggestion Types
// =============================================================================

/// Challenge suggestion information for DM approval
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChallengeSuggestionInfo {
    pub challenge_id: String,
    pub challenge_name: String,
    pub skill_name: String,
    pub difficulty_display: String,
    pub confidence: String,
    pub reasoning: String,
    /// Target player character ID for skill modifier lookup
    #[serde(default)]
    pub target_pc_id: Option<String>,
    /// Optional editable outcomes for DM modification
    #[serde(default)]
    pub outcomes: Option<ChallengeSuggestionOutcomes>,
}

/// Editable challenge outcomes for DM modification
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ChallengeSuggestionOutcomes {
    #[serde(default)]
    pub success: Option<String>,
    #[serde(default)]
    pub failure: Option<String>,
    #[serde(default)]
    pub critical_success: Option<String>,
    #[serde(default)]
    pub critical_failure: Option<String>,
}

/// Narrative event suggestion information for DM approval
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NarrativeEventSuggestionInfo {
    pub event_id: String,
    pub event_name: String,
    pub description: String,
    pub scene_direction: String,
    pub confidence: String,
    pub reasoning: String,
    pub matched_triggers: Vec<String>,
    /// Suggested outcome (can be cleared/modified by DM)
    #[serde(default)]
    pub suggested_outcome: Option<String>,
}

// =============================================================================
// Character Archetypes - Re-exported from domain-types (see top of file)
// =============================================================================

// =============================================================================
// Monomyth Stages - Re-exported from domain-types (see top of file)
// =============================================================================

// =============================================================================
// Game Time
// =============================================================================

/// Game time representation for wire transfer
///
/// Uses simple numeric fields for efficient JSON serialization.
/// Conversion from domain GameTime happens in the adapter layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTime {
    /// Day number (currently ordinal-style, 1-based; calendar is planned)
    pub day: u32,
    /// Hour (0-23)
    pub hour: u8,
    /// Minute (0-59)
    pub minute: u8,
    /// Whether time is paused
    pub is_paused: bool,
}

impl Default for GameTime {
    fn default() -> Self {
        Self {
            day: 1,
            hour: 8,
            minute: 0,
            is_paused: true,
        }
    }
}

impl GameTime {
    /// Create a new game time
    pub fn new(day: u32, hour: u8, minute: u8, is_paused: bool) -> Self {
        Self {
            day,
            hour,
            minute,
            is_paused,
        }
    }

    /// Get the time of day period
    pub fn time_of_day(&self) -> &'static str {
        match self.hour {
            5..=11 => "morning",
            12..=17 => "afternoon",
            18..=21 => "evening",
            _ => "night",
        }
    }

    /// Format as display string (e.g., "Day 3, 9:00 AM")
    pub fn display(&self) -> String {
        let period = if self.hour >= 12 { "PM" } else { "AM" };
        let display_hour = if self.hour == 0 {
            12
        } else if self.hour > 12 {
            self.hour - 12
        } else {
            self.hour
        };
        format!(
            "Day {}, {}:{:02} {}",
            self.day, display_hour, self.minute, period
        )
    }
}

// =============================================================================
// Time Mode
// =============================================================================

/// How time suggestions are handled (wire format)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeMode {
    /// Time only advances via explicit DM action
    Manual,
    /// System suggests, DM approves (default)
    #[default]
    Suggested,
    /// Time advances automatically
    Auto,
}

// =============================================================================
// Time Cost Configuration
// =============================================================================

/// Time costs for various actions (wire format)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeCostConfig {
    /// Minutes for travel between locations
    pub travel_location: u32,
    /// Minutes for travel between regions
    pub travel_region: u32,
    /// Minutes for short rest
    pub rest_short: u32,
    /// Minutes for long rest
    pub rest_long: u32,
    /// Minutes per conversation exchange
    pub conversation: u32,
    /// Minutes per challenge attempt
    pub challenge: u32,
    /// Minutes for scene transitions
    pub scene_transition: u32,
}

impl Default for TimeCostConfig {
    fn default() -> Self {
        Self {
            travel_location: 60,
            travel_region: 10,
            rest_short: 60,
            rest_long: 480,
            conversation: 0,
            challenge: 10,
            scene_transition: 0,
        }
    }
}

// =============================================================================
// Game Time Configuration
// =============================================================================

/// Complete time configuration (wire format)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameTimeConfig {
    /// How time suggestions are handled
    pub mode: TimeMode,
    /// Default time costs per action type
    pub time_costs: TimeCostConfig,
    /// Whether to show exact time to players
    pub show_time_to_players: bool,
}

impl Default for GameTimeConfig {
    fn default() -> Self {
        Self {
            mode: TimeMode::default(),
            time_costs: TimeCostConfig::default(),
            show_time_to_players: true,
        }
    }
}

// =============================================================================
// Time Suggestion
// =============================================================================

/// A time suggestion awaiting DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSuggestionData {
    /// Unique ID for this suggestion
    pub suggestion_id: String,
    /// PC that triggered this suggestion
    pub pc_id: String,
    /// PC name for display
    pub pc_name: String,
    /// Type of action (travel_location, rest_short, etc.)
    pub action_type: String,
    /// Human-readable description
    pub action_description: String,
    /// Suggested time cost in minutes
    pub suggested_minutes: u32,
    /// Current time before advancement
    pub current_time: GameTime,
    /// Time after advancement if approved
    pub resulting_time: GameTime,
    /// If period changes, (from_period, to_period)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_change: Option<(String, String)>,
}

// =============================================================================
// Time Advance Data
// =============================================================================

/// Data about a time advancement (for broadcasting)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeAdvanceData {
    /// Time before advancement
    pub previous_time: GameTime,
    /// Time after advancement
    pub new_time: GameTime,
    /// Minutes that were advanced
    pub minutes_advanced: u32,
    /// Human-readable reason
    pub reason: String,
    /// Whether the time period changed
    pub period_changed: bool,
    /// New period name if changed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_period: Option<String>,
}

// =============================================================================
// Time Suggestion Decision
// =============================================================================

/// DM's decision on a time suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum TimeSuggestionDecision {
    /// Accept the suggested time cost
    Approve,
    /// Modify the time cost
    Modify { minutes: u32 },
    /// Skip this time suggestion (no advancement)
    Skip,
}

// =============================================================================
// Lore Types
// =============================================================================

/// Category of lore (wire format)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoreCategoryData {
    Historical,
    Legend,
    Secret,
    Common,
    Technical,
    Political,
    Natural,
    Religious,
    #[serde(other)]
    Unknown,
}

/// Lore chunk for wire transfer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoreChunkData {
    pub id: String,
    pub order: u32,
    #[serde(default)]
    pub title: Option<String>,
    pub content: String,
    #[serde(default)]
    pub discovery_hint: Option<String>,
}

/// Lore entry for wire transfer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoreData {
    pub id: String,
    pub world_id: String,
    pub title: String,
    pub summary: String,
    pub category: LoreCategoryData,
    pub chunks: Vec<LoreChunkData>,
    pub is_common_knowledge: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// How lore was discovered (wire format)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum LoreDiscoverySourceData {
    ReadBook {
        book_name: String,
    },
    Conversation {
        npc_id: String,
        npc_name: String,
    },
    Investigation,
    DmGranted {
        reason: Option<String>,
    },
    CommonKnowledge,
    LlmDiscovered {
        context: String,
    },
    #[serde(other)]
    Unknown,
}

/// Character's knowledge of lore (wire format)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoreKnowledgeData {
    pub lore_id: String,
    pub character_id: String,
    /// Empty = knows all chunks
    pub known_chunk_ids: Vec<String>,
    pub discovery_source: LoreDiscoverySourceData,
    pub discovered_at: String,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Lore summary for list views
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoreSummaryData {
    pub id: String,
    pub title: String,
    pub category: LoreCategoryData,
    pub is_common_knowledge: bool,
    pub chunk_count: u32,
    /// How many chunks the character knows (for partial knowledge display)
    #[serde(default)]
    pub known_chunk_count: Option<u32>,
}

// =============================================================================
// Visual State Types
// =============================================================================

/// Time of day period (wire format, matches domain::TimeOfDay)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeOfDayData {
    Morning,
    Afternoon,
    Evening,
    Night,
    #[serde(other)]
    Unknown,
}

/// Activation rule for visual states (wire format)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ActivationRuleData {
    Always,
    DateExact {
        month: u32,
        day: u32,
    },
    DateRange {
        start_month: u32,
        start_day: u32,
        end_month: u32,
        end_day: u32,
    },
    TimeOfDay {
        period: TimeOfDayData,
    },
    EventTriggered {
        event_id: String,
        event_name: String,
    },
    FlagSet {
        flag_name: String,
    },
    CharacterPresent {
        character_id: String,
        character_name: String,
    },
    Custom {
        description: String,
        #[serde(default)]
        llm_prompt: Option<String>,
    },
    #[serde(other)]
    Unknown,
}

/// How rules are combined (wire format)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivationLogicData {
    All,
    Any,
    AtLeast(u32),
    #[serde(other)]
    Unknown,
}

impl Default for ActivationLogicData {
    fn default() -> Self {
        ActivationLogicData::All
    }
}

/// Location state for wire transfer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationStateData {
    pub id: String,
    pub location_id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub backdrop_override: Option<String>,
    #[serde(default)]
    pub atmosphere_override: Option<String>,
    #[serde(default)]
    pub ambient_sound: Option<String>,
    #[serde(default)]
    pub map_overlay: Option<String>,
    pub activation_rules: Vec<ActivationRuleData>,
    #[serde(default)]
    pub activation_logic: ActivationLogicData,
    pub priority: i32,
    pub is_default: bool,
}

/// Region state for wire transfer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionStateData {
    pub id: String,
    pub region_id: String,
    pub location_id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub backdrop_override: Option<String>,
    #[serde(default)]
    pub atmosphere_override: Option<String>,
    #[serde(default)]
    pub ambient_sound: Option<String>,
    pub activation_rules: Vec<ActivationRuleData>,
    #[serde(default)]
    pub activation_logic: ActivationLogicData,
    pub priority: i32,
    pub is_default: bool,
}

/// Resolved state info for staging (lightweight)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedStateInfoData {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub backdrop_override: Option<String>,
    #[serde(default)]
    pub atmosphere_override: Option<String>,
    #[serde(default)]
    pub ambient_sound: Option<String>,
}

/// How visual state was resolved
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum VisualStateSourceData {
    #[default]
    HardRulesOnly,
    WithLlmEvaluation,
    DmOverride,
    Default,
    #[serde(other)]
    Unknown,
}

/// Complete resolved visual state for staging
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedVisualStateData {
    #[serde(default)]
    pub location_state: Option<ResolvedStateInfoData>,
    #[serde(default)]
    pub region_state: Option<ResolvedStateInfoData>,
}

/// State option for DM selection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateOptionData {
    pub id: String,
    pub name: String,
    pub priority: i32,
    pub is_default: bool,
    /// Why this state was suggested (rule match description)
    #[serde(default)]
    pub match_reason: Option<String>,
}
