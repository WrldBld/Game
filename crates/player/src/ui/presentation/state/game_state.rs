//! Game state management using Dioxus signals
//!
//! Central game state for the Player application.

use dioxus::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

/// Maximum number of pending time suggestions to store (oldest are evicted)
const MAX_TIME_SUGGESTIONS: usize = 50;

/// Maximum number of NPC mood entries to track (LRU eviction)
const MAX_NPC_MOODS: usize = 200;

use crate::application::dto::{
    CharacterData as SceneCharacterState, EntityChangedData, GameTime, InteractionData,
    NavigationData, NpcDispositionData, NpcPresenceData, RegionData as SceneRegionInfo,
    RegionItemData, SceneData as SceneSnapshot, SessionWorldSnapshot, SplitPartyLocation,
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

/// View mode for Director - normal or viewing as a specific character
#[derive(Clone, Debug, PartialEq, Default)]
pub enum ViewMode {
    /// Normal Director view
    #[default]
    Director,
    /// Viewing as a specific character (read-only PC perspective)
    ViewingAsCharacter {
        /// The character ID being viewed as
        character_id: String,
        /// The character's name for display
        character_name: String,
    },
}

/// Staging pending data (player waiting for DM to approve staging)
#[derive(Clone, Debug, PartialEq)]
pub struct StagingPendingData {
    /// The region where staging is pending
    pub region_id: String,
    /// Region name for display
    pub region_name: String,
    /// When the staging request started (epoch milliseconds from PlatformPort::now_millis)
    pub started_at_ms: u64,
    /// How many seconds until auto-approve (0 = no auto-approve)
    pub timeout_seconds: u64,
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
    pub is_hidden_from_players: bool,
}

impl From<crate::application::dto::StagedNpcInfo> for StagedNpcData {
    fn from(info: crate::application::dto::StagedNpcInfo) -> Self {
        Self {
            character_id: info.character_id,
            name: info.name,
            sprite_asset: info.sprite_asset,
            portrait_asset: info.portrait_asset,
            is_present: info.is_present,
            reasoning: info.reasoning,
            is_hidden_from_players: info.is_hidden_from_players,
        }
    }
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

/// Staging status for a specific region (for DM panel display)
#[derive(Clone, Debug, PartialEq)]
pub enum RegionStagingStatus {
    /// No staging set - will prompt when player enters
    None,
    /// Staging approval is pending (player waiting)
    Pending,
    /// Staging is active with NPCs present
    Active {
        /// The staging ID (if known)
        staging_id: String,
        /// Names of NPCs currently staged in this region
        npc_names: Vec<String>,
    },
}

/// Time suggestion pending DM approval
#[derive(Clone, Debug, PartialEq)]
pub struct TimeSuggestionData {
    pub suggestion_id: String,
    pub pc_id: String,
    pub pc_name: String,
    pub action_type: String,
    pub action_description: String,
    pub suggested_minutes: u32,
    pub current_time: GameTime,
    pub resulting_time: GameTime,
    pub period_change: Option<(String, String)>,
}

/// Time mode for the world
#[derive(Clone, Debug, PartialEq, Default)]
pub enum TimeMode {
    /// DM manually controls time
    Manual,
    /// System suggests, DM approves
    #[default]
    Suggested,
}

impl std::str::FromStr for TimeMode {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "manual" => TimeMode::Manual,
            "suggested" => TimeMode::Suggested,
            // Auto mode is intentionally treated as Suggested.
            "auto" | "automatic" => TimeMode::Suggested,
            _ => TimeMode::Suggested,
        })
    }
}

