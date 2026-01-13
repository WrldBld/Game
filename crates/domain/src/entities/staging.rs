//! Staging entity - DM-approved NPC presence and visual state for a region
//!
//! # Neo4j Relationships
//! - `(Region)-[:CURRENT_STAGING]->(Staging)` - Active staging for region
//! - `(Region)-[:HAS_STAGING]->(Staging)` - Historical stagings
//! - `(Staging)-[:INCLUDES_NPC {is_present, reasoning}]->(Character)` - NPCs in staging
//! - `(Staging)-[:USES_LOCATION_STATE]->(LocationState)` - Visual state at location level
//! - `(Staging)-[:USES_REGION_STATE]->(RegionState)` - Visual state at region level

use crate::value_objects::MoodState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wrldbldr_domain::{
    CharacterId, LocationId, LocationStateId, RegionId, RegionStateId, StagingId, WorldId,
};

/// A DM-approved configuration of NPC presence and visual state for a region
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Staging {
    pub id: StagingId,
    pub region_id: RegionId,
    pub location_id: LocationId,
    pub world_id: WorldId,
    /// NPCs included in this staging with their presence status
    pub npcs: Vec<StagedNpc>,
    /// Game time when this staging was approved
    pub game_time: DateTime<Utc>,
    /// Real time when DM approved
    pub approved_at: DateTime<Utc>,
    /// How long valid in game hours
    pub ttl_hours: i32,
    /// Client ID of approving DM
    pub approved_by: String,
    /// How this staging was created
    pub source: StagingSource,
    /// Optional DM guidance for LLM regeneration
    pub dm_guidance: Option<String>,
    /// Whether this is the current active staging
    pub is_active: bool,

    // Visual State
    /// Resolved location state for this staging (if any)
    pub location_state_id: Option<LocationStateId>,
    /// Resolved region state for this staging (if any)
    pub region_state_id: Option<RegionStateId>,
    /// How the visual state was resolved
    pub visual_state_source: VisualStateSource,
    /// LLM reasoning for soft rule evaluation (if any)
    pub visual_state_reasoning: Option<String>,
}

/// An NPC with presence status in a staging
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedNpc {
    pub character_id: CharacterId,
    /// Denormalized for display
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    /// Whether NPC is present in this staging
    pub is_present: bool,
    /// When true, NPC is present but hidden from players
    pub is_hidden_from_players: bool,
    /// Reasoning for presence/absence (from rules or LLM)
    pub reasoning: String,
    /// NPC's current mood for this staging (Tier 2 of emotional model)
    /// Affects default expression and dialogue tone
    /// Set by DM during staging approval, or defaults to character's default_mood
    pub mood: MoodState,
    /// When true, character data was not found during staging approval.
    /// This NPC was included with empty defaults and may need attention.
    #[serde(default)]
    pub has_incomplete_data: bool,
}

impl StagedNpc {
    /// Returns true if this NPC should be visible to players.
    /// An NPC is visible when present and not hidden from players.
    pub fn is_visible_to_players(&self) -> bool {
        self.is_present && !self.is_hidden_from_players
    }
}

/// How a staging was created
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StagingSource {
    /// Created from deterministic rules
    RuleBased,
    /// Created with LLM reasoning
    LlmBased,
    /// DM manually customized the staging
    DmCustomized,
    /// DM pre-staged before player arrival
    PreStaged,
    /// Auto-approved on timeout (using rule-based NPCs)
    AutoApproved,
    /// Unknown source (for forward compatibility)
    #[serde(other)]
    Unknown,
}

/// How visual state was resolved for a staging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VisualStateSource {
    /// States resolved from hard rules only
    #[default]
    HardRulesOnly,
    /// States included LLM soft rule evaluation
    WithLlmEvaluation,
    /// DM manually selected states
    DmOverride,
    /// Using default states (no specific rules matched)
    Default,
}

/// Summary of resolved visual state for display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedVisualState {
    pub location_state: Option<ResolvedStateInfo>,
    pub region_state: Option<ResolvedStateInfo>,
}

/// Info about a resolved state for display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedStateInfo {
    pub id: String,
    pub name: String,
    pub backdrop_override: Option<String>,
    pub atmosphere_override: Option<String>,
    pub ambient_sound: Option<String>,
}

