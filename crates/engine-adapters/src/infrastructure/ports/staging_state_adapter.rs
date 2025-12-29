//! Staging State Adapter
//!
//! Implements `StagingStatePort` and `StagingStateExtPort` by wrapping `WorldStateManager`.
//! This adapter bridges the use case layer's abstract port interface with the
//! infrastructure's concrete state management.

use std::sync::Arc;
use uuid::Uuid;

use wrldbldr_domain::value_objects::StagingContext;
use wrldbldr_domain::{GameTime, RegionId, WorldId};
use wrldbldr_engine_app::application::services::staging_service::{
    StagedNpcProposal, StagingProposal,
};
use wrldbldr_engine_ports::inbound::{
    PendingStagingData, PendingStagingInfo, ProposedNpc, RegeneratedNpc, StagingStateExtPort,
    StagingStatePort, WaitingPcInfo,
};
use wrldbldr_engine_ports::outbound::StagedNpcData;

use crate::infrastructure::{WaitingPc, WorldPendingStagingApproval, WorldStateManager};

/// Adapter that implements staging state ports using WorldStateManager
pub struct StagingStateAdapter {
    world_state: Arc<WorldStateManager>,
}

impl StagingStateAdapter {
    /// Create a new adapter wrapping the given WorldStateManager
    pub fn new(world_state: Arc<WorldStateManager>) -> Self {
        Self { world_state }
    }

    /// Convert domain StagedNpcData to infrastructure proposal NPCs (StagedNpcProposal)
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

    /// Convert infrastructure proposal NPC (StagedNpcProposal) to use case ProposedNpc
    fn proposal_npc_to_proposed_npc(npc: &StagedNpcProposal) -> ProposedNpc {
        ProposedNpc {
            character_id: npc.character_id.clone(),
            name: npc.name.clone(),
            sprite_asset: npc.sprite_asset.clone(),
            portrait_asset: npc.portrait_asset.clone(),
            is_present: npc.is_present,
            is_hidden_from_players: npc.is_hidden_from_players,
            reasoning: npc.reasoning.clone(),
        }
    }

    /// Convert infrastructure WaitingPc to use case WaitingPcInfo
    fn waiting_pc_to_info(pc: &WaitingPc) -> WaitingPcInfo {
        WaitingPcInfo {
            pc_id: wrldbldr_domain::PlayerCharacterId::from_uuid(pc.pc_id),
            pc_name: pc.pc_name.clone(),
            user_id: pc.user_id.clone(),
        }
    }

    /// Convert infrastructure WorldPendingStagingApproval to use case PendingStagingInfo
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
                .map(Self::waiting_pc_to_info)
                .collect(),
            rule_based_npcs: approval
                .proposal
                .rule_based_npcs
                .iter()
                .map(Self::proposal_npc_to_proposed_npc)
                .collect(),
            llm_based_npcs: approval
                .proposal
                .llm_based_npcs
                .iter()
                .map(Self::proposal_npc_to_proposed_npc)
                .collect(),
        }
    }
}

#[async_trait::async_trait]
impl StagingStatePort for StagingStateAdapter {
    fn get_game_time(&self, world_id: &WorldId) -> Option<GameTime> {
        self.world_state.get_game_time(world_id)
    }

    fn has_pending_staging(&self, world_id: &WorldId, region_id: &RegionId) -> bool {
        self.world_state
            .get_pending_staging_for_region(world_id, region_id)
            .is_some()
    }

    fn add_waiting_pc(
        &self,
        world_id: &WorldId,
        region_id: &RegionId,
        pc_id: Uuid,
        pc_name: String,
        user_id: String,
        client_id: String,
    ) {
        self.world_state
            .add_waiting_pc_to_staging(world_id, region_id, pc_id, pc_name, user_id, client_id);
    }

    fn store_pending_staging(&self, pending: PendingStagingData) {
        // Convert PendingStagingData to StagingProposal
        // Note: StagingProposal uses string IDs, not typed IDs
        let proposal = StagingProposal {
            request_id: pending.request_id.clone(),
            region_id: pending.region_id.to_string(),
            location_id: pending.location_id.to_string(),
            world_id: pending.world_id.to_string(),
            rule_based_npcs: pending
                .rule_based_npcs
                .iter()
                .map(Self::staged_npc_data_to_proposal_npc)
                .collect(),
            llm_based_npcs: pending
                .llm_based_npcs
                .iter()
                .map(Self::staged_npc_data_to_proposal_npc)
                .collect(),
            default_ttl_hours: pending.default_ttl_hours,
            context: StagingContext::new("", "", "", "", ""), // Empty context
        };

        let mut approval = WorldPendingStagingApproval::new(
            pending.request_id,
            pending.region_id,
            pending.location_id,
            pending.world_id,
            pending.region_name,
            pending.location_name,
            proposal,
        );

        // Add waiting PCs
        for pc in &pending.waiting_pcs {
            approval.add_waiting_pc(
                *pc.pc_id.as_uuid(),
                pc.pc_name.clone(),
                pc.user_id.clone(),
                String::new(), // client_id not in WaitingPcData
            );
        }

        self.world_state
            .add_pending_staging(&pending.world_id, approval);
    }
}

#[async_trait::async_trait]
impl StagingStateExtPort for StagingStateAdapter {
    fn get_pending_staging(
        &self,
        world_id: &WorldId,
        request_id: &str,
    ) -> Option<PendingStagingInfo> {
        self.world_state
            .get_pending_staging_by_request_id(world_id, request_id)
            .map(|approval| Self::approval_to_info(&approval))
    }

    fn remove_pending_staging(&self, world_id: &WorldId, request_id: &str) {
        self.world_state
            .remove_pending_staging(world_id, request_id);
    }

    fn update_llm_suggestions(
        &self,
        world_id: &WorldId,
        request_id: &str,
        npcs: Vec<RegeneratedNpc>,
    ) {
        // Use the mutable accessor to update the LLM suggestions
        // First, find the region ID for this request
        if let Some(pending) = self
            .world_state
            .get_pending_staging_by_request_id(world_id, request_id)
        {
            self.world_state.with_pending_staging_for_region_mut(
                world_id,
                &pending.region_id,
                |approval| {
                    // Update the LLM-based NPCs in the proposal
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waiting_pc_conversion() {
        let waiting_pc = WaitingPc {
            pc_id: Uuid::new_v4(),
            pc_name: "Test PC".to_string(),
            user_id: "user123".to_string(),
            client_id: "client456".to_string(),
        };

        let info = StagingStateAdapter::waiting_pc_to_info(&waiting_pc);

        assert_eq!(info.pc_name, "Test PC");
        assert_eq!(info.user_id, "user123");
    }
}
