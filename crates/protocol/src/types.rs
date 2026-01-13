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
// Time Format
// =============================================================================

/// How time is displayed to players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeFormat {
    /// "9:00 AM"
    #[default]
    TwelveHour,
    /// "09:00"
    TwentyFourHour,
    /// "Morning" (period only, no specific time)
    PeriodOnly,
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
    /// Time format preference for display
    #[serde(default)]
    pub time_format: TimeFormat,
}

impl Default for GameTimeConfig {
    fn default() -> Self {
        Self {
            mode: TimeMode::default(),
            time_costs: TimeCostConfig::default(),
            show_time_to_players: true,
            time_format: TimeFormat::default(),
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
    /// Unknown decision type for forward compatibility
    #[serde(other)]
    Unknown,
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

impl From<wrldbldr_domain::TimeOfDay> for TimeOfDayData {
    fn from(tod: wrldbldr_domain::TimeOfDay) -> Self {
        match tod {
            wrldbldr_domain::TimeOfDay::Morning => TimeOfDayData::Morning,
            wrldbldr_domain::TimeOfDay::Afternoon => TimeOfDayData::Afternoon,
            wrldbldr_domain::TimeOfDay::Evening => TimeOfDayData::Evening,
            wrldbldr_domain::TimeOfDay::Night => TimeOfDayData::Night,
        }
    }
}

impl From<TimeOfDayData> for wrldbldr_domain::TimeOfDay {
    fn from(tod: TimeOfDayData) -> Self {
        match tod {
            TimeOfDayData::Morning => wrldbldr_domain::TimeOfDay::Morning,
            TimeOfDayData::Afternoon => wrldbldr_domain::TimeOfDay::Afternoon,
            TimeOfDayData::Evening => wrldbldr_domain::TimeOfDay::Evening,
            TimeOfDayData::Night => wrldbldr_domain::TimeOfDay::Night,
            // Unknown defaults to Morning (safe fallback)
            TimeOfDayData::Unknown => wrldbldr_domain::TimeOfDay::Morning,
        }
    }
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivationLogicData {
    #[default]
    All,
    Any,
    AtLeast(u32),
    #[serde(other)]
    Unknown,
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

// =============================================================================
// Trigger Schema Types (for Visual Trigger Builder)
// =============================================================================

/// Complete schema describing all available trigger types for the visual builder
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerSchema {
    /// All available trigger types
    pub trigger_types: Vec<TriggerTypeSchema>,
    /// Available logic options (All, Any, AtLeast)
    pub logic_options: Vec<TriggerLogicOption>,
}

/// Schema for a single trigger type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerTypeSchema {
    /// Internal type name (e.g., "PlayerEntersLocation")
    pub type_name: String,
    /// Display label for UI
    pub label: String,
    /// Description of what this trigger does
    pub description: String,
    /// Category for grouping in UI
    pub category: TriggerCategory,
    /// Fields required/available for this trigger type
    pub fields: Vec<TriggerFieldSchema>,
}

/// Category for grouping trigger types in UI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriggerCategory {
    /// Location-based triggers
    Location,
    /// NPC/character-based triggers
    Character,
    /// Item/inventory triggers
    Inventory,
    /// Challenge/combat triggers
    Challenge,
    /// Time-based triggers
    Time,
    /// Flag/state triggers
    State,
    /// Event-based triggers
    Event,
    /// Custom/LLM-evaluated triggers
    Custom,
    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
}

impl TriggerCategory {
    pub fn label(&self) -> &'static str {
        match self {
            TriggerCategory::Location => "Location",
            TriggerCategory::Character => "Character",
            TriggerCategory::Inventory => "Inventory",
            TriggerCategory::Challenge => "Challenge",
            TriggerCategory::Time => "Time",
            TriggerCategory::State => "State",
            TriggerCategory::Event => "Event",
            TriggerCategory::Custom => "Custom",
            TriggerCategory::Unknown => "Unknown",
        }
    }
}

/// Schema for a single field within a trigger type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerFieldSchema {
    /// Field name (matches JSON key)
    pub name: String,
    /// Display label for UI
    pub label: String,
    /// Field data type
    pub field_type: TriggerFieldType,
    /// Whether this field is required
    pub required: bool,
    /// Help text for UI
    #[serde(default)]
    pub description: Option<String>,
    /// Default value (as JSON)
    #[serde(default)]
    pub default_value: Option<serde_json::Value>,
}

/// Data type for a trigger field
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriggerFieldType {
    /// Plain text string
    String,
    /// Numeric value (integer)
    Integer,
    /// Numeric value (float)
    Float,
    /// Boolean (checkbox)
    Boolean,
    /// Reference to a Location entity (picker)
    LocationRef,
    /// Reference to a Region entity (picker)
    RegionRef,
    /// Reference to a Character/NPC entity (picker)
    CharacterRef,
    /// Reference to a Challenge entity (picker)
    ChallengeRef,
    /// Reference to a NarrativeEvent entity (picker)
    EventRef,
    /// Reference to an Item (picker or text)
    ItemRef,
    /// Time of day enum (Morning, Afternoon, Evening, Night)
    TimeOfDay,
    /// Array of keyword strings (tag input)
    Keywords,
    /// Sentiment value (-1.0 to 1.0)
    Sentiment,
    /// Unknown variant for forward compatibility
    #[serde(other)]
    Unknown,
}

/// Option for trigger logic selection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerLogicOption {
    /// Value to use (e.g., "all", "any", "atLeast")
    pub value: String,
    /// Display label
    pub label: String,
    /// Description
    pub description: String,
    /// Whether this option requires a count parameter
    pub requires_count: bool,
}

