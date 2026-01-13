//! World CRUD operations.

use std::sync::Arc;

use wrldbldr_domain::WorldId;

use crate::entities::World;
use crate::infrastructure::ports::ClockPort;

use super::ManagementError;

pub struct WorldCrud {
    world: Arc<World>,
    clock: Arc<dyn ClockPort>,
}

impl WorldCrud {
    pub fn new(world: Arc<World>, clock: Arc<dyn ClockPort>) -> Self {
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
        if name.trim().is_empty() {
            return Err(ManagementError::InvalidInput(
                "World name cannot be empty".to_string(),
            ));
        }

        let now = self.clock.now();
        let mut world =
            wrldbldr_domain::World::new(name, description.clone().unwrap_or_default(), now);

        if world.description.is_empty() {
            if let Some(setting) = setting {
                world.description = setting;
            }
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
            .ok_or(ManagementError::NotFound)?;

        let now = self.clock.now();

        if let Some(name) = name {
            world.update_name(name, now);
        }
        if let Some(description) = description {
            world.update_description(description, now);
        } else if let Some(setting) = setting {
            world.update_description(setting, now);
        }

        self.world.save(&world).await?;
        Ok(world)
    }

    pub async fn delete(&self, world_id: WorldId) -> Result<(), ManagementError> {
        self.world.delete(world_id).await?;
        Ok(())
    }
}
