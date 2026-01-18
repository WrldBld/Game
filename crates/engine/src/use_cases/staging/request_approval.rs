//! Request staging approval use case.

use std::sync::Arc;

use uuid::Uuid;
use wrldbldr_domain::{
    LocationId, PlayerCharacter, PlayerCharacterId, RegionId, Staging as DomainStaging, WorldId,
};

use crate::infrastructure::ports::{
    CharacterRepo, ClockPort, FlagRepo, LlmPort, LocationRepo, PendingStagingRequest, SettingsRepo,
    StagingRepo, TimeSuggestion, WorldRepo,
};
use crate::stores::{PendingStagingStore as PendingStaging, TimeSuggestionStore};
use crate::use_cases::visual_state::{ResolveVisualState, StateResolutionContext};

use super::suggestions::{generate_llm_based_suggestions, generate_rule_based_suggestions};
use super::types::{
    GameTimeData, PreviousStagingData, ResolvedStateInfo, ResolvedVisualState, StagedNpc,
    StagingApprovalData, StagingPendingData, StagingRequestResult, StateOption, WaitingPc,
};
use super::{get_settings_with_fallback, StagingError, DEFAULT_STAGING_TIMEOUT_SECONDS};

/// IO dependencies for staging requests (WS-state owned).
pub struct StagingApprovalContext<'a> {
    pub pending_time_suggestions: &'a TimeSuggestionStore,
    pub pending_staging_requests: &'a PendingStaging,
}

/// Request input for staging approval.
pub struct StagingApprovalInput {
    pub world_id: WorldId,
    pub region: wrldbldr_domain::Region,
    pub pc: PlayerCharacter,
    pub previous_staging: Option<DomainStaging>,
    pub time_suggestion: Option<TimeSuggestion>,
    pub guidance: Option<String>,
}

/// Use case for building and broadcasting a staging approval request.
pub struct RequestStagingApproval {
    character: Arc<dyn CharacterRepo>,
    staging: Arc<dyn StagingRepo>,
    location: Arc<dyn LocationRepo>,
    world: Arc<dyn WorldRepo>,
    flag: Arc<dyn FlagRepo>,
    visual_state: Arc<ResolveVisualState>,
    settings: Arc<dyn SettingsRepo>,
    llm: Arc<dyn LlmPort>,
    clock: Arc<dyn ClockPort>,
}

