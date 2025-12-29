//! Staging entity - DM-approved NPC presence for a region
//!
//! # Neo4j Relationships
//! - `(Region)-[:CURRENT_STAGING]->(Staging)` - Active staging for region
//! - `(Region)-[:HAS_STAGING]->(Staging)` - Historical stagings
//! - `(Staging)-[:INCLUDES_NPC {is_present, reasoning}]->(Character)` - NPCs in staging

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wrldbldr_domain::{CharacterId, LocationId, RegionId, StagingId, WorldId};

/// A DM-approved configuration of NPC presence for a region
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
}

impl Staging {
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
        }
    }

    pub fn with_sprite(mut self, asset: impl Into<String>) -> Self {
        self.sprite_asset = Some(asset.into());
        self
    }

    pub fn with_portrait(mut self, asset: impl Into<String>) -> Self {
        self.portrait_asset = Some(asset.into());
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
            _ => Err(format!("Unknown staging source: {}", s)),
        }
    }
}
