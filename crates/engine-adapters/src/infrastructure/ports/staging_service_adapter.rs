//! Staging Service Adapter
//!
//! Implements `StagingServicePort` and `StagingServiceExtPort` by wrapping `StagingService`.
//! This adapter bridges the use case layer's abstract port interface with the
//! application's staging service.

use std::sync::Arc;

use wrldbldr_domain::entities::{StagedNpc, StagingSource};
use wrldbldr_domain::{GameTime, LocationId, RegionId, WorldId};
use wrldbldr_engine_app::application::services::staging_service::{
    ApprovedNpcData as ServiceApprovedNpcData, StagingService, StagedNpcProposal,
};
use wrldbldr_engine_app::application::use_cases::{
    ApprovedNpcData, RegeneratedNpc, StagingProposalData, StagingServiceExtPort, StagingServicePort,
};
use wrldbldr_engine_ports::outbound::{
    LlmPort, NarrativeEventRepositoryPort, RegionRepositoryPort, StagedNpcData, StagingRepositoryPort,
};

/// Adapter that implements staging service ports using StagingService
///
/// This is generic over the concrete service types to match StagingService's generics.
pub struct StagingServiceAdapter<L, R, N, S>
where
    L: LlmPort,
    R: RegionRepositoryPort,
    N: NarrativeEventRepositoryPort,
    S: StagingRepositoryPort,
{
    staging_service: Arc<StagingService<L, R, N, S>>,
}

