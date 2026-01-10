//! Staging use cases.
//!
//! Handles staging approval requests, regeneration, and approval application.

use std::{collections::HashMap, sync::Arc};

use chrono::{Datelike, Timelike};
use serde::Deserialize;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::api::connections::ConnectionManager;
use crate::entities::{
    Character, Flag, Location, LocationStateEntity, RegionStateEntity, Staging, World,
};
use crate::infrastructure::ports::{
    ChatMessage, LlmPort, LlmRequest, NpcRegionRelationType, RepoError, SettingsRepo,
};
use crate::use_cases::time::TimeSuggestion;
use crate::use_cases::visual_state::{ResolveVisualState, StateResolutionContext};
use wrldbldr_domain::{
    CharacterId, LocationId, PlayerCharacter, RegionId, Staging as DomainStaging, StagingSource,
    WorldId,
};
use wrldbldr_protocol::{
    ApprovedNpcInfo, NpcPresentInfo, PreviousStagingInfo, ServerMessage, StagedNpcInfo,
    WaitingPcInfo,
};

/// Timeout in seconds before a pending staging request auto-approves.
/// This is the delay shown to players while waiting for DM approval.
/// Not to be confused with TTL (time-to-live), which controls how long
/// approved staging remains valid (configured via `default_presence_cache_ttl_hours`).
pub const DEFAULT_STAGING_TIMEOUT_SECONDS: u64 = 30;

/// Container for staging use cases.
pub struct StagingUseCases {
    pub request_approval: Arc<RequestStagingApproval>,
    pub regenerate: Arc<RegenerateStagingSuggestions>,
    pub approve: Arc<ApproveStagingRequest>,
    pub auto_approve_timeout: Arc<AutoApproveStagingTimeout>,
}

impl StagingUseCases {
    pub fn new(
        request_approval: Arc<RequestStagingApproval>,
        regenerate: Arc<RegenerateStagingSuggestions>,
        approve: Arc<ApproveStagingRequest>,
        auto_approve_timeout: Arc<AutoApproveStagingTimeout>,
    ) -> Self {
        Self {
            request_approval,
            regenerate,
            approve,
            auto_approve_timeout,
        }
    }
}

/// Pending staging request tracking (request_id -> region/location).
#[derive(Debug, Clone)]
pub struct PendingStagingRequest {
    pub region_id: RegionId,
    pub location_id: LocationId,
    pub world_id: WorldId,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// IO dependencies for staging requests (WS-state owned).
pub struct StagingApprovalContext<'a> {
    pub connections: &'a ConnectionManager,
    pub pending_time_suggestions: &'a RwLock<HashMap<Uuid, TimeSuggestion>>,
    pub pending_staging_requests: &'a RwLock<HashMap<String, PendingStagingRequest>>,
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
    character: Arc<Character>,
    staging: Arc<Staging>,
    location: Arc<Location>,
    world: Arc<World>,
    flag: Arc<Flag>,
    visual_state: Arc<ResolveVisualState>,
    settings: Arc<dyn SettingsRepo>,
    llm: Arc<dyn LlmPort>,
}

