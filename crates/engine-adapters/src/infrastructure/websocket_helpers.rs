//! Helper functions for WebSocket queue integration
//!
//! These functions assist with building prompts and processing queue items
//! in the WebSocket handler and background workers.

use std::sync::Arc;

use uuid::Uuid;
use wrldbldr_domain::entities::NarrativeEvent;
use wrldbldr_domain::value_objects::{
    ActiveChallengeContext, ActiveNarrativeEventContext, CharacterContext, ConversationTurn,
    GamePromptRequest, MotivationsContext, PlayerActionContext, RegionItemContext, SceneContext,
    SocialStanceContext,
};
use wrldbldr_domain::{PlayerCharacterId, WorldId};
use wrldbldr_engine_app::application::dto::PlayerActionItem;
use wrldbldr_engine_app::application::services::{
    ActantialContextService, ChallengeService, MoodService,
    NarrativeEventService, SettingsService, SkillService, WorldService,
};
use wrldbldr_engine_ports::outbound::{
    CharacterRepositoryPort, PlayerCharacterRepositoryPort, QueueError, RegionRepositoryPort,
};

use crate::infrastructure::world_state_manager::{ConversationEntry, Speaker, WorldStateManager};

/// Build a GamePromptRequest from a PlayerActionItem
///
/// This function gathers all necessary context from the world snapshot,
/// conversation history, and domain services to create a complete prompt
/// for the LLM to generate an NPC response.
#[allow(clippy::too_many_arguments)]
pub async fn build_prompt_from_action(
    world_id: WorldId,
    world_service: &Arc<dyn WorldService>,
    world_state: &Arc<WorldStateManager>,
    challenge_service: &Arc<dyn ChallengeService>,
    skill_service: &Arc<dyn SkillService>,
    narrative_event_service: &Arc<dyn NarrativeEventService>,
    _character_repo: &Arc<dyn CharacterRepositoryPort>,
    pc_repo: &Arc<dyn PlayerCharacterRepositoryPort>,
    region_repo: &Arc<dyn RegionRepositoryPort>,
    _settings_service: &Arc<SettingsService>,
    _mood_service: &Arc<dyn MoodService>,
    _actantial_service: &Arc<dyn ActantialContextService>,
    action: &PlayerActionItem,
) -> Result<GamePromptRequest, QueueError> {
    // 1. Get world snapshot for scene and character data
    let snapshot = world_service
        .export_world_snapshot(world_id.clone())
        .await
        .map_err(|e| QueueError::Backend(format!("Failed to export world snapshot: {}", e)))?;

    // 2. Get current scene from world state or snapshot
    let current_scene_id = world_state
        .get_current_scene(&world_id)
        .or_else(|| snapshot.current_scene.as_ref().map(|s| s.id.clone()));

    // 3. Get PC's current region for item context
    let region_id = if let Some(pc_uuid) = action.pc_id {
        let pc_id = PlayerCharacterId::from_uuid(pc_uuid);
        match pc_repo.get(pc_id).await {
            Ok(Some(pc)) => pc.current_region_id,
            Ok(None) => {
                tracing::debug!("PC {} not found for region item context", pc_uuid);
                None
            }
            Err(e) => {
                tracing::debug!("Failed to fetch PC for region item context: {}", e);
                None
            }
        }
    } else {
        None
    };

    // 4. Fetch items in the PC's current region for LLM context
    let region_items: Vec<RegionItemContext> = if let Some(rid) = region_id {
        match region_repo.get_region_items(rid).await {
            Ok(items) => items
                .into_iter()
                .map(|item| RegionItemContext {
                    name: item.name,
                    description: item.description,
                    item_type: item.item_type,
                })
                .collect(),
            Err(e) => {
                tracing::debug!("Failed to fetch region items for LLM context: {}", e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // 5. Build scene context from current scene
    // Also capture IDs for dialogue persistence (P1.2)
    let (scene_context, scene_id_for_persistence, location_id_for_persistence) = if let Some(scene_id) = &current_scene_id {
        // Find scene in snapshot
        let scene = snapshot
            .scenes
            .iter()
            .find(|s| &s.id == scene_id)
            .or(snapshot.current_scene.as_ref());

        if let Some(scene) = scene {
            let ctx = SceneContext {
                scene_name: scene.name.clone(),
                location_name: scene.location_id.clone(), // SceneData has location_id, not location_name
                time_context: scene.time_context.clone(),
                present_characters: scene.featured_characters.clone(), // featured_characters, not characters
                region_items,
            };
            // Capture IDs for persistence
            let scene_id = Some(scene.id.clone());
            let location_id = Some(scene.location_id.clone());
            (ctx, scene_id, location_id)
        } else {
            (default_scene_context(), None, None)
        }
    } else {
        (default_scene_context(), None, None)
    };

    // Get game time for dialogue persistence
    let game_time_for_persistence = world_state
        .get_game_time(&world_id)
        .map(|gt| gt.display_date());

    // 6. Get directorial context
    let directorial_notes = world_state
        .get_directorial_context(&world_id)
        .map(|dc| {
            let mut notes = Vec::new();
            if !dc.scene_notes.is_empty() {
                notes.push(format!("Scene Notes: {}", dc.scene_notes));
            }
            if !dc.tone.is_empty() {
                notes.push(format!("Tone: {}", dc.tone));
            }
            if !dc.forbidden_topics.is_empty() {
                notes.push(format!("Forbidden Topics: {}", dc.forbidden_topics.join(", ")));
            }
            notes.join("\n")
        })
        .unwrap_or_default();

    // 7. Convert conversation history
    let conversation_history: Vec<ConversationTurn> = world_state
        .get_conversation_history(&world_id)
        .into_iter()
        .map(conversation_entry_to_turn)
        .collect();

    // 8. Build player action context
    let player_action = PlayerActionContext {
        action_type: action.action_type.clone(),
        target: action.target.clone(),
        dialogue: action.dialogue.clone(),
    };

    // 9. Find responding character (NPC being addressed) with mood and actantial context
    let responding_character = find_responding_character(
        &action.target,
        &snapshot.characters,
        action.pc_id,
        _mood_service,
        _actantial_service,
    )
    .await;

    // 10. Get active challenges for the current scene
    let active_challenges = get_active_challenges(
        challenge_service,
        skill_service,
        &current_scene_id,
    )
    .await;

    // 11. Get active narrative events with featured NPC names
    let active_narrative_events = get_active_narrative_events(
        narrative_event_service,
        _character_repo,
        &world_id,
    )
    .await;

    // 12. Build the complete prompt request
    Ok(GamePromptRequest {
        world_id: Some(world_id.to_string()),
        player_action,
        scene_context,
        directorial_notes,
        conversation_history,
        responding_character,
        active_challenges,
        active_narrative_events,
        context_budget: None, // Use default budget
        // P1.2: Context for dialogue persistence
        scene_id: scene_id_for_persistence,
        location_id: location_id_for_persistence,
        game_time: game_time_for_persistence,
    })
}

fn default_scene_context() -> SceneContext {
    SceneContext {
        scene_name: "Unknown Scene".to_string(),
        location_name: "Unknown Location".to_string(),
        time_context: "Unspecified".to_string(),
        present_characters: Vec::new(),
        region_items: Vec::new(),
    }
}

fn conversation_entry_to_turn(entry: ConversationEntry) -> ConversationTurn {
    let speaker = match entry.speaker {
        Speaker::Player { pc_name, .. } => pc_name,
        Speaker::Npc { npc_name, .. } => npc_name,
        Speaker::System => "System".to_string(),
        Speaker::Dm => "DM".to_string(),
    };

    ConversationTurn {
        speaker,
        text: entry.message,
    }
}

async fn find_responding_character(
    target: &Option<String>,
    characters: &[wrldbldr_engine_ports::outbound::CharacterData],
    pc_id: Option<Uuid>,
    mood_service: &Arc<dyn MoodService>,
    actantial_service: &Arc<dyn ActantialContextService>,
) -> CharacterContext {
    // Try to find character by name in target
    let target_name = target.as_ref().map(|s| s.to_lowercase());

    // Search in snapshot characters first
    let character_data = characters
        .iter()
        .find(|c| {
            target_name
                .as_ref()
                .map(|t| c.name.to_lowercase().contains(t))
                .unwrap_or(false)
        })
        .or_else(|| characters.first()); // Fallback to first character

    if let Some(char_data) = character_data {
        // Try to get mood if we have both NPC ID and PC ID
        let current_mood = get_npc_mood_toward_pc(
            &char_data.id,
            pc_id,
            mood_service,
        ).await;

        // Try to get actantial context (motivations and social stance)
        let (motivations, social_stance) = get_actantial_context(
            &char_data.id,
            actantial_service,
        ).await;

        CharacterContext {
            character_id: Some(char_data.id.clone()),
            name: char_data.name.clone(),
            archetype: char_data.archetype.clone(),
            current_mood,
            motivations,
            social_stance,
            relationship_to_player: None,
        }
    } else {
        // No character found - return a minimal context
        CharacterContext {
            character_id: None,
            name: target.clone().unwrap_or_else(|| "Unknown".to_string()),
            archetype: String::new(),
            current_mood: None,
            motivations: None,
            social_stance: None,
            relationship_to_player: None,
        }
    }
}

/// Get the NPC's mood toward a specific PC
async fn get_npc_mood_toward_pc(
    npc_id_str: &str,
    pc_id: Option<Uuid>,
    mood_service: &Arc<dyn MoodService>,
) -> Option<String> {
    // Need both NPC ID and PC ID to query mood
    let pc_uuid = pc_id?;
    
    let npc_uuid = Uuid::parse_str(npc_id_str).ok()?;
    let npc_id = wrldbldr_domain::CharacterId::from_uuid(npc_uuid);
    let pc_id = wrldbldr_domain::PlayerCharacterId::from_uuid(pc_uuid);
    
    // Get the mood state
    match mood_service.get_mood(npc_id, pc_id).await {
        Ok(mood_state) => {
            // Convert mood level to a descriptive string
            Some(format!("{:?}", mood_state.mood))
        }
        Err(e) => {
            tracing::debug!(
                npc_id = %npc_id_str,
                pc_id = %pc_uuid,
                error = %e,
                "Could not get NPC mood toward PC, using default"
            );
            None
        }
    }
}

/// Get actantial context (motivations and social stance) for a character
async fn get_actantial_context(
    character_id_str: &str,
    actantial_service: &Arc<dyn ActantialContextService>,
) -> (Option<MotivationsContext>, Option<SocialStanceContext>) {
    let Ok(character_uuid) = Uuid::parse_str(character_id_str) else {
        return (None, None);
    };
    
    let character_id = wrldbldr_domain::CharacterId::from_uuid(character_uuid);
    
    match actantial_service.get_context(character_id).await {
        Ok(context) => {
            let motivations = Some(context.to_motivations_context());
            let social_stance = Some(context.to_social_stance_context());
            (motivations, social_stance)
        }
        Err(e) => {
            tracing::debug!(
                character_id = %character_id_str,
                error = %e,
                "Could not get actantial context for character"
            );
            (None, None)
        }
    }
}

async fn get_active_challenges(
    challenge_service: &Arc<dyn ChallengeService>,
    skill_service: &Arc<dyn SkillService>,
    current_scene_id: &Option<String>,
) -> Vec<ActiveChallengeContext> {
    let Some(scene_id_str) = current_scene_id else {
        return Vec::new();
    };

    let Ok(scene_id) = Uuid::parse_str(scene_id_str) else {
        return Vec::new();
    };

    let scene_id = wrldbldr_domain::SceneId::from_uuid(scene_id);
    
    let challenges = match challenge_service.list_by_scene(scene_id).await {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut result = Vec::new();
    for challenge in challenges {
        // Get required skill name
        let skill_name = if let Ok(Some(skill_id)) = challenge_service
            .get_required_skill(challenge.id)
            .await
        {
            if let Ok(Some(skill)) = skill_service.get_skill(skill_id).await {
                skill.name
            } else {
                "Unknown Skill".to_string()
            }
        } else {
            "Unknown Skill".to_string()
        };

        // Build trigger hints from trigger condition descriptions
        let trigger_hints: Vec<String> = challenge.trigger_conditions
            .iter()
            .map(|tc| tc.description.clone())
            .collect();

        result.push(ActiveChallengeContext {
            id: challenge.id.to_string(),
            name: challenge.name.clone(),
            description: challenge.description.clone(),
            skill_name,
            difficulty_display: format!("{:?}", challenge.difficulty),
            trigger_hints,
        });
    }

    result
}

async fn get_active_narrative_events(
    narrative_event_service: &Arc<dyn NarrativeEventService>,
    character_repo: &Arc<dyn CharacterRepositoryPort>,
    world_id: &WorldId,
) -> Vec<ActiveNarrativeEventContext> {
    // Get pending (not yet triggered) narrative events
    let events: Vec<NarrativeEvent> = match narrative_event_service.list_pending(*world_id).await {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut result = Vec::with_capacity(events.len());

    for event in events {
        // Extract trigger hints from trigger conditions
        let trigger_hints: Vec<String> = event.trigger_conditions
            .iter()
            .map(|t| t.description.clone())
            .collect();

        // Fetch featured NPC names
        let featured_npc_names = get_featured_npc_names(
            narrative_event_service,
            character_repo,
            event.id,
        ).await;

        result.push(ActiveNarrativeEventContext {
            id: event.id.to_string(),
            name: event.name.clone(),
            description: event.description.clone(),
            scene_direction: event.scene_direction.clone(),
            trigger_hints,
            featured_npc_names,
            priority: event.priority,
        });
    }

    result
}

/// Fetch featured NPC names for a narrative event
async fn get_featured_npc_names(
    narrative_event_service: &Arc<dyn NarrativeEventService>,
    character_repo: &Arc<dyn CharacterRepositoryPort>,
    event_id: wrldbldr_domain::NarrativeEventId,
) -> Vec<String> {
    // Get featured NPCs for this event
    let featured_npcs = match narrative_event_service.get_featured_npcs(event_id).await {
        Ok(npcs) => npcs,
        Err(e) => {
            tracing::debug!(
                event_id = %event_id,
                error = %e,
                "Could not get featured NPCs for narrative event"
            );
            return Vec::new();
        }
    };

    // Fetch character names for each featured NPC
    let mut names = Vec::with_capacity(featured_npcs.len());
    for featured_npc in featured_npcs {
        match character_repo.get(featured_npc.character_id).await {
            Ok(Some(character)) => {
                names.push(character.name);
            }
            Ok(None) => {
                tracing::debug!(
                    character_id = %featured_npc.character_id,
                    "Featured NPC character not found"
                );
            }
            Err(e) => {
                tracing::debug!(
                    character_id = %featured_npc.character_id,
                    error = %e,
                    "Could not fetch featured NPC character"
                );
            }
        }
    }

    names
}