impl<L, R, N, S> StagingServiceAdapter<L, R, N, S>
where
    L: LlmPort,
    R: RegionRepositoryPort,
    N: NarrativeEventRepositoryPort,
    S: StagingRepositoryPort,
{
    /// Create a new adapter wrapping the given StagingService
    pub fn new(staging_service: Arc<StagingService<L, R, N, S>>) -> Self {
        Self { staging_service }
    }

    /// Convert StagedNpcProposal to StagedNpcData
    fn proposal_to_staged_npc_data(proposal: &StagedNpcProposal) -> StagedNpcData {
        let character_id = uuid::Uuid::parse_str(&proposal.character_id)
            .map(wrldbldr_domain::CharacterId::from_uuid)
            .unwrap_or_else(|_| wrldbldr_domain::CharacterId::from_uuid(uuid::Uuid::nil()));

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

    /// Convert StagedNpcProposal to RegeneratedNpc
    fn proposal_to_regenerated_npc(proposal: &StagedNpcProposal) -> RegeneratedNpc {
        RegeneratedNpc {
            character_id: proposal.character_id.clone(),
            name: proposal.name.clone(),
            sprite_asset: proposal.sprite_asset.clone(),
            portrait_asset: proposal.portrait_asset.clone(),
            is_present: proposal.is_present,
            is_hidden_from_players: proposal.is_hidden_from_players,
            reasoning: proposal.reasoning.clone(),
        }
    }

    /// Convert use case ApprovedNpcData to service ApprovedNpcData
    fn convert_approved_npc(npc: &ApprovedNpcData) -> ServiceApprovedNpcData {
        ServiceApprovedNpcData {
            character_id: npc.character_id,
            name: npc.name.clone(),
            sprite_asset: npc.sprite_asset.clone(),
            portrait_asset: npc.portrait_asset.clone(),
            is_present: npc.is_present,
            is_hidden_from_players: npc.is_hidden_from_players,
            reasoning: npc.reasoning.clone(),
        }
    }
}

#[async_trait::async_trait]
impl<L, R, N, S> StagingServicePort for StagingServiceAdapter<L, R, N, S>
where
    L: LlmPort + Send + Sync,
    R: RegionRepositoryPort + Send + Sync,
    N: NarrativeEventRepositoryPort + Send + Sync,
    S: StagingRepositoryPort + Send + Sync,
{
    async fn get_current_staging(
        &self,
        region_id: RegionId,
        game_time: &GameTime,
    ) -> Result<Option<Vec<StagedNpc>>, String> {
        match self.staging_service.get_current_staging(region_id, game_time).await {
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
                location_name,
                game_time,
                ttl_hours,
                dm_guidance,
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
impl<L, R, N, S> StagingServiceExtPort for StagingServiceAdapter<L, R, N, S>
where
    L: LlmPort + Send + Sync,
    R: RegionRepositoryPort + Send + Sync,
    N: NarrativeEventRepositoryPort + Send + Sync,
    S: StagingRepositoryPort + Send + Sync,
{
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
        let service_npcs: Vec<ServiceApprovedNpcData> = approved_npcs
            .iter()
            .map(Self::convert_approved_npc)
            .collect();

        let staging = self
            .staging_service
            .approve_staging(
                region_id,
                location_id,
                world_id,
                game_time,
                service_npcs,
                ttl_hours,
                source,
                approved_by,
                None, // dm_guidance not used in this flow
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(staging.npcs)
    }

    async fn regenerate_suggestions(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        location_name: &str,
        game_time: &GameTime,
        guidance: &str,
    ) -> Result<Vec<RegeneratedNpc>, String> {
        let proposals = self
            .staging_service
            .regenerate_suggestions(world_id, region_id, location_name, game_time, guidance)
            .await
            .map_err(|e| e.to_string())?;

        Ok(proposals
            .iter()
            .map(Self::proposal_to_regenerated_npc)
            .collect())
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
        let service_npcs: Vec<ServiceApprovedNpcData> =
            npcs.iter().map(Self::convert_approved_npc).collect();

        let staging = self
            .staging_service
            .pre_stage_region(
                region_id,
                location_id,
                world_id,
                game_time,
                service_npcs,
                ttl_hours,
                dm_user_id,
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

        let data = StagingServiceAdapter::<
            wrldbldr_engine_adapters_test_stubs::StubLlm,
            wrldbldr_engine_adapters_test_stubs::StubRegionRepo,
            wrldbldr_engine_adapters_test_stubs::StubNarrativeRepo,
            wrldbldr_engine_adapters_test_stubs::StubStagingRepo,
        >::proposal_to_staged_npc_data(&proposal);

        assert_eq!(data.name, "Test NPC");
        assert!(data.is_present);
    }
}

// Test stubs module for unit tests
#[cfg(test)]
mod wrldbldr_engine_adapters_test_stubs {
    use async_trait::async_trait;
    use wrldbldr_engine_ports::outbound::*;
    use wrldbldr_domain::*;

    pub struct StubLlm;
    #[async_trait]
    impl LlmPort for StubLlm {
        async fn generate(&self, _: LlmRequest) -> Result<LlmResponse, String> {
            Ok(LlmResponse { content: "[]".to_string() })
        }
        async fn generate_streaming(&self, _: LlmRequest) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<String, String>> + Send>>, String> {
            unimplemented!()
        }
    }

    pub struct StubRegionRepo;
    #[async_trait]
    impl RegionRepositoryPort for StubRegionRepo {
        async fn get(&self, _: RegionId) -> Result<Option<wrldbldr_domain::entities::Region>, String> { Ok(None) }
        async fn list_by_location(&self, _: LocationId) -> Result<Vec<wrldbldr_domain::entities::Region>, String> { Ok(vec![]) }
        async fn save(&self, _: &wrldbldr_domain::entities::Region) -> Result<RegionId, String> { Ok(RegionId::from_uuid(uuid::Uuid::new_v4())) }
        async fn update(&self, _: &wrldbldr_domain::entities::Region) -> Result<(), String> { Ok(()) }
        async fn delete(&self, _: RegionId) -> Result<(), String> { Ok(()) }
        async fn get_connections(&self, _: RegionId) -> Result<Vec<wrldbldr_domain::entities::RegionConnection>, String> { Ok(vec![]) }
        async fn add_connection(&self, _: RegionId, _: RegionId, _: Option<String>) -> Result<(), String> { Ok(()) }
        async fn remove_connection(&self, _: RegionId, _: RegionId) -> Result<(), String> { Ok(()) }
        async fn update_connection(&self, _: RegionId, _: RegionId, _: bool, _: Option<String>) -> Result<(), String> { Ok(()) }
        async fn get_exits(&self, _: RegionId) -> Result<Vec<wrldbldr_domain::entities::LocationExit>, String> { Ok(vec![]) }
        async fn add_exit(&self, _: RegionId, _: LocationId, _: RegionId, _: Option<String>) -> Result<(), String> { Ok(()) }
        async fn remove_exit(&self, _: RegionId, _: LocationId) -> Result<(), String> { Ok(()) }
        async fn get_region_items(&self, _: RegionId) -> Result<Vec<wrldbldr_domain::entities::Item>, String> { Ok(vec![]) }
        async fn add_item_to_region(&self, _: RegionId, _: ItemId) -> Result<(), String> { Ok(()) }
        async fn remove_item_from_region(&self, _: RegionId, _: ItemId) -> Result<(), String> { Ok(()) }
    }

    pub struct StubNarrativeRepo;
    #[async_trait]
    impl NarrativeEventRepositoryPort for StubNarrativeRepo {
        async fn get(&self, _: wrldbldr_domain::NarrativeEventId) -> Result<Option<wrldbldr_domain::entities::NarrativeEvent>, String> { Ok(None) }
        async fn list_by_world(&self, _: WorldId) -> Result<Vec<wrldbldr_domain::entities::NarrativeEvent>, String> { Ok(vec![]) }
        async fn list_by_scene(&self, _: SceneId) -> Result<Vec<wrldbldr_domain::entities::NarrativeEvent>, String> { Ok(vec![]) }
        async fn list_active(&self, _: WorldId) -> Result<Vec<wrldbldr_domain::entities::NarrativeEvent>, String> { Ok(vec![]) }
        async fn list_active_for_region(&self, _: RegionId) -> Result<Vec<wrldbldr_domain::entities::NarrativeEvent>, String> { Ok(vec![]) }
        async fn list_triggered_for_pc(&self, _: PlayerCharacterId) -> Result<Vec<wrldbldr_domain::entities::NarrativeEvent>, String> { Ok(vec![]) }
        async fn save(&self, _: &wrldbldr_domain::entities::NarrativeEvent) -> Result<wrldbldr_domain::NarrativeEventId, String> { Ok(wrldbldr_domain::NarrativeEventId::from_uuid(uuid::Uuid::new_v4())) }
        async fn update(&self, _: &wrldbldr_domain::entities::NarrativeEvent) -> Result<(), String> { Ok(()) }
        async fn delete(&self, _: wrldbldr_domain::NarrativeEventId) -> Result<(), String> { Ok(()) }
    }

    pub struct StubStagingRepo;
    #[async_trait]
    impl StagingRepositoryPort for StubStagingRepo {
        async fn save(&self, _: &wrldbldr_domain::entities::Staging) -> Result<wrldbldr_domain::StagingId, String> { Ok(wrldbldr_domain::StagingId::from_uuid(uuid::Uuid::new_v4())) }
        async fn get(&self, _: wrldbldr_domain::StagingId) -> Result<Option<wrldbldr_domain::entities::Staging>, String> { Ok(None) }
        async fn get_current(&self, _: RegionId) -> Result<Option<wrldbldr_domain::entities::Staging>, String> { Ok(None) }
        async fn get_history(&self, _: RegionId, _: u32) -> Result<Vec<wrldbldr_domain::entities::Staging>, String> { Ok(vec![]) }
        async fn set_current(&self, _: wrldbldr_domain::StagingId) -> Result<(), String> { Ok(()) }
        async fn invalidate_all(&self, _: RegionId) -> Result<(), String> { Ok(()) }
    }
}
