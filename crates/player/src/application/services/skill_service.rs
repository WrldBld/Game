//! Skill Service - Application service for skill management
//!
//! This service provides use case implementations for listing, creating,
//! updating, and deleting skills via WebSocket request/response pattern.

use std::sync::Arc;

use serde::Serialize;

use crate::application::dto::{SkillCategory, SkillData};
use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::ports::outbound::GameConnectionPort;
use wrldbldr_protocol::{RequestPayload, SkillRequest};

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

// From impls for protocol conversion at the boundary
impl From<&CreateSkillRequest> for wrldbldr_protocol::requests::CreateSkillData {
    fn from(req: &CreateSkillRequest) -> Self {
        Self {
            name: req.name.clone(),
            description: if req.description.is_empty() {
                None
            } else {
                Some(req.description.clone())
            },
            category: Some(req.category.to_string()),
            attribute: req.base_attribute.clone(),
        }
    }
}

impl From<&UpdateSkillRequest> for wrldbldr_protocol::requests::UpdateSkillData {
    fn from(req: &UpdateSkillRequest) -> Self {
        Self {
            name: req.name.clone(),
            description: req.description.clone(),
            category: req.category.as_ref().map(|c| c.to_string()),
            attribute: req.base_attribute.clone(),
            is_hidden: req.is_hidden,
        }
    }
}

/// Skill service for managing skills
///
/// This service provides methods for skill-related operations
/// using WebSocket request/response pattern via the `GameConnectionPort`.
#[derive(Clone)]
pub struct SkillService {
    connection: Arc<dyn GameConnectionPort>,
}

impl SkillService {
    /// Create a new SkillService with the given connection
    pub fn new(connection: Arc<dyn GameConnectionPort>) -> Self {
        Self { connection }
    }

    /// List all skills in a world
    pub async fn list_skills(&self, world_id: &str) -> Result<Vec<SkillData>, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::Skill(SkillRequest::ListSkills {
                    world_id: world_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Get a single skill by ID
    pub async fn get_skill(&self, skill_id: &str) -> Result<SkillData, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::Skill(SkillRequest::GetSkill {
                    skill_id: skill_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Create a new skill
    pub async fn create_skill(
        &self,
        world_id: &str,
        request: &CreateSkillRequest,
    ) -> Result<SkillData, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::Skill(SkillRequest::CreateSkill {
                    world_id: world_id.to_string(),
                    data: request.into(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Update an existing skill
    pub async fn update_skill(
        &self,
        skill_id: &str,
        request: &UpdateSkillRequest,
    ) -> Result<SkillData, ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::Skill(SkillRequest::UpdateSkill {
                    skill_id: skill_id.to_string(),
                    data: request.into(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Update skill visibility
    pub async fn update_skill_visibility(
        &self,
        skill_id: &str,
        is_hidden: bool,
    ) -> Result<SkillData, ServiceError> {
        let request = UpdateSkillRequest {
            name: None,
            description: None,
            category: None,
            base_attribute: None,
            is_hidden: Some(is_hidden),
        };

        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::Skill(SkillRequest::UpdateSkill {
                    skill_id: skill_id.to_string(),
                    data: (&request).into(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Delete a skill
    pub async fn delete_skill(&self, skill_id: &str) -> Result<(), ServiceError> {
        let result = self
            .connection
            .request_with_timeout(
                RequestPayload::Skill(SkillRequest::DeleteSkill {
                    skill_id: skill_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }
}
