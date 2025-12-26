//! Player-specific services
//!
//! This module provides a grouped structure for player-related services,
//! using trait objects where possible for flexibility and testability.

use std::sync::Arc;

use wrldbldr_engine_app::application::services::{
    PlayerCharacterService, SceneResolutionService,
    SessionJoinService, SheetTemplateService,
};

/// Services for player management and character operations
///
/// This struct groups services related to players: character sheets,
/// player characters, scene resolution, and session joining.
///
/// Services with traits use `Arc<dyn Trait>`:
/// - `player_character_service`, `scene_resolution_service`
///
/// Concrete services remain as-is:
/// - `sheet_template_service` (no trait, simple CRUD wrapper)
/// - `session_join_service` (complex session logic)
pub struct PlayerServices {
    /// Character sheet template management
    pub sheet_template_service: SheetTemplateService,
    
    /// Player character CRUD and inventory management
    pub player_character_service: Arc<dyn PlayerCharacterService>,
    
    /// Scene resolution and availability checking
    pub scene_resolution_service: Arc<dyn SceneResolutionService>,
    
    /// Session creation and joining
    pub session_join_service: Arc<SessionJoinService>,
}

impl PlayerServices {
    /// Creates a new PlayerServices instance with all player-related services
    pub fn new(
        sheet_template_service: SheetTemplateService,
        player_character_service: Arc<dyn PlayerCharacterService>,
        scene_resolution_service: Arc<dyn SceneResolutionService>,
        session_join_service: Arc<SessionJoinService>,
    ) -> Self {
        Self {
            sheet_template_service,
            player_character_service,
            scene_resolution_service,
            session_join_service,
        }
    }
}
