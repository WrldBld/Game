//! NPC Service - Application service for NPC disposition, mood, and region relationships
//!
//! This service provides use case implementations for managing NPC relationships
//! with PCs, mood states in regions, and character-region relationships.

use serde::Deserialize;

use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::infrastructure::messaging::CommandBus;
use wrldbldr_domain::{DispositionLevel, MoodState};
use wrldbldr_shared::{NpcRequest, RequestPayload};

/// Response data for NPC dispositions
#[derive(Clone, Debug, Deserialize)]
pub struct NpcDispositionsResponse {
    pub dispositions: Vec<NpcDispositionInfo>,
}

/// NPC disposition info
#[derive(Clone, Debug, Deserialize)]
pub struct NpcDispositionInfo {
    pub npc_id: String,
    pub npc_name: String,
    pub disposition: DispositionLevel,
    pub relationship: String,
    #[serde(default)]
    pub last_reason: Option<String>,
}

/// Response data for NPC mood
#[derive(Clone, Debug, Deserialize)]
pub struct NpcMoodResponse {
    pub npc_id: String,
    pub region_id: String,
    pub mood: MoodState,
    #[serde(default)]
    pub last_reason: Option<String>,
}

/// Response data for character-region relationships
#[derive(Clone, Debug, Deserialize)]
pub struct CharacterRegionRelationshipsResponse {
    pub relationships: Vec<CharacterRegionRelationship>,
}

/// Character-region relationship info
#[derive(Clone, Debug, Deserialize)]
pub struct CharacterRegionRelationship {
    pub region_id: String,
    pub region_name: String,
    pub location_id: String,
    pub location_name: String,
    pub relationship_type: String,
}

/// Response data for region NPCs list
#[derive(Clone, Debug, Deserialize)]
pub struct RegionNpcsResponse {
    pub npcs: Vec<RegionNpcInfo>,
}

/// Region NPC info
#[derive(Clone, Debug, Deserialize)]
pub struct RegionNpcInfo {
    pub npc_id: String,
    pub npc_name: String,
    pub relationship_type: String,
}

/// NPC service for managing NPC relationships and region associations
///
/// This service provides methods for NPC disposition, mood, and
/// character-region relationship operations using the NpcRequest protocol
/// via the `CommandBus`.
#[derive(Clone)]
pub struct NpcService {
    commands: CommandBus,
}

impl NpcService {
    /// Create a new NpcService with the given command bus
    pub fn new(commands: CommandBus) -> Self {
        Self { commands }
    }

    // === Disposition Operations ===

    /// Set NPC disposition toward a PC
    pub async fn set_disposition(
        &self,
        npc_id: &str,
        pc_id: &str,
        disposition: DispositionLevel,
        reason: Option<String>,
    ) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Npc(NpcRequest::SetNpcDisposition {
                    npc_id: npc_id.to_string(),
                    pc_id: pc_id.to_string(),
                    disposition,
                    reason,
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse_empty()
    }

    /// Set NPC relationship with a PC
    pub async fn set_relationship(
        &self,
        npc_id: &str,
        pc_id: &str,
        relationship: String,
    ) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Npc(NpcRequest::SetNpcRelationship {
                    npc_id: npc_id.to_string(),
                    pc_id: pc_id.to_string(),
                    relationship,
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse_empty()
    }

    /// Get all NPC dispositions for a PC
    pub async fn get_dispositions(
        &self,
        pc_id: &str,
    ) -> Result<NpcDispositionsResponse, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Npc(NpcRequest::GetNpcDispositions {
                    pc_id: pc_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }

    // === Mood Operations ===

    /// Set NPC mood in a region
    pub async fn set_mood(
        &self,
        npc_id: &str,
        region_id: &str,
        mood: MoodState,
        reason: Option<String>,
    ) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Npc(NpcRequest::SetNpcMood {
                    npc_id: npc_id.to_string(),
                    region_id: region_id.to_string(),
                    mood,
                    reason,
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse_empty()
    }

    /// Get NPC mood in a region
    pub async fn get_mood(
        &self,
        npc_id: &str,
        region_id: &str,
    ) -> Result<NpcMoodResponse, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Npc(NpcRequest::GetNpcMood {
                    npc_id: npc_id.to_string(),
                    region_id: region_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }

    // === Character-Region Relationship Operations ===

    /// List all region relationships for a character
    pub async fn list_character_regions(
        &self,
        character_id: &str,
    ) -> Result<CharacterRegionRelationshipsResponse, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Npc(NpcRequest::ListCharacterRegionRelationships {
                    character_id: character_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }

    /// Set character's home region
    pub async fn set_home_region(
        &self,
        character_id: &str,
        region_id: &str,
    ) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Npc(NpcRequest::SetCharacterHomeRegion {
                    character_id: character_id.to_string(),
                    region_id: region_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse_empty()
    }

    /// Set character's work region
    pub async fn set_work_region(
        &self,
        character_id: &str,
        region_id: &str,
    ) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Npc(NpcRequest::SetCharacterWorkRegion {
                    character_id: character_id.to_string(),
                    region_id: region_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse_empty()
    }

    /// Remove a character-region relationship
    pub async fn remove_region_relationship(
        &self,
        character_id: &str,
        region_id: &str,
        relationship_type: String,
    ) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Npc(NpcRequest::RemoveCharacterRegionRelationship {
                    character_id: character_id.to_string(),
                    region_id: region_id.to_string(),
                    relationship_type,
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse_empty()
    }

    /// List all NPCs associated with a region
    pub async fn list_region_npcs(
        &self,
        region_id: &str,
    ) -> Result<RegionNpcsResponse, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Npc(NpcRequest::ListRegionNpcs {
                    region_id: region_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }
}
