//! Auto-approve staging timeout use case.

use std::sync::Arc;

use wrldbldr_domain::StagingSource;

use crate::infrastructure::ports::PendingStagingRequest;
use crate::repositories::character::Character;
use crate::repositories::location::Location;
use crate::repositories::staging::Staging;
use crate::repositories::{LocationStateEntity, RegionStateEntity, Settings, World};

use super::approve::{ApproveStagingInput, ApproveStagingRequest, StagingReadyPayload};
use super::suggestions::generate_rule_based_suggestions;
use super::types::ApprovedNpc;
use super::{get_settings_with_fallback, StagingError};

/// Use case for auto-approving expired staging requests.
pub struct AutoApproveStagingTimeout {
    character: Arc<Character>,
    staging: Arc<Staging>,
    world: Arc<World>,
    location: Arc<Location>,
    location_state: Arc<LocationStateEntity>,
    region_state: Arc<RegionStateEntity>,
    settings: Arc<Settings>,
}

impl AutoApproveStagingTimeout {
    pub fn new(
        character: Arc<Character>,
        staging: Arc<Staging>,
        world: Arc<World>,
        location: Arc<Location>,
        location_state: Arc<LocationStateEntity>,
        region_state: Arc<RegionStateEntity>,
        settings: Arc<Settings>,
    ) -> Self {
        Self {
            character,
            staging,
            world,
            location,
            location_state,
            region_state,
            settings,
        }
    }

    /// Auto-approve a single expired staging request with rule-based NPCs.
    pub async fn execute(
        &self,
        request_id: String,
        pending: PendingStagingRequest,
    ) -> Result<StagingReadyPayload, StagingError> {
        let settings =
            get_settings_with_fallback(self.settings.as_ref(), pending.world_id, "auto-approval")
                .await;

        // Fetch NPCs for region once
        let npcs_for_region = self
            .character
            .get_npcs_for_region(pending.region_id)
            .await
            .unwrap_or_default();

        // Generate rule-based NPC suggestions
        let rule_based_npcs =
            generate_rule_based_suggestions(&npcs_for_region, &self.staging, pending.region_id)
                .await;

        // Convert to ApprovedNpc domain type
        let approved_npcs: Vec<ApprovedNpc> = rule_based_npcs
            .into_iter()
            .map(|npc| ApprovedNpc {
                character_id: npc.character_id,
                is_present: npc.is_present,
                reasoning: Some(format!("[Auto-approved] {}", npc.reasoning)),
                is_hidden_from_players: npc.is_hidden_from_players,
                mood: npc.mood,
            })
            .collect();

        let input = ApproveStagingInput {
            region_id: pending.region_id,
            location_id: Some(pending.location_id),
            world_id: pending.world_id,
            approved_by: "system".to_string(),
            ttl_hours: settings.default_presence_cache_ttl_hours,
            source: StagingSource::AutoApproved,
            approved_npcs,
            location_state_id: None,
            region_state_id: None,
        };

        // Delegate to the approve use case
        let approve_use_case = ApproveStagingRequest::new(
            self.staging.clone(),
            self.world.clone(),
            self.character.clone(),
            self.location.clone(),
            self.location_state.clone(),
            self.region_state.clone(),
        );

        let result = approve_use_case.execute(input).await?;

        tracing::info!(
            request_id = %request_id,
            region_id = %pending.region_id,
            world_id = %pending.world_id,
            "Auto-approved staging on timeout"
        );

        Ok(result)
    }
}
