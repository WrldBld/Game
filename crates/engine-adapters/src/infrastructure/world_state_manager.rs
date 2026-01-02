use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use uuid::Uuid;
use wrldbldr_domain::value_objects::{
    ConversationEntry, DirectorialNotes, DomainNpcMotivation, PacingGuidance, PendingApprovalItem,
};
use wrldbldr_domain::{GameTime, LocationId, RegionId, WorldId};
use wrldbldr_engine_ports::outbound::{
    ClockPort, WorldApprovalPort, WorldConversationPort, WorldDirectorialPort, WorldLifecyclePort,
    WorldScenePort, WorldTimePort,
};
use wrldbldr_engine_ports::outbound::{
    DirectorialContextData, PendingStagingData, PendingStagingInfo, RegeneratedNpc, StagedNpcData,
    StagingStateExtPort, StagingStatePort, WaitingPcInfo, WorldStateUpdatePort,
};

use crate::infrastructure::websocket::directorial_converters::parse_tone;

/// In-memory implementation of the world state sub-traits.
///
/// Manages per-world state (game time, conversation, approvals) using DashMap
/// for thread-safe concurrent access.
///
/// # Architecture Note
///
/// This adapter also contains staging approval methods that are NOT part of
/// the world state port traits because they depend on `StagingProposal` (a boundary DTO).
/// These methods are accessed directly by handlers that need staging functionality.
pub struct WorldStateManager {
    states: DashMap<WorldId, WorldState>,
    clock: Arc<dyn ClockPort>,
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

    /// DM's directorial context (runtime guidance for NPCs)
    directorial_context: Option<DirectorialNotes>,
}

// =============================================================================
// Adapter-internal staging types
// =============================================================================
//
// These types are internal to the WorldStateManager adapter. They were previously
// in engine-app, but adapters cannot depend on engine-app in hexagonal architecture.
// These are only used for in-memory storage and don't cross layer boundaries.

/// Staged NPC in a proposal (adapter-internal type)
#[derive(Debug, Clone)]
pub struct StagedNpcProposal {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: String,
}

/// Full staging proposal (adapter-internal type)
///
/// Contains the NPC suggestions from both rule-based and LLM-based sources.
#[derive(Debug, Clone)]
pub struct StagingProposal {
    pub request_id: String,
    pub region_id: String,
    pub location_id: String,
    pub world_id: String,
    pub rule_based_npcs: Vec<StagedNpcProposal>,
    pub llm_based_npcs: Vec<StagedNpcProposal>,
    pub default_ttl_hours: i32,
    pub context: wrldbldr_domain::value_objects::StagingContext,
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
        created_at: DateTime<Utc>,
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
            created_at,
        }
    }

    /// Add a PC to the waiting list (avoids duplicates)
    pub fn add_waiting_pc(
        &mut self,
        pc_id: Uuid,
        pc_name: String,
        user_id: String,
        client_id: String,
    ) {
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
    pub fn new(clock: Arc<dyn ClockPort>) -> Self {
        Self {
            states: DashMap::new(),
            clock,
        }
    }

    // === Pending Staging Approvals ===
    //
    // NOTE: These methods are NOT part of the world state port traits because they
    // depend on StagingProposal from engine-app. They are adapter-specific methods
    // accessed directly by handlers that need staging functionality.

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
        states
            .pending_staging_approvals
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
        states
            .pending_staging_approvals
            .iter()
            .find(|p| p.request_id == request_id)
            .cloned()
    }

    /// Add a pending staging approval
    pub fn add_pending_staging(&self, world_id: &WorldId, approval: WorldPendingStagingApproval) {
        let now = self.clock.now();
        self.states
            .entry(*world_id)
            .and_modify(|state| {
                state.pending_staging_approvals.push(approval.clone());
            })
            .or_insert_with(|| WorldState {
                game_time: GameTime::new(now),
                conversation_history: Vec::new(),
                pending_approvals: Vec::new(),
                pending_staging_approvals: vec![approval],
                current_scene_id: None,
                directorial_context: None,
            });
    }

    /// Remove pending staging approval by request ID
    pub fn remove_pending_staging(
        &self,
        world_id: &WorldId,
        request_id: &str,
    ) -> Option<WorldPendingStagingApproval> {
        self.states.get_mut(world_id).and_then(|mut state| {
            state
                .pending_staging_approvals
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
            state
                .pending_staging_approvals
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
            if let Some(approval) = states
                .pending_staging_approvals
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
            state
                .pending_staging_approvals
                .iter_mut()
                .find(|p| &p.region_id == region_id)
                .map(f)
        })
    }
}

