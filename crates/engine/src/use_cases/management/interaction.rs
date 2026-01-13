//! Interaction CRUD operations.

use std::sync::Arc;

use wrldbldr_domain::{InteractionId, SceneId};

use crate::entities::Interaction;

use super::ManagementError;

pub struct InteractionCrud {
    interaction: Arc<Interaction>,
}

impl InteractionCrud {
    pub fn new(interaction: Arc<Interaction>) -> Self {
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
        if name.trim().is_empty() {
            return Err(ManagementError::InvalidInput(
                "Interaction name cannot be empty".to_string(),
            ));
        }

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
                interaction =
                    interaction.with_condition(wrldbldr_domain::InteractionCondition::Custom(
                        trigger,
                    ));
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
        let mut interaction = self
            .interaction
            .get(interaction_id)
            .await?
            .ok_or(ManagementError::NotFound)?;

        if let Some(name) = name {
            if name.trim().is_empty() {
                return Err(ManagementError::InvalidInput(
                    "Interaction name cannot be empty".to_string(),
                ));
            }
            interaction.name = name;
        }
        if let Some(description) = description {
            interaction.prompt_hints = description;
        }
        if let Some(trigger) = trigger {
            if trigger.trim().is_empty() {
                interaction.conditions.clear();
            } else {
                interaction.conditions =
                    vec![wrldbldr_domain::InteractionCondition::Custom(trigger)];
            }
        }
        if let Some(available) = available {
            interaction.is_available = available;
        }

        self.interaction.save(&interaction).await?;
        Ok(interaction)
    }

    pub async fn delete(&self, interaction_id: InteractionId) -> Result<(), ManagementError> {
        self.interaction.delete(interaction_id).await?;
        Ok(())
    }
}
