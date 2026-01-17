//! Scene CRUD operations.

use std::sync::Arc;

use wrldbldr_domain::{ActId, LocationId, SceneId, SceneName};

use crate::infrastructure::ports::SceneRepo;

use super::ManagementError;

pub struct SceneCrud {
    scene: Arc<dyn SceneRepo>,
}

impl SceneCrud {
    pub fn new(scene: Arc<dyn SceneRepo>) -> Self {
        Self { scene }
    }

    pub async fn list_for_act(
        &self,
        act_id: ActId,
    ) -> Result<Vec<wrldbldr_domain::Scene>, ManagementError> {
        Ok(self.scene.list_for_act(act_id).await?)
    }

    pub async fn get(
        &self,
        scene_id: SceneId,
    ) -> Result<Option<wrldbldr_domain::Scene>, ManagementError> {
        Ok(self.scene.get(scene_id).await?)
    }

    pub async fn create(
        &self,
        act_id: ActId,
        name: String,
        description: Option<String>,
        location_id: Option<LocationId>,
    ) -> Result<wrldbldr_domain::Scene, ManagementError> {
        let name =
            SceneName::new(name).map_err(|e| ManagementError::InvalidInput(e.to_string()))?;

        let location_id = location_id.ok_or_else(|| {
            ManagementError::InvalidInput("Scene location_id is required".to_string())
        })?;

        let mut scene = wrldbldr_domain::Scene::new(act_id, name, location_id);
        if let Some(description) = description {
            scene = scene.with_directorial_notes(description);
        }

        self.scene.save(&scene).await?;
        Ok(scene)
    }

    pub async fn update(
        &self,
        scene_id: SceneId,
        name: Option<String>,
        description: Option<String>,
        location_id: Option<LocationId>,
    ) -> Result<wrldbldr_domain::Scene, ManagementError> {
        let mut scene = self
            .scene
            .get(scene_id)
            .await?
            .ok_or(ManagementError::NotFound {
                entity_type: "Scene",
                id: scene_id.to_string(),
            })?;

        if let Some(name) = name {
            let name =
                SceneName::new(name).map_err(|e| ManagementError::InvalidInput(e.to_string()))?;
            scene.set_name(name);
        }
        if let Some(description) = description {
            scene.set_directorial_notes(description);
        }
        if let Some(location_id) = location_id {
            scene.set_location(location_id);
        }

        self.scene.save(&scene).await?;
        Ok(scene)
    }

    pub async fn delete(&self, scene_id: SceneId) -> Result<(), ManagementError> {
        self.scene.delete(scene_id).await?;
        Ok(())
    }
}
