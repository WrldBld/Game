//! Request staging approval use case.

use std::sync::Arc;

use uuid::Uuid;
use wrldbldr_domain::{
    LocationId, PlayerCharacter, PlayerCharacterId, RegionId, Staging as DomainStaging, WorldId,
};

use crate::infrastructure::ports::{
    CharacterRepo, ClockPort, FlagRepo, LlmPort, LocationRepo, PendingStagingRequest, PromptTemplateRepo, SettingsRepo,
    StagingRepo, TimeSuggestion, WorldRepo,
};
use crate::stores::{PendingStagingStore as PendingStaging, TimeSuggestionStore};
use crate::use_cases::visual_state::{ResolveVisualState, StateResolutionContext};

use super::suggestions::{generate_llm_based_suggestions, generate_rule_based_suggestions};
use super::types::{
    GameTimeData, PreviousStagingData, ResolvedStateInfo, ResolvedVisualState, StagedNpc,
    StagingApprovalData, StagingPendingData, StagingRequestResult, StateOption, WaitingPc,
};
use super::{get_settings_with_fallback, StagingError};

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
    /// Optional presence cache TTL override (None = use location or settings default)
    pub presence_cache_ttl_hours: Option<i32>,
    /// Optional LLM presence flag override (None = use location or settings default)
    pub use_llm_presence: Option<bool>,
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
    prompt_templates: Arc<dyn PromptTemplateRepo>,
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
        prompt_templates: Arc<dyn PromptTemplateRepo>,
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
            prompt_templates,
        }
    }

    pub async fn execute(
        &self,
        ctx: &StagingApprovalContext<'_>,
        input: StagingApprovalInput,
    ) -> Result<StagingRequestResult, StagingError> {
        let request_id = Uuid::new_v4().to_string();

        let world = self
            .world
            .get(input.world_id)
            .await?
            .ok_or(StagingError::WorldNotFound(input.world_id))?;
        let game_time = world.game_time();

        let settings =
            get_settings_with_fallback(self.settings.as_ref(), input.world_id, "staging").await;

        let timeout_seconds = settings.staging_timeout_seconds();

        // Fetch location to get staging defaults
        let location = self
            .location
            .get_location(input.region.location_id())
            .await;

        let (location_name, effective_presence_ttl_hours, effective_use_llm_presence) = match location {
            Ok(Some(l)) => {
                // Merge input overrides with location defaults (input takes precedence)
                let effective_ttl = input
                    .presence_cache_ttl_hours
                    .unwrap_or_else(|| l.presence_cache_ttl_hours());
                let effective_llm = input
                    .use_llm_presence
                    .unwrap_or_else(|| l.use_llm_presence());
                (
                    l.name().to_string(),
                    effective_ttl,
                    effective_llm,
                )
            }
            Ok(None) => {
                tracing::warn!(location_id = %input.region.location_id(), "Location not found for staging request, using settings defaults");
                let effective_ttl = input
                    .presence_cache_ttl_hours
                    .unwrap_or_else(|| settings.default_presence_cache_ttl_hours());
                let effective_llm = input
                    .use_llm_presence
                    .unwrap_or_else(|| settings.default_use_llm_presence());
                (
                    "Unknown Location".to_string(),
                    effective_ttl,
                    effective_llm,
                )
            }
            Err(e) => {
                tracing::warn!(location_id = %input.region.location_id(), error = %e, "Failed to fetch location for staging request, using settings defaults");
                let effective_ttl = input
                    .presence_cache_ttl_hours
                    .unwrap_or_else(|| settings.default_presence_cache_ttl_hours());
                let effective_llm = input
                    .use_llm_presence
                    .unwrap_or_else(|| settings.default_use_llm_presence());
                (
                    "Unknown Location".to_string(),
                    effective_ttl,
                    effective_llm,
                )
            }
        };

        // Validate effective TTL - must be positive
        if effective_presence_ttl_hours <= 0 {
            return Err(StagingError::Validation(format!(
                "presence_cache_ttl_hours must be positive, got {}",
                effective_presence_ttl_hours
            )));
        }

        // Issue 4.1 fix: Fetch NPCs once and pass to both suggestion functions
        // Fail fast if we can't fetch NPCs - staging approval requires NPC list
        let npcs_for_region = self
            .character
            .get_npcs_for_region(input.region.id())
            .await?;

        // Generate rule-based suggestions (always generated)
        let rule_based_npcs = generate_rule_based_suggestions(
            &npcs_for_region,
            self.staging.as_ref(),
            input.region.id(),
        )
        .await;

        // Resolve temperature from settings (before the if block)
        let temperature = if effective_use_llm_presence {
            let world_temp = self.settings
                .get_for_world(input.world_id)
                .await
                .ok()
                .flatten()
                .map(|s| s.llm_temperature_staging());

            match world_temp {
                Some(t) => t,
                None => {
                    self.settings
                        .get_global()
                        .await
                        .ok()
                        .flatten()
                        .map(|s| s.llm_temperature_staging())
                        .unwrap_or(0.7)
                }
            }
        } else {
            0.7 // Default, won't be used
        };

        // Gate LLM suggestions based on effective use_llm_presence setting
        let llm_based_npcs = if effective_use_llm_presence {
            tracing::info!(
                region_id = %input.region.id(),
                "LLM presence enabled, generating suggestions"
            );

            // Load system prompt from template repository
            let system_prompt = self
                .prompt_templates
                .resolve_template(Some(input.world_id), crate::prompt_templates::keys::SYSTEM_STAGING_SUGGESTIONS)
                .await
                .ok()
                .flatten()
                .unwrap_or_default();

            generate_llm_based_suggestions(
                &npcs_for_region,
                self.llm.as_ref(),
                input.region.name().as_str(),
                &location_name,
                input.guidance.as_deref(),
                temperature,
                &system_prompt,
            )
            .await
        } else {
            tracing::info!(
                region_id = %input.region.id(),
                "LLM presence disabled, skipping LLM suggestions"
            );
            vec![]
        };

        let (resolved_visual_state, available_location_states, available_region_states) = self
            .resolve_visual_states(
                input.world_id,
                input.region.location_id(),
                input.region.id(),
                input.pc.id(),
            )
            .await;

        // Insert pending staging request now that we have the available state IDs
        ctx.pending_staging_requests
            .insert(
                request_id.clone(),
                PendingStagingRequest {
                    region_id: input.region.id(),
                    location_id: input.region.location_id(),
                    world_id: input.world_id,
                    created_at: self.clock.now(),
                    allowed_location_state_ids: available_location_states
                        .iter()
                        .map(|s| s.id.clone())
                        .collect(),
                    allowed_region_state_ids: available_region_states
                        .iter()
                        .map(|s| s.id.clone())
                        .collect(),
                },
            )
            .await;

        // Convert previous staging to domain type
        let previous_staging = input.previous_staging.map(|s| PreviousStagingData {
            staging_id: s.id(),
            approved_at: s.approved_at(),
            npcs: s
                .npcs()
                .iter()
                .map(|n| StagedNpc {
                    character_id: n.character_id,
                    name: n.name.clone(),
                    sprite_asset: n.sprite_asset.as_ref().map(|a| a.to_string()),
                    portrait_asset: n.portrait_asset.as_ref().map(|a| a.to_string()),
                    is_present: n.is_present(),
                    reasoning: n.reasoning.clone(),
                    is_hidden_from_players: n.is_hidden_from_players(),
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
                timeout_seconds,
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
                    second: game_time.second(),
                    is_paused: game_time.is_paused(),
                },
                previous_staging,
                rule_based_npcs,
                llm_based_npcs,
                default_ttl_hours: effective_presence_ttl_hours,
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
                let match_reason = if s.evaluation.is_active() {
                    Some(s.evaluation.matched_rules().join(", "))
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
                let match_reason = if s.evaluation.is_active() {
                    Some(s.evaluation.matched_rules().join(", "))
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

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
        use super::*;
        use crate::infrastructure::app_settings::AppSettings;
        use crate::infrastructure::ports::{LlmPort, LlmRequest, LlmResponse, MockCharacterRepo, MockStagingRepo, MockLocationRepo, MockWorldRepo, MockFlagRepo, MockSettingsRepo, MockLocationStateRepo, MockRegionStateRepo};
        use crate::use_cases::visual_state::ResolveVisualState;
        use crate::stores::PendingStagingStore as PendingStaging;
        use crate::stores::TimeSuggestionStore;
        use std::sync::Arc;
        use wrldbldr_domain::{
            Location, LocationId, LocationName, PlayerCharacterId, WorldId, LocationType,
        };

        // Mock LLM port for testing
        struct MockLlmPort {
            should_fail: bool,
        }

        impl MockLlmPort {
            fn new() -> Self {
                Self { should_fail: false }
            }

            fn failing() -> Self {
                Self { should_fail: true }
            }
        }

        #[async_trait::async_trait]
        impl LlmPort for MockLlmPort {
            async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, crate::infrastructure::ports::LlmError> {
                if self.should_fail {
                    Err(crate::infrastructure::ports::LlmError::RequestFailed("Mock failure".to_string()))
                } else {
                    Ok(LlmResponse {
                        content: r#"[{"name": "Test NPC", "reason": "Test reason"}]"#.to_string(),
                        finish_reason: crate::infrastructure::ports::FinishReason::Stop,
                        usage: None,
                    })
                }
            }
        }

        /// Test that use_llm_presence=false bypasses LLM port calls
        #[tokio::test]
        async fn test_llm_presence_disabled_skips_llm_calls() {
            // Setup mocks
            let llm = Arc::new(MockLlmPort::failing()); // This LLM always fails
            let mut mock_char = MockCharacterRepo::new();
            let mut mock_staging = MockStagingRepo::new();
            let mut mock_location = MockLocationRepo::new();
            let mut mock_world = MockWorldRepo::new();
            let mock_flag = MockFlagRepo::new();
            let mut mock_settings = MockSettingsRepo::new();
            let mut mock_location_state = MockLocationStateRepo::new();
            let mut mock_region_state = MockRegionStateRepo::new();
            let mock_clock = Arc::new(crate::infrastructure::ports::MockClockPort::new());

            // Configure world mock
            mock_world
                .expect_get()
                .returning(|_| Ok(Some(crate::test_fixtures::world_seeder::create_test_world(
                    WorldId::new(),
                    chrono::Utc::now(),
                ))));

            // Configure location mock with TTL=0 (invalid)
            let location_id = LocationId::new();
            mock_location
                .expect_get_location()
                .return_once(move |_| {
                    Ok(Some(
                        Location::new(
                            WorldId::new(),
                            LocationName::new("Test Location").expect("valid location name"),
                            LocationType::Interior,
                        )
                        .with_presence_ttl(0),
                    ))
                });

            // Configure character repo to return empty NPCs
            mock_char
                .expect_get_npcs_for_region()
                .returning(|_| Ok(vec![]));

            // Configure staging repo
            mock_staging
                .expect_get_staged_npcs()
                .returning(|_| Ok(vec![]));

            // Configure settings
            mock_settings
                .expect_get_for_world()
                .returning(|_| Ok(Some(AppSettings::default())));

            // Configure visual state repos to return empty lists
            mock_location_state
                .expect_list_for_location()
                .returning(|_| Ok(vec![]));
            mock_region_state
                .expect_list_for_region()
                .returning(|_| Ok(vec![]));

            // Create mock prompt templates
            let mock_prompt_templates = Arc::new(crate::infrastructure::ports::MockPromptTemplateRepo::new());

            // Create visual state use case with mocks
            let visual_state = Arc::new(ResolveVisualState::new(
                Arc::new(mock_location_state),
                Arc::new(mock_region_state),
            ));

            // Create use case
            let use_case = RequestStagingApproval::new(
                Arc::new(mock_char),
                Arc::new(mock_staging),
                Arc::new(mock_location),
                Arc::new(mock_world),
                Arc::new(mock_flag),
                visual_state,
                Arc::new(mock_settings),
                llm,
                mock_clock,
                mock_prompt_templates,
            );

            // Create context
            let pending_time_suggestions = TimeSuggestionStore::default();
            let pending_staging_requests = PendingStaging::new(Arc::new(crate::api::websocket::PendingStagingStoreImpl::new()));
            let ctx = StagingApprovalContext {
                pending_time_suggestions: &pending_time_suggestions,
                pending_staging_requests: &pending_staging_requests,
            };

            // Create input WITHOUT TTL override (should use location default which is 0)
            let region = crate::test_fixtures::world_seeder::create_test_region(location_id);
            let input = StagingApprovalInput {
                world_id: WorldId::new(),
                region,
                pc: {
                    let pc_id = PlayerCharacterId::new();
                    let world_id = WorldId::new();
                    crate::test_fixtures::world_seeder::create_test_pc(pc_id, world_id, chrono::Utc::now())
                },
                previous_staging: None,
                time_suggestion: None,
                guidance: None,
                presence_cache_ttl_hours: None, // Uses location default of 0 (invalid)
                use_llm_presence: None,
            };

            // Execute - should fail with validation error
            let result = use_case.execute(&ctx, input).await;

            assert!(
                result.is_err(),
                "Should fail when effective TTL is zero"
            );

            let error = result.expect_err("should fail");
            match error {
                StagingError::Validation(msg) => {
                    assert!(
                        msg.contains("must be positive") && msg.contains("0"),
                        "Validation error should mention 'must be positive' and '0', got: {}",
                        msg
                    );
                }
                _ => panic!("Expected Validation error, got: {:?}", error),
            }
        }

        /// Test that invalid TTL (negative) is rejected with validation error
        #[tokio::test]
        async fn test_invalid_ttl_negative_rejected() {
            // Setup mocks
            let llm = Arc::new(MockLlmPort::new());
            let mut mock_char = MockCharacterRepo::new();
            let mut mock_staging = MockStagingRepo::new();
            let mut mock_location = MockLocationRepo::new();
            let mut mock_world = MockWorldRepo::new();
            let mock_flag = MockFlagRepo::new();
            let mut mock_settings = MockSettingsRepo::new();
            let mut mock_location_state = MockLocationStateRepo::new();
            let mut mock_region_state = MockRegionStateRepo::new();
            let mock_clock = Arc::new(crate::infrastructure::ports::MockClockPort::new());

            // Configure world mock
            mock_world
                .expect_get()
                .returning(|_| Ok(Some(crate::test_fixtures::world_seeder::create_test_world(
                    WorldId::new(),
                    chrono::Utc::now(),
                ))));

            // Configure location mock with valid default TTL
            let location_id = LocationId::new();
            mock_location
                .expect_get_location()
                .return_once(move |_| {
                    Ok(Some(
                        Location::new(
                            WorldId::new(),
                            LocationName::new("Test Location").expect("valid location name"),
                            LocationType::Interior,
                        )
                        .with_presence_ttl(5),
                    ))
                });

            // Configure character repo to return empty NPCs
            mock_char
                .expect_get_npcs_for_region()
                .returning(|_| Ok(vec![]));

            // Configure staging repo
            mock_staging
                .expect_get_staged_npcs()
                .returning(|_| Ok(vec![]));

            // Configure settings
            mock_settings
                .expect_get_for_world()
                .returning(|_| Ok(Some(AppSettings::default())));

            // Configure visual state repos to return empty lists
            mock_location_state
                .expect_list_for_location()
                .returning(|_| Ok(vec![]));
            mock_region_state
                .expect_list_for_region()
                .returning(|_| Ok(vec![]));

            // Create mock prompt templates
            let mock_prompt_templates = Arc::new(crate::infrastructure::ports::MockPromptTemplateRepo::new());

            // Create visual state use case with mocks
            let visual_state = Arc::new(ResolveVisualState::new(
                Arc::new(mock_location_state),
                Arc::new(mock_region_state),
            ));

            // Create use case
            let use_case = RequestStagingApproval::new(
                Arc::new(mock_char),
                Arc::new(mock_staging),
                Arc::new(mock_location),
                Arc::new(mock_world),
                Arc::new(mock_flag),
                visual_state,
                Arc::new(mock_settings),
                llm,
                mock_clock,
                mock_prompt_templates,
            );

            // Create context
            let pending_time_suggestions = TimeSuggestionStore::default();
            let pending_staging_requests = PendingStaging::new(Arc::new(crate::api::websocket::PendingStagingStoreImpl::new()));
            let ctx = StagingApprovalContext {
                pending_time_suggestions: &pending_time_suggestions,
                pending_staging_requests: &pending_staging_requests,
            };

            // Create input WITH negative TTL override
            let region = crate::test_fixtures::world_seeder::create_test_region(location_id);
            let input = StagingApprovalInput {
                world_id: WorldId::new(),
                region,
                pc: {
                    let pc_id = PlayerCharacterId::new();
                    let world_id = WorldId::new();
                    crate::test_fixtures::world_seeder::create_test_pc(pc_id, world_id, chrono::Utc::now())
                },
                previous_staging: None,
                time_suggestion: None,
                guidance: None,
                presence_cache_ttl_hours: Some(-5), // Negative TTL (invalid)
                use_llm_presence: None,
            };

            // Execute - should fail with validation error
            let result = use_case.execute(&ctx, input).await;

            assert!(
                result.is_err(),
                "Should fail when effective TTL is negative"
            );

            let error = result.expect_err("should fail");
            match error {
                StagingError::Validation(msg) => {
                    assert!(
                        msg.contains("must be positive") && msg.contains("-5"),
                        "Validation error should mention 'must be positive' and '-5', got: {}",
                        msg
                    );
                }
                _ => panic!("Expected Validation error, got: {:?}", error),
            }
        }
    }
