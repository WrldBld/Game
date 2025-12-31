//! Staging Service Adapter
//!
//! Implements `StagingServicePort` and `StagingServiceExtPort` (inbound) by wrapping
//! `StagingServicePort` (outbound).
//!
//! This adapter bridges the use case layer's abstract port interface with the
//! application's staging service port.

use std::sync::Arc;

use wrldbldr_domain::entities::{StagedNpc, StagingSource};
use wrldbldr_domain::{CharacterId, GameTime, LocationId, RegionId, WorldId};
use wrldbldr_engine_ports::inbound::{
    RegeneratedNpc, StagingProposalData, StagingServiceExtPort,
    StagingServicePort as InboundStagingServicePort,
};
use wrldbldr_engine_ports::outbound::{
    ApprovedNpc, ApprovedNpcData, StagedNpcData, StagedNpcProposal,
    StagingServicePort as OutboundStagingServicePort,
};

/// Adapter that implements staging service ports (inbound) using StagingServicePort (outbound)
pub struct StagingServiceAdapter {
    staging_service: Arc<dyn OutboundStagingServicePort>,
}

impl StagingServiceAdapter {
    /// Create a new adapter wrapping the given StagingServicePort
    pub fn new(staging_service: Arc<dyn OutboundStagingServicePort>) -> Self {
        Self { staging_service }
    }

    /// Convert StagedNpcProposal to StagedNpcData
    fn proposal_to_staged_npc_data(proposal: &StagedNpcProposal) -> StagedNpcData {
        let character_id = uuid::Uuid::parse_str(&proposal.character_id)
            .map(CharacterId::from_uuid)
            .unwrap_or_else(|_| CharacterId::from_uuid(uuid::Uuid::nil()));

        StagedNpcData {
            character_id,
            name: proposal.name.clone(),
            sprite_asset: proposal.sprite_asset.clone(),
            portrait_asset: proposal.portrait_asset.clone(),
            is_present: proposal.is_present,
            is_hidden_from_players: proposal.is_hidden_from_players,
            reasoning: proposal.reasoning.clone(),
        }
    }

    /// Convert ApprovedNpcData to ApprovedNpc
    fn approved_npc_data_to_approved_npc(data: &ApprovedNpcData) -> ApprovedNpc {
        ApprovedNpc {
            character_id: data.character_id,
            name: data.name.clone(),
            sprite_asset: data.sprite_asset.clone(),
            portrait_asset: data.portrait_asset.clone(),
            is_present: data.is_present,
            is_hidden_from_players: data.is_hidden_from_players,
            reasoning: data.reasoning.clone(),
        }
    }
}

#[async_trait::async_trait]
impl InboundStagingServicePort for StagingServiceAdapter {
    async fn get_current_staging(
        &self,
        region_id: RegionId,
        game_time: &GameTime,
    ) -> Result<Option<Vec<StagedNpc>>, String> {
        match self
            .staging_service
            .get_current_staging(region_id, game_time.clone())
            .await
        {
            Ok(Some(staging)) => Ok(Some(staging.npcs)),
            Ok(None) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    async fn generate_proposal(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        location_id: LocationId,
        location_name: &str,
        game_time: &GameTime,
        ttl_hours: i32,
        dm_guidance: Option<&str>,
    ) -> Result<StagingProposalData, String> {
        let proposal = self
            .staging_service
            .generate_proposal(
                world_id,
                region_id,
                location_id,
                location_name.to_string(),
                game_time.clone(),
                ttl_hours,
                dm_guidance.map(|s| s.to_string()),
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(StagingProposalData {
            request_id: proposal.request_id,
            rule_based_npcs: proposal
                .rule_based_npcs
                .iter()
                .map(Self::proposal_to_staged_npc_data)
                .collect(),
            llm_based_npcs: proposal
                .llm_based_npcs
                .iter()
                .map(Self::proposal_to_staged_npc_data)
                .collect(),
        })
    }
}

#[async_trait::async_trait]
impl StagingServiceExtPort for StagingServiceAdapter {
    async fn approve_staging(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        game_time: &GameTime,
        approved_npcs: Vec<ApprovedNpcData>,
        ttl_hours: i32,
        source: StagingSource,
        approved_by: &str,
    ) -> Result<Vec<StagedNpc>, String> {
        // Convert ApprovedNpcData to ApprovedNpc
        let approved_npcs: Vec<ApprovedNpc> = approved_npcs
            .iter()
            .map(Self::approved_npc_data_to_approved_npc)
            .collect();

        let staging = self
            .staging_service
            .approve_staging(
                region_id,
                location_id,
                world_id,
                game_time.clone(),
                approved_npcs,
                ttl_hours,
                source,
                approved_by.to_string(),
                None, // dm_guidance not used in this flow
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(staging.npcs)
    }

    async fn regenerate_suggestions(
        &self,
        _world_id: WorldId,
        _region_id: RegionId,
        _location_name: &str,
        _game_time: &GameTime,
        _guidance: &str,
    ) -> Result<Vec<RegeneratedNpc>, String> {
        // The outbound StagingServicePort doesn't expose regenerate_suggestions
        // This would need a port extension
        Err("regenerate_suggestions not supported by StagingServicePort".to_string())
    }

    async fn pre_stage_region(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        game_time: &GameTime,
        npcs: Vec<ApprovedNpcData>,
        ttl_hours: i32,
        dm_user_id: &str,
    ) -> Result<Vec<StagedNpc>, String> {
        // Pre-stage uses approve_staging with StagingSource::DmManual
        let approved_npcs: Vec<ApprovedNpc> = npcs
            .iter()
            .map(Self::approved_npc_data_to_approved_npc)
            .collect();

        let staging = self
            .staging_service
            .approve_staging(
                region_id,
                location_id,
                world_id,
                game_time.clone(),
                approved_npcs,
                ttl_hours,
                StagingSource::DmCustomized,
                dm_user_id.to_string(),
                None,
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(staging.npcs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_proposal_to_staged_npc_data() {
        let proposal = StagedNpcProposal {
            character_id: Uuid::new_v4().to_string(),
            name: "Test NPC".to_string(),
            sprite_asset: Some("sprite.png".to_string()),
            portrait_asset: None,
            is_present: true,
            is_hidden_from_players: false,
            reasoning: "Test reasoning".to_string(),
        };

        let data = StagingServiceAdapter::proposal_to_staged_npc_data(&proposal);

        assert_eq!(data.name, "Test NPC");
        assert!(data.is_present);
    }
}
