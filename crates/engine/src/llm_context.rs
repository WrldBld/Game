//! LLM context types - Types for building LLM prompts from game state.
//!
//! These DTOs are serialized to JSON for outbound LLM requests and are
//! intentionally owned by the engine (not the domain) to keep domain pure.

use serde::{Deserialize, Serialize};

use crate::infrastructure::app_settings::ContextBudgetConfig;
use wrldbldr_domain::ActorType;

/// Request for generating an NPC response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePromptRequest {
    /// World ID for per-world prompt template resolution (UUID string)
    #[serde(default)]
    pub world_id: Option<String>,
    /// The player's action that triggered this response
    pub player_action: PlayerActionContext,
    /// Current scene information
    pub scene_context: SceneContext,
    /// Director's notes for guiding the AI response
    pub directorial_notes: String,
    /// Previous conversation turns for context
    pub conversation_history: Vec<ConversationTurn>,
    /// The NPC who is responding
    pub responding_character: CharacterContext,
    /// Active challenges that could be triggered
    pub active_challenges: Vec<ActiveChallengeContext>,
    /// Active narrative events that could be triggered
    pub active_narrative_events: Vec<ActiveNarrativeEventContext>,
    /// Optional token budget configuration for prompt size limits
    /// When present, the system prompt will be truncated to fit within the budget
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_budget: Option<ContextBudgetConfig>,

    // =========================================================================
    // Context for dialogue persistence (P1.2)
    // These fields are populated in build_prompt_from_action and propagated
    // through LLMRequestItem -> ApprovalItem for use in record_dialogue_exchange
    // =========================================================================
    /// Current scene ID (UUID string) for story event recording
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene_id: Option<String>,
    /// Current location ID (UUID string) for story event recording
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location_id: Option<String>,
    /// Current game time as display string for story event recording
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_time: Option<String>,
}

/// Context about the player's action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerActionContext {
    /// Type of action: "speak", "examine", "use_item", etc.
    pub action_type: String,
    /// Target of the action (NPC name, object, etc.)
    pub target: Option<String>,
    /// Dialogue content if the action is speech
    pub dialogue: Option<String>,
}

/// Context about the current scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneContext {
    /// Name of the current scene
    pub scene_name: String,
    /// Name of the location
    pub location_name: String,
    /// Time of day / narrative time context
    pub time_context: String,
    /// Names of characters present in the scene
    pub present_characters: Vec<String>,
    /// Items visible in the current region (for NPC awareness)
    #[serde(default)]
    pub region_items: Vec<RegionItemContext>,
}

/// Context about an item visible in the current region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionItemContext {
    /// Item's display name
    pub name: String,
    /// Brief description of the item
    pub description: Option<String>,
    /// Type of item (e.g., "Weapon", "Key", "Quest")
    pub item_type: Option<String>,
}

/// Context about the responding character.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterContext {
    /// Character's unique identifier (for story event recording)
    #[serde(default)]
    pub character_id: Option<String>,
    /// Character's name
    pub name: String,
    /// Character archetype / personality summary
    pub archetype: String,
    /// Current emotional state (Tier 2 of emotional model)
    /// E.g., "Anxious", "Calm", "Excited"
    pub current_mood: Option<String>,
    /// NPC's disposition toward the player (Tier 1 of emotional model)
    /// E.g., "Friendly", "Suspicious", "Hostile"
    /// This is the emotional stance, separate from relationship level
    #[serde(default)]
    pub disposition_toward_player: Option<String>,
    /// Character's motivations (rich actantial model context)
    #[serde(default)]
    pub motivations: Option<MotivationsContext>,
    /// Character's social stance (aggregated allies/enemies)
    #[serde(default)]
    pub social_stance: Option<SocialStanceContext>,
    /// How this character relates to the player (relationship level)
    /// E.g., "Stranger", "Acquaintance", "Friend", "Ally"
    pub relationship_to_player: Option<String>,
    /// Available expression names for this character's sprite sheet (Tier 3 of emotional model)
    /// E.g., ["neutral", "happy", "sad", "angry", "surprised"]
    /// Used for expression markers in dialogue: *happy*, *suspicious*
    #[serde(default)]
    pub available_expressions: Option<Vec<String>>,
    /// Available action descriptions for this character
    /// E.g., ["crosses arms", "narrows eyes", "sighs"]
    /// Used for action markers in dialogue: *crosses arms*
    #[serde(default)]
    pub available_actions: Option<Vec<String>>,
}

