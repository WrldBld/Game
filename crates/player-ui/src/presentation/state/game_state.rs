//! Game state management using Dioxus signals
//!
//! Central game state for the Player application.

use dioxus::prelude::*;
use std::sync::Arc;

use wrldbldr_player_app::application::dto::SessionWorldSnapshot;
use wrldbldr_protocol::{
    CharacterData as SceneCharacterState,
    GameTime,
    InteractionData,
    NavigationData,
    NpcPresenceData,
    RegionData as SceneRegionInfo,
    SceneData as SceneSnapshot,
};


/// Approach event data (NPC approaching player)
#[derive(Clone, Debug, PartialEq)]
pub struct ApproachEventData {
    /// The NPC's ID
    pub npc_id: String,
    /// The NPC's name
    pub npc_name: String,
    /// The NPC's sprite asset URL (if any)
    pub npc_sprite: Option<String>,
    /// Narrative description of the approach
    pub description: String,
}

/// Location event data (location-wide event)
#[derive(Clone, Debug, PartialEq)]
pub struct LocationEventData {
    /// The region where the event occurred
    pub region_id: String,
    /// Narrative description of the event
    pub description: String,
}

/// Staging pending data (player waiting for DM to approve staging)
#[derive(Clone, Debug, PartialEq)]
pub struct StagingPendingData {
    /// The region where staging is pending
    pub region_id: String,
    /// Region name for display
    pub region_name: String,
}

/// Staged NPC data for DM approval UI
#[derive(Clone, Debug, PartialEq)]
pub struct StagedNpcData {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub reasoning: String,
}

/// Previous staging info for reference
#[derive(Clone, Debug, PartialEq)]
pub struct PreviousStagingData {
    pub staging_id: String,
    pub approved_at: String,
    pub npcs: Vec<StagedNpcData>,
}

/// PC waiting for staging approval
#[derive(Clone, Debug, PartialEq)]
pub struct WaitingPcData {
    pub pc_id: String,
    pub pc_name: String,
    pub player_id: String,
}

/// Staging approval data for DM popup
#[derive(Clone, Debug, PartialEq)]
pub struct StagingApprovalData {
    pub request_id: String,
    pub region_id: String,
    pub region_name: String,
    pub location_id: String,
    pub location_name: String,
    pub game_time: GameTime,
    pub previous_staging: Option<PreviousStagingData>,
    pub rule_based_npcs: Vec<StagedNpcData>,
    pub llm_based_npcs: Vec<StagedNpcData>,
    pub default_ttl_hours: i32,
    pub waiting_pcs: Vec<WaitingPcData>,
}

/// Central game state stored as Dioxus signals
#[derive(Clone)]
pub struct GameState {
    /// Loaded world data (from session snapshot)
    pub world: Signal<Option<Arc<SessionWorldSnapshot>>>,
    /// Current scene data (from server SceneUpdate)
    pub current_scene: Signal<Option<SceneSnapshot>>,
    /// Characters in the current scene
    pub scene_characters: Signal<Vec<SceneCharacterState>>,
    /// Available interactions in the scene
    pub interactions: Signal<Vec<InteractionData>>,
    /// Current region data (from SceneChanged)
    pub current_region: Signal<Option<SceneRegionInfo>>,
    /// Navigation options from current region
    pub navigation: Signal<Option<NavigationData>>,
    /// NPCs present in the current region
    pub npcs_present: Signal<Vec<NpcPresenceData>>,
    /// Currently selected PC ID
    pub selected_pc_id: Signal<Option<String>>,
    /// Current game time
    pub game_time: Signal<Option<GameTime>>,
    /// Active approach event (NPC approaching player)
    pub approach_event: Signal<Option<ApproachEventData>>,
    /// Active location event (location-wide event)
    pub location_event: Signal<Option<LocationEventData>>,
    /// Staging pending for player (waiting for DM approval)
    pub staging_pending: Signal<Option<StagingPendingData>>,
    /// Pending staging approval for DM
    pub pending_staging_approval: Signal<Option<StagingApprovalData>>,
}

impl GameState {
    /// Create a new GameState with empty signals
    pub fn new() -> Self {
        Self {
            world: Signal::new(None),
            current_scene: Signal::new(None),
            scene_characters: Signal::new(Vec::new()),
            interactions: Signal::new(Vec::new()),
            current_region: Signal::new(None),
            navigation: Signal::new(None),
            npcs_present: Signal::new(Vec::new()),
            selected_pc_id: Signal::new(None),
            game_time: Signal::new(None),
            approach_event: Signal::new(None),
            location_event: Signal::new(None),
            staging_pending: Signal::new(None),
            pending_staging_approval: Signal::new(None),
        }
    }