impl TimeMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            TimeMode::Manual => "Manual",
            TimeMode::Suggested => "Suggested",
        }
    }
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
    /// Items visible in the current region (can be picked up)
    pub region_items: Signal<Vec<RegionItemData>>,
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
    /// Counter to trigger inventory refresh (incremented when inventory changes)
    pub inventory_refresh_counter: Signal<u32>,
    /// Counter to trigger observations refresh (incremented when NPC locations are shared)
    pub observations_refresh_counter: Signal<u32>,
    /// Split party warning - locations where PCs are distributed (empty = party together)
    pub split_party_locations: Signal<Vec<SplitPartyLocation>>,
    /// Current view mode (Director or ViewingAsCharacter)
    pub view_mode: Signal<ViewMode>,
    /// Counter to trigger actantial/motivations refresh (incremented on wants/goals changes)
    pub actantial_refresh_counter: Signal<u32>,
    /// NPC dispositions toward the currently selected PC (populated from NpcDispositionsResponse)
    pub npc_dispositions: Signal<Vec<NpcDispositionData>>,
    /// Per-region staging status for DM panel (updated from staging events)
    pub region_staging_statuses: Signal<HashMap<String, RegionStagingStatus>>,
    /// Pending time suggestions for DM approval
    pub pending_time_suggestions: Signal<Vec<TimeSuggestionData>>,
    /// Current time mode for the world
    pub time_mode: Signal<TimeMode>,
    /// Whether game time is currently paused
    pub time_paused: Signal<bool>,
    /// Current moods of NPCs in the scene (npc_id -> mood string)
    /// Updated by NpcMoodChanged events, used for expression/sprite display
    pub npc_moods: Signal<HashMap<String, String>>,
    /// Whether the backdrop is transitioning (fade effect during scene change)
    pub backdrop_transitioning: Signal<bool>,
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
            region_items: Signal::new(Vec::new()),
            selected_pc_id: Signal::new(None),
            game_time: Signal::new(None),
            approach_event: Signal::new(None),
            location_event: Signal::new(None),
            staging_pending: Signal::new(None),
            pending_staging_approval: Signal::new(None),
            inventory_refresh_counter: Signal::new(0),
            observations_refresh_counter: Signal::new(0),
            split_party_locations: Signal::new(Vec::new()),
            view_mode: Signal::new(ViewMode::default()),
            actantial_refresh_counter: Signal::new(0),
            npc_dispositions: Signal::new(Vec::new()),
            region_staging_statuses: Signal::new(HashMap::new()),
            pending_time_suggestions: Signal::new(Vec::new()),
            time_mode: Signal::new(TimeMode::default()),
            time_paused: Signal::new(true),
            npc_moods: Signal::new(HashMap::new()),
            backdrop_transitioning: Signal::new(false),
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
        region_items: Vec<RegionItemData>,
    ) {
        // Trigger backdrop fade transition
        self.trigger_backdrop_transition();

        self.selected_pc_id.set(Some(pc_id));
        self.current_region.set(Some(region));
        self.npcs_present.set(npcs_present);
        self.navigation.set(Some(navigation));
        self.region_items.set(region_items);
        // Clear NPC moods when changing scene - they'll be repopulated from staging
        self.clear_npc_moods();
    }

    /// Trigger a backdrop fade transition effect
    pub fn trigger_backdrop_transition(&mut self) {
        self.backdrop_transitioning.set(true);
    }

    /// Clear the backdrop transition state (called after animation completes)
    pub fn clear_backdrop_transition(&mut self) {
        self.backdrop_transitioning.set(false);
    }

    /// Remove an item from region_items (for optimistic pickup updates)
    pub fn remove_region_item(&mut self, item_id: &str) {
        let items = self.region_items.read();
        let filtered: Vec<RegionItemData> = items
            .iter()
            .filter(|item| item.id != item_id)
            .cloned()
            .collect();
        drop(items);
        self.region_items.set(filtered);
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
    ///
    /// `started_at_ms` should be obtained from `PlatformPort::now_millis()`
    pub fn set_staging_pending(
        &mut self,
        region_id: String,
        region_name: String,
        timeout_seconds: u64,
        started_at_ms: u64,
    ) {
        self.staging_pending.set(Some(StagingPendingData {
            region_id,
            region_name,
            started_at_ms,
            timeout_seconds,
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

    /// Trigger an inventory refresh (increments counter to signal UI components)
    pub fn trigger_inventory_refresh(&mut self) {
        let current = *self.inventory_refresh_counter.read();
        self.inventory_refresh_counter.set(current.wrapping_add(1));
    }

    /// Trigger an observations refresh (increments counter to signal UI components)
    pub fn trigger_observations_refresh(&mut self) {
        let current = *self.observations_refresh_counter.read();
        self.observations_refresh_counter
            .set(current.wrapping_add(1));
    }

    /// Trigger an actantial/motivations refresh (increments counter to signal UI components)
    pub fn trigger_actantial_refresh(&mut self) {
        let current = *self.actantial_refresh_counter.read();
        self.actantial_refresh_counter.set(current.wrapping_add(1));
    }

    /// Set NPC dispositions (from NpcDispositionsResponse)
    pub fn set_npc_dispositions(&mut self, dispositions: Vec<NpcDispositionData>) {
        self.npc_dispositions.set(dispositions);
    }

    /// Update a single NPC disposition (from NpcDispositionChanged)
    pub fn update_npc_disposition(
        &mut self,
        npc_id: &str,
        disposition: String,
        relationship: String,
        reason: Option<String>,
    ) {
        let mut dispositions = self.npc_dispositions.write();
        if let Some(d) = dispositions.iter_mut().find(|d| d.npc_id == npc_id) {
            d.disposition = disposition;
            d.relationship = relationship;
            d.last_reason = reason;
        }
        // Note: If NPC not in list, we don't add it - this is expected behavior
        // The full disposition list should be fetched via GetNpcDispositions request
    }

    /// Clear NPC dispositions (when changing scene or PC)
    pub fn clear_npc_dispositions(&mut self) {
        self.npc_dispositions.set(Vec::new());
    }

    /// Update an NPC's current mood (from NpcMoodChanged event)
    /// This is used for character expression/sprite display.
    /// Enforces MAX_NPC_MOODS limit by removing oldest entries when full.
    pub fn update_npc_mood(&mut self, npc_id: String, mood: String) {
        let mut moods = self.npc_moods.write();
        // If this is a new NPC and we're at the limit, remove an arbitrary entry
        if !moods.contains_key(&npc_id) && moods.len() >= MAX_NPC_MOODS {
            // Remove the first entry (arbitrary, but consistent)
            if let Some(key_to_remove) = moods.keys().next().cloned() {
                moods.remove(&key_to_remove);
            }
        }
        moods.insert(npc_id, mood);
    }

    /// Get the current mood for an NPC (returns None if not set)
    pub fn get_npc_mood(&self, npc_id: &str) -> Option<String> {
        self.npc_moods.read().get(npc_id).cloned()
    }

    /// Clear all NPC moods (when changing scene)
    pub fn clear_npc_moods(&mut self) {
        self.npc_moods.write().clear();
    }

    /// Set the staging status for a specific region
    pub fn set_region_staging_status(&mut self, region_id: String, status: RegionStagingStatus) {
        self.region_staging_statuses
            .write()
            .insert(region_id, status);
    }

    /// Get the staging status for a specific region
    pub fn get_region_staging_status(&self, region_id: &str) -> RegionStagingStatus {
        self.region_staging_statuses
            .read()
            .get(region_id)
            .cloned()
            .unwrap_or(RegionStagingStatus::None)
    }

    /// Clear all region staging statuses (when disconnecting or changing world)
    pub fn clear_region_staging_statuses(&mut self) {
        self.region_staging_statuses.write().clear();
    }

    /// Add a pending time suggestion (for DM approval)
    /// Enforces MAX_TIME_SUGGESTIONS limit by removing oldest entries
    pub fn add_time_suggestion(&mut self, suggestion: TimeSuggestionData) {
        let mut suggestions = self.pending_time_suggestions.write();
        // Remove oldest suggestions if we're at the limit
        while suggestions.len() >= MAX_TIME_SUGGESTIONS {
            suggestions.remove(0);
        }
        suggestions.push(suggestion);
    }

    /// Remove a time suggestion by ID (after DM action)
    pub fn remove_time_suggestion(&mut self, suggestion_id: &str) {
        self.pending_time_suggestions
            .write()
            .retain(|s| s.suggestion_id != suggestion_id);
    }

    /// Clear all pending time suggestions
    pub fn clear_time_suggestions(&mut self) {
        self.pending_time_suggestions.write().clear();
    }

    /// Set the time mode
    pub fn set_time_mode(&mut self, mode: TimeMode) {
        self.time_mode.set(mode);
    }

    /// Set whether time is paused
    pub fn set_time_paused(&mut self, paused: bool) {
        self.time_paused.set(paused);
    }

    /// Trigger appropriate refresh based on entity change notification
    pub fn trigger_entity_refresh(&mut self, entity_changed: &EntityChangedData) {
        match entity_changed.entity_type.as_str() {
            "Character" | "PlayerCharacter" => {
                // Characters might affect scenes, observations, etc.
                self.trigger_observations_refresh();
            }
            "Goal" | "Want" | "ActantialView" => {
                self.trigger_actantial_refresh();
            }
            "Observation" => {
                self.trigger_observations_refresh();
            }
            // For other entity types, we might need to trigger world reload
            // but for now just log them
            other => {
                tracing::debug!("Entity change for {} - no specific refresh handler", other);
            }
        }
    }

    /// Update split party locations (from SplitPartyNotification)
    pub fn set_split_party_locations(&mut self, locations: Vec<SplitPartyLocation>) {
        self.split_party_locations.set(locations);
    }

    /// Clear split party warning (party is together again)
    pub fn clear_split_party(&mut self) {
        self.split_party_locations.set(Vec::new());
    }

    /// Check if party is currently split
    pub fn is_party_split(&self) -> bool {
        self.split_party_locations.read().len() > 1
    }

    /// Start viewing as a specific character (read-only perspective mode)
    ///
    /// This updates both view_mode and selected_pc_id to ensure the UI
    /// shows the correct character's perspective.
    pub fn start_viewing_as(&mut self, character_id: String, character_name: String) {
        // Update selected_pc_id so scene/location views show this character's perspective
        self.selected_pc_id.set(Some(character_id.clone()));
        self.view_mode.set(ViewMode::ViewingAsCharacter {
            character_id,
            character_name,
        });
    }

    /// Stop viewing as character and return to normal Director mode
    ///
    /// Note: This doesn't clear selected_pc_id - the DM may still want to
    /// view a specific PC's scene data even when not in "viewing as" mode.
    pub fn stop_viewing_as(&mut self) {
        self.view_mode.set(ViewMode::Director);
    }

    /// Check if currently viewing as a character
    pub fn is_viewing_as_character(&self) -> bool {
        matches!(*self.view_mode.read(), ViewMode::ViewingAsCharacter { .. })
    }

    /// Get the character ID being viewed as (if any)
    pub fn viewing_as_character_id(&self) -> Option<String> {
        match &*self.view_mode.read() {
            ViewMode::ViewingAsCharacter { character_id, .. } => Some(character_id.clone()),
            ViewMode::Director => None,
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
        self.region_items.set(Vec::new());
        self.game_time.set(None);
        self.approach_event.set(None);
        self.location_event.set(None);
        self.staging_pending.set(None);
        self.pending_staging_approval.set(None);
        self.split_party_locations.set(Vec::new());
        self.view_mode.set(ViewMode::Director);
        self.selected_pc_id.set(None);
        self.npc_dispositions.set(Vec::new());
        self.npc_moods.write().clear();
        self.region_staging_statuses.write().clear();
        self.pending_time_suggestions.write().clear();
        self.time_mode.set(TimeMode::default());
        self.time_paused.set(true);
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
