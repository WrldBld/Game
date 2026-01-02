//! Player Services Container - Port-based abstraction for player-facing services
//!
//! This module provides `PlayerServices`, a grouped structure for player character
//! and session-related services using **port traits** from `wrldbldr-engine-ports`.
//!
//! # Architecture
//!
//! This struct groups all services that directly support player interactions,
//! including character management, sheet templates, and scene resolution.
//! All fields use port traits for clean hexagonal architecture boundaries.
//!
//! # Services Included
//!
//! - **Sheet Template Service**: Character sheet template management
//! - **Player Character Service**: Player character lifecycle operations
//! - **Scene Resolution Service**: Scene context and state resolution
//!
//! # Usage
//!
//! ```ignore
//! use wrldbldr_engine_composition::PlayerServices;
//!
//! let player_services = PlayerServices::new(
//!     sheet_template_service,
//!     player_character_service,
//!     scene_resolution_service,
//! );
//!
//! // Access via port traits
//! let character = player_services.player_character_service.get_by_id(pc_id).await?;
//! ```

use std::sync::Arc;

// Internal service traits (NOT ports - internal app-layer contracts)
use wrldbldr_engine_app::application::services::internal::{
    SceneResolutionServicePort, SheetTemplateServicePort,
};
// True outbound ports (adapter-implemented infrastructure)
use wrldbldr_engine_ports::outbound::PlayerCharacterServicePort;

/// Container for player-facing services.
///
/// This struct groups all services that support player interactions with the game,
/// including character creation, management, and scene context resolution.
///
/// All fields are `Arc<dyn ...Port>` for:
/// - Shared ownership across handlers and workers
/// - Dynamic dispatch enabling mock injection for tests
/// - No generic type parameters for simpler composition
///
/// # Service Categories
///
/// ## Character Management
/// - `sheet_template_service`: Templates for character sheets (stats, skills, inventory layouts)
/// - `player_character_service`: CRUD operations for player-controlled characters
///
/// ## Scene Context
/// - `scene_resolution_service`: Resolves current scene state including NPCs, items, and exits
#[derive(Clone)]
pub struct PlayerServices {
    /// Service for character sheet template management.
    ///
    /// Provides access to sheet templates that define the structure of
    /// character sheets including stat blocks, skill lists, and inventory
    /// layouts. Templates can be customized per world or character type.
    pub sheet_template_service: Arc<dyn SheetTemplateServicePort>,

    /// Service for player character lifecycle operations.
    ///
    /// Handles creation, retrieval, update, and deletion of player-controlled
    /// characters. Manages character state including stats, inventory,
    /// and current location.
    pub player_character_service: Arc<dyn PlayerCharacterServicePort>,

    /// Service for scene context resolution.
    ///
    /// Resolves the current scene state for a player character, including
    /// present NPCs, available items, exits, and active challenges. Used
    /// to build the context for player actions and LLM prompts.
    pub scene_resolution_service: Arc<dyn SceneResolutionServicePort>,
}

impl PlayerServices {
    /// Creates a new `PlayerServices` instance with all player-facing services.
    ///
    /// # Arguments
    ///
    /// All arguments are `Arc<dyn ...Port>` to allow any implementation:
    ///
    /// * `sheet_template_service` - For character sheet template access
    /// * `player_character_service` - For player character operations
    /// * `scene_resolution_service` - For scene context resolution
    ///
    /// # Example
    ///
    /// ```ignore
    /// let player_services = PlayerServices::new(
    ///     Arc::new(sheet_template_impl) as Arc<dyn SheetTemplateServicePort>,
    ///     Arc::new(player_char_impl) as Arc<dyn PlayerCharacterServicePort>,
    ///     Arc::new(scene_resolution_impl) as Arc<dyn SceneResolutionServicePort>,
    /// );
    /// ```
    pub fn new(
        sheet_template_service: Arc<dyn SheetTemplateServicePort>,
        player_character_service: Arc<dyn PlayerCharacterServicePort>,
        scene_resolution_service: Arc<dyn SceneResolutionServicePort>,
    ) -> Self {
        Self {
            sheet_template_service,
            player_character_service,
            scene_resolution_service,
        }
    }
}

impl std::fmt::Debug for PlayerServices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerServices")
            .field(
                "sheet_template_service",
                &"Arc<dyn SheetTemplateServicePort>",
            )
            .field(
                "player_character_service",
                &"Arc<dyn PlayerCharacterServicePort>",
            )
            .field(
                "scene_resolution_service",
                &"Arc<dyn SceneResolutionServicePort>",
            )
            .finish()
    }
}