impl TriggerSchema {
    /// Generate the complete trigger schema
    pub fn generate() -> Self {
        Self {
            trigger_types: Self::generate_trigger_types(),
            logic_options: vec![
                TriggerLogicOption {
                    value: "all".to_string(),
                    label: "All must match".to_string(),
                    description: "All conditions must be true (AND)".to_string(),
                    requires_count: false,
                },
                TriggerLogicOption {
                    value: "any".to_string(),
                    label: "Any can match".to_string(),
                    description: "Any single condition can be true (OR)".to_string(),
                    requires_count: false,
                },
                TriggerLogicOption {
                    value: "atLeast".to_string(),
                    label: "At least N".to_string(),
                    description: "At least N conditions must be true".to_string(),
                    requires_count: true,
                },
            ],
        }
    }

    fn generate_trigger_types() -> Vec<TriggerTypeSchema> {
        vec![
            // Location triggers
            TriggerTypeSchema {
                type_name: "PlayerEntersLocation".to_string(),
                label: "Player Enters Location".to_string(),
                description: "Triggers when a player enters a specific location (city/area)"
                    .to_string(),
                category: TriggerCategory::Location,
                fields: vec![
                    TriggerFieldSchema {
                        name: "location_id".to_string(),
                        label: "Location".to_string(),
                        field_type: TriggerFieldType::LocationRef,
                        required: true,
                        description: Some("The location that triggers this event".to_string()),
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "location_name".to_string(),
                        label: "Location Name".to_string(),
                        field_type: TriggerFieldType::String,
                        required: false,
                        description: Some("Display name (auto-filled from selection)".to_string()),
                        default_value: None,
                    },
                ],
            },
            TriggerTypeSchema {
                type_name: "TimeAtLocation".to_string(),
                label: "Time at Location".to_string(),
                description: "Triggers when player is at location during specific time".to_string(),
                category: TriggerCategory::Location,
                fields: vec![
                    TriggerFieldSchema {
                        name: "location_id".to_string(),
                        label: "Location".to_string(),
                        field_type: TriggerFieldType::LocationRef,
                        required: true,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "location_name".to_string(),
                        label: "Location Name".to_string(),
                        field_type: TriggerFieldType::String,
                        required: false,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "time_context".to_string(),
                        label: "Time Context".to_string(),
                        field_type: TriggerFieldType::String,
                        required: true,
                        description: Some(
                            "Time description (e.g., 'at night', 'during the festival')"
                                .to_string(),
                        ),
                        default_value: None,
                    },
                ],
            },
            // Character triggers
            TriggerTypeSchema {
                type_name: "NpcAction".to_string(),
                label: "NPC Action".to_string(),
                description: "Triggers when an NPC performs a specific action".to_string(),
                category: TriggerCategory::Character,
                fields: vec![
                    TriggerFieldSchema {
                        name: "npc_id".to_string(),
                        label: "NPC".to_string(),
                        field_type: TriggerFieldType::CharacterRef,
                        required: true,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "npc_name".to_string(),
                        label: "NPC Name".to_string(),
                        field_type: TriggerFieldType::String,
                        required: false,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "action_keywords".to_string(),
                        label: "Action Keywords".to_string(),
                        field_type: TriggerFieldType::Keywords,
                        required: true,
                        description: Some("Keywords that identify the action".to_string()),
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "action_description".to_string(),
                        label: "Action Description".to_string(),
                        field_type: TriggerFieldType::String,
                        required: true,
                        description: Some("Human-readable description of the action".to_string()),
                        default_value: None,
                    },
                ],
            },
            TriggerTypeSchema {
                type_name: "DialogueTopic".to_string(),
                label: "Dialogue Topic".to_string(),
                description: "Triggers when a specific topic is discussed".to_string(),
                category: TriggerCategory::Character,
                fields: vec![
                    TriggerFieldSchema {
                        name: "keywords".to_string(),
                        label: "Topic Keywords".to_string(),
                        field_type: TriggerFieldType::Keywords,
                        required: true,
                        description: Some("Keywords that identify the topic".to_string()),
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "with_npc".to_string(),
                        label: "With NPC".to_string(),
                        field_type: TriggerFieldType::CharacterRef,
                        required: false,
                        description: Some("Optional: Only trigger with specific NPC".to_string()),
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "npc_name".to_string(),
                        label: "NPC Name".to_string(),
                        field_type: TriggerFieldType::String,
                        required: false,
                        description: None,
                        default_value: None,
                    },
                ],
            },
            TriggerTypeSchema {
                type_name: "RelationshipThreshold".to_string(),
                label: "Relationship Threshold".to_string(),
                description: "Triggers when relationship sentiment reaches a threshold".to_string(),
                category: TriggerCategory::Character,
                fields: vec![
                    TriggerFieldSchema {
                        name: "character_id".to_string(),
                        label: "Character".to_string(),
                        field_type: TriggerFieldType::CharacterRef,
                        required: true,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "character_name".to_string(),
                        label: "Character Name".to_string(),
                        field_type: TriggerFieldType::String,
                        required: false,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "with_character".to_string(),
                        label: "With Character".to_string(),
                        field_type: TriggerFieldType::CharacterRef,
                        required: true,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "with_character_name".to_string(),
                        label: "With Character Name".to_string(),
                        field_type: TriggerFieldType::String,
                        required: false,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "min_sentiment".to_string(),
                        label: "Min Sentiment".to_string(),
                        field_type: TriggerFieldType::Sentiment,
                        required: false,
                        description: Some("Minimum sentiment (-1.0 to 1.0)".to_string()),
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "max_sentiment".to_string(),
                        label: "Max Sentiment".to_string(),
                        field_type: TriggerFieldType::Sentiment,
                        required: false,
                        description: Some("Maximum sentiment (-1.0 to 1.0)".to_string()),
                        default_value: None,
                    },
                ],
            },
            // Inventory triggers
            TriggerTypeSchema {
                type_name: "HasItem".to_string(),
                label: "Has Item".to_string(),
                description: "Triggers when player has a specific item".to_string(),
                category: TriggerCategory::Inventory,
                fields: vec![
                    TriggerFieldSchema {
                        name: "item_name".to_string(),
                        label: "Item Name".to_string(),
                        field_type: TriggerFieldType::String,
                        required: true,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "quantity".to_string(),
                        label: "Quantity".to_string(),
                        field_type: TriggerFieldType::Integer,
                        required: false,
                        description: Some("Minimum quantity required (default: 1)".to_string()),
                        default_value: Some(serde_json::json!(1)),
                    },
                ],
            },
            TriggerTypeSchema {
                type_name: "MissingItem".to_string(),
                label: "Missing Item".to_string(),
                description: "Triggers when player does NOT have a specific item".to_string(),
                category: TriggerCategory::Inventory,
                fields: vec![TriggerFieldSchema {
                    name: "item_name".to_string(),
                    label: "Item Name".to_string(),
                    field_type: TriggerFieldType::String,
                    required: true,
                    description: None,
                    default_value: None,
                }],
            },
            // Challenge triggers
            TriggerTypeSchema {
                type_name: "ChallengeCompleted".to_string(),
                label: "Challenge Completed".to_string(),
                description: "Triggers when a challenge is completed".to_string(),
                category: TriggerCategory::Challenge,
                fields: vec![
                    TriggerFieldSchema {
                        name: "challenge_id".to_string(),
                        label: "Challenge".to_string(),
                        field_type: TriggerFieldType::ChallengeRef,
                        required: true,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "challenge_name".to_string(),
                        label: "Challenge Name".to_string(),
                        field_type: TriggerFieldType::String,
                        required: false,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "requires_success".to_string(),
                        label: "Requires Success".to_string(),
                        field_type: TriggerFieldType::Boolean,
                        required: false,
                        description: Some(
                            "If set, only triggers on success or failure".to_string(),
                        ),
                        default_value: None,
                    },
                ],
            },
            TriggerTypeSchema {
                type_name: "CombatResult".to_string(),
                label: "Combat Result".to_string(),
                description: "Triggers after combat ends".to_string(),
                category: TriggerCategory::Challenge,
                fields: vec![
                    TriggerFieldSchema {
                        name: "victory".to_string(),
                        label: "Victory Required".to_string(),
                        field_type: TriggerFieldType::Boolean,
                        required: false,
                        description: Some("If set, only triggers on victory or defeat".to_string()),
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "involved_npc".to_string(),
                        label: "Involved NPC".to_string(),
                        field_type: TriggerFieldType::CharacterRef,
                        required: false,
                        description: Some(
                            "Optional: Only trigger if this NPC was involved".to_string(),
                        ),
                        default_value: None,
                    },
                ],
            },
            // Time triggers
            TriggerTypeSchema {
                type_name: "TurnCount".to_string(),
                label: "Turn Count".to_string(),
                description: "Triggers after a certain number of turns".to_string(),
                category: TriggerCategory::Time,
                fields: vec![
                    TriggerFieldSchema {
                        name: "turns".to_string(),
                        label: "Turn Count".to_string(),
                        field_type: TriggerFieldType::Integer,
                        required: true,
                        description: Some("Number of turns to wait".to_string()),
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "since_event".to_string(),
                        label: "Since Event".to_string(),
                        field_type: TriggerFieldType::EventRef,
                        required: false,
                        description: Some(
                            "Count turns since this event (or session start)".to_string(),
                        ),
                        default_value: None,
                    },
                ],
            },
            // State triggers
            TriggerTypeSchema {
                type_name: "FlagSet".to_string(),
                label: "Flag Set".to_string(),
                description: "Triggers when a game flag is set to true".to_string(),
                category: TriggerCategory::State,
                fields: vec![TriggerFieldSchema {
                    name: "flag_name".to_string(),
                    label: "Flag Name".to_string(),
                    field_type: TriggerFieldType::String,
                    required: true,
                    description: Some("Name of the flag to check".to_string()),
                    default_value: None,
                }],
            },
            TriggerTypeSchema {
                type_name: "FlagNotSet".to_string(),
                label: "Flag Not Set".to_string(),
                description: "Triggers when a game flag is NOT set (or false)".to_string(),
                category: TriggerCategory::State,
                fields: vec![TriggerFieldSchema {
                    name: "flag_name".to_string(),
                    label: "Flag Name".to_string(),
                    field_type: TriggerFieldType::String,
                    required: true,
                    description: Some("Name of the flag to check".to_string()),
                    default_value: None,
                }],
            },
            TriggerTypeSchema {
                type_name: "StatThreshold".to_string(),
                label: "Stat Threshold".to_string(),
                description: "Triggers when a character stat reaches a threshold".to_string(),
                category: TriggerCategory::State,
                fields: vec![
                    TriggerFieldSchema {
                        name: "character_id".to_string(),
                        label: "Character".to_string(),
                        field_type: TriggerFieldType::CharacterRef,
                        required: true,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "stat_name".to_string(),
                        label: "Stat Name".to_string(),
                        field_type: TriggerFieldType::String,
                        required: true,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "min_value".to_string(),
                        label: "Min Value".to_string(),
                        field_type: TriggerFieldType::Integer,
                        required: false,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "max_value".to_string(),
                        label: "Max Value".to_string(),
                        field_type: TriggerFieldType::Integer,
                        required: false,
                        description: None,
                        default_value: None,
                    },
                ],
            },
            // Event triggers
            TriggerTypeSchema {
                type_name: "EventCompleted".to_string(),
                label: "Event Completed".to_string(),
                description: "Triggers when another narrative event is completed".to_string(),
                category: TriggerCategory::Event,
                fields: vec![
                    TriggerFieldSchema {
                        name: "event_id".to_string(),
                        label: "Event".to_string(),
                        field_type: TriggerFieldType::EventRef,
                        required: true,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "event_name".to_string(),
                        label: "Event Name".to_string(),
                        field_type: TriggerFieldType::String,
                        required: false,
                        description: None,
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "outcome_name".to_string(),
                        label: "Required Outcome".to_string(),
                        field_type: TriggerFieldType::String,
                        required: false,
                        description: Some(
                            "Optional: Only trigger if this outcome was selected".to_string(),
                        ),
                        default_value: None,
                    },
                ],
            },
            // Custom triggers
            TriggerTypeSchema {
                type_name: "Custom".to_string(),
                label: "Custom Condition".to_string(),
                description: "A custom condition described in natural language".to_string(),
                category: TriggerCategory::Custom,
                fields: vec![
                    TriggerFieldSchema {
                        name: "description".to_string(),
                        label: "Description".to_string(),
                        field_type: TriggerFieldType::String,
                        required: true,
                        description: Some("Describe when this should trigger".to_string()),
                        default_value: None,
                    },
                    TriggerFieldSchema {
                        name: "llm_evaluation".to_string(),
                        label: "LLM Evaluation".to_string(),
                        field_type: TriggerFieldType::Boolean,
                        required: false,
                        description: Some("If true, LLM will evaluate this condition".to_string()),
                        default_value: Some(serde_json::json!(true)),
                    },
                ],
            },
        ]
    }
}