fn staged_npc_data_to_proposal_npc(npc: &StagedNpcData) -> StagedNpcProposal {
    StagedNpcProposal {
        character_id: npc.character_id.to_string(),
        name: npc.name.clone(),
        sprite_asset: npc.sprite_asset.clone(),
        portrait_asset: npc.portrait_asset.clone(),
        is_present: npc.is_present,
        is_hidden_from_players: npc.is_hidden_from_players,
        reasoning: npc.reasoning.clone(),
    }
}

fn waiting_pc_to_info(pc: &WaitingPc) -> WaitingPcInfo {
    WaitingPcInfo {
        pc_id: wrldbldr_domain::PlayerCharacterId::from_uuid(pc.pc_id),
        pc_name: pc.pc_name.clone(),
        user_id: pc.user_id.clone(),
    }
}

fn approval_to_info(approval: &WorldPendingStagingApproval) -> PendingStagingInfo {
    PendingStagingInfo {
        request_id: approval.request_id.clone(),
        world_id: approval.world_id,
        region_id: approval.region_id,
        location_id: approval.location_id,
        region_name: approval.region_name.clone(),
        location_name: approval.location_name.clone(),
        waiting_pcs: approval
            .waiting_pcs
            .iter()
            .map(waiting_pc_to_info)
            .collect(),
        rule_based_npcs: approval
            .proposal
            .rule_based_npcs
            .iter()
            .map(|npc| wrldbldr_engine_ports::outbound::ProposedNpc {
                character_id: npc.character_id.clone(),
                name: npc.name.clone(),
                sprite_asset: npc.sprite_asset.clone(),
                portrait_asset: npc.portrait_asset.clone(),
                is_present: npc.is_present,
                is_hidden_from_players: npc.is_hidden_from_players,
                reasoning: npc.reasoning.clone(),
            })
            .collect(),
        llm_based_npcs: approval
            .proposal
            .llm_based_npcs
            .iter()
            .map(|npc| wrldbldr_engine_ports::outbound::ProposedNpc {
                character_id: npc.character_id.clone(),
                name: npc.name.clone(),
                sprite_asset: npc.sprite_asset.clone(),
                portrait_asset: npc.portrait_asset.clone(),
                is_present: npc.is_present,
                is_hidden_from_players: npc.is_hidden_from_players,
                reasoning: npc.reasoning.clone(),
            })
            .collect(),
    }
}

fn directorial_context_to_notes(context: DirectorialContextData) -> DirectorialNotes {
    let npc_motivations = context
        .npc_motivations
        .into_iter()
        .map(|m| {
            let motivation =
                DomainNpcMotivation::new(m.emotional_state.unwrap_or_default(), m.motivation);
            (m.character_id, motivation)
        })
        .collect();

    DirectorialNotes {
        general_notes: context.dm_notes.unwrap_or_default(),
        tone: parse_tone(&context.scene_mood.unwrap_or_default()),
        npc_motivations,
        forbidden_topics: Vec::new(),
        allowed_tools: Vec::new(),
        suggested_beats: Vec::new(),
        pacing: context
            .pacing
            .as_ref()
            .map(|p| match p.to_lowercase().as_str() {
                "fast" => PacingGuidance::Fast,
                "slow" => PacingGuidance::Slow,
                "building" => PacingGuidance::Building,
                "urgent" => PacingGuidance::Urgent,
                _ => PacingGuidance::Natural,
            })
            .unwrap_or(PacingGuidance::Natural),
    }
}

impl WorldStateUpdatePort for WorldStateManager {
    fn set_current_scene(&self, world_id: &WorldId, scene_id: Option<String>) {
        WorldScenePort::set_current_scene(self, world_id, scene_id);
    }

    fn set_directorial_context(&self, world_id: &WorldId, context: DirectorialContextData) {
        let notes = directorial_context_to_notes(context);
        WorldDirectorialPort::set_directorial_context(self, world_id, notes);
    }
}

impl StagingStatePort for WorldStateManager {
    fn get_game_time(&self, world_id: &WorldId) -> Option<GameTime> {
        WorldTimePort::get_game_time(self, world_id)
    }

    fn has_pending_staging(&self, world_id: &WorldId, region_id: &RegionId) -> bool {
        WorldStateManager::get_pending_staging_for_region(self, world_id, region_id).is_some()
    }

    fn add_waiting_pc(
        &self,
        world_id: &WorldId,
        region_id: &RegionId,
        pc_id: uuid::Uuid,
        pc_name: String,
        user_id: String,
        client_id: String,
    ) {
        WorldStateManager::add_waiting_pc_to_staging(
            self, world_id, region_id, pc_id, pc_name, user_id, client_id,
        );
    }

