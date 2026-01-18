// Join world - types for future join flow
#![allow(dead_code)]

use std::sync::Arc;

use crate::infrastructure::ports::{
    CharacterRepo, LocationRepo, PlayerCharacterRepo, RepoError, SceneRepo, WorldRepo,
};
use wrldbldr_domain::{LocationId, PlayerCharacterId, WorldId, WorldRole};

use super::types::{
    CharacterSummary, LocationSummary, PlayerCharacterSummary, SceneSummary, WorldSnapshot,
    WorldSummary,
};

/// Use case for joining a world and building the session snapshot.
pub struct JoinWorld {
    world: Arc<dyn WorldRepo>,
    location: Arc<dyn LocationRepo>,
    character: Arc<dyn CharacterRepo>,
    scene: Arc<dyn SceneRepo>,
    player_character: Arc<dyn PlayerCharacterRepo>,
}

impl JoinWorld {
    pub fn new(
        world: Arc<dyn WorldRepo>,
        location: Arc<dyn LocationRepo>,
        character: Arc<dyn CharacterRepo>,
        scene: Arc<dyn SceneRepo>,
        player_character: Arc<dyn PlayerCharacterRepo>,
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

        let locations = self.location.list_locations_in_world(world_id).await?;
        let characters = self.character.list_in_world(world_id).await?;
        let current_scene = self.scene.get_current(world_id).await?;

        // Build scene summary
        let current_scene_summary = if let Some(scene) = current_scene.as_ref() {
            // Get location via graph edge
            let location_id = self
                .scene
                .get_location(scene.id())
                .await
                .ok()
                .flatten()
                .unwrap_or_else(LocationId::new);

            // Get featured characters via graph edge
            let featured_characters: Vec<String> = self
                .scene
                .get_featured_characters(scene.id())
                .await
                .ok()
                .unwrap_or_default()
                .iter()
                .map(|sc| sc.character_id.to_string())
                .collect();

            Some(SceneSummary {
                id: scene.id(),
                name: scene.name().to_string(),
                location_id,
                time_context: format!("{:?}", scene.time_context()),
                backdrop_override: scene.backdrop_override().map(|s| s.to_string()),
                featured_characters,
                directorial_notes: {
                    let notes = scene.directorial_notes();
                    if notes.is_empty() {
                        None
                    } else {
                        Some(notes.to_string())
                    }
                },
            })
        } else {
            None
        };

        let scenes = current_scene_summary
            .clone()
            .map(|s| vec![s])
            .unwrap_or_default();

        // Build typed snapshot
        let snapshot = WorldSnapshot {
            world: WorldSummary {
                id: world.id(),
                name: world.name().as_str().to_string(),
                description: world.description().as_str().to_string(),
                rule_system: world.rule_system().name.clone(),
                created_at: world.created_at(),
                updated_at: world.updated_at(),
            },
            locations: locations
                .into_iter()
                .map(|loc| LocationSummary {
                    id: loc.id(),
                    name: loc.name().as_str().to_string(),
                    description: loc.description().as_str().to_string(),
                    location_type: format!("{:?}", loc.location_type()),
                    backdrop_asset: loc.backdrop_asset().map(|s| s.to_string()),
                    parent_id: None,
                })
                .collect(),
            characters: characters
                .into_iter()
                .map(|c| CharacterSummary {
                    id: c.id(),
                    name: c.name().to_string(),
                    description: c.description().as_str().to_string(),
                    archetype: format!("{:?}", c.current_archetype()),
                    sprite_asset: c.sprite_asset().map(|s| s.to_string()),
                    portrait_asset: c.portrait_asset().map(|s| s.to_string()),
                    is_alive: c.is_alive(),
                    is_active: c.is_active(),
                })
                .collect(),
            scenes,
            current_scene: current_scene_summary,
        };

        let your_pc = if include_pc {
            self.load_pc(pc_id).await
        } else {
            None
        };

        Ok(JoinWorldResult {
            world_id,
            snapshot,
            your_pc,
        })
    }

    pub async fn execute_with_role(
        &self,
        world_id: WorldId,
        role: WorldRole,
        pc_id: Option<PlayerCharacterId>,
    ) -> Result<JoinWorldResult, JoinWorldError> {
        let include_pc = matches!(role, WorldRole::Player);
        self.execute(world_id, pc_id, include_pc).await
    }

    async fn load_pc(&self, pc_id: Option<PlayerCharacterId>) -> Option<PlayerCharacterSummary> {
        let pc_id = pc_id?;
        match self.player_character.get(pc_id).await {
            Ok(Some(pc)) => Some(PlayerCharacterSummary {
                id: pc.id(),
                name: pc.name().to_string(),
                description: pc.description().map(|s| s.to_string()),
                portrait_asset: pc.portrait_asset().map(|s| s.to_string()),
                sprite_asset: pc.sprite_asset().map(|s| s.to_string()),
                current_location_id: pc.current_location_id(),
                current_region_id: pc.current_region_id(),
            }),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(pc_id = ?pc_id, error = %e, "Failed to load PC for session");
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct JoinWorldResult {
    pub world_id: WorldId,
    pub snapshot: WorldSnapshot,
    pub your_pc: Option<PlayerCharacterSummary>,
}

#[derive(Debug, thiserror::Error)]
pub enum JoinWorldError {
    #[error("World not found")]
    WorldNotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