// =============================================================================
// Motivation Context (Actantial Model for LLM)
// =============================================================================

/// Complete motivations context for LLM prompt.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MotivationsContext {
    /// Known motivations (player knows about these)
    #[serde(default)]
    pub known: Vec<MotivationEntry>,
    /// Suspected motivations (player senses something)
    #[serde(default)]
    pub suspected: Vec<MotivationEntry>,
    /// Secret motivations (player has no idea)
    #[serde(default)]
    pub secret: Vec<SecretMotivationEntry>,
}

/// A known or suspected motivation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotivationEntry {
    /// Description of the motivation
    pub description: String,
    /// Priority level (1 = primary)
    pub priority: u32,
    /// Intensity description (e.g., "Strong", "Moderate")
    pub intensity: String,
    /// What the character is targeting (if any)
    pub target: Option<String>,
    /// Characters who help achieve this
    #[serde(default)]
    pub helpers: Vec<ActantialActorEntry>,
    /// Characters who oppose this
    #[serde(default)]
    pub opponents: Vec<ActantialActorEntry>,
}

/// A secret motivation with behavioral guidance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMotivationEntry {
    /// Description of the secret motivation
    pub description: String,
    /// Priority level (1 = primary)
    pub priority: u32,
    /// Intensity description
    pub intensity: String,
    /// What the character is targeting (if any)
    pub target: Option<String>,
    /// Characters who help achieve this
    #[serde(default)]
    pub helpers: Vec<ActantialActorEntry>,
    /// Characters who oppose this
    #[serde(default)]
    pub opponents: Vec<ActantialActorEntry>,
    /// Who/what initiated this motivation
    pub sender: Option<ActantialActorEntry>,
    /// Who benefits from this being fulfilled
    pub receiver: Option<ActantialActorEntry>,
    /// How to behave when probed about this secret
    pub deflection_behavior: String,
    /// Subtle behavioral tells that hint at this motivation
    #[serde(default)]
    pub tells: Vec<String>,
}

/// An actor in the actantial model (helper, opponent, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActantialActorEntry {
    /// Name of the actor
    pub name: String,
    /// Whether this is an NPC or PC
    pub actor_type: ActorType,
    /// Reason for this role assignment
    pub reason: String,
}

/// Social stance summary for LLM.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SocialStanceContext {
    /// Characters the NPC considers allies
    #[serde(default)]
    pub allies: Vec<SocialRelationEntry>,
    /// Characters the NPC considers enemies
    #[serde(default)]
    pub enemies: Vec<SocialRelationEntry>,
}

/// A social relation entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialRelationEntry {
    /// Name of the character
    pub name: String,
    /// Whether this is an NPC or PC
    pub character_type: ActorType,
    /// Reasons for this relationship (aggregated from wants)
    pub reasons: Vec<String>,
}

/// A single turn in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Name of the speaker
    pub speaker: String,
    /// What was said
    pub text: String,
}

/// Context about an active challenge that may be triggered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveChallengeContext {
    /// Unique identifier for the challenge
    pub id: String,
    /// Display name of the challenge
    pub name: String,
    /// Full description of the challenge
    pub description: String,
    /// Name of the skill required
    pub skill_name: String,
    /// Human-readable difficulty display (e.g. "DC 15", "Hard")
    pub difficulty_display: String,
    /// Keywords/phrases that trigger this challenge
    pub trigger_hints: Vec<String>,
}

/// Context about an active narrative event that may be triggered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveNarrativeEventContext {
    /// Unique identifier for the narrative event
    pub id: String,
    /// Display name of the event
    pub name: String,
    /// Brief description of what this event represents
    pub description: String,
    /// Scene direction text to help DM narrate when triggered
    pub scene_direction: String,
    /// Keywords/phrases that indicate this event should trigger
    pub trigger_hints: Vec<String>,
    /// Names of NPCs featured in this event
    pub featured_npc_names: Vec<String>,
    /// Priority level (higher = more important)
    pub priority: i32,
}