impl Staging {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        game_time: DateTime<Utc>,
        approved_by: impl Into<String>,
        source: StagingSource,
        ttl_hours: i32,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: StagingId::new(),
            region_id,
            location_id,
            world_id,
            npcs: Vec::new(),
            game_time,
            approved_at: now,
            ttl_hours,
            approved_by: approved_by.into(),
            source,
            dm_guidance: None,
            is_active: true,
            location_state_id: None,
            region_state_id: None,
            visual_state_source: VisualStateSource::default(),
            visual_state_reasoning: None,
        }
    }

    pub fn with_npcs(mut self, npcs: Vec<StagedNpc>) -> Self {
        self.npcs = npcs;
        self
    }

    pub fn with_guidance(mut self, guidance: impl Into<String>) -> Self {
        self.dm_guidance = Some(guidance.into());
        self
    }

    pub fn with_location_state(mut self, state_id: LocationStateId) -> Self {
        self.location_state_id = Some(state_id);
        self
    }

    pub fn with_region_state(mut self, state_id: RegionStateId) -> Self {
        self.region_state_id = Some(state_id);
        self
    }

    pub fn with_visual_state_source(mut self, source: VisualStateSource) -> Self {
        self.visual_state_source = source;
        self
    }

    pub fn with_visual_state_reasoning(mut self, reasoning: impl Into<String>) -> Self {
        self.visual_state_reasoning = Some(reasoning.into());
        self
    }

    /// Check if this staging has any visual state configured
    pub fn has_visual_state(&self) -> bool {
        self.location_state_id.is_some() || self.region_state_id.is_some()
    }

    /// Check if staging has expired based on game time
    pub fn is_expired(&self, current_game_time: &DateTime<Utc>) -> bool {
        let duration = chrono::Duration::hours(self.ttl_hours as i64);
        current_game_time > &(self.game_time + duration)
    }

    /// Get only present NPCs
    pub fn present_npcs(&self) -> Vec<&StagedNpc> {
        self.npcs.iter().filter(|n| n.is_present).collect()
    }

    /// Get present NPCs that are visible to players
    pub fn present_visible_npcs(&self) -> Vec<&StagedNpc> {
        self.npcs
            .iter()
            .filter(|n| n.is_present && !n.is_hidden_from_players)
            .collect()
    }
}

impl StagedNpc {
    pub fn new(
        character_id: CharacterId,
        name: impl Into<String>,
        is_present: bool,
        reasoning: impl Into<String>,
    ) -> Self {
        Self {
            character_id,
            name: name.into(),
            sprite_asset: None,
            portrait_asset: None,
            is_present,
            is_hidden_from_players: false,
            reasoning: reasoning.into(),
            mood: MoodState::default(),
            has_incomplete_data: false,
        }
    }

    pub fn with_incomplete_data(mut self, incomplete: bool) -> Self {
        self.has_incomplete_data = incomplete;
        self
    }

    pub fn with_sprite(mut self, asset: impl Into<String>) -> Self {
        self.sprite_asset = Some(asset.into());
        self
    }

    pub fn with_portrait(mut self, asset: impl Into<String>) -> Self {
        self.portrait_asset = Some(asset.into());
        self
    }

    pub fn with_mood(mut self, mood: MoodState) -> Self {
        self.mood = mood;
        self
    }
}

impl std::fmt::Display for StagingSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StagingSource::RuleBased => write!(f, "rule"),
            StagingSource::LlmBased => write!(f, "llm"),
            StagingSource::DmCustomized => write!(f, "custom"),
            StagingSource::PreStaged => write!(f, "prestaged"),
            StagingSource::AutoApproved => write!(f, "auto"),
            StagingSource::Unknown => write!(f, "unknown"),
        }
    }
}

impl std::str::FromStr for StagingSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rule" | "rulebased" => Ok(StagingSource::RuleBased),
            "llm" | "llmbased" => Ok(StagingSource::LlmBased),
            "custom" | "dmcustomized" => Ok(StagingSource::DmCustomized),
            "prestaged" => Ok(StagingSource::PreStaged),
            "auto" | "autoapproved" => Ok(StagingSource::AutoApproved),
            "unknown" => Ok(StagingSource::Unknown),
            _ => Ok(StagingSource::Unknown), // Forward compatibility
        }
    }
}

impl std::fmt::Display for VisualStateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisualStateSource::HardRulesOnly => write!(f, "rules"),
            VisualStateSource::WithLlmEvaluation => write!(f, "llm"),
            VisualStateSource::DmOverride => write!(f, "dm"),
            VisualStateSource::Default => write!(f, "default"),
        }
    }
}

impl std::str::FromStr for VisualStateSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rules" | "hardrulesonly" => Ok(VisualStateSource::HardRulesOnly),
            "llm" | "withllmevaluation" => Ok(VisualStateSource::WithLlmEvaluation),
            "dm" | "dmoverride" => Ok(VisualStateSource::DmOverride),
            "default" => Ok(VisualStateSource::Default),
            _ => Err(format!("Unknown visual state source: {}", s)),
        }
    }
}

impl ResolvedVisualState {
    pub fn new() -> Self {
        Self {
            location_state: None,
            region_state: None,
        }
    }

    pub fn with_location_state(mut self, info: ResolvedStateInfo) -> Self {
        self.location_state = Some(info);
        self
    }

    pub fn with_region_state(mut self, info: ResolvedStateInfo) -> Self {
        self.region_state = Some(info);
        self
    }

    pub fn has_any(&self) -> bool {
        self.location_state.is_some() || self.region_state.is_some()
    }
}

impl Default for ResolvedVisualState {
    fn default() -> Self {
        Self::new()
    }
}

impl ResolvedStateInfo {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            backdrop_override: None,
            atmosphere_override: None,
            ambient_sound: None,
        }
    }

    pub fn with_backdrop(mut self, path: impl Into<String>) -> Self {
        self.backdrop_override = Some(path.into());
        self
    }

    pub fn with_atmosphere(mut self, atmosphere: impl Into<String>) -> Self {
        self.atmosphere_override = Some(atmosphere.into());
        self
    }

    pub fn with_ambient_sound(mut self, path: impl Into<String>) -> Self {
        self.ambient_sound = Some(path.into());
        self
    }
}
