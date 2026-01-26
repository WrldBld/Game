// Act management - methods for future story arc features
#![allow(dead_code)]

//! Act management operations.

use std::sync::Arc;

use wrldbldr_domain::{ActId, WorldId};

use crate::infrastructure::ports::ActRepo;
use crate::use_cases::validation::require_non_empty;

use super::ManagementError;

pub struct ActManagement {
    act: Arc<dyn ActRepo>,
}

impl ActManagement {
    pub fn new(act: Arc<dyn ActRepo>) -> Self {
        Self { act }
    }

    pub async fn list_in_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<wrldbldr_domain::Act>, ManagementError> {
        Ok(self.act.list_in_world(world_id).await?)
    }

    pub async fn get(
        &self,
        act_id: ActId,
    ) -> Result<Option<wrldbldr_domain::Act>, ManagementError> {
        Ok(self.act.get(act_id).await?)
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        name: String,
        description: Option<String>,
        order: Option<u32>,
    ) -> Result<wrldbldr_domain::Act, ManagementError> {
        require_non_empty(&name, "Act name")?;

        let mut act = wrldbldr_domain::Act::new(
            world_id,
            name,
            wrldbldr_domain::MonomythStage::OrdinaryWorld,
            order.unwrap_or(0),
        );

        if let Some(description) = description {
            act = act.with_description(description);
        }

        self.act.save(&act).await?;
        Ok(act)
    }

    pub async fn update(
        &self,
        act_id: ActId,
        name: Option<String>,
        description: Option<String>,
        order: Option<u32>,
    ) -> Result<wrldbldr_domain::Act, ManagementError> {
        let mut act = self
            .act
            .get(act_id)
            .await?
            .ok_or(ManagementError::NotFound {
                entity_type: "Act",
                id: act_id.to_string(),
            })?;

        // Rebuild act with updated values using from_storage
        let new_name = if let Some(name) = name {
            require_non_empty(&name, "Act name")?;
            name
        } else {
            act.name().to_string()
        };

        let new_description = description.unwrap_or_else(|| act.description().to_string());
        let new_order = order.unwrap_or_else(|| act.order());

        act = wrldbldr_domain::Act::from_storage(
            act.id(),
            act.world_id(),
            new_name,
            act.stage(),
            new_description,
            new_order,
        );

        self.act.save(&act).await?;
        Ok(act)
    }

    pub async fn delete(&self, act_id: ActId) -> Result<(), ManagementError> {
        self.act.delete(act_id).await?;
        Ok(())
    }
}
