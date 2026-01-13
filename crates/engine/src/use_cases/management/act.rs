//! Act CRUD operations.

use std::sync::Arc;

use wrldbldr_domain::{ActId, WorldId};

use crate::repositories::Act;

use super::ManagementError;

pub struct ActCrud {
    act: Arc<Act>,
}

impl ActCrud {
    pub fn new(act: Arc<Act>) -> Self {
        Self { act }
    }

    pub async fn list_in_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<wrldbldr_domain::Act>, ManagementError> {
        Ok(self.act.list_in_world(world_id).await?)
    }

    pub async fn get(&self, act_id: ActId) -> Result<Option<wrldbldr_domain::Act>, ManagementError> {
        Ok(self.act.get(act_id).await?)
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        name: String,
        description: Option<String>,
        order: Option<u32>,
    ) -> Result<wrldbldr_domain::Act, ManagementError> {
        if name.trim().is_empty() {
            return Err(ManagementError::InvalidInput(
                "Act name cannot be empty".to_string(),
            ));
        }

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
            .ok_or(ManagementError::NotFound)?;

        if let Some(name) = name {
            if name.trim().is_empty() {
                return Err(ManagementError::InvalidInput(
                    "Act name cannot be empty".to_string(),
                ));
            }
            act.name = name;
        }

        if let Some(description) = description {
            act.description = description;
        }

        if let Some(order) = order {
            act.order = order;
        }

        self.act.save(&act).await?;
        Ok(act)
    }

    pub async fn delete(&self, act_id: ActId) -> Result<(), ManagementError> {
        self.act.delete(act_id).await?;
        Ok(())
    }
}
