//! Application-layer request DTOs
//!
//! These DTOs are used by player-app services to construct requests.
//! They are converted to protocol types at the service boundary before
//! being sent via CommandBus.
//!
//! This isolates the application layer from wire format concerns.
//!
//! Note: Many services define their own local DTOs with richer types
//! (e.g., SkillCategory enum instead of String). Those services have
//! their own From impls defined inline. This module provides shared
//! DTOs for services that don't need custom types.

use serde::{Deserialize, Serialize};

// ============================================================================
// World Requests
// ============================================================================

/// Application-layer DTO for creating a world
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreateWorldRequest {
    pub name: String,
    pub description: Option<String>,
    pub setting: Option<String>,
}

impl From<CreateWorldRequest> for wrldbldr_protocol::CreateWorldData {
    fn from(req: CreateWorldRequest) -> Self {
        Self {
            name: req.name,
            description: req.description,
            setting: req.setting,
        }
    }
}

// ============================================================================
// Character Requests
// ============================================================================

/// Application-layer DTO for creating a character
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreateCharacterRequest {
    pub name: String,
    pub description: Option<String>,
    pub archetype: Option<String>,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
}

impl From<CreateCharacterRequest> for wrldbldr_protocol::requests::CreateCharacterData {
    fn from(req: CreateCharacterRequest) -> Self {
        Self {
            name: req.name,
            description: req.description,
            archetype: req.archetype,
            sprite_asset: req.sprite_asset,
            portrait_asset: req.portrait_asset,
        }
    }
}

/// Application-layer DTO for updating a character
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdateCharacterRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_alive: Option<bool>,
    pub is_active: Option<bool>,
}

impl From<UpdateCharacterRequest> for wrldbldr_protocol::requests::UpdateCharacterData {
    fn from(req: UpdateCharacterRequest) -> Self {
        Self {
            name: req.name,
            description: req.description,
            sprite_asset: req.sprite_asset,
            portrait_asset: req.portrait_asset,
            is_alive: req.is_alive,
            is_active: req.is_active,
        }
    }
}

/// Application-layer DTO for changing a character's archetype
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChangeArchetypeRequest {
    pub new_archetype: String,
    pub reason: String,
}

impl From<ChangeArchetypeRequest> for wrldbldr_protocol::requests::ChangeArchetypeData {
    fn from(req: ChangeArchetypeRequest) -> Self {
        Self {
            new_archetype: req.new_archetype,
            reason: req.reason,
        }
    }
}

// ============================================================================
// Suggestion Requests
// ============================================================================

/// Application-layer DTO for suggestion context
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SuggestionContext {
    pub entity_type: Option<String>,
    pub entity_name: Option<String>,
    pub world_setting: Option<String>,
    pub hints: Option<String>,
    pub additional_context: Option<String>,
    pub world_id: Option<uuid::Uuid>,
}

impl From<SuggestionContext> for wrldbldr_protocol::SuggestionContextData {
    fn from(ctx: SuggestionContext) -> Self {
        Self {
            entity_type: ctx.entity_type,
            entity_name: ctx.entity_name,
            world_setting: ctx.world_setting,
            hints: ctx.hints,
            additional_context: ctx.additional_context,
            world_id: ctx.world_id,
        }
    }
}

// ============================================================================
// Challenge Requests
// ============================================================================

/// Application-layer DTO for creating a challenge
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreateChallengeRequest {
    pub name: String,
    pub description: Option<String>,
    pub skill_id: String,
    pub difficulty: String,
    pub success_outcome: Option<String>,
    pub failure_outcome: Option<String>,
}

impl From<CreateChallengeRequest> for wrldbldr_protocol::CreateChallengeData {
    fn from(req: CreateChallengeRequest) -> Self {
        Self {
            name: req.name,
            description: req.description,
            skill_id: req.skill_id,
            difficulty: req.difficulty,
            success_outcome: req.success_outcome,
            failure_outcome: req.failure_outcome,
        }
    }
}

/// Application-layer DTO for updating a challenge
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdateChallengeRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub skill_id: Option<String>,
    pub difficulty: Option<String>,
    pub success_outcome: Option<String>,
    pub failure_outcome: Option<String>,
}

impl From<UpdateChallengeRequest> for wrldbldr_protocol::UpdateChallengeData {
    fn from(req: UpdateChallengeRequest) -> Self {
        Self {
            name: req.name,
            description: req.description,
            skill_id: req.skill_id,
            difficulty: req.difficulty,
            success_outcome: req.success_outcome,
            failure_outcome: req.failure_outcome,
        }
    }
}