impl RequestStagingApproval {
    pub fn new(
        character: Arc<Character>,
        staging: Arc<Staging>,
        location: Arc<Location>,
        world: Arc<World>,
        flag: Arc<Flag>,
        visual_state: Arc<ResolveVisualState>,
        settings: Arc<dyn SettingsRepo>,
        llm: Arc<dyn LlmPort>,
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
        }
    }

    pub async fn execute(
        &self,
        ctx: &StagingApprovalContext<'_>,
        input: StagingApprovalInput,
    ) -> Result<ServerMessage, StagingError> {
        let request_id = Uuid::new_v4().to_string();

        {
            let mut guard = ctx.pending_staging_requests.write().await;
            guard.insert(
                request_id.clone(),
                PendingStagingRequest {
                    region_id: input.region.id,
                    location_id: input.region.location_id,
                    world_id: input.world_id,
                    created_at: chrono::Utc::now(),
                },
            );
        }

        let world = self
            .world
            .get(input.world_id)
            .await?
            .ok_or(StagingError::WorldNotFound)?;
        let now = world.game_time.current();

        // Fetch world settings for configurable TTL values, falling back to defaults
        // if settings cannot be loaded (to avoid breaking staging on settings fetch failure)
        let settings = match self.settings.get_for_world(input.world_id).await {
            Ok(Some(s)) => s,
            Ok(None) => wrldbldr_domain::AppSettings::default(),
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    world_id = %input.world_id,
                    "Failed to load world settings for staging, using defaults"
                );
                wrldbldr_domain::AppSettings::default()
            }
        };

        let location_name = self
            .location
            .get(input.region.location_id)
            .await
            .ok()
            .flatten()
            .map(|l| l.name)
            .unwrap_or_else(|| "Unknown Location".to_string());

        let rule_based_npcs =
            generate_rule_based_suggestions(&self.character, &self.staging, input.region.id).await;
        let llm_based_npcs = generate_llm_based_suggestions(
            &self.character,
            self.llm.as_ref(),
            input.region.id,
            &input.region.name,
            &location_name,
            input.guidance.as_deref(),
        )
        .await;

        let (resolved_visual_state, available_location_states, available_region_states) = self
            .resolve_visual_states(input.world_id, input.region.location_id, input.region.id)
            .await;

        let previous_staging = input.previous_staging.map(|s| PreviousStagingInfo {
            staging_id: s.id.to_string(),
            approved_at: s.approved_at.to_rfc3339(),
            npcs: s
                .npcs
                .into_iter()
                .map(|n| StagedNpcInfo {
                    character_id: n.character_id.to_string(),
                    name: n.name,
                    sprite_asset: n.sprite_asset,
                    portrait_asset: n.portrait_asset,
                    is_present: n.is_present,
                    reasoning: n.reasoning,
                    is_hidden_from_players: n.is_hidden_from_players,
                    mood: Some(n.mood.to_string()),
                })
                .collect(),
        });

        let approval_msg = ServerMessage::StagingApprovalRequired {
            request_id: request_id.clone(),
            region_id: input.region.id.to_string(),
            region_name: input.region.name.clone(),
            location_id: input.region.location_id.to_string(),
            location_name: location_name.clone(),
            game_time: wrldbldr_protocol::types::GameTime {
                day: now.ordinal() as u32,
                hour: now.hour() as u8,
                minute: now.minute() as u8,
                is_paused: world.game_time.is_paused(),
            },
            previous_staging,
            rule_based_npcs,
            llm_based_npcs,
            default_ttl_hours: settings.default_presence_cache_ttl_hours,
            waiting_pcs: vec![WaitingPcInfo {
                pc_id: input.pc.id.to_string(),
                pc_name: input.pc.name.clone(),
                player_id: input.pc.user_id.clone(),
            }],
            resolved_visual_state,
            available_location_states,
            available_region_states,
        };

        ctx.connections
            .broadcast_to_dms(input.world_id, approval_msg)
            .await;

        if let Some(time_suggestion) = input.time_suggestion {
            ctx.pending_time_suggestions
                .write()
                .await
                .insert(time_suggestion.id, time_suggestion.clone());
            let suggestion_msg = ServerMessage::TimeSuggestion {
                data: time_suggestion.to_protocol(),
            };
            ctx.connections
                .broadcast_to_dms(input.world_id, suggestion_msg)
                .await;
        }

        Ok(ServerMessage::StagingPending {
            region_id: input.region.id.to_string(),
            region_name: input.region.name,
            timeout_seconds: DEFAULT_STAGING_TIMEOUT_SECONDS,
        })
    }

    async fn resolve_visual_states(
        &self,
        world_id: WorldId,
        location_id: LocationId,
        region_id: RegionId,
    ) -> (
        Option<wrldbldr_protocol::types::ResolvedVisualStateData>,
        Vec<wrldbldr_protocol::types::StateOptionData>,
        Vec<wrldbldr_protocol::types::StateOptionData>,
    ) {
        let game_time = match self.world.get(world_id).await {
            Ok(Some(w)) => w.game_time,
            _ => return (None, vec![], vec![]),
        };

        let world_flags = self
            .flag
            .get_world_flags(world_id)
            .await
            .unwrap_or_default();

        let context =
            StateResolutionContext::new(world_id, game_time).with_world_flags(world_flags);

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
            Some(wrldbldr_protocol::types::ResolvedVisualStateData {
                location_state: resolution.location_state.as_ref().map(|s| {
                    wrldbldr_protocol::types::ResolvedStateInfoData {
                        id: s.id.clone(),
                        name: s.name.clone(),
                        backdrop_override: s.backdrop_override.clone(),
                        atmosphere_override: s.atmosphere_override.clone(),
                        ambient_sound: s.ambient_sound.clone(),
                    }
                }),
                region_state: resolution.region_state.as_ref().map(|s| {
                    wrldbldr_protocol::types::ResolvedStateInfoData {
                        id: s.id.clone(),
                        name: s.name.clone(),
                        backdrop_override: s.backdrop_override.clone(),
                        atmosphere_override: s.atmosphere_override.clone(),
                        ambient_sound: s.ambient_sound.clone(),
                    }
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
                wrldbldr_protocol::types::StateOptionData {
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
                wrldbldr_protocol::types::StateOptionData {
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

/// Use case for regenerating LLM staging suggestions.
pub struct RegenerateStagingSuggestions {
    location: Arc<Location>,
    character: Arc<Character>,
    llm: Arc<dyn LlmPort>,
}

impl RegenerateStagingSuggestions {
    pub fn new(location: Arc<Location>, character: Arc<Character>, llm: Arc<dyn LlmPort>) -> Self {
        Self {
            location,
            character,
            llm,
        }
    }

    pub async fn execute(
        &self,
        region_id: RegionId,
        guidance: Option<&str>,
    ) -> Result<Vec<StagedNpcInfo>, StagingError> {
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(StagingError::RegionNotFound)?;

        let location_name = self
            .location
            .get(region.location_id)
            .await
            .ok()
            .flatten()
            .map(|l| l.name)
            .unwrap_or_else(|| "Unknown Location".to_string());

        Ok(generate_llm_based_suggestions(
            &self.character,
            self.llm.as_ref(),
            region_id,
            &region.name,
            &location_name,
            guidance,
        )
        .await)
    }
}

/// Use case for applying DM staging approvals.
pub struct ApproveStagingRequest {
    staging: Arc<Staging>,
    world: Arc<World>,
    character: Arc<Character>,
    location: Arc<Location>,
    location_state: Arc<LocationStateEntity>,
    region_state: Arc<RegionStateEntity>,
}

impl ApproveStagingRequest {
    pub fn new(
        staging: Arc<Staging>,
        world: Arc<World>,
        character: Arc<Character>,
        location: Arc<Location>,
        location_state: Arc<LocationStateEntity>,
        region_state: Arc<RegionStateEntity>,
    ) -> Self {
        Self {
            staging,
            world,
            character,
            location,
            location_state,
            region_state,
        }
    }

    pub async fn execute(
        &self,
        input: ApproveStagingInput,
    ) -> Result<StagingReadyPayload, StagingError> {
        // Validate approved_npcs array
        // Note: Empty approved_npcs is explicitly allowed - it represents a staging
        // with no NPCs present (e.g., an empty room or wilderness area)
        self.validate_approved_npcs(&input.approved_npcs)?;

        let world = self
            .world
            .get(input.world_id)
            .await?
            .ok_or(StagingError::WorldNotFound)?;

        let location_id = match input.location_id {
            Some(id) => id,
            None => {
                let region = self
                    .location
                    .get_region(input.region_id)
                    .await?
                    .ok_or(StagingError::RegionNotFound)?;
                region.location_id
            }
        };

        let current_game_time = world.game_time.current();
        let approved_at = chrono::Utc::now();

        let staged_npcs = self.build_staged_npcs(&input.approved_npcs).await;

        let staging = wrldbldr_domain::Staging::new(
            input.region_id,
            location_id,
            input.world_id,
            current_game_time,
            input.approved_by.clone(),
            input.source,
            input.ttl_hours,
            approved_at,
        )
        .with_npcs(staged_npcs);

        self.staging.save_pending(&staging).await?;
        self.staging
            .activate_staging(staging.id, input.region_id)
            .await?;

        if let Some(loc_state_str) = &input.location_state_id {
            if let Ok(loc_uuid) = Uuid::parse_str(loc_state_str) {
                let loc_state_id = wrldbldr_domain::LocationStateId::from_uuid(loc_uuid);
                // Validate that the location state exists before setting it as active
                match self.location_state.get(loc_state_id).await {
                    Ok(Some(_)) => {
                        if let Err(e) = self
                            .location_state
                            .set_active(location_id, loc_state_id)
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to set active location state");
                        }
                    }
                    Ok(None) => {
                        tracing::warn!(
                            location_state_id = %loc_state_str,
                            location_id = %location_id,
                            "Location state ID provided but not found in database, skipping"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            location_state_id = %loc_state_str,
                            "Failed to validate location state existence"
                        );
                    }
                }
            }
        }

        if let Some(reg_state_str) = &input.region_state_id {
            if let Ok(reg_uuid) = Uuid::parse_str(reg_state_str) {
                let reg_state_id = wrldbldr_domain::RegionStateId::from_uuid(reg_uuid);
                // Validate that the region state exists before setting it as active
                match self.region_state.get(reg_state_id).await {
                    Ok(Some(_)) => {
                        if let Err(e) = self
                            .region_state
                            .set_active(input.region_id, reg_state_id)
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to set active region state");
                        }
                    }
                    Ok(None) => {
                        tracing::warn!(
                            region_state_id = %reg_state_str,
                            region_id = %input.region_id,
                            "Region state ID provided but not found in database, skipping"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            region_state_id = %reg_state_str,
                            "Failed to validate region state existence"
                        );
                    }
                }
            }
        }

        let npcs_present = self.build_npcs_present(&input.approved_npcs).await;
        let visual_state = self
            .build_visual_state_for_staging(location_id, input.region_id)
            .await;

        Ok(StagingReadyPayload {
            region_id: input.region_id,
            npcs_present,
            visual_state,
        })
    }

    async fn build_staged_npcs(
        &self,
        approved_npcs: &[ApprovedNpcInfo],
    ) -> Vec<wrldbldr_domain::StagedNpc> {
        let mut staged_npcs = Vec::new();

        for npc_info in approved_npcs {
            let char_uuid = match Uuid::parse_str(&npc_info.character_id) {
                Ok(u) => u,
                Err(e) => {
                    tracing::warn!(
                        character_id = %npc_info.character_id,
                        error = %e,
                        "Failed to parse character UUID during staging approval, skipping NPC"
                    );
                    continue;
                }
            };
            let character_id = CharacterId::from_uuid(char_uuid);

            let character = self.character.get(character_id).await.ok().flatten();
            let (name, sprite_asset, portrait_asset, default_mood, has_incomplete_data) =
                match character {
                    Some(c) => (c.name, c.sprite_asset, c.portrait_asset, c.default_mood, false),
                    None => {
                        tracing::warn!(
                            character_id = %npc_info.character_id,
                            "Character not found during staging approval, NPC will have incomplete data"
                        );
                        (
                            String::new(),
                            None,
                            None,
                            wrldbldr_domain::MoodState::default(),
                            true,
                        )
                    }
                };

            let mood = npc_info
                .mood
                .as_deref()
                .and_then(|m| m.parse::<wrldbldr_domain::MoodState>().ok())
                .unwrap_or(default_mood);

            staged_npcs.push(wrldbldr_domain::StagedNpc {
                character_id,
                name,
                sprite_asset,
                portrait_asset,
                is_present: npc_info.is_present,
                is_hidden_from_players: npc_info.is_hidden_from_players,
                reasoning: npc_info.reasoning.clone().unwrap_or_default(),
                mood,
                has_incomplete_data,
            });
        }

        staged_npcs
    }

    async fn build_npcs_present(&self, approved_npcs: &[ApprovedNpcInfo]) -> Vec<NpcPresentInfo> {
        let mut npcs_present = Vec::new();
        for npc_info in approved_npcs {
            if npc_info.is_present && !npc_info.is_hidden_from_players {
                let (name, sprite_asset, portrait_asset) =
                    match Uuid::parse_str(&npc_info.character_id) {
                        Ok(char_uuid) => {
                            let char_id = CharacterId::from_uuid(char_uuid);
                            match self.character.get(char_id).await {
                                Ok(Some(character)) => (
                                    character.name,
                                    character.sprite_asset,
                                    character.portrait_asset,
                                ),
                                Ok(None) => {
                                    tracing::warn!(
                                        character_id = %npc_info.character_id,
                                        "Character not found when building NPCs present, using empty defaults"
                                    );
                                    (String::new(), None, None)
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        character_id = %npc_info.character_id,
                                        error = %e,
                                        "Failed to fetch character when building NPCs present, using empty defaults"
                                    );
                                    (String::new(), None, None)
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                character_id = %npc_info.character_id,
                                error = %e,
                                "Failed to parse character UUID when building NPCs present, using empty defaults"
                            );
                            (String::new(), None, None)
                        }
                    };

                npcs_present.push(NpcPresentInfo {
                    character_id: npc_info.character_id.clone(),
                    name,
                    sprite_asset,
                    portrait_asset,
                    is_hidden_from_players: npc_info.is_hidden_from_players,
                    mood: npc_info.mood.clone(),
                });
            }
        }

        npcs_present
    }

    async fn build_visual_state_for_staging(
        &self,
        location_id: LocationId,
        region_id: RegionId,
    ) -> Option<wrldbldr_protocol::types::ResolvedVisualStateData> {
        let location_state = self
            .location_state
            .get_active(location_id)
            .await
            .ok()
            .flatten();
        let region_state = self.region_state.get_active(region_id).await.ok().flatten();

        if location_state.is_none() && region_state.is_none() {
            return None;
        }

        Some(wrldbldr_protocol::types::ResolvedVisualStateData {
            location_state: location_state.map(|s| {
                wrldbldr_protocol::types::ResolvedStateInfoData {
                    id: s.id.to_string(),
                    name: s.name,
                    backdrop_override: s.backdrop_override,
                    atmosphere_override: s.atmosphere_override,
                    ambient_sound: s.ambient_sound,
                }
            }),
            region_state: region_state.map(|s| wrldbldr_protocol::types::ResolvedStateInfoData {
                id: s.id.to_string(),
                name: s.name,
                backdrop_override: s.backdrop_override,
                atmosphere_override: s.atmosphere_override,
                ambient_sound: s.ambient_sound,
            }),
        })
    }

    /// Validates the approved_npcs array.
    ///
    /// Validation rules:
    /// - Empty array is allowed (represents staging with no NPCs)
    /// - Each NPC must have a non-empty character_id
    /// - Each character_id must be a valid UUID format
    fn validate_approved_npcs(&self, approved_npcs: &[ApprovedNpcInfo]) -> Result<(), StagingError> {
        for (index, npc_info) in approved_npcs.iter().enumerate() {
            // Check for empty character_id
            if npc_info.character_id.is_empty() {
                return Err(StagingError::Validation(format!(
                    "NPC at index {} has empty character_id",
                    index
                )));
            }

            // Validate UUID format
            if Uuid::parse_str(&npc_info.character_id).is_err() {
                return Err(StagingError::Validation(format!(
                    "NPC at index {} has invalid character_id '{}': not a valid UUID",
                    index, npc_info.character_id
                )));
            }
        }

        // Log when empty array is explicitly approved
        if approved_npcs.is_empty() {
            tracing::debug!("Staging approved with empty NPC list (no NPCs present)");
        }

        Ok(())
    }
}

pub struct ApproveStagingInput {
    pub region_id: RegionId,
    pub location_id: Option<LocationId>,
    pub world_id: WorldId,
    pub approved_by: String,
    pub ttl_hours: i32,
    pub source: StagingSource,
    pub approved_npcs: Vec<ApprovedNpcInfo>,
    pub location_state_id: Option<String>,
    pub region_state_id: Option<String>,
}

pub struct StagingReadyPayload {
    pub region_id: RegionId,
    pub npcs_present: Vec<NpcPresentInfo>,
    pub visual_state: Option<wrldbldr_protocol::types::ResolvedVisualStateData>,
}

#[derive(Debug, thiserror::Error)]
pub enum StagingError {
    #[error("World not found")]
    WorldNotFound,
    #[error("Region not found")]
    RegionNotFound,
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

#[derive(Deserialize)]
struct LlmSuggestion {
    name: String,
    reason: String,
}

async fn generate_rule_based_suggestions(
    character: &Character,
    staging: &Staging,
    region_id: RegionId,
) -> Vec<StagedNpcInfo> {
    let npcs_with_relationships = character
        .get_npcs_for_region(region_id)
        .await
        .ok()
        .unwrap_or_default();

    let mut suggestions: Vec<StagedNpcInfo> = npcs_with_relationships
        .into_iter()
        .filter(|n| n.relationship_type != NpcRegionRelationType::Avoids)
        .map(|npc| {
            let reasoning = match npc.relationship_type {
                NpcRegionRelationType::HomeRegion => "Lives here".to_string(),
                NpcRegionRelationType::WorksAt => match npc.shift.as_deref() {
                    Some("day") => "Works here (day shift)".to_string(),
                    Some("night") => "Works here (night shift)".to_string(),
                    _ => "Works here".to_string(),
                },
                NpcRegionRelationType::Frequents => {
                    let freq = npc.frequency.as_deref().unwrap_or("sometimes");
                    let time = npc.time_of_day.as_deref();
                    match time {
                        Some(t) => format!("Frequents this area {} ({})", freq, t),
                        None => format!("Frequents this area ({})", freq),
                    }
                }
                NpcRegionRelationType::Avoids => "Avoids this area".to_string(),
            };

            StagedNpcInfo {
                character_id: npc.character_id.to_string(),
                name: npc.name,
                sprite_asset: npc.sprite_asset,
                portrait_asset: npc.portrait_asset,
                is_present: true,
                reasoning,
                is_hidden_from_players: false,
                mood: Some(npc.default_mood.to_string()),
            }
        })
        .collect();

    if let Ok(staged_npcs) = staging.get_staged_npcs(region_id).await {
        for staged in staged_npcs {
            if !suggestions
                .iter()
                .any(|s| s.character_id == staged.character_id.to_string())
            {
                suggestions.push(StagedNpcInfo {
                    character_id: staged.character_id.to_string(),
                    name: staged.name,
                    sprite_asset: staged.sprite_asset,
                    portrait_asset: staged.portrait_asset,
                    is_present: staged.is_present,
                    reasoning: staged.reasoning,
                    is_hidden_from_players: staged.is_hidden_from_players,
                    mood: Some(staged.mood.to_string()),
                });
            }
        }
    }

    suggestions
}

async fn generate_llm_based_suggestions(
    character: &Character,
    llm: &dyn LlmPort,
    region_id: RegionId,
    region_name: &str,
    location_name: &str,
    guidance: Option<&str>,
) -> Vec<StagedNpcInfo> {
    let npcs_with_relationships = match character.get_npcs_for_region(region_id).await {
        Ok(npcs) => npcs,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to get NPCs for LLM staging");
            return vec![];
        }
    };

    let candidates: Vec<_> = npcs_with_relationships
        .into_iter()
        .filter(|n| n.relationship_type != NpcRegionRelationType::Avoids)
        .collect();

    if candidates.is_empty() {
        return vec![];
    }

    let npc_list: String = candidates
        .iter()
        .enumerate()
        .map(|(i, npc)| {
            let relationship = match npc.relationship_type {
                NpcRegionRelationType::HomeRegion => "lives here",
                NpcRegionRelationType::WorksAt => "works here",
                NpcRegionRelationType::Frequents => "frequents this area",
                NpcRegionRelationType::Avoids => "avoids this area",
            };
            format!("{}. {} ({})", i + 1, npc.name, relationship)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let guidance_text = guidance
        .filter(|g| !g.is_empty())
        .map(|g| format!("\n\nDM's guidance: {}", g))
        .unwrap_or_default();

    let system_prompt = "You are a helpful TTRPG assistant helping decide which NPCs should be present in a scene. \
        Respond with a JSON array of objects, each with 'name' (exact name from the list) and 'reason' (brief explanation). \
        Select 1-4 NPCs that would logically be present. Only include NPCs from the provided list.";

    let user_prompt = format!(
        "Region: {} (in {})\n\nAvailable NPCs:\n{}{}\n\nWhich NPCs should be present? Respond with JSON only.",
        region_name, location_name, npc_list, guidance_text
    );

    let request = LlmRequest::new(vec![ChatMessage::user(&user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = match llm.generate(request).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::warn!(error = %e, "LLM staging suggestion failed");
            return vec![];
        }
    };

    let suggestions = parse_llm_staging_response(&response.content, &candidates);

    tracing::info!(
        region = %region_name,
        suggestion_count = suggestions.len(),
        "Generated LLM staging suggestions"
    );

    suggestions
}

/// Normalizes a name for matching by trimming whitespace, converting to lowercase,
/// and collapsing multiple consecutive whitespace characters into single spaces.
fn normalize_name(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn parse_llm_staging_response(
    content: &str,
    candidates: &[crate::infrastructure::ports::NpcWithRegionInfo],
) -> Vec<StagedNpcInfo> {
    let json_start = content.find('[');
    let json_end = content.rfind(']');

    let json_str = match (json_start, json_end) {
        (Some(start), Some(end)) if end > start => &content[start..=end],
        _ => {
            tracing::warn!(
                content = %content,
                "LLM staging response did not contain a valid JSON array - returning empty suggestions"
            );
            return vec![];
        }
    };

    let parsed: Vec<LlmSuggestion> = match serde_json::from_str(json_str) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                error = %e,
                json = %json_str,
                "Failed to parse LLM staging JSON response - returning empty suggestions"
            );
            return vec![];
        }
    };

    parsed
        .into_iter()
        .filter_map(|suggestion| {
            let npc = candidates
                .iter()
                .find(|c| normalize_name(&c.name) == normalize_name(&suggestion.name))?;

            Some(StagedNpcInfo {
                character_id: npc.character_id.to_string(),
                name: npc.name.clone(),
                sprite_asset: npc.sprite_asset.clone(),
                portrait_asset: npc.portrait_asset.clone(),
                is_present: true,
                reasoning: format!("[LLM] {}", suggestion.reason),
                is_hidden_from_players: false,
                mood: Some(npc.default_mood.to_string()),
            })
        })
        .collect()
}

/// Use case for auto-approving expired staging requests.
pub struct AutoApproveStagingTimeout {
    character: Arc<Character>,
    staging: Arc<Staging>,
    world: Arc<World>,
    location: Arc<Location>,
    location_state: Arc<LocationStateEntity>,
    region_state: Arc<RegionStateEntity>,
    settings: Arc<dyn SettingsRepo>,
}

impl AutoApproveStagingTimeout {
    pub fn new(
        character: Arc<Character>,
        staging: Arc<Staging>,
        world: Arc<World>,
        location: Arc<Location>,
        location_state: Arc<LocationStateEntity>,
        region_state: Arc<RegionStateEntity>,
        settings: Arc<dyn SettingsRepo>,
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
        // Fetch world settings for configurable TTL values, falling back to defaults
        // if settings cannot be loaded (to avoid breaking staging on settings fetch failure)
        let settings = match self.settings.get_for_world(pending.world_id).await {
            Ok(Some(s)) => s,
            Ok(None) => wrldbldr_domain::AppSettings::default(),
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    world_id = %pending.world_id,
                    "Failed to load world settings for auto-approval, using defaults"
                );
                wrldbldr_domain::AppSettings::default()
            }
        };

        // Generate rule-based NPC suggestions
        let rule_based_npcs =
            generate_rule_based_suggestions(&self.character, &self.staging, pending.region_id)
                .await;

        // Convert to ApprovedNpcInfo format
        let approved_npcs: Vec<ApprovedNpcInfo> = rule_based_npcs
            .into_iter()
            .map(|npc| ApprovedNpcInfo {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_name_trims_whitespace() {
        assert_eq!(normalize_name("  John Smith  "), "john smith");
    }

    #[test]
    fn normalize_name_collapses_multiple_spaces() {
        assert_eq!(normalize_name("John    Smith"), "john smith");
    }

    #[test]
    fn normalize_name_handles_tabs_and_newlines() {
        assert_eq!(normalize_name("John\t\nSmith"), "john smith");
    }

    #[test]
    fn normalize_name_lowercases() {
        assert_eq!(normalize_name("JOHN SMITH"), "john smith");
    }

    #[test]
    fn normalize_name_combined() {
        assert_eq!(
            normalize_name("  Marcus   the   Bartender  "),
            "marcus the bartender"
        );
    }

    #[test]
    fn normalize_name_empty_string() {
        assert_eq!(normalize_name(""), "");
    }

    #[test]
    fn normalize_name_whitespace_only() {
        assert_eq!(normalize_name("   \t\n   "), "");
    }

    #[test]
    fn normalize_name_unicode_characters() {
        // Unicode letters should be preserved, only lowercased
        assert_eq!(normalize_name("José García"), "josé garcía");
        assert_eq!(normalize_name("Müller"), "müller");
        assert_eq!(normalize_name("北京"), "北京"); // Non-Latin scripts preserved
    }

    #[test]
    fn normalize_name_unicode_whitespace() {
        // Various unicode whitespace characters should be normalized
        assert_eq!(normalize_name("John\u{00A0}Smith"), "john smith"); // Non-breaking space
        assert_eq!(normalize_name("John\u{2003}Smith"), "john smith"); // Em space
    }
}