    fn store_pending_staging(&self, pending: PendingStagingData) {
        let proposal = StagingProposal {
            request_id: pending.request_id.clone(),
            region_id: pending.region_id.to_string(),
            location_id: pending.location_id.to_string(),
            world_id: pending.world_id.to_string(),
            rule_based_npcs: pending
                .rule_based_npcs
                .iter()
                .map(staged_npc_data_to_proposal_npc)
                .collect(),
            llm_based_npcs: pending
                .llm_based_npcs
                .iter()
                .map(staged_npc_data_to_proposal_npc)
                .collect(),
            default_ttl_hours: pending.default_ttl_hours,
            context: wrldbldr_domain::value_objects::StagingContext::new("", "", "", "", ""),
        };

        let mut approval = WorldPendingStagingApproval::new(
            pending.request_id,
            pending.region_id,
            pending.location_id,
            pending.world_id,
            pending.region_name,
            pending.location_name,
            proposal,
            self.clock.now(),
        );

        for pc in &pending.waiting_pcs {
            approval.add_waiting_pc(
                *pc.pc_id.as_uuid(),
                pc.pc_name.clone(),
                pc.user_id.clone(),
                String::new(),
            );
        }

        WorldStateManager::add_pending_staging(self, &pending.world_id, approval);
    }
}

impl StagingStateExtPort for WorldStateManager {
    fn get_pending_staging(
        &self,
        world_id: &WorldId,
        request_id: &str,
    ) -> Option<PendingStagingInfo> {
        WorldStateManager::get_pending_staging_by_request_id(self, world_id, request_id)
            .as_ref()
            .map(approval_to_info)
    }

    fn remove_pending_staging(&self, world_id: &WorldId, request_id: &str) {
        let _ = WorldStateManager::remove_pending_staging(self, world_id, request_id);
    }

    fn update_llm_suggestions(
        &self,
        world_id: &WorldId,
        request_id: &str,
        npcs: Vec<RegeneratedNpc>,
    ) {
        if let Some(pending) =
            WorldStateManager::get_pending_staging_by_request_id(self, world_id, request_id)
        {
            let _ = WorldStateManager::with_pending_staging_for_region_mut(
                self,
                world_id,
                &pending.region_id,
                |approval| {
                    approval.proposal.llm_based_npcs = npcs
                        .iter()
                        .map(|npc| StagedNpcProposal {
                            character_id: npc.character_id.clone(),
                            name: npc.name.clone(),
                            sprite_asset: npc.sprite_asset.clone(),
                            portrait_asset: npc.portrait_asset.clone(),
                            is_present: npc.is_present,
                            is_hidden_from_players: npc.is_hidden_from_players,
                            reasoning: npc.reasoning.clone(),
                        })
                        .collect();
                },
            );
        }
    }
}

// === WorldTimePort Implementation ===

impl WorldTimePort for WorldStateManager {
    fn get_game_time(&self, world_id: &WorldId) -> Option<GameTime> {
        self.states
            .get(world_id)
            .map(|state| state.game_time.clone())
    }

    fn set_game_time(&self, world_id: &WorldId, time: GameTime) {
        self.states
            .entry(*world_id)
            .and_modify(|state| state.game_time = time.clone())
            .or_insert_with(|| WorldState {
                game_time: time,
                conversation_history: Vec::new(),
                pending_approvals: Vec::new(),
                pending_staging_approvals: Vec::new(),
                current_scene_id: None,
                directorial_context: None,
            });
    }

    fn advance_game_time(&self, world_id: &WorldId, hours: i64, minutes: i64) -> Option<GameTime> {
        let mut state = self.states.get_mut(world_id)?;
        let duration = chrono::Duration::hours(hours) + chrono::Duration::minutes(minutes);
        state.game_time.advance(duration);
        Some(state.game_time.clone())
    }
}

// === WorldConversationPort Implementation ===

impl WorldConversationPort for WorldStateManager {
    fn add_conversation(&self, world_id: &WorldId, entry: ConversationEntry) {
        let now = self.clock.now();
        self.states
            .entry(*world_id)
            .and_modify(|state| {
                state.conversation_history.push(entry.clone());
                // Keep only last 30 entries
                if state.conversation_history.len() > 30 {
                    state
                        .conversation_history
                        .drain(0..(state.conversation_history.len() - 30));
                }
            })
            .or_insert_with(|| WorldState {
                game_time: GameTime::new(now),
                conversation_history: vec![entry],
                pending_approvals: Vec::new(),
                pending_staging_approvals: Vec::new(),
                current_scene_id: None,
                directorial_context: None,
            });
    }