impl RequestStagingApproval {
    pub fn new(
        character: Arc<dyn CharacterRepo>,
        staging: Arc<dyn StagingRepo>,
        location: Arc<dyn LocationRepo>,
        world: Arc<dyn WorldRepo>,
        flag: Arc<dyn FlagRepo>,
        visual_state: Arc<ResolveVisualState>,
        settings: Arc<dyn SettingsRepo>,
        llm: Arc<dyn LlmPort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            character,
            staging,
            location,
            world,
            flag,
            visual_state,
            settings,
            llm,
            clock,
        }
    }

    pub async fn execute(
        &self,
        ctx: &StagingApprovalContext<'_>,
        input: StagingApprovalInput,
    ) -> Result<StagingRequestResult, StagingError> {
        let request_id = Uuid::new_v4().to_string();

        ctx.pending_staging_requests
            .insert(
                request_id.clone(),
                PendingStagingRequest {
                    region_id: input.region.id(),
                    location_id: input.region.location_id(),
                    world_id: input.world_id,
                    created_at: self.clock.now(),
                },
            )
            .await;

        let world = self
            .world
            .get(input.world_id)
            .await?
            .ok_or(StagingError::WorldNotFound(input.world_id))?;
        let game_time = world.game_time();

        let settings =
            get_settings_with_fallback(self.settings.as_ref(), input.world_id, "staging").await;

        let location_name = match self.location.get_location(input.region.location_id()).await {
            Ok(Some(l)) => l.name().to_string(),
            Ok(None) => {
                tracing::warn!(location_id = %input.region.location_id(), "Location not found for staging request");
                "Unknown Location".to_string()
            }
            Err(e) => {
                tracing::warn!(location_id = %input.region.location_id(), error = %e, "Failed to fetch location for staging request");
                "Unknown Location".to_string()
            }
        };

        // Issue 4.1 fix: Fetch NPCs once and pass to both suggestion functions
        // Fail fast if we can't fetch NPCs - staging approval requires NPC list
        let npcs_for_region = self
            .character
            .get_npcs_for_region(input.region.id())
            .await?;

        let rule_based_npcs = generate_rule_based_suggestions(
            &npcs_for_region,
            self.staging.as_ref(),
            input.region.id(),
        )
        .await;
        let llm_based_npcs = generate_llm_based_suggestions(
            &npcs_for_region,
            self.llm.as_ref(),
            input.region.name().as_str(),
            &location_name,
            input.guidance.as_deref(),
        )
        .await;

        let (resolved_visual_state, available_location_states, available_region_states) = self
            .resolve_visual_states(
                input.world_id,
                input.region.location_id(),
                input.region.id(),
                input.pc.id(),
            )
            .await;

        // Convert previous staging to domain type
        let previous_staging = input.previous_staging.map(|s| PreviousStagingData {
            staging_id: s.id().into(),
            approved_at: s.approved_at(),
            npcs: s
                .npcs()
                .iter()
                .map(|n| StagedNpc {
                    character_id: n.character_id,
                    name: n.name.clone(),
                    sprite_asset: n.sprite_asset.as_ref().map(|a| a.to_string()),
                    portrait_asset: n.portrait_asset.as_ref().map(|a| a.to_string()),
                    is_present: n.is_present,
                    reasoning: n.reasoning.clone(),
                    is_hidden_from_players: n.is_hidden_from_players,
                    mood: Some(n.mood.to_string()),
                })
                .collect(),
        });

        // Store time suggestion if present
        if let Some(ref time_suggestion) = input.time_suggestion {
            ctx.pending_time_suggestions
                .insert(time_suggestion.id, time_suggestion.clone())
                .await;
        }

        // Build domain result - API layer will convert to protocol and notify DMs
        Ok(StagingRequestResult {
            pending: StagingPendingData {
                region_id: input.region.id(),
                region_name: input.region.name().to_string(),
                timeout_seconds: DEFAULT_STAGING_TIMEOUT_SECONDS,
            },
            approval: StagingApprovalData {
                request_id,
                region_id: input.region.id(),
                region_name: input.region.name().to_string(),
                location_id: input.region.location_id(),
                location_name,
                game_time: GameTimeData {
                    day: game_time.day(),
                    hour: game_time.hour(),
                    minute: game_time.minute(),
                    is_paused: game_time.is_paused(),
                },
                previous_staging,
                rule_based_npcs,
                llm_based_npcs,
                default_ttl_hours: settings.default_presence_cache_ttl_hours(),
                waiting_pcs: vec![WaitingPc {
                    pc_id: input.pc.id(),
                    pc_name: input.pc.name().to_string(),
                    player_id: input.pc.user_id().to_string(),
                }],
                resolved_visual_state,
                available_location_states,
                available_region_states,
            },
            time_suggestion: input.time_suggestion,
        })
    }

    async fn resolve_visual_states(
        &self,
        world_id: WorldId,
        location_id: LocationId,
        region_id: RegionId,
        pc_id: PlayerCharacterId,
    ) -> (
        Option<ResolvedVisualState>,
        Vec<StateOption>,
        Vec<StateOption>,
    ) {
        let game_time = match self.world.get(world_id).await {
            Ok(Some(w)) => w.game_time().clone(),
            _ => return (None, vec![], vec![]),
        };

        let world_flags = match self.flag.get_world_flags(world_id).await {
            Ok(flags) => flags,
            Err(e) => {
                tracing::warn!(world_id = %world_id, error = %e, "Failed to fetch world flags for visual state, using empty flags");
                vec![]
            }
        };

        let pc_flags = match self.flag.get_pc_flags(pc_id).await {
            Ok(flags) => flags,
            Err(e) => {
                tracing::warn!(pc_id = %pc_id, error = %e, "Failed to fetch PC flags for visual state, using empty flags");
                vec![]
            }
        };

        let context = StateResolutionContext::new(world_id, game_time)
            .with_world_flags(world_flags)
            .with_pc_flags(pc_flags);

        let resolution = match self
            .visual_state
            .execute(location_id, region_id, &context)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to resolve visual states");
                return (None, vec![], vec![]);
            }
        };

        let resolved = if resolution.is_complete {
            Some(ResolvedVisualState {
                location_state: resolution
                    .location_state
                    .as_ref()
                    .map(|s| ResolvedStateInfo {
                        id: s.id.clone(),
                        name: s.name.clone(),
                        backdrop_override: s.backdrop_override.clone(),
                        atmosphere_override: s.atmosphere_override.clone(),
                        ambient_sound: s.ambient_sound.clone(),
                    }),
                region_state: resolution.region_state.as_ref().map(|s| ResolvedStateInfo {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    backdrop_override: s.backdrop_override.clone(),
                    atmosphere_override: s.atmosphere_override.clone(),
                    ambient_sound: s.ambient_sound.clone(),
                }),
            })
        } else {
            None
        };

        let available_location = resolution
            .available_location_states
            .iter()
            .map(|s| {
                let match_reason = if s.evaluation.is_active {
                    Some(s.evaluation.matched_rules.join(", "))
                } else {
                    None
                };
                StateOption {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    priority: s.priority,
                    is_default: s.is_default,
                    match_reason,
                }
            })
            .collect();

        let available_region = resolution
            .available_region_states
            .iter()
            .map(|s| {
                let match_reason = if s.evaluation.is_active {
                    Some(s.evaluation.matched_rules.join(", "))
                } else {
                    None
                };
                StateOption {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    priority: s.priority,
                    is_default: s.is_default,
                    match_reason,
                }
            })
            .collect();

        (resolved, available_location, available_region)
    }
}
