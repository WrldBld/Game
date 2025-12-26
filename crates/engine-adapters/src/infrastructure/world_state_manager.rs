use dashmap::DashMap;
use uuid::Uuid;
use wrldbldr_domain::{WorldId, GameTime, LocationId, RegionId};
use wrldbldr_engine_app::application::services::staging_service::StagingProposal;
use chrono::{DateTime, Utc};

/// Manages per-world state (game time, conversation, approvals)
pub struct WorldStateManager {
    states: DashMap<WorldId, WorldState>,
}

struct WorldState {
    /// Current game time for this world
    game_time: GameTime,
    
    /// Conversation history (last 30 entries)
    conversation_history: Vec<ConversationEntry>,
    
    /// Pending DM approvals
    pending_approvals: Vec<PendingApprovalItem>,
    
    /// Pending staging approvals (rich type with full data)
    pending_staging_approvals: Vec<WorldPendingStagingApproval>,
    
    /// Current scene ID (if any)
    current_scene_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationEntry {
    pub timestamp: DateTime<Utc>,
    pub speaker: Speaker,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Speaker {
    Player { pc_id: String, pc_name: String },
    Npc { npc_id: String, npc_name: String },
    System,
    Dm,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PendingApprovalItem {
    pub approval_id: String,
    pub approval_type: ApprovalType,
    pub created_at: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalType {
    Dialogue,
    Challenge,
    NarrativeEvent,
    ChallengeOutcome,
}

/// Pending staging approval with full data for handler access
///
/// This is the rich type used by WorldStateManager, containing all data
/// needed by handlers (MoveToRegion, ExitToLocation, staging handlers)
/// to process staging approvals.
///
/// Note: This type is stored in-memory only and does not need serialization.
/// The typed IDs (RegionId, LocationId, WorldId) are used for type safety.
#[derive(Debug, Clone)]
pub struct WorldPendingStagingApproval {
    /// Unique request ID for tracking
    pub request_id: String,
    
    /// Region being staged
    pub region_id: RegionId,
    
    /// Location containing the region
    pub location_id: LocationId,
    
    /// World this staging is for
    pub world_id: WorldId,
    
    /// Region name (for display)
    pub region_name: String,
    
    /// Location name (for display)
    pub location_name: String,
    
    /// The staging proposal with rule-based and LLM suggestions
    pub proposal: StagingProposal,
    
    /// PCs waiting for this staging to complete
    pub waiting_pcs: Vec<WaitingPc>,
    
    /// When this was created
    pub created_at: DateTime<Utc>,
}

/// A PC waiting for staging approval to complete
#[derive(Debug, Clone)]
pub struct WaitingPc {
    /// Player character ID
    pub pc_id: Uuid,
    
    /// Player character name (for display)
    pub pc_name: String,
    
    /// User ID controlling this PC
    pub user_id: String,
    
    /// Client ID for sending messages
    pub client_id: String,
}

impl WorldPendingStagingApproval {
    /// Create a new pending staging approval
    pub fn new(
        request_id: String,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        region_name: String,
        location_name: String,
        proposal: StagingProposal,
    ) -> Self {
        Self {
            request_id,
            region_id,
            location_id,
            world_id,
            region_name,
            location_name,
            proposal,
            waiting_pcs: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Add a PC to the waiting list (avoids duplicates)
    pub fn add_waiting_pc(&mut self, pc_id: Uuid, pc_name: String, user_id: String, client_id: String) {
        if !self.waiting_pcs.iter().any(|w| w.pc_id == pc_id) {
            self.waiting_pcs.push(WaitingPc {
                pc_id,
                pc_name,
                user_id,
                client_id,
            });
        }
    }
}

impl WorldStateManager {
    pub fn new() -> Self {
        Self {
            states: DashMap::new(),
        }
    }
    
    // === Lifecycle ===
    
    /// Initialize state for a new world
    pub fn initialize_world(&self, world_id: WorldId, initial_time: GameTime) {
        let state = WorldState {
            game_time: initial_time,
            conversation_history: Vec::new(),
            pending_approvals: Vec::new(),
            pending_staging_approvals: Vec::new(),
            current_scene_id: None,
        };
        self.states.insert(world_id, state);
    }
    
    /// Clean up when world connection is closed
    pub fn cleanup_world(&self, world_id: &WorldId) {
        self.states.remove(world_id);
    }
    
    // === Game Time ===
    
    pub fn get_game_time(&self, world_id: &WorldId) -> Option<GameTime> {
        self.states.get(world_id).map(|state| state.game_time.clone())
    }
    
    pub fn set_game_time(&self, world_id: &WorldId, time: GameTime) {
        self.states.entry(world_id.clone())
            .and_modify(|state| state.game_time = time.clone())
            .or_insert_with(|| WorldState {
                game_time: time,
                conversation_history: Vec::new(),
                pending_approvals: Vec::new(),
                pending_staging_approvals: Vec::new(),
                current_scene_id: None,
            });
    }
    
    /// Advance the game time for a world by the specified hours and minutes
    pub fn advance_game_time(&self, world_id: &WorldId, hours: i64, minutes: i64) -> Option<GameTime> {
        let mut state = self.states.get_mut(world_id)?;
        let duration = chrono::Duration::hours(hours) + chrono::Duration::minutes(minutes);
        state.game_time.advance(duration);
        Some(state.game_time.clone())
    }
    
    // === Conversation History ===
    
    pub fn get_conversation_history(&self, world_id: &WorldId) -> Vec<ConversationEntry> {
        self.states
            .get(world_id)
            .map(|state| state.conversation_history.clone())
            .unwrap_or_default()
    }
    
    pub fn add_conversation(&self, world_id: &WorldId, entry: ConversationEntry) {
        self.states.entry(world_id.clone())
            .and_modify(|state| {
                state.conversation_history.push(entry.clone());
                // Keep only last 30 entries
                if state.conversation_history.len() > 30 {
                    state.conversation_history.drain(0..(state.conversation_history.len() - 30));
                }
            })
            .or_insert_with(|| WorldState {
                game_time: GameTime::default(),
                conversation_history: vec![entry],
                pending_approvals: Vec::new(),
                pending_staging_approvals: Vec::new(),
                current_scene_id: None,
            });
    }
    
    pub fn clear_conversation_history(&self, world_id: &WorldId) {
        if let Some(mut state) = self.states.get_mut(world_id) {
            state.conversation_history.clear();
        }
    }
    
    // === Pending Approvals ===
    
    pub fn get_pending_approvals(&self, world_id: &WorldId) -> Vec<PendingApprovalItem> {
        self.states
            .get(world_id)
            .map(|state| state.pending_approvals.clone())
            .unwrap_or_default()
    }
    
    pub fn add_pending_approval(&self, world_id: &WorldId, item: PendingApprovalItem) {
        self.states.entry(world_id.clone())
            .and_modify(|state| {
                state.pending_approvals.push(item.clone());
            })
            .or_insert_with(|| WorldState {
                game_time: GameTime::default(),
                conversation_history: Vec::new(),
                pending_approvals: vec![item],
                pending_staging_approvals: Vec::new(),
                current_scene_id: None,
            });
    }
    
    pub fn remove_pending_approval(
        &self,
        world_id: &WorldId,
        approval_id: &str,
    ) -> Option<PendingApprovalItem> {
        self.states.get_mut(world_id).and_then(|mut state| {
            state.pending_approvals
                .iter()
                .position(|item| item.approval_id == approval_id)
                .map(|index| state.pending_approvals.remove(index))
        })
    }
    
    // === Pending Staging Approvals ===
    
    /// Get all pending staging approvals for a world
    pub fn get_all_pending_staging(&self, world_id: &WorldId) -> Vec<WorldPendingStagingApproval> {
        self.states
            .get(world_id)
            .map(|state| state.pending_staging_approvals.clone())
            .unwrap_or_default()
    }
    
    /// Get pending staging approval for a specific region
    pub fn get_pending_staging_for_region(
        &self,
        world_id: &WorldId,
        region_id: &RegionId,
    ) -> Option<WorldPendingStagingApproval> {
        let states = self.states.get(world_id)?;
        states.pending_staging_approvals
            .iter()
            .find(|p| &p.region_id == region_id)
            .cloned()
    }
    
    /// Get pending staging approval by request ID
    pub fn get_pending_staging_by_request_id(
        &self,
        world_id: &WorldId,
        request_id: &str,
    ) -> Option<WorldPendingStagingApproval> {
        let states = self.states.get(world_id)?;
        states.pending_staging_approvals
            .iter()
            .find(|p| p.request_id == request_id)
            .cloned()
    }
    
    /// Add a pending staging approval
    pub fn add_pending_staging(&self, world_id: &WorldId, approval: WorldPendingStagingApproval) {
        self.states.entry(world_id.clone())
            .and_modify(|state| {
                state.pending_staging_approvals.push(approval.clone());
            })
            .or_insert_with(|| WorldState {
                game_time: GameTime::default(),
                conversation_history: Vec::new(),
                pending_approvals: Vec::new(),
                pending_staging_approvals: vec![approval],
                current_scene_id: None,
            });
    }
    
    /// Remove pending staging approval by request ID
    pub fn remove_pending_staging(
        &self,
        world_id: &WorldId,
        request_id: &str,
    ) -> Option<WorldPendingStagingApproval> {
        self.states.get_mut(world_id).and_then(|mut state| {
            state.pending_staging_approvals
                .iter()
                .position(|item| item.request_id == request_id)
                .map(|index| state.pending_staging_approvals.remove(index))
        })
    }
    
    /// Remove pending staging approval for a specific region
    pub fn remove_pending_staging_for_region(
        &self,
        world_id: &WorldId,
        region_id: &RegionId,
    ) -> Option<WorldPendingStagingApproval> {
        self.states.get_mut(world_id).and_then(|mut state| {
            state.pending_staging_approvals
                .iter()
                .position(|p| &p.region_id == region_id)
                .map(|index| state.pending_staging_approvals.remove(index))
        })
    }
    
    /// Add a waiting PC to a pending staging approval for a region
    pub fn add_waiting_pc_to_staging(
        &self,
        world_id: &WorldId,
        region_id: &RegionId,
        pc_id: Uuid,
        pc_name: String,
        user_id: String,
        client_id: String,
    ) -> bool {
        if let Some(mut states) = self.states.get_mut(world_id) {
            if let Some(approval) = states.pending_staging_approvals
                .iter_mut()
                .find(|p| &p.region_id == region_id)
            {
                approval.add_waiting_pc(pc_id, pc_name, user_id, client_id);
                return true;
            }
        }
        false
    }
    
    /// Get waiting PCs for a staging approval by region
    pub fn get_waiting_pcs_for_staging(
        &self,
        world_id: &WorldId,
        region_id: &RegionId,
    ) -> Vec<WaitingPc> {
        self.states
            .get(world_id)
            .map(|s| {
                s.pending_staging_approvals
                    .iter()
                    .find(|p| &p.region_id == region_id)
                    .map(|p| p.waiting_pcs.clone())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }
    
    /// Get a mutable reference to a pending staging approval for a region
    /// 
    /// Returns a guard that allows mutation of the approval.
    /// Note: This uses DashMap's internal locking - be careful with long holds.
    pub fn with_pending_staging_for_region_mut<F, R>(
        &self,
        world_id: &WorldId,
        region_id: &RegionId,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut WorldPendingStagingApproval) -> R,
    {
        self.states.get_mut(world_id).and_then(|mut state| {
            state.pending_staging_approvals
                .iter_mut()
                .find(|p| &p.region_id == region_id)
                .map(f)
        })
    }
    
    // === Current Scene ===
    
    pub fn get_current_scene(&self, world_id: &WorldId) -> Option<String> {
        self.states
            .get(world_id)
            .and_then(|state| state.current_scene_id.clone())
    }
    
    pub fn set_current_scene(&self, world_id: &WorldId, scene_id: Option<String>) {
        self.states.entry(world_id.clone())
            .and_modify(|state| {
                state.current_scene_id = scene_id.clone();
            })
            .or_insert_with(|| WorldState {
                game_time: GameTime::default(),
                conversation_history: Vec::new(),
                pending_approvals: Vec::new(),
                pending_staging_approvals: Vec::new(),
                current_scene_id: scene_id,
            });
    }
}

impl Default for WorldStateManager {
    fn default() -> Self {
        Self::new()
    }
}
