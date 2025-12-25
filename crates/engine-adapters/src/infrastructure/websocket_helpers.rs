//! Helper functions for WebSocket queue integration
//!
//! These functions assist with building prompts and processing queue items
//! in the WebSocket handler and background workers.

use wrldbldr_engine_app::application::dto::PlayerActionItem;
use wrldbldr_engine_ports::outbound::{CharacterRepositoryPort, PlayerCharacterRepositoryPort, QueueError, RegionRepositoryPort};
use wrldbldr_engine_app::application::services::{
    ActantialContextService, ActantialContextServiceImpl,
    ChallengeService, ChallengeServiceImpl, MoodService, MoodServiceImpl,
    NarrativeEventService, NarrativeEventServiceImpl,
    SettingsService, SkillService, SkillServiceImpl,
};
use wrldbldr_domain::value_objects::{
    ActiveChallengeContext, ActiveNarrativeEventContext, CharacterContext, ConversationTurn,
    GamePromptRequest, PlayerActionContext, RegionItemContext, SceneContext,
};
use wrldbldr_domain::{CharacterId, PlayerCharacterId};
use crate::infrastructure::session::SessionManager;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Build a GamePromptRequest from a PlayerActionItem using session context
pub async fn build_prompt_from_action(
    sessions: &Arc<RwLock<SessionManager>>,
    challenge_service: &Arc<ChallengeServiceImpl>,
    skill_service: &Arc<SkillServiceImpl>,
    narrative_event_service: &Arc<NarrativeEventServiceImpl>,
    character_repo: &Arc<dyn CharacterRepositoryPort>,
    pc_repo: &Arc<dyn PlayerCharacterRepositoryPort>,
    region_repo: &Arc<dyn RegionRepositoryPort>,
    settings_service: &Arc<SettingsService>,
    mood_service: &Arc<MoodServiceImpl>,
    actantial_service: &Arc<ActantialContextServiceImpl>,
    action: &PlayerActionItem,
) -> Result<GamePromptRequest, QueueError> {
    // Get session context
    let sessions_read = sessions.read().await;
    let session = sessions_read
        .get_session(wrldbldr_domain::SessionId::from_uuid(action.session_id))
        .ok_or_else(|| QueueError::Backend("Session not found".to_string()))?;

    let world_snapshot = &session.world_snapshot;
    
    // Get current scene
    let current_scene = match &world_snapshot.current_scene_id {
        Some(scene_id_str) => {
            world_snapshot
                .scenes
                .iter()
                .find(|s| s.id.to_string() == *scene_id_str)
        }
        None => {
            tracing::warn!("No current scene set in world snapshot");
            world_snapshot.scenes.first()
        }
    };

    let current_scene = current_scene.ok_or_else(|| {
        QueueError::Backend("No scenes available in world snapshot".to_string())
    })?;

    // Get location
    let location = world_snapshot
        .locations
        .iter()
        .find(|l| l.id == current_scene.location_id);

    // Determine responding character
    let responding_character = if let Some(target_name) = &action.target {
        world_snapshot
            .characters
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(target_name))
    } else {
        current_scene
            .featured_characters
            .first()
            .and_then(|char_id| {
                world_snapshot.characters.iter().find(|c| c.id == *char_id)
            })
    };

    let responding_character = responding_character.ok_or_else(|| {
        QueueError::Backend("No responding character found".to_string())
    })?;

    // Fetch region items if PC has a current region
    let region_items = if let Some(pc_uuid) = action.pc_id {
        let pc_id = PlayerCharacterId::from_uuid(pc_uuid);
        match pc_repo.get(pc_id).await {
            Ok(Some(pc)) => {
                if let Some(region_id) = pc.current_region_id {
                    match region_repo.get_region_items(region_id).await {
                        Ok(items) => items
                            .into_iter()
                            .map(|item| RegionItemContext {
                                name: item.name,
                                description: item.description,
                                item_type: item.item_type,
                            })
                            .collect(),
                        Err(e) => {
                            tracing::warn!(
                                region_id = %region_id,
                                error = %e,
                                "Failed to fetch region items for LLM context"
                            );
                            vec![]
                        }
                    }
                } else {
                    vec![]
                }
            }
            Ok(None) => {
                tracing::warn!(pc_id = %pc_uuid, "PC not found for region items");
                vec![]
            }
            Err(e) => {
                tracing::warn!(pc_id = %pc_uuid, error = %e, "Failed to fetch PC for region items");
                vec![]
            }
        }
    } else {
        vec![]
    };

    // Build scene context
    let scene_context = SceneContext {
        scene_name: current_scene.name.clone(),
        location_name: location
            .map(|l| l.name.clone())
            .unwrap_or_else(|| "Unknown".to_string()),
        time_context: match &current_scene.time_context {
            wrldbldr_domain::entities::TimeContext::Unspecified => "Unspecified".to_string(),
            wrldbldr_domain::entities::TimeContext::TimeOfDay(tod) => format!("{:?}", tod),
            wrldbldr_domain::entities::TimeContext::During(s) => s.clone(),
            wrldbldr_domain::entities::TimeContext::Custom(s) => s.clone(),
        },
        present_characters: current_scene
            .featured_characters
            .iter()
            .filter_map(|char_id| {
                world_snapshot
                    .characters
                    .iter()
                    .find(|c| c.id == *char_id)
                    .map(|c| c.name.clone())
            })
            .collect(),
        region_items,
    };

    // Build character context with wants fetched from graph
    let character_wants = match character_repo.get_wants(responding_character.id).await {
        Ok(wants) => wants
            .into_iter()
            .map(|cw| {
                // Format want for LLM context: include description and optionally intensity
                if cw.want.intensity > 0.7 {
                    format!("{} (strong)", cw.want.description)
                } else if cw.want.intensity < 0.3 {
                    format!("{} (mild)", cw.want.description)
                } else {
                    cw.want.description
                }
            })
            .collect(),
        Err(e) => {
            tracing::warn!(
                "Failed to fetch wants for character {}: {}",
                responding_character.id,
                e
            );
            Vec::new()
        }
    };

    // Fetch NPC mood toward PC for LLM context (P1.4)
    let (current_mood, relationship_to_player) = if let Some(pc_uuid) = action.pc_id {
        let pc_id = PlayerCharacterId::from_uuid(pc_uuid);
        let npc_id = CharacterId::from_uuid(responding_character.id.into());
        match mood_service.get_mood(npc_id, pc_id).await {
            Ok(mood_state) => (
                Some(format!("{:?}", mood_state.mood)),
                Some(format!("{:?}", mood_state.relationship)),
            ),
            Err(e) => {
                tracing::warn!(
                    npc_id = %responding_character.id,
                    pc_id = %pc_uuid,
                    error = %e,
                    "Failed to fetch NPC mood for LLM context"
                );
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    // Fetch actantial context (motivations and social views)
    let _ = character_wants; // No longer needed - using actantial service instead
    let (motivations, social_stance) = match actantial_service
        .get_context(responding_character.id)
        .await
    {
        Ok(ctx) => (
            Some(ctx.to_motivations_context()),
            Some(ctx.to_social_stance_context()),
        ),
        Err(e) => {
            tracing::warn!(
                "Failed to get actantial context for {}: {}",
                responding_character.id,
                e
            );
            (None, None)
        }
    };
    
    let character_context = CharacterContext {
        character_id: Some(responding_character.id.to_string()),
        name: responding_character.name.clone(),
        archetype: format!("{:?}", responding_character.current_archetype),
        current_mood,
        motivations,
        social_stance,
        relationship_to_player,
    };

    // Get directorial notes
    let directorial_notes = current_scene.directorial_notes.clone();

    // Extract world_id from the session's world snapshot
    let world_id = world_snapshot.world.id;

    // Get per-world settings for conversation history limit
    let settings = settings_service.get_for_world(world_id).await;

    // Get conversation history from session using configurable limit
    let conversation_history = session
        .get_recent_history(settings.conversation_history_turns)
        .iter()
        .map(|turn| ConversationTurn {
            speaker: turn.speaker.clone(),
            text: turn.content.clone(),
        })
        .collect();

    // Query active challenges and convert to ActiveChallengeContext
    let active_challenges: Vec<ActiveChallengeContext> = match challenge_service
        .list_active(world_id)
        .await
    {
        Ok(challenges) => {
            let mut contexts = Vec::new();
            for c in challenges {
                let challenge_id = c.id;
                
                // Fetch skill_id from REQUIRES_SKILL edge
                let skill_id = match challenge_service.get_required_skill(challenge_id).await {
                    Ok(sid) => sid,
                    Err(e) => {
                        tracing::warn!("Failed to get required skill for challenge {}: {}", challenge_id, e);
                        None
                    }
                };

                // Look up skill name
                let skill_name = if let Some(sid) = skill_id {
                    match skill_service.get_skill(sid).await {
                        Ok(Some(skill)) => skill.name,
                        Ok(None) => {
                            tracing::warn!("Skill {} not found for challenge {}", sid, challenge_id);
                            sid.to_string()
                        }
                        Err(e) => {
                            tracing::error!("Failed to look up skill {}: {}", sid, e);
                            sid.to_string()
                        }
                    }
                } else {
                    "Unknown Skill".to_string()
                };

                contexts.push(ActiveChallengeContext {
                    id: c.id.to_string(),
                    name: c.name,
                    skill_name,
                    difficulty_display: c.difficulty.display(),
                    description: c.description,
                    trigger_hints: c
                        .trigger_conditions
                        .iter()
                        .map(|t| t.description.clone())
                        .collect(),
                });
            }
            contexts
        }
        Err(e) => {
            tracing::warn!("Failed to load active challenges: {}", e);
            vec![]
        }
    };

    // Query active narrative events and convert to ActiveNarrativeEventContext
    // NOTE: featured_npcs are now stored as FEATURES_NPC edges and would require
    // additional queries to fetch. For LLM context performance, we leave this empty.
    // The LLM can still understand event context from trigger_hints and scene_direction.
    let active_narrative_events: Vec<ActiveNarrativeEventContext> = match narrative_event_service
        .list_active(world_id)
        .await
    {
        Ok(events) => events
            .into_iter()
            .map(|e| ActiveNarrativeEventContext {
                id: e.id.to_string(),
                name: e.name,
                description: e.description,
                scene_direction: e.scene_direction,
                priority: e.priority,
                trigger_hints: e
                    .trigger_conditions
                    .iter()
                    .map(|t| t.description.clone())
                    .collect(),
                // TODO: If featured NPC names are needed for LLM context, fetch via
                // narrative_event_service.get_featured_npcs(e.id) and resolve names
                featured_npc_names: vec![],
            })
            .collect(),
        Err(e) => {
            tracing::warn!("Failed to load active narrative events: {}", e);
            vec![]
        }
    };

    // Build the prompt request
    Ok(GamePromptRequest {
        world_id: Some(world_id.to_string()),
        player_action: PlayerActionContext {
            action_type: action.action_type.clone(),
            target: action.target.clone(),
            dialogue: action.dialogue.clone(),
        },
        scene_context,
        directorial_notes,
        conversation_history,
        responding_character: character_context,
        active_challenges,
        active_narrative_events,
    })
}
