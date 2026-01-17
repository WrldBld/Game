//! Interaction CRUD operations.

use std::sync::Arc;

use wrldbldr_domain::{InteractionId, SceneId};

use crate::repositories::InteractionRepository;
use crate::use_cases::validation::require_non_empty;

use super::ManagementError;

pub struct InteractionCrud {
    interaction: Arc<InteractionRepository>,
}

impl InteractionCrud {
    pub fn new(interaction: Arc<InteractionRepository>) -> Self {
        Self { interaction }
    }

    pub async fn list_for_scene(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<wrldbldr_domain::InteractionTemplate>, ManagementError> {
        Ok(self.interaction.list_for_scene(scene_id).await?)
    }

    pub async fn get(
        &self,
        interaction_id: InteractionId,
    ) -> Result<Option<wrldbldr_domain::InteractionTemplate>, ManagementError> {
        Ok(self.interaction.get(interaction_id).await?)
    }

    pub async fn create(
        &self,
        scene_id: SceneId,
        name: String,
        description: Option<String>,
        trigger: Option<String>,
        available: Option<bool>,
    ) -> Result<wrldbldr_domain::InteractionTemplate, ManagementError> {
        require_non_empty(&name, "Interaction name")?;

        let mut interaction = wrldbldr_domain::InteractionTemplate::new(
            scene_id,
            name,
            wrldbldr_domain::InteractionType::Custom("Custom".to_string()),
            wrldbldr_domain::InteractionTarget::None,
        );

        if let Some(description) = description {
            interaction = interaction.with_prompt_hints(description);
        }
        if let Some(trigger) = trigger {
            if !trigger.trim().is_empty() {
                interaction = interaction
                    .with_condition(wrldbldr_domain::InteractionCondition::Custom(trigger));
            }
        }
        if available == Some(false) {
            interaction = interaction.disabled();
        }

        self.interaction.save(&interaction).await?;
        Ok(interaction)
    }

    pub async fn update(
        &self,
        interaction_id: InteractionId,
        name: Option<String>,
        description: Option<String>,
        trigger: Option<String>,
        available: Option<bool>,
    ) -> Result<wrldbldr_domain::InteractionTemplate, ManagementError> {
        let existing = self
            .interaction
            .get(interaction_id)
            .await?
            .ok_or(ManagementError::NotFound)?;

        // Rebuild the interaction with updated values
        let updated_name = match name {
            Some(ref n) => {
                require_non_empty(n, "Interaction name")?;
                n.clone()
            }
            None => existing.name().to_string(),
        };

        let updated_hints = description.unwrap_or_else(|| existing.prompt_hints().to_string());

        let updated_conditions = match trigger {
            Some(t) if t.trim().is_empty() => Vec::new(),
            Some(t) => vec![wrldbldr_domain::InteractionCondition::Custom(t)],
            None => existing.conditions().to_vec(),
        };

        let updated_available = available.unwrap_or_else(|| existing.is_available());

        let interaction = wrldbldr_domain::InteractionTemplate::from_stored(
            existing.id(),
            existing.scene_id(),
            updated_name,
            existing.interaction_type().clone(),
            existing.target().clone(),
            updated_hints,
            existing.allowed_tools().to_vec(),
            updated_conditions,
            updated_available,
            existing.order(),
        );

        self.interaction.save(&interaction).await?;
        Ok(interaction)
    }

    pub async fn delete(&self, interaction_id: InteractionId) -> Result<(), ManagementError> {
        self.interaction.delete(interaction_id).await?;
        Ok(())
    }
}
