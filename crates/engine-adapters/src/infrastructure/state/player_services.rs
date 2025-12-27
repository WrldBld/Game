//! Player-specific services
//!
//! This module provides a grouped structure for player-related services,
//! using trait objects where possible for flexibility and testability.

use std::sync::Arc;

use wrldbldr_engine_app::application::services::{
    PlayerCharacterService, SceneResolutionService, SheetTemplateService,
};

/// Services for player management and character operations
///
/// This struct groups services related to players: character sheets,
/// player characters, and scene resolution.
///
/// All services use `Arc<...>` for shared ownership.
pub struct PlayerServices {
    /// Character sheet template management
    pub sheet_template_service: Arc<SheetTemplateService>,
    
    /// Player character CRUD and inventory management
    pub player_character_service: Arc<dyn PlayerCharacterService>,
    
    /// Scene resolution and availability checking
    pub scene_resolution_service: Arc<dyn SceneResolutionService>,
}

impl PlayerServices {
    /// Creates a new PlayerServices instance with all player-related services
    pub fn new(
        sheet_template_service: Arc<SheetTemplateService>,
        player_character_service: Arc<dyn PlayerCharacterService>,
        scene_resolution_service: Arc<dyn SceneResolutionService>,
    ) -> Self {
        Self {
            sheet_template_service,
            player_character_service,
            scene_resolution_service,
        }
    }
}