    /// Load a session world snapshot
    pub fn load_world(&mut self, snapshot: SessionWorldSnapshot) {
        self.world.set(Some(Arc::new(snapshot)));
    }

    /// Update from ServerMessage::SceneUpdate
    pub fn apply_scene_update(
        &mut self,
        scene: SceneSnapshot,
        characters: Vec<SceneCharacterState>,
        interactions: Vec<InteractionData>,
    ) {
        self.current_scene.set(Some(scene));
        self.scene_characters.set(characters);
        self.interactions.set(interactions);
    }

    /// Update from ServerMessage::SceneChanged (navigation)
    pub fn apply_scene_changed(
        &mut self,
        pc_id: String,
        region: SceneRegionInfo,
        npcs_present: Vec<NpcPresenceData>,
        navigation: NavigationData,
    ) {
        self.selected_pc_id.set(Some(pc_id));
        self.current_region.set(Some(region));
        self.npcs_present.set(npcs_present);
        self.navigation.set(Some(navigation));
    }

    /// Update from ServerMessage::GameTimeUpdated
    pub fn apply_game_time_update(&mut self, game_time: GameTime) {
        self.game_time.set(Some(game_time));
    }

    /// Set an approach event (NPC approaching player)
    pub fn set_approach_event(
        &mut self,
        npc_id: String,
        npc_name: String,
        npc_sprite: Option<String>,
        description: String,
    ) {
        self.approach_event.set(Some(ApproachEventData {
            npc_id,
            npc_name,
            npc_sprite,
            description,
        }));
    }

    /// Clear the approach event (player dismissed it)
    pub fn clear_approach_event(&mut self) {
        self.approach_event.set(None);
    }

    /// Set a location event
    pub fn set_location_event(&mut self, region_id: String, description: String) {
        self.location_event.set(Some(LocationEventData {
            region_id,
            description,
        }));
    }

    /// Clear the location event (player dismissed it or timeout)
    pub fn clear_location_event(&mut self) {
        self.location_event.set(None);
    }

    /// Set staging as pending (player waiting for DM approval)
    pub fn set_staging_pending(&mut self, region_id: String, region_name: String) {
        self.staging_pending.set(Some(StagingPendingData {
            region_id,
            region_name,
        }));
    }

    /// Clear staging pending (staging was approved or cancelled)
    pub fn clear_staging_pending(&mut self) {
        self.staging_pending.set(None);
    }

    /// Set pending staging approval data (for DM)
    pub fn set_pending_staging_approval(&mut self, data: StagingApprovalData) {
        self.pending_staging_approval.set(Some(data));
    }

    /// Clear pending staging approval (DM approved or dismissed)
    pub fn clear_pending_staging_approval(&mut self) {
        self.pending_staging_approval.set(None);
    }

    /// Update LLM suggestions in pending staging approval (after regeneration)
    pub fn update_staging_llm_suggestions(&mut self, llm_based_npcs: Vec<StagedNpcData>) {
        let mut current = self.pending_staging_approval.write();
        if let Some(ref mut data) = *current {
            data.llm_based_npcs = llm_based_npcs;
        }
    }

    /// Get the backdrop URL for the current scene
    pub fn backdrop_url(&self) -> Option<String> {
        // First check scene override, then location backdrop
        let scene_binding = self.current_scene.read();
        if let Some(scene) = scene_binding.as_ref() {
            if let Some(ref backdrop) = scene.backdrop_asset {
                return Some(backdrop.clone());
            }
        }

        // Fall back to location backdrop from world data
        let world_binding = self.world.read();
        if let (Some(scene), Some(world)) = (scene_binding.as_ref(), world_binding.as_ref()) {
            if let Some(location) = world.get_location(&scene.location_id) {
                return location.backdrop_asset.clone();
            }
        }

        None
    }

    /// Clear all scene data (e.g., when disconnecting)
    pub fn clear_scene(&mut self) {
        self.current_scene.set(None);
        self.scene_characters.set(Vec::new());
        self.interactions.set(Vec::new());
        self.current_region.set(None);
        self.navigation.set(None);
        self.npcs_present.set(Vec::new());
        self.game_time.set(None);
        self.approach_event.set(None);
        self.location_event.set(None);
        self.staging_pending.set(None);
        self.pending_staging_approval.set(None);
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.world.set(None);
        self.clear_scene();
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}
