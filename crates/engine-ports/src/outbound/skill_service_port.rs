//! Skill service port - Interface for skill operations
//!
//! This port abstracts skill business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::entities::{Skill, SkillCategory};
use wrldbldr_domain::{SkillId, WorldId};

/// Request to create a new skill
#[derive(Debug, Clone)]
pub struct CreateSkillRequest {
    pub name: String,
    pub description: String,
    pub category: SkillCategory,
    pub base_attribute: Option<String>,
}

/// Request to update an existing skill
#[derive(Debug, Clone)]
pub struct UpdateSkillRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<SkillCategory>,
    pub base_attribute: Option<String>,
    pub is_hidden: Option<bool>,
    pub order: Option<u32>,
}

/// Port for skill service operations
///
/// This trait defines the application use cases for skill management,
/// including listing, creating, updating, and deleting skills.
#[async_trait]
pub trait SkillServicePort: Send + Sync {
    /// List all skills for a world
    ///
    /// If the world has no skills yet, returns the default skills
    /// for the world's rule system.
    async fn list_skills(&self, world_id: WorldId) -> Result<Vec<Skill>>;

    /// Get a single skill by ID
    async fn get_skill(&self, skill_id: SkillId) -> Result<Option<Skill>>;

    /// Create a custom skill for a world
    async fn create_skill(&self, world_id: WorldId, request: CreateSkillRequest) -> Result<Skill>;

    /// Update a skill
    async fn update_skill(&self, skill_id: SkillId, request: UpdateSkillRequest) -> Result<Skill>;

    /// Update skill visibility
    async fn update_visibility(&self, skill_id: SkillId, is_hidden: bool) -> Result<Skill>;

    /// Delete a custom skill
    ///
    /// Note: Only custom skills can be deleted. Default skills should be hidden instead.
    async fn delete_skill(&self, skill_id: SkillId) -> Result<()>;

    /// Initialize default skills for a world based on its rule system
    async fn initialize_defaults(&self, world_id: WorldId) -> Result<Vec<Skill>>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    pub SkillServicePort {}

    #[async_trait]
    impl SkillServicePort for SkillServicePort {
        async fn list_skills(&self, world_id: WorldId) -> Result<Vec<Skill>>;
        async fn get_skill(&self, skill_id: SkillId) -> Result<Option<Skill>>;
        async fn create_skill(&self, world_id: WorldId, request: CreateSkillRequest) -> Result<Skill>;
        async fn update_skill(&self, skill_id: SkillId, request: UpdateSkillRequest) -> Result<Skill>;
        async fn update_visibility(&self, skill_id: SkillId, is_hidden: bool) -> Result<Skill>;
        async fn delete_skill(&self, skill_id: SkillId) -> Result<()>;
        async fn initialize_defaults(&self, world_id: WorldId) -> Result<Vec<Skill>>;
    }
}
