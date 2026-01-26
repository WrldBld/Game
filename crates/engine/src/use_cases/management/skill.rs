//! Skill management operations.

use std::sync::Arc;

use wrldbldr_domain::{SkillCategory, SkillId, WorldId};

use crate::infrastructure::ports::ContentRepo;
use crate::use_cases::validation::require_non_empty;

use super::ManagementError;

pub struct SkillManagement {
    content: Arc<dyn ContentRepo>,
}

impl SkillManagement {
    pub fn new(content: Arc<dyn ContentRepo>) -> Self {
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

        let skill = wrldbldr_domain::Skill {
            id: SkillId::new(),
            world_id,
            name,
            description: description.unwrap_or_default(),
            category: category_value,
            base_attribute: attribute
                .filter(|a| !a.trim().is_empty())
                .and_then(|a| a.parse().ok()),
            is_custom: true,
            is_hidden: false,
            order: 0,
        };

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
        let skill = self
            .content
            .get_skill(skill_id)
            .await?
            .ok_or(ManagementError::NotFound {
                entity_type: "Skill",
                id: skill_id.to_string(),
            })?;

        // Rebuild skill with updated values
        let new_name = if let Some(name) = name {
            require_non_empty(&name, "Skill name")?;
            name
        } else {
            skill.name.clone()
        };

        let new_description = description.unwrap_or_else(|| skill.description.clone());
        let new_category = if let Some(category) = category {
            category.parse::<SkillCategory>()?
        } else {
            skill.category
        };

        let new_base_attribute = match attribute {
            Some(attr) if attr.trim().is_empty() => None,
            Some(attr) => attr.parse().ok(),
            None => skill.base_attribute,
        };

        let new_is_hidden = is_hidden.unwrap_or(skill.is_hidden);

        let updated_skill = wrldbldr_domain::Skill {
            id: skill.id,
            world_id: skill.world_id,
            name: new_name,
            description: new_description,
            category: new_category,
            base_attribute: new_base_attribute,
            is_custom: skill.is_custom,
            is_hidden: new_is_hidden,
            order: skill.order,
        };

        self.content.save_skill(&updated_skill).await?;
        Ok(updated_skill)
    }

    pub async fn delete(&self, skill_id: SkillId) -> Result<(), ManagementError> {
        self.content.delete_skill(skill_id).await?;
        Ok(())
    }
}
