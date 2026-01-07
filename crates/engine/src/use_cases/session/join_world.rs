use std::sync::Arc;

use serde_json::Value;

use crate::entities::{Character, Location, PlayerCharacter, Scene, World};
use crate::infrastructure::ports::RepoError;
use wrldbldr_domain::{PlayerCharacterId, WorldId};

/// Use case for joining a world and building the session snapshot.
pub struct JoinWorld {
    world: Arc<World>,
    location: Arc<Location>,
    character: Arc<Character>,
    scene: Arc<Scene>,
    player_character: Arc<PlayerCharacter>,
}

impl JoinWorld {
    pub fn new(
        world: Arc<World>,
        location: Arc<Location>,
        character: Arc<Character>,
        scene: Arc<Scene>,
        player_character: Arc<PlayerCharacter>,
    ) -> Self {
        Self {
            world,
            location,
            character,
            scene,
            player_character,
        }
    }

    pub async fn execute(
        &self,
        world_id: WorldId,
        pc_id: Option<PlayerCharacterId>,
        include_pc: bool,
    ) -> Result<JoinWorldResult, JoinWorldError> {
        let world = self
            .world
            .get(world_id)
            .await?
            .ok_or(JoinWorldError::WorldNotFound)?;

        let locations = self.location.list_in_world(world_id).await.unwrap_or_default();
        let characters = self.character.list_in_world(world_id).await.unwrap_or_default();
        let current_scene = self.scene.get_current(world_id).await.unwrap_or(None);

        let current_scene_json = current_scene.as_ref().map(|scene| {
            let featured = scene
                .featured_characters
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>();

            serde_json::json!({
                "id": scene.id.to_string(),
                "name": scene.name,
                "location_id": scene.location_id.to_string(),
                "time_context": format!("{:?}", scene.time_context),
                "backdrop_override": scene.backdrop_override,
                "featured_characters": featured,
                "directorial_notes": scene.directorial_notes,
            })
        });

        let scenes_json = current_scene_json
            .as_ref()
            .map(|s| vec![s.clone()])
            .unwrap_or_else(Vec::new);

        let snapshot = serde_json::json!({
            "world": {
                "id": world.id.to_string(),
                "name": world.name,
                "description": world.description,
                "rule_system": world.rule_system,
                "created_at": world.created_at.to_rfc3339(),
                "updated_at": world.updated_at.to_rfc3339(),
            },
            "locations": locations.into_iter().map(|loc| {
                serde_json::json!({
                    "id": loc.id.to_string(),
                    "name": loc.name,
                    "description": loc.description,
                    "location_type": format!("{:?}", loc.location_type),
                    "backdrop_asset": loc.backdrop_asset,
                    "parent_id": null,
                })
            }).collect::<Vec<_>>(),
            "characters": characters.into_iter().map(|c| {
                serde_json::json!({
                    "id": c.id.to_string(),
                    "name": c.name,
                    "description": c.description,
                    "archetype": format!("{:?}", c.current_archetype),
                    "sprite_asset": c.sprite_asset,
                    "portrait_asset": c.portrait_asset,
                    "is_alive": c.is_alive,
                    "is_active": c.is_active,
                })
            }).collect::<Vec<_>>(),
            "scenes": scenes_json,
            "current_scene": current_scene_json,
        });

        let your_pc = if include_pc { self.load_pc(pc_id).await } else { None };

        Ok(JoinWorldResult {
            world_id,
            snapshot,
            your_pc,
        })
    }

    async fn load_pc(&self, pc_id: Option<PlayerCharacterId>) -> Option<Value> {
        let pc_id = pc_id?;
        match self.player_character.get(pc_id).await {
            Ok(Some(pc)) => Some(serde_json::json!({
                "id": pc.id.to_string(),
                "name": pc.name,
                "description": pc.description,
                "portrait_asset": pc.portrait_asset,
                "sprite_asset": pc.sprite_asset,
                "current_location_id": pc.current_location_id.to_string(),
                "current_region_id": pc.current_region_id.map(|id| id.to_string()),
            })),
            Ok(None) => None,
            Err(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct JoinWorldResult {
    pub world_id: WorldId,
    pub snapshot: Value,
    pub your_pc: Option<Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum JoinWorldError {
    #[error("World not found")]
    WorldNotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
