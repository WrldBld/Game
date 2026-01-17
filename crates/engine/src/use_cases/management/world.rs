//! World CRUD operations.

use std::sync::Arc;

use wrldbldr_domain::value_objects::{Description, WorldName};
use wrldbldr_domain::WorldId;

use crate::infrastructure::ports::{ClockPort, WorldRepo};

use super::ManagementError;

pub struct WorldCrud {
    world: Arc<dyn WorldRepo>,
    clock: Arc<dyn ClockPort>,
}

impl WorldCrud {
    pub fn new(world: Arc<dyn WorldRepo>, clock: Arc<dyn ClockPort>) -> Self {
        Self { world, clock }
    }

    pub async fn list(&self) -> Result<Vec<wrldbldr_domain::World>, ManagementError> {
        Ok(self.world.list_all().await?)
    }

    pub async fn get(
        &self,
        world_id: WorldId,
    ) -> Result<Option<wrldbldr_domain::World>, ManagementError> {
        Ok(self.world.get(world_id).await?)
    }

    pub async fn create(
        &self,
        name: String,
        description: Option<String>,
        setting: Option<String>,
    ) -> Result<wrldbldr_domain::World, ManagementError> {
        let world_name = WorldName::new(name)
            .map_err(|e| ManagementError::InvalidInput(format!("Invalid world name: {}", e)))?;

        let now = self.clock.now();
        let mut world = wrldbldr_domain::World::new(world_name, now);

        // Set description from either description or setting parameter
        let desc_str = description.or(setting).unwrap_or_default();
        if !desc_str.is_empty() {
            let desc = Description::new(&desc_str).map_err(|e| {
                ManagementError::InvalidInput(format!("Invalid description: {}", e))
            })?;
            world.set_description(desc, now);
        }

        self.world.save(&world).await?;
        Ok(world)
    }

    pub async fn update(
        &self,
        world_id: WorldId,
        name: Option<String>,
        description: Option<String>,
        setting: Option<String>,
    ) -> Result<wrldbldr_domain::World, ManagementError> {
        let mut world = self
            .world
            .get(world_id)
            .await?
            .ok_or(ManagementError::NotFound {
                entity_type: "World",
                id: world_id.to_string(),
            })?;

        let now = self.clock.now();

        if let Some(name) = name {
            let world_name = WorldName::new(name)
                .map_err(|e| ManagementError::InvalidInput(format!("Invalid world name: {}", e)))?;
            world.set_name(world_name, now);
        }
        if let Some(description) = description {
            let desc = Description::new(&description).map_err(|e| {
                ManagementError::InvalidInput(format!("Invalid description: {}", e))
            })?;
            world.set_description(desc, now);
        } else if let Some(setting) = setting {
            let desc = Description::new(&setting).map_err(|e| {
                ManagementError::InvalidInput(format!("Invalid description: {}", e))
            })?;
            world.set_description(desc, now);
        }

        self.world.save(&world).await?;
        Ok(world)
    }

    pub async fn delete(&self, world_id: WorldId) -> Result<(), ManagementError> {
        self.world.delete(world_id).await?;
        Ok(())
    }
}
