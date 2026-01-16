//! Skill CRUD operations.

use std::sync::Arc;

use wrldbldr_domain::{SkillCategory, SkillId, WorldId};

use crate::repositories::Content;
use crate::use_cases::validation::require_non_empty;

use super::ManagementError;

pub struct SkillCrud {
    content: Arc<Content>,
}

impl SkillCrud {
    pub fn new(content: Arc<Content>) -> Self {
        Self { content }
    }

    pub async fn list_in_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<wrldbldr_domain::Skill>, ManagementError> {
        Ok(self.content.list_skills_in_world(world_id).await?)
    }

    pub async fn get(
        &self,
        skill_id: SkillId,
    ) -> Result<Option<wrldbldr_domain::Skill>, ManagementError> {
        Ok(self.content.get_skill(skill_id).await?)
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        name: String,
        description: Option<String>,
        category: Option<String>,
        attribute: Option<String>,
    ) -> Result<wrldbldr_domain::Skill, ManagementError> {
        require_non_empty(&name, "Skill name")?;

        let category_value = match category {
            Some(category) => category.parse::<SkillCategory>()?,
            None => SkillCategory::Other,
        };

        let mut skill = wrldbldr_domain::Skill::custom(world_id, name, category_value);
        if let Some(description) = description {
            skill = skill.with_description(description);
        }
        if let Some(attribute) = attribute {
            if !attribute.trim().is_empty() {
                skill = skill.with_base_attribute(attribute);
            }
        }

        self.content.save_skill(&skill).await?;
        Ok(skill)
    }

    pub async fn update(
        &self,
        skill_id: SkillId,
        name: Option<String>,
        description: Option<String>,
        category: Option<String>,
        attribute: Option<String>,
        is_hidden: Option<bool>,
    ) -> Result<wrldbldr_domain::Skill, ManagementError> {
        let mut skill = self
            .content
            .get_skill(skill_id)
            .await?
            .ok_or(ManagementError::NotFound)?;

        if let Some(name) = name {
            require_non_empty(&name, "Skill name")?;
            skill.name = name;
        }
        if let Some(description) = description {
            skill.description = description;
        }
        if let Some(category) = category {
            skill.category = category.parse::<SkillCategory>()?;
        }
        if let Some(attribute) = attribute {
            if attribute.trim().is_empty() {
                skill.base_attribute = None;
            } else {
                skill.base_attribute = Some(attribute);
            }
        }
        if let Some(is_hidden) = is_hidden {
            skill.is_hidden = is_hidden;
        }

        self.content.save_skill(&skill).await?;
        Ok(skill)
    }

    pub async fn delete(&self, skill_id: SkillId) -> Result<(), ManagementError> {
        self.content.delete_skill(skill_id).await?;
        Ok(())
    }
}
