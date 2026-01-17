//! Skill CRUD operations.

use std::sync::Arc;

use wrldbldr_domain::{SkillCategory, SkillId, WorldId};

use crate::repositories::ContentRepository;
use crate::use_cases::validation::require_non_empty;

use super::ManagementError;

pub struct SkillCrud {
    content: Arc<ContentRepository>,
}

impl SkillCrud {
    pub fn new(content: Arc<ContentRepository>) -> Self {
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

        // Rebuild skill with updated values
        let new_name = if let Some(name) = name {
            require_non_empty(&name, "Skill name")?;
            name
        } else {
            skill.name().to_string()
        };

        let new_description = description.unwrap_or_else(|| skill.description().to_string());
        let new_category = if let Some(category) = category {
            category.parse::<SkillCategory>()?
        } else {
            skill.category()
        };

        let new_base_attribute = match attribute {
            Some(attr) if attr.trim().is_empty() => None,
            Some(attr) => Some(attr),
            None => skill.base_attribute().map(|s| s.to_string()),
        };

        let new_is_hidden = is_hidden.unwrap_or_else(|| skill.is_hidden());

        skill = wrldbldr_domain::Skill::from_parts(
            skill.id(),
            skill.world_id(),
            new_name,
            new_description,
            new_category,
            new_base_attribute,
            skill.is_custom(),
            new_is_hidden,
            skill.order(),
        );

        self.content.save_skill(&skill).await?;
        Ok(skill)
    }

    pub async fn delete(&self, skill_id: SkillId) -> Result<(), ManagementError> {
        self.content.delete_skill(skill_id).await?;
        Ok(())
    }
}
