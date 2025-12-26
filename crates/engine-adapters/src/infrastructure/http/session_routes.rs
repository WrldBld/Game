use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Timelike;
use serde::{Deserialize, Serialize};

use crate::infrastructure::state::AppState;
use wrldbldr_engine_app::application::dto::{SessionInfo, WorldSnapshot};
use wrldbldr_engine_app::application::services::world_service::WorldService;

use wrldbldr_engine_ports::outbound::PlayerWorldSnapshot;

use wrldbldr_domain::{SessionId, WorldId};

/// List all active sessions.
pub async fn list_sessions(State(state): State<Arc<AppState>>) -> Json<Vec<SessionInfo>> {
    let sessions = state.sessions.read().await;

    let infos = sessions
        .get_session_ids()
        .into_iter()
        .filter_map(|session_id| sessions.get_session(session_id).map(|s| (session_id, s)))
        .map(|(session_id, session)| {
            let dm_user_id = session
                .dm_user_id
                .clone()
                .or_else(|| session.get_dm().map(|p| p.user_id.clone()))
                .unwrap_or_default();

            let active_player_count = session
                .participants
                .values()
                .filter(|p| p.role == wrldbldr_protocol::ParticipantRole::Player)
                .count();

            SessionInfo {
                session_id: session_id.to_string(),
                world_id: session.world_id.to_string(),
                dm_user_id,
                active_player_count,
                created_at: session.created_at.timestamp(),
            }
        })
        .collect();

    Json(infos)
}