    fn get_conversation_history(
        &self,
        world_id: &WorldId,
        limit: Option<usize>,
    ) -> Vec<ConversationEntry> {
        self.states
            .get(world_id)
            .map(|state| {
                let history = &state.conversation_history;
                match limit {
                    Some(n) if n < history.len() => history[history.len() - n..].to_vec(),
                    _ => history.clone(),
                }
            })
            .unwrap_or_default()
    }

    fn clear_conversation_history(&self, world_id: &WorldId) {
        if let Some(mut state) = self.states.get_mut(world_id) {
            state.conversation_history.clear();
        }
    }
}

// === WorldApprovalPort Implementation ===

impl WorldApprovalPort for WorldStateManager {
    fn add_pending_approval(&self, world_id: &WorldId, item: PendingApprovalItem) {
        let now = self.clock.now();
        self.states
            .entry(*world_id)
            .and_modify(|state| {
                state.pending_approvals.push(item.clone());
            })
            .or_insert_with(|| WorldState {
                game_time: GameTime::new(now),
                conversation_history: Vec::new(),
                pending_approvals: vec![item],
                pending_staging_approvals: Vec::new(),
                current_scene_id: None,
                directorial_context: None,
            });
    }

    fn remove_pending_approval(
        &self,
        world_id: &WorldId,
        approval_id: &str,
    ) -> Option<PendingApprovalItem> {
        self.states.get_mut(world_id).and_then(|mut state| {
            state
                .pending_approvals
                .iter()
                .position(|item| item.approval_id == approval_id)
                .map(|index| state.pending_approvals.remove(index))
        })
    }

    fn get_pending_approvals(&self, world_id: &WorldId) -> Vec<PendingApprovalItem> {
        self.states
            .get(world_id)
            .map(|state| state.pending_approvals.clone())
            .unwrap_or_default()
    }
}

// === WorldScenePort Implementation ===

impl WorldScenePort for WorldStateManager {
    fn get_current_scene(&self, world_id: &WorldId) -> Option<String> {
        self.states
            .get(world_id)
            .and_then(|state| state.current_scene_id.clone())
    }

    fn set_current_scene(&self, world_id: &WorldId, scene_id: Option<String>) {
        let now = self.clock.now();
        self.states
            .entry(*world_id)
            .and_modify(|state| {
                state.current_scene_id = scene_id.clone();
            })
            .or_insert_with(|| WorldState {
                game_time: GameTime::new(now),
                conversation_history: Vec::new(),
                pending_approvals: Vec::new(),
                pending_staging_approvals: Vec::new(),
                current_scene_id: scene_id,
                directorial_context: None,
            });
    }
}

// === WorldDirectorialPort Implementation ===

impl WorldDirectorialPort for WorldStateManager {
    fn get_directorial_context(&self, world_id: &WorldId) -> Option<DirectorialNotes> {
        self.states
            .get(world_id)
            .and_then(|state| state.directorial_context.clone())
    }

    fn set_directorial_context(&self, world_id: &WorldId, notes: DirectorialNotes) {
        let now = self.clock.now();
        self.states
            .entry(*world_id)
            .and_modify(|state| {
                state.directorial_context = Some(notes.clone());
            })
            .or_insert_with(|| WorldState {
                game_time: GameTime::new(now),
                conversation_history: Vec::new(),
                pending_approvals: Vec::new(),
                pending_staging_approvals: Vec::new(),
                current_scene_id: None,
                directorial_context: Some(notes),
            });
    }

    fn clear_directorial_context(&self, world_id: &WorldId) {
        if let Some(mut state) = self.states.get_mut(world_id) {
            state.directorial_context = None;
        }
    }
}

// === WorldLifecyclePort Implementation ===

impl WorldLifecyclePort for WorldStateManager {
    fn initialize_world(&self, world_id: &WorldId, initial_time: GameTime) {
        let state = WorldState {
            game_time: initial_time,
            conversation_history: Vec::new(),
            pending_approvals: Vec::new(),
            pending_staging_approvals: Vec::new(),
            current_scene_id: None,
            directorial_context: None,
        };
        self.states.insert(*world_id, state);
    }

    fn cleanup_world(&self, world_id: &WorldId) {
        self.states.remove(world_id);
    }

    fn is_world_initialized(&self, world_id: &WorldId) -> bool {
        self.states.contains_key(world_id)
    }
}
