//! Scene resolution service port - Interface for scene resolution operations
//!
//! This port abstracts scene resolution business logic from infrastructure.
//! It provides methods for determining which scene to show based on player
//! character locations and entry conditions.
//!
//! # Design Notes
//!
//! Scene resolution handles the complex logic of determining the appropriate
//! scene based on:
//! - Player character locations
//! - Split party scenarios (PCs at different locations)
//! - Scene entry conditions (completed scenes, items, flags, etc.)
//!
//! # Scene Entry Conditions
//!
//! Scenes can have entry conditions that must be met before they are shown:
//! - `CompletedScene(SceneId)` - PC must have completed another scene
//! - `HasItem(ItemId)` - PC must possess a specific item
//! - `KnowsCharacter(CharacterId)` - PC must have observed/met an NPC
//! - `FlagSet(String)` - A game flag must be set (world or PC scope)
//! - `Custom(String)` - Custom condition (evaluated by LLM)

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use wrldbldr_domain::entities::Scene;
use wrldbldr_domain::{LocationId, PlayerCharacterId, WorldId};

/// Result of scene resolution for a world.
///
/// Contains the resolved scene (if any) along with information about
/// party split status and PC locations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneResolutionResult {
    /// The resolved scene (if any).
    ///
    /// Will be `None` if no PCs are in the world or no scene matches
    /// the current conditions.
    pub scene: Option<Scene>,

    /// Whether the party is split across multiple locations.
    ///
    /// When `true`, PCs are at different locations and may need
    /// separate scene handling.
    pub is_split_party: bool,

    /// Locations where PCs are currently located.
    ///
    /// For split party scenarios, this contains all unique locations.
    /// For single-location scenarios, this will have one entry.
    pub pc_locations: Vec<LocationId>,
}

/// Port for scene resolution service operations.
///
/// This trait provides methods for resolving scenes based on PC locations
/// and entry conditions. It handles both world-level and individual PC
/// scene resolution.
///
/// # Usage
///
/// Infrastructure adapters should depend on this trait rather than importing
/// the service directly from engine-app, maintaining proper hexagonal
/// architecture boundaries.
#[async_trait]
pub trait SceneResolutionServicePort: Send + Sync {
    /// Resolve the scene for a world based on PC locations.
    ///
    /// This method determines the appropriate scene by:
    /// 1. Finding all PCs in the world
    /// 2. Grouping PCs by location
    /// 3. Determining if the party is split
    /// 4. Finding scenes at the PC locations
    ///
    /// For split parties, the scene at the first PC's location is returned,
    /// but `is_split_party` will be `true` to indicate special handling may
    /// be needed.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The ID of the world to resolve scenes for
    ///
    /// # Returns
    ///
    /// A `SceneResolutionResult` containing the scene (if found) and
    /// party location information.
    async fn resolve_scene_for_world(&self, world_id: &WorldId) -> Result<SceneResolutionResult>;

    /// Resolve the scene for a specific player character.
    ///
    /// This method finds the appropriate scene for a single PC by:
    /// 1. Getting the PC's current location
    /// 2. Finding scenes at that location
    /// 3. Evaluating entry conditions for each scene
    /// 4. Returning the first scene where all conditions are met
    ///
    /// Entry conditions are evaluated in order:
    /// - `CompletedScene` - Checks scene completion records
    /// - `HasItem` - Checks PC inventory
    /// - `KnowsCharacter` - Checks observation records
    /// - `FlagSet` - Checks PC-scoped then world-scoped flags
    /// - `Custom` - Currently treated as always met (LLM evaluation TODO)
    ///
    /// # Arguments
    ///
    /// * `pc_id` - The ID of the player character
    ///
    /// # Returns
    ///
    /// `Ok(Some(scene))` if a matching scene is found, `Ok(None)` if no
    /// scene matches the PC's location and conditions.
    ///
    /// # Errors
    ///
    /// Returns an error if the PC is not found or database queries fail.
    async fn resolve_scene_for_pc(&self, pc_id: PlayerCharacterId) -> Result<Option<Scene>>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    /// Mock implementation of SceneResolutionServicePort for testing.
    pub SceneResolutionServicePort {}

    #[async_trait]
    impl SceneResolutionServicePort for SceneResolutionServicePort {
        async fn resolve_scene_for_world(&self, world_id: &WorldId) -> Result<SceneResolutionResult>;
        async fn resolve_scene_for_pc(&self, pc_id: PlayerCharacterId) -> Result<Option<Scene>>;
    }
}