/// List active sessions for a specific world.
pub async fn list_world_sessions(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Json<Vec<SessionInfo>> {
    let sessions = state.sessions.read().await;
    let world_uuid = match uuid::Uuid::parse_str(&world_id) {
        Ok(id) => id,
        Err(_) => return Json(Vec::new()),
    };
    let world_id = WorldId::from_uuid(world_uuid);

    let infos = sessions
        .get_session_ids()
        .into_iter()
        .filter_map(|session_id| sessions.get_session(session_id).map(move |s| (session_id, s)))
        .filter(|(_, session)| session.world_id == world_id)
        .map(|(session_id, session)| {
            let dm_user_id = session
                .dm_user_id
                .clone()
                .or_else(|| session.get_dm().map(|p| p.user_id.clone()))
                .unwrap_or_default();

            let active_player_count = session
                .participants
                .values()
                .filter(|p| p.role == wrldbldr_protocol::ParticipantRole::Player)
                .count();

            SessionInfo {
                session_id: session_id.to_string(),
                world_id: session.world_id.to_string(),
                dm_user_id,
                active_player_count,
                created_at: session.created_at.timestamp(),
            }
        })
        .collect();

    Json(infos)
}

#[derive(serde::Deserialize)]
pub struct CreateSessionRequest {
    pub dm_user_id: String,
}

/// Idempotently create or return the DM's session for a world.
pub async fn create_or_get_dm_session(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
    Json(body): Json<CreateSessionRequest>,
) -> Result<Json<SessionInfo>, StatusCode> {
    let world_uuid = uuid::Uuid::parse_str(&world_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let world_id = WorldId::from_uuid(world_uuid);

    // First, see if a session for this world already exists
    {
        let sessions = state.sessions.read().await;
        if let Some(session_id) = sessions.find_session_for_world(world_id) {
            if let Some(session) = sessions.get_session(session_id) {
                let dm_user_id = session
                    .dm_user_id
                    .clone()
                    .or_else(|| session.get_dm().map(|p| p.user_id.clone()))
                    .unwrap_or(body.dm_user_id.clone());

                    let active_player_count = session
                        .participants
                        .values()
                        .filter(|p| p.role == wrldbldr_protocol::ParticipantRole::Player)
                        .count();




                let info = SessionInfo {
                    session_id: session_id.to_string(),
                    world_id: session.world_id.to_string(),
                    dm_user_id,
                    active_player_count,
                    created_at: session.created_at.timestamp(),
                };

                return Ok(Json(info));
            }
        }
    }

    // Otherwise create a new session for this world
    let player_snapshot = state
        .core.world_service
        .export_world_snapshot(world_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let internal_snapshot = convert_to_internal_snapshot(&player_snapshot);

    let mut sessions = state.sessions.write().await;

    let session_id = sessions.create_session_with_id(SessionId::new(), world_id, internal_snapshot);

    // Set DM owner metadata for the new session
    if let Some(s) = sessions.get_session_mut(session_id) {
        if s.dm_user_id.is_none() {
            s.dm_user_id = Some(body.dm_user_id.clone());
        }
    }

    let info = {
        let session = sessions
            .get_session(session_id)
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        SessionInfo {
            session_id: session_id.to_string(),
            world_id: session.world_id.to_string(),
            dm_user_id: body.dm_user_id,
            active_player_count: 0,
            created_at: session.created_at.timestamp(),
        }
    };

    Ok(Json(info))
}

/// Convert PlayerWorldSnapshot DTO to internal WorldSnapshot with domain types
///
/// This adapter function converts the serializable PlayerWorldSnapshot (with string IDs)
/// into a WorldSnapshot containing actual domain entities (with typed IDs).
fn convert_to_internal_snapshot(player_snapshot: &PlayerWorldSnapshot) -> WorldSnapshot {
    use chrono::Utc;
    use wrldbldr_domain::{
        ActId, CampbellArchetype, Character, CharacterId, Location, LocationId, LocationType,
        MoodLevel, Scene, SceneId, StatBlock, TimeContext, World,
    };

    // Convert world data
    let world_id = uuid::Uuid::parse_str(&player_snapshot.world.id)
        .map(WorldId::from_uuid)
        .unwrap_or_else(|_| WorldId::new());

    let now = Utc::now();

    let world = World {
        id: world_id,
        name: player_snapshot.world.name.clone(),
        description: player_snapshot.world.description.clone(),
        rule_system: match &player_snapshot.world.rule_system.variant {
            wrldbldr_protocol::RuleSystemVariant::DnD5e => {
                wrldbldr_domain::RuleSystemConfig::from_variant(
                    wrldbldr_domain::RuleSystemVariant::Dnd5e,
                )
            }
            wrldbldr_protocol::RuleSystemVariant::Pathfinder2e => {
                wrldbldr_domain::RuleSystemConfig::from_variant(
                    wrldbldr_domain::RuleSystemVariant::Pathfinder2e,
                )
            }
            wrldbldr_protocol::RuleSystemVariant::CallOfCthulhu => {
                wrldbldr_domain::RuleSystemConfig::from_variant(
                    wrldbldr_domain::RuleSystemVariant::CallOfCthulhu7e,
                )
            }
            wrldbldr_protocol::RuleSystemVariant::RuneQuest => {
                wrldbldr_domain::RuleSystemConfig::from_variant(
                    wrldbldr_domain::RuleSystemVariant::RuneQuest,
                )
            }
            wrldbldr_protocol::RuleSystemVariant::FateCore => {
                wrldbldr_domain::RuleSystemConfig::from_variant(
                    wrldbldr_domain::RuleSystemVariant::FateCore,
                )
            }
            wrldbldr_protocol::RuleSystemVariant::PbtA => {
                wrldbldr_domain::RuleSystemConfig::from_variant(
                    wrldbldr_domain::RuleSystemVariant::PoweredByApocalypse,
                )
            }
            wrldbldr_protocol::RuleSystemVariant::Custom(name) => {
                // Preserve well-known generic/named variants that are serialized via `Custom`
                let variant = match name.as_str() {
                    "generic_d20" => wrldbldr_domain::RuleSystemVariant::GenericD20,
                    "generic_d100" => wrldbldr_domain::RuleSystemVariant::GenericD100,
                    "kids_on_bikes" => wrldbldr_domain::RuleSystemVariant::KidsOnBikes,
                    _ => wrldbldr_domain::RuleSystemVariant::Custom(name.clone()),
                };
                wrldbldr_domain::RuleSystemConfig::from_variant(variant)
            }
        },
        game_time: wrldbldr_domain::GameTime::default(),
        created_at: now,
        updated_at: now,
    };

    // Convert locations
    let locations: Vec<Location> = player_snapshot
        .locations
        .iter()
        .map(|l| {
            let location_id = uuid::Uuid::parse_str(&l.id)
                .map(LocationId::from_uuid)
                .unwrap_or_else(|_| LocationId::new());

            Location {
                id: location_id,
                world_id,
                name: l.name.clone(),
                description: l.description.clone(),
                location_type: LocationType::Interior,
                backdrop_asset: l.backdrop_asset.clone(),
                map_asset: None,
                parent_map_bounds: None,
                default_region_id: None,
                atmosphere: l.atmosphere.clone(),
                presence_cache_ttl_hours: 3,
                use_llm_presence: true,
            }
        })
        .collect();

    // Convert characters
    let characters: Vec<Character> = player_snapshot
        .characters
        .iter()
        .map(|c| {
            let character_id = uuid::Uuid::parse_str(&c.id)
                .map(CharacterId::from_uuid)
                .unwrap_or_else(|_| CharacterId::new());

            Character {
                id: character_id,
                world_id,
                name: c.name.clone(),
                description: c.description.clone(),
                sprite_asset: c.sprite_asset.clone(),
                portrait_asset: c.portrait_asset.clone(),
                base_archetype: CampbellArchetype::Ally,
                current_archetype: CampbellArchetype::Ally,
                archetype_history: Vec::new(),
                stats: StatBlock::default(),
                is_alive: c.is_alive,
                is_active: c.is_active,
                default_mood: MoodLevel::Neutral,
            }
        })
        .collect();

    // Convert scenes
    let scenes: Vec<Scene> = player_snapshot
        .scenes
        .iter()
        .map(|s| {
            let scene_id = uuid::Uuid::parse_str(&s.id)
                .map(SceneId::from_uuid)
                .unwrap_or_else(|_| SceneId::new());
            let location_id = uuid::Uuid::parse_str(&s.location_id)
                .map(LocationId::from_uuid)
                .unwrap_or_else(|_| LocationId::new());
            let featured_characters: Vec<CharacterId> = s
                .featured_characters
                .iter()
                .filter_map(|cid| uuid::Uuid::parse_str(cid).map(CharacterId::from_uuid).ok())
                .collect();

            Scene {
                id: scene_id,
                act_id: ActId::new(),
                name: s.name.clone(),
                location_id,
                time_context: TimeContext::Unspecified,
                backdrop_override: s.backdrop_override.clone(),
                entry_conditions: Vec::new(),
                featured_characters,
                directorial_notes: s.directorial_notes.clone(),
                order: 0,
            }
        })
        .collect();

    WorldSnapshot {
        world,
        locations,
        characters,
        scenes,
        current_scene_id: player_snapshot
            .current_scene
            .as_ref()
            .map(|s| s.id.clone()),
    }
}

// =============================================================================
// Game Time Routes (Phase 23F)
// =============================================================================

/// Response DTO for game time
#[derive(Debug, Clone, Serialize)]
pub struct GameTimeResponse {
    /// Canonical wire representation of game time.
    pub game_time: wrldbldr_protocol::GameTime,
}

/// Request DTO for advancing game time
#[derive(Debug, Clone, Deserialize)]
pub struct AdvanceGameTimeRequest {
    /// Number of hours to advance (can be 0)
    #[serde(default)]
    pub hours: u32,
    /// Number of days to advance (can be 0)
    #[serde(default)]
    pub days: u32,
}

/// Get current game time for a session
///
/// GET /api/sessions/{session_id}/game-time
pub async fn get_game_time(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<GameTimeResponse>, (StatusCode, String)> {
    let session_uuid = uuid::Uuid::parse_str(&session_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid session ID".to_string()))?;
    let session_id = SessionId::from_uuid(session_uuid);

    let sessions = state.sessions.read().await;
    let session = sessions
        .get_session(session_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    let gt = session.game_time();
    let game_time = wrldbldr_protocol::GameTime::new(
        gt.day_ordinal(),
        gt.current().hour() as u8,
        gt.current().minute() as u8,
        gt.is_paused(),
    );

    Ok(Json(GameTimeResponse { game_time }))
}

/// Advance game time for a session
///
/// POST /api/sessions/{session_id}/game-time/advance
pub async fn advance_game_time(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(req): Json<AdvanceGameTimeRequest>,
) -> Result<Json<GameTimeResponse>, (StatusCode, String)> {
    let session_uuid = uuid::Uuid::parse_str(&session_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid session ID".to_string()))?;
    let session_id = SessionId::from_uuid(session_uuid);

    let mut sessions = state.sessions.write().await;
    let session = sessions
        .get_session_mut(session_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    // Advance by hours and/or days
    if req.hours > 0 {
        session.advance_time_hours(req.hours);
    }
    if req.days > 0 {
        session.advance_time_days(req.days);
    }

    let gt = session.game_time();
    let game_time = wrldbldr_protocol::GameTime::new(
        gt.day_ordinal(),
        gt.current().hour() as u8,
        gt.current().minute() as u8,
        gt.is_paused(),
    );

    Ok(Json(GameTimeResponse { game_time }))
}
