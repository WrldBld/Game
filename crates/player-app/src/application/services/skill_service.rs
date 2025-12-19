//! Skill Service - Application service for skill management
//!
//! This service provides use case implementations for listing, creating,
//! updating, and deleting skills. It abstracts away the HTTP client
//! details from the presentation layer.

use serde::Serialize;

use crate::application::dto::{SkillCategory, SkillData};
use wrldbldr_player_ports::outbound::{ApiError, ApiPort};

/// Request to create a new skill
#[derive(Clone, Debug, Serialize)]
pub struct CreateSkillRequest {
    pub name: String,
    pub description: String,
    pub category: SkillCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_attribute: Option<String>,
}

/// Request to update a skill
#[derive(Clone, Debug, Serialize)]
pub struct UpdateSkillRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<SkillCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_attribute: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_hidden: Option<bool>,
}

/// Skill service for managing skills
///
/// This service provides methods for skill-related operations
/// while depending only on the `ApiPort` trait, not concrete
/// infrastructure implementations.
pub struct SkillService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> SkillService<A> {
    /// Create a new SkillService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    /// List all skills in a world
    pub async fn list_skills(&self, world_id: &str) -> Result<Vec<SkillData>, ApiError> {
        let path = format!("/api/worlds/{}/skills", world_id);
        self.api.get(&path).await
    }

    /// Get a single skill by ID
    pub async fn get_skill(
        &self,
        world_id: &str,
        skill_id: &str,
    ) -> Result<SkillData, ApiError> {
        let path = format!("/api/worlds/{}/skills/{}", world_id, skill_id);
        self.api.get(&path).await
    }

    /// Create a new skill
    pub async fn create_skill(
        &self,
        world_id: &str,
        request: &CreateSkillRequest,
    ) -> Result<SkillData, ApiError> {
        let path = format!("/api/worlds/{}/skills", world_id);
        self.api.post(&path, request).await
    }

    /// Update an existing skill
    pub async fn update_skill(
        &self,
        world_id: &str,
        skill_id: &str,
        request: &UpdateSkillRequest,
    ) -> Result<SkillData, ApiError> {
        let path = format!("/api/worlds/{}/skills/{}", world_id, skill_id);
        self.api.put(&path, request).await
    }

    /// Update skill visibility
    pub async fn update_skill_visibility(
        &self,
        world_id: &str,
        skill_id: &str,
        is_hidden: bool,
    ) -> Result<SkillData, ApiError> {
        let path = format!("/api/worlds/{}/skills/{}", world_id, skill_id);
        let request = UpdateSkillRequest {
            name: None,
            description: None,
            category: None,
            base_attribute: None,
            is_hidden: Some(is_hidden),
        };
        self.api.put(&path, &request).await
    }

    /// Delete a skill
    pub async fn delete_skill(&self, world_id: &str, skill_id: &str) -> Result<(), ApiError> {
        let path = format!("/api/worlds/{}/skills/{}", world_id, skill_id);
        self.api.delete(&path).await
    }
}

impl<A: ApiPort + Clone> Clone for SkillService<A> {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}
