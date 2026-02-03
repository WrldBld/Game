//! Multi-phase LLM pipeline modules.
//!
//! This module implements the phased LLM pipeline for NPC dialogue:
//! - Phase 0: Context Digest (deterministic game state summary)
//! - Phase 1: Challenge Trigger Analysis
//! - Phase 2: Event Trigger Analysis
//! - Phase 3: Tool Intent Detection
//! - Phase 4: Behavior Analysis
//! - Phase 5: Final Response Generation
//!
//! Each phase is processed as a distinct queue item, enabling:
//! - Deterministic outputs with stable VCR fingerprints
//! - DM oversight at each decision point
//! - Retry of individual phases without recomputing prior phases

pub mod parsers;

// Re-export PhaseParseError for use in queue error handling
pub use parsers::PhaseParseError;

use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::infrastructure::ports::{LlmPort, PromptTemplateRepo, QueuePort};
use crate::llm_context::GamePromptRequest;
use crate::queue_types::{
    LlmPhase, LlmRequestData, Phase0ContextDigest, Phase1ChallengeTriggers, Phase2EventTriggers,
    Phase3ToolIntent, Phase4BehaviorAnalysis, Phase5FinalResponse, PhaseOutputs,
};

/// Context for executing a single phase.
#[derive(Clone)]
pub struct PhaseExecutionContext {
    /// The LLM port for making requests
    pub llm: Arc<dyn LlmPort>,
    /// Prompt template repository
    pub prompt_templates: Arc<dyn PromptTemplateRepo>,
    /// Queue for enqueuing next phase
    pub queue: Arc<dyn QueuePort>,
}

/// Result of executing a phase.
#[derive(Debug)]
pub struct PhaseResult {
    /// Whether the phase completed successfully
    pub success: bool,
    /// The raw LLM response content
    pub raw_response: String,
    /// Parsed output (type depends on phase)
    pub output: PhaseOutput,
    /// Error message if failed
    pub error: Option<String>,
}

/// Output types for each phase.
#[derive(Debug, Clone)]
pub enum PhaseOutput {
    ContextDigest(Phase0ContextDigest),
    ChallengeTriggers(Phase1ChallengeTriggers),
    EventTriggers(Phase2EventTriggers),
    ToolIntent(Phase3ToolIntent),
    BehaviorAnalysis(Phase4BehaviorAnalysis),
    FinalResponse(Phase5FinalResponse),
    None,
}

/// Build a deterministic context digest (Phase 0).
///
/// This is typically computed non-LLM for determinism, but can use
/// LLM if context exceeds budget.
pub fn build_context_digest(prompt: &GamePromptRequest) -> Phase0ContextDigest {
    // Sort present characters alphabetically for determinism
    let mut present_chars = prompt.scene_context.present_characters.clone();
    present_chars.sort();

    // Extract recent topics from conversation history (deterministic ordering)
    let recent_topics_set: std::collections::BTreeSet<String> = prompt
        .conversation_history
        .iter()
        .rev()
        .take(3)
        .flat_map(|turn| {
            // Simple topic extraction - in production could use NLP
            turn.text
                .split_whitespace()
                .filter(|w| w.len() > 4)
                .map(|w| {
                    w.to_lowercase()
                        .trim_matches(|c: char| !c.is_alphanumeric())
                        .to_string()
                })
                .collect::<Vec<_>>()
        })
        .filter(|w| !w.is_empty())
        .collect();
    let recent_topics: Vec<String> = recent_topics_set.into_iter().take(5).collect();

    let normalize_dialogue = |text: &str| {
        let cleaned: String = text
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c.is_whitespace() {
                    c
                } else {
                    ' '
                }
            })
            .collect();
        cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
    };

    Phase0ContextDigest {
        scene_summary: format!(
            "Location: {} | Time: {} | Present: {}",
            prompt.scene_context.location_name,
            prompt.scene_context.time_context,
            present_chars.join(", ")
        ),
        player_action_digest: format!(
            "Type: {} | Target: {} | Dialogue: {}",
            prompt.player_action.action_type,
            prompt.player_action.target.as_deref().unwrap_or("none"),
            normalize_dialogue(prompt.player_action.dialogue.as_deref().unwrap_or("none"))
        ),
        npc_summary: format!(
            "Name: {} | Archetype: {} | Disposition: {}",
            prompt.responding_character.name,
            prompt.responding_character.archetype,
            prompt
                .responding_character
                .disposition_toward_player
                .as_deref()
                .unwrap_or("unknown")
        ),
        active_challenge_count: prompt.active_challenges.len(),
        active_event_count: prompt.active_narrative_events.len(),
        recent_topics,
        directorial_excerpt: Some(prompt.directorial_notes.clone())
            .filter(|s| !s.is_empty())
            .map(|s| {
                // Truncate to first 200 chars for digest (safe for UTF-8)
                let mut chars = s.chars();
                let excerpt: String = chars.by_ref().take(200).collect();
                if chars.next().is_some() {
                    format!("{}...", excerpt)
                } else {
                    s
                }
            }),
    }
}

fn format_context_digest_block(context: Option<&Phase0ContextDigest>) -> String {
    match context {
        Some(ctx) => {
            let topics = if ctx.recent_topics.is_empty() {
                "none".to_string()
            } else {
                ctx.recent_topics.join(", ")
            };
            let directorial = ctx.directorial_excerpt.as_deref().unwrap_or("none");
            format!(
                "CONTEXT DIGEST:\n- Scene: {}\n- Player action: {}\n- NPC: {}\n- Active: {} challenges, {} events\n- Recent topics: {}\n- Directorial: {}",
                ctx.scene_summary,
                ctx.player_action_digest,
                ctx.npc_summary,
                ctx.active_challenge_count,
                ctx.active_event_count,
                topics,
                directorial
            )
        }
        None => "CONTEXT DIGEST:\n- none".to_string(),
    }
}

fn format_challenge_results(challenges: Option<&Phase1ChallengeTriggers>) -> String {
    challenges
        .map(|c| {
            c.challenges
                .iter()
                .map(|ch| {
                    let quote = ch.trigger_quote.as_deref().unwrap_or("none");
                    format!(
                        "- {}: {} (quote: {})",
                        ch.challenge_name,
                        if ch.should_trigger { "YES" } else { "NO" },
                        quote
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "None".to_string())
}

/// Build the prompt for a specific phase.
pub fn build_phase_prompt(
    phase: LlmPhase,
    base_prompt: &GamePromptRequest,
    phase_outputs: &PhaseOutputs,
) -> String {
    match phase {
        LlmPhase::ContextDigest => {
            // Phase 0 is typically deterministic, but if LLM-based:
            format_context_digest_prompt(base_prompt)
        }
        LlmPhase::ChallengeTriggers => {
            format_challenge_trigger_prompt(base_prompt, phase_outputs.phase0_context.as_ref())
        }
        LlmPhase::EventTriggers => format_event_trigger_prompt(
            base_prompt,
            phase_outputs.phase0_context.as_ref(),
            phase_outputs.phase1_challenges.as_ref(),
        ),
        LlmPhase::ToolIntent => format_tool_intent_prompt(
            base_prompt,
            phase_outputs.phase0_context.as_ref(),
            phase_outputs.phase1_challenges.as_ref(),
            phase_outputs.phase2_events.as_ref(),
        ),
        LlmPhase::BehaviorAnalysis => format_behavior_analysis_prompt(
            base_prompt,
            phase_outputs.phase0_context.as_ref(),
            phase_outputs.phase1_challenges.as_ref(),
            phase_outputs.phase2_events.as_ref(),
            phase_outputs.phase3_tools.as_ref(),
        ),
        LlmPhase::FinalResponse => format_final_response_prompt(
            base_prompt,
            phase_outputs.phase0_context.as_ref(),
            phase_outputs.phase1_challenges.as_ref(),
            phase_outputs.phase2_events.as_ref(),
            phase_outputs.phase3_tools.as_ref(),
            phase_outputs.phase4_behavior.as_ref(),
        ),
    }
}

fn format_context_digest_prompt(prompt: &GamePromptRequest) -> String {
    format!(
        "Create a deterministic context digest for this game state:\n\n\
        SCENE:\n\
        - Location: {}\n\
        - Time: {}\n\
        - Present Characters: {}\n\n\
        PLAYER ACTION:\n\
        - Type: {}\n\
        - Target: {}\n\
        - Dialogue: {}\n\n\
        RESPONDING NPC:\n\
        - Name: {}\n\
        - Archetype: {}\n\
        - Disposition: {}\n\n\
        ACTIVE ELEMENTS:\n\
        - Challenges: {}\n\
        - Events: {}\n\n\
        CONVERSATION HISTORY:\n\
        {}",
        prompt.scene_context.location_name,
        prompt.scene_context.time_context,
        prompt.scene_context.present_characters.join(", "),
        prompt.player_action.action_type,
        prompt.player_action.target.as_deref().unwrap_or("none"),
        prompt.player_action.dialogue.as_deref().unwrap_or("none"),
        prompt.responding_character.name,
        prompt.responding_character.archetype,
        prompt
            .responding_character
            .disposition_toward_player
            .as_deref()
            .unwrap_or("unknown"),
        prompt.active_challenges.len(),
        prompt.active_narrative_events.len(),
        prompt
            .conversation_history
            .iter()
            .rev()
            .take(5)
            .map(|t| { format!("- {}: {}", t.speaker, t.text) })
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn format_challenge_trigger_prompt(
    prompt: &GamePromptRequest,
    context: Option<&Phase0ContextDigest>,
) -> String {
    let context_block = format_context_digest_block(context);
    let challenges_str = prompt
        .active_challenges
        .iter()
        .map(|c| {
            format!(
                "- {} [{}]: {}\n  Skill: {} {}\n  Trigger hints: {}",
                c.name,
                c.id,
                c.description,
                c.skill_name,
                c.difficulty_display,
                c.trigger_hints.format_for_prompt()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        "Analyze whether the player's action triggers any active challenges.\n\n\
        {}\n\n\
        PLAYER DIALOGUE: \"{}\"\n\n\
        ACTIVE CHALLENGES:\n{}\n\n\
        For each challenge, determine if the player's words match any trigger keywords.",
        context_block,
        prompt.player_action.dialogue.as_deref().unwrap_or(""),
        if challenges_str.is_empty() {
            "None"
        } else {
            &challenges_str
        }
    )
}

fn format_event_trigger_prompt(
    prompt: &GamePromptRequest,
    context: Option<&Phase0ContextDigest>,
    challenges: Option<&Phase1ChallengeTriggers>,
) -> String {
    let context_block = format_context_digest_block(context);
    let challenge_results = format_challenge_results(challenges);
    let events_str = prompt
        .active_narrative_events
        .iter()
        .map(|e| {
            format!(
                "- {} [{}]: {}\n  Scene direction: {}\n  Trigger hints: {}",
                e.name,
                e.id,
                e.description,
                e.scene_direction,
                e.trigger_hints.format_for_prompt()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        "Analyze whether the player's action triggers any active narrative events.\n\n\
        {}\n\n\
        PHASE 1 CHALLENGE RESULTS:\n{}\n\n\
        PLAYER DIALOGUE: \"{}\"\n\n\
        ACTIVE NARRATIVE EVENTS:\n{}\n\n\
        Remember: AT MOST ONE event can be YES. Choose the strongest match.",
        context_block,
        challenge_results,
        prompt.player_action.dialogue.as_deref().unwrap_or(""),
        if events_str.is_empty() {
            "None"
        } else {
            &events_str
        }
    )
}

fn format_tool_intent_prompt(
    prompt: &GamePromptRequest,
    context: Option<&Phase0ContextDigest>,
    challenges: Option<&Phase1ChallengeTriggers>,
    events: Option<&Phase2EventTriggers>,
) -> String {
    let context_block = format_context_digest_block(context);
    let triggered_challenges = challenges
        .map(|c| {
            c.challenges
                .iter()
                .filter(|ch| ch.should_trigger)
                .map(|ch| format!("- {} (confidence: {})", ch.challenge_name, ch.confidence))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|s| !s.is_empty());

    let triggered_events = events.and_then(|e| {
        e.selected_event
            .as_ref()
            .map(|name| format!("- {} (selected)", name))
    });

    let motivations_summary = prompt
        .responding_character
        .motivations
        .as_ref()
        .map(|m| {
            let known = if m.known.is_empty() {
                "none".to_string()
            } else {
                m.known
                    .iter()
                    .map(|w| w.description.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let suspected = if m.suspected.is_empty() {
                "none".to_string()
            } else {
                m.suspected
                    .iter()
                    .map(|w| w.description.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let secret = if m.secret.is_empty() {
                "none".to_string()
            } else {
                m.secret
                    .iter()
                    .map(|w| w.description.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!(
                "Known: {}\nSuspected: {}\nSecret: {}",
                known, suspected, secret
            )
        })
        .unwrap_or_else(|| "Known: none\nSuspected: none\nSecret: none".to_string());

    format!(
        "Identify appropriate game tools based on the player's action.\n\n\
        {}\n\n\
        PLAYER DIALOGUE: \"{}\"\n\n\
        NPC: {} (disposition: {})\n\n\
        NPC MOTIVATIONS:\n{}\n\n\
        TRIGGERED CHALLENGES:\n{}\n\n\
        TRIGGERED EVENTS:\n{}\n\n\
        Determine what tools (if any) would be appropriate for the NPC to use.",
        context_block,
        prompt.player_action.dialogue.as_deref().unwrap_or(""),
        prompt.responding_character.name,
        prompt
            .responding_character
            .disposition_toward_player
            .as_deref()
            .unwrap_or("neutral"),
        motivations_summary,
        triggered_challenges.as_deref().unwrap_or("None"),
        triggered_events.as_deref().unwrap_or("None")
    )
}

fn format_behavior_analysis_prompt(
    prompt: &GamePromptRequest,
    context: Option<&Phase0ContextDigest>,
    challenges: Option<&Phase1ChallengeTriggers>,
    events: Option<&Phase2EventTriggers>,
    tools: Option<&Phase3ToolIntent>,
) -> String {
    let context_block = format_context_digest_block(context);
    let challenge_results = format_challenge_results(challenges);
    let event_result = events
        .and_then(|e| e.selected_event.as_ref())
        .map(|name| format!("- {} (selected)", name))
        .unwrap_or_else(|| "None".to_string());
    let has_reward_tool = tools
        .map(|t| t.proposed_tools.iter().any(|tool| tool.name == "give_item"))
        .unwrap_or(false);

    let tool_summary = tools
        .map(|t| {
            if t.proposed_tools.is_empty() {
                "None".to_string()
            } else {
                t.proposed_tools
                    .iter()
                    .map(|tool| format!("- {}: {}", tool.name, tool.description))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        })
        .unwrap_or_else(|| "None".to_string());

    let secret_motivations = prompt
        .responding_character
        .motivations
        .as_ref()
        .map(|m| {
            if m.secret.is_empty() {
                "None".to_string()
            } else {
                m.secret
                    .iter()
                    .map(|s| {
                        format!(
                            "- {} (deflection: {})",
                            s.description, s.deflection_behavior
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        })
        .unwrap_or_else(|| "None".to_string());

    format!(
        "Analyze the NPC's behavioral stance for this response.\n\n\
        {}\n\n\
        PLAYER DIALOGUE: \"{}\"\n\n\
        NPC: {} (archetype: {}, mood: {:?})\n\n\
        PHASE 1 CHALLENGE RESULTS:\n{}\n\n\
        PHASE 2 EVENT RESULT:\n{}\n\n\
        PHASE 3 TOOL INTENTS:\n{}\n\n\
        SECRET MOTIVATIONS:\n{}\n\n\
        REWARD REQUESTED: {}\n\n\
        Determine:\n\
        1. Should the NPC deflect (hide secret motivations)?\n\
        2. What is the emotional stance?\n\
        3. Should mood or disposition change?",
        context_block,
        prompt.player_action.dialogue.as_deref().unwrap_or(""),
        prompt.responding_character.name,
        prompt.responding_character.archetype,
        prompt.responding_character.current_mood,
        challenge_results,
        event_result,
        tool_summary,
        secret_motivations,
        if has_reward_tool { "Yes" } else { "No" }
    )
}

fn format_final_response_prompt(
    prompt: &GamePromptRequest,
    context: Option<&Phase0ContextDigest>,
    challenges: Option<&Phase1ChallengeTriggers>,
    events: Option<&Phase2EventTriggers>,
    tools: Option<&Phase3ToolIntent>,
    behavior: Option<&Phase4BehaviorAnalysis>,
) -> String {
    let context_block = format_context_digest_block(context);
    let triggered_challenges = challenges
        .map(|c| {
            c.challenges
                .iter()
                .filter(|ch| ch.should_trigger)
                .map(|ch| format!("- {}", ch.challenge_name))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|s| !s.is_empty());

    let triggered_event = events.and_then(|e| e.selected_event.as_ref());

    let proposed_tools = tools
        .map(|t| {
            t.proposed_tools
                .iter()
                .map(|tool| format!("- {}: {}", tool.name, tool.description))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|s| !s.is_empty());

    let behavior_guidance = behavior.map(|b| {
        let mut parts = Vec::new();
        if b.should_deflect {
            parts.push(format!(
                "DEFLECTION: {}",
                b.deflection_behavior.as_deref().unwrap_or("Change subject")
            ));
        }
        if let Some(mood) = &b.recommended_mood {
            parts.push(format!("MOOD: {}", mood));
        }
        parts.push(format!("STANCE: {}", b.emotional_stance));
        parts.join("\n")
    });

    format!(
        "Generate the NPC's dialogue response based on all prior analysis.\n\n\
        {}\n\n\
        PLAYER DIALOGUE: \"{}\"\n\n\
        NPC: {} (archetype: {})\n\n\
        APPROVED TRIGGERS:\n\
        Challenges:\n{}\n\n\
        Event: {}\n\n\
        PROPOSED TOOLS:\n{}\n\n\
        BEHAVIOR GUIDANCE:\n{}\n\n\
        DIRECTORIAL NOTES:\n{}\n\n\
        Generate natural dialogue that incorporates the approved decisions.",
        context_block,
        prompt.player_action.dialogue.as_deref().unwrap_or(""),
        prompt.responding_character.name,
        prompt.responding_character.archetype,
        triggered_challenges.as_deref().unwrap_or("None"),
        triggered_event.map(|s| s.as_str()).unwrap_or("None"),
        proposed_tools.as_deref().unwrap_or("None"),
        behavior_guidance.as_deref().unwrap_or("Neutral stance"),
        prompt.directorial_notes
    )
}

/// Generate a stable phase key for VCR fingerprinting.
///
/// The key includes phase number, prompt key, and content hash
/// to ensure stable cassette matching across runs. It also includes
/// a hash of the phase prompt or prior phase outputs to ensure that
/// retries with different upstream outputs produce different keys.
pub fn generate_phase_key(
    phase: LlmPhase,
    base_request: &LlmRequestData,
    prompt: &GamePromptRequest,
    phase_outputs: Option<&PhaseOutputs>,
) -> String {
    let mut hasher = Sha256::new();

    // Include phase identifier
    hasher.update(phase.short_id().as_bytes());
    hasher.update(b":");

    // Include world ID
    hasher.update(base_request.world_id.to_string().as_bytes());
    hasher.update(b":");

    // Include player dialogue (normalized)
    let dialogue = prompt
        .player_action
        .dialogue
        .as_deref()
        .unwrap_or("")
        .to_lowercase();
    hasher.update(dialogue.as_bytes());
    hasher.update(b":");

    // Include NPC name
    hasher.update(prompt.responding_character.name.as_bytes());
    hasher.update(b":");

    // Include hash of phase outputs or prompt content to ensure idempotency
    // reflects the actual phase inputs. This prevents stale phase keys when
    // prior phases are retried and produce different outputs.
    if let Some(outputs) = phase_outputs {
        // Hash the serialized phase outputs that feed into this phase
        match serde_json::to_string(outputs) {
            Ok(json) => {
                hasher.update(json.as_bytes());
            }
            Err(e) => {
                // Fallback: hash the phase prompt content
                let phase_prompt = build_phase_prompt(phase, prompt, outputs);
                hasher.update(phase_prompt.as_bytes());
                tracing::warn!(
                    error = %e,
                    phase = ?phase,
                    "Failed to serialize phase outputs for key generation, using prompt content"
                );
            }
        }
    } else {
        // For Phase 1 (first LLM phase), hash the phase prompt directly
        // since there are no prior phase outputs yet
        let empty_outputs = PhaseOutputs::default();
        let phase_prompt = build_phase_prompt(phase, prompt, &empty_outputs);
        hasher.update(phase_prompt.as_bytes());
    }

    let hash = hex::encode(hasher.finalize());
    format!("{}-{:.16}", phase.short_id(), hash)
}

/// Generate a stable phase key from a precomputed content hash.
///
/// This variant is used when the phase outputs are already serialized
/// and we want to avoid re-serializing them.
pub fn generate_phase_key_with_hash(
    phase: LlmPhase,
    base_request: &LlmRequestData,
    prompt: &GamePromptRequest,
    content_hash: &str,
) -> String {
    let mut hasher = Sha256::new();

    // Include phase identifier
    hasher.update(phase.short_id().as_bytes());
    hasher.update(b":");

    // Include world ID
    hasher.update(base_request.world_id.to_string().as_bytes());
    hasher.update(b":");

    // Include player dialogue (normalized)
    let dialogue = prompt
        .player_action
        .dialogue
        .as_deref()
        .unwrap_or("")
        .to_lowercase();
    hasher.update(dialogue.as_bytes());
    hasher.update(b":");

    // Include NPC name
    hasher.update(prompt.responding_character.name.as_bytes());
    hasher.update(b":");

    // Include the precomputed content hash
    hasher.update(content_hash.as_bytes());

    let hash = hex::encode(hasher.finalize());
    format!("{}-{:.16}", phase.short_id(), hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_context_digest() {
        let prompt = GamePromptRequest {
            world_id: None,
            player_action: crate::llm_context::PlayerActionContext {
                action_type: "speak".to_string(),
                target: Some("NPC".to_string()),
                dialogue: Some("Hello".to_string()),
            },
            scene_context: crate::llm_context::SceneContext {
                scene_name: "Test".to_string(),
                location_name: "Tavern".to_string(),
                time_context: "Evening".to_string(),
                present_characters: vec!["Player".to_string(), "NPC".to_string()],
                region_items: vec![],
            },
            directorial_notes: "Test notes".to_string(),
            conversation_history: vec![],
            responding_character: crate::llm_context::CharacterContext {
                character_id: None,
                name: "NPC".to_string(),
                archetype: "Merchant".to_string(),
                current_mood: Some("neutral".to_string()),
                disposition_toward_player: Some("friendly".to_string()),
                motivations: None,
                social_stance: None,
                relationship_to_player: None,
                available_expressions: None,
                available_actions: None,
            },
            active_challenges: vec![],
            active_narrative_events: vec![],
            context_budget: None,
            scene_id: None,
            location_id: None,
            game_time: None,
        };

        let digest = build_context_digest(&prompt);
        assert!(digest.scene_summary.contains("Tavern"));
        assert!(digest.npc_summary.contains("Merchant"));
    }

    #[test]
    fn test_generate_phase_key_includes_phase_outputs() {
        use crate::queue_types::{
            Phase0ContextDigest, Phase1ChallengeTriggers, PhaseChallengeAnalysis, PhaseOutputs,
        };
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let base_request = LlmRequestData {
            request_type: crate::queue_types::LlmRequestType::NpcResponse {
                action_item_id: wrldbldr_domain::QueueItemId::new(),
            },
            world_id,
            pc_id: None,
            prompt: None,
            suggestion_context: None,
            callback_id: "test".to_string(),
            conversation_id: None,
            temperature: None,
            phase_meta: None,
        };

        let prompt = GamePromptRequest {
            world_id: Some(world_id.to_string()),
            player_action: crate::llm_context::PlayerActionContext {
                action_type: "speak".to_string(),
                target: Some("NPC".to_string()),
                dialogue: Some("Hello".to_string()),
            },
            scene_context: crate::llm_context::SceneContext {
                scene_name: "Test".to_string(),
                location_name: "Tavern".to_string(),
                time_context: "Evening".to_string(),
                present_characters: vec!["Player".to_string(), "NPC".to_string()],
                region_items: vec![],
            },
            directorial_notes: "Test notes".to_string(),
            conversation_history: vec![],
            responding_character: crate::llm_context::CharacterContext {
                character_id: None,
                name: "NPC".to_string(),
                archetype: "Merchant".to_string(),
                current_mood: Some("neutral".to_string()),
                disposition_toward_player: Some("friendly".to_string()),
                motivations: None,
                social_stance: None,
                relationship_to_player: None,
                available_expressions: None,
                available_actions: None,
            },
            active_challenges: vec![],
            active_narrative_events: vec![],
            context_budget: None,
            scene_id: None,
            location_id: None,
            game_time: None,
        };

        // Test with empty phase outputs
        let empty_outputs = PhaseOutputs::default();
        let key_empty = generate_phase_key(
            LlmPhase::ChallengeTriggers,
            &base_request,
            &prompt,
            Some(&empty_outputs),
        );

        // Test with Phase 0 context digest populated
        let mut outputs_with_context = PhaseOutputs::default();
        outputs_with_context.phase0_context = Some(Phase0ContextDigest {
            scene_summary: "Different scene".to_string(),
            player_action_digest: "Different action".to_string(),
            npc_summary: "Different NPC".to_string(),
            active_challenge_count: 5,
            active_event_count: 3,
            recent_topics: vec!["topic1".to_string()],
            directorial_excerpt: Some("Different notes".to_string()),
        });
        let key_with_context = generate_phase_key(
            LlmPhase::ChallengeTriggers,
            &base_request,
            &prompt,
            Some(&outputs_with_context),
        );

        // Keys should be different when phase outputs differ
        assert_ne!(
            key_empty, key_with_context,
            "Phase keys should differ when PhaseOutputs differ"
        );
    }

    #[test]
    fn test_generate_phase_key_different_phase1_outputs_produce_different_keys() {
        use crate::queue_types::{
            Phase0ContextDigest, Phase1ChallengeTriggers, PhaseChallengeAnalysis, PhaseOutputs,
        };
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let base_request = LlmRequestData {
            request_type: crate::queue_types::LlmRequestType::NpcResponse {
                action_item_id: wrldbldr_domain::QueueItemId::new(),
            },
            world_id,
            pc_id: None,
            prompt: None,
            suggestion_context: None,
            callback_id: "test".to_string(),
            conversation_id: None,
            temperature: None,
            phase_meta: None,
        };

        let prompt = GamePromptRequest {
            world_id: Some(world_id.to_string()),
            player_action: crate::llm_context::PlayerActionContext {
                action_type: "speak".to_string(),
                target: Some("NPC".to_string()),
                dialogue: Some("Hello".to_string()),
            },
            scene_context: crate::llm_context::SceneContext {
                scene_name: "Test".to_string(),
                location_name: "Tavern".to_string(),
                time_context: "Evening".to_string(),
                present_characters: vec!["Player".to_string(), "NPC".to_string()],
                region_items: vec![],
            },
            directorial_notes: "Test notes".to_string(),
            conversation_history: vec![],
            responding_character: crate::llm_context::CharacterContext {
                character_id: None,
                name: "NPC".to_string(),
                archetype: "Merchant".to_string(),
                current_mood: Some("neutral".to_string()),
                disposition_toward_player: Some("friendly".to_string()),
                motivations: None,
                social_stance: None,
                relationship_to_player: None,
                available_expressions: None,
                available_actions: None,
            },
            active_challenges: vec![],
            active_narrative_events: vec![],
            context_budget: None,
            scene_id: None,
            location_id: None,
            game_time: None,
        };

        // Build base phase outputs with Phase 0
        let mut base_outputs = PhaseOutputs::default();
        base_outputs.phase0_context = Some(Phase0ContextDigest {
            scene_summary: "Scene".to_string(),
            player_action_digest: "Action".to_string(),
            npc_summary: "NPC".to_string(),
            active_challenge_count: 1,
            active_event_count: 0,
            recent_topics: vec![],
            directorial_excerpt: None,
        });

        // Phase 1 with no triggers
        let mut outputs_no_trigger = base_outputs.clone();
        outputs_no_trigger.phase1_challenges = Some(Phase1ChallengeTriggers {
            challenges: vec![PhaseChallengeAnalysis {
                challenge_id: "ch1".to_string(),
                challenge_name: "Test Challenge".to_string(),
                trigger_quote: None,
                should_trigger: false,
                confidence: "low".to_string(),
                reasoning: "No match".to_string(),
            }],
        });

        // Phase 1 with trigger
        let mut outputs_with_trigger = base_outputs.clone();
        outputs_with_trigger.phase1_challenges = Some(Phase1ChallengeTriggers {
            challenges: vec![PhaseChallengeAnalysis {
                challenge_id: "ch1".to_string(),
                challenge_name: "Test Challenge".to_string(),
                trigger_quote: Some("trigger word".to_string()),
                should_trigger: true,
                confidence: "high".to_string(),
                reasoning: "Match found".to_string(),
            }],
        });

        let key_no_trigger = generate_phase_key(
            LlmPhase::EventTriggers,
            &base_request,
            &prompt,
            Some(&outputs_no_trigger),
        );

        let key_with_trigger = generate_phase_key(
            LlmPhase::EventTriggers,
            &base_request,
            &prompt,
            Some(&outputs_with_trigger),
        );

        // Keys should be different when Phase 1 outputs differ
        assert_ne!(
            key_no_trigger, key_with_trigger,
            "Phase keys should differ when Phase 1 outputs differ"
        );
    }

    #[test]
    fn test_generate_phase_key_same_outputs_produce_same_keys() {
        use crate::queue_types::{Phase0ContextDigest, PhaseOutputs};
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let base_request = LlmRequestData {
            request_type: crate::queue_types::LlmRequestType::NpcResponse {
                action_item_id: wrldbldr_domain::QueueItemId::new(),
            },
            world_id,
            pc_id: None,
            prompt: None,
            suggestion_context: None,
            callback_id: "test".to_string(),
            conversation_id: None,
            temperature: None,
            phase_meta: None,
        };

        let prompt = GamePromptRequest {
            world_id: Some(world_id.to_string()),
            player_action: crate::llm_context::PlayerActionContext {
                action_type: "speak".to_string(),
                target: Some("NPC".to_string()),
                dialogue: Some("Hello".to_string()),
            },
            scene_context: crate::llm_context::SceneContext {
                scene_name: "Test".to_string(),
                location_name: "Tavern".to_string(),
                time_context: "Evening".to_string(),
                present_characters: vec!["Player".to_string(), "NPC".to_string()],
                region_items: vec![],
            },
            directorial_notes: "Test notes".to_string(),
            conversation_history: vec![],
            responding_character: crate::llm_context::CharacterContext {
                character_id: None,
                name: "NPC".to_string(),
                archetype: "Merchant".to_string(),
                current_mood: Some("neutral".to_string()),
                disposition_toward_player: Some("friendly".to_string()),
                motivations: None,
                social_stance: None,
                relationship_to_player: None,
                available_expressions: None,
                available_actions: None,
            },
            active_challenges: vec![],
            active_narrative_events: vec![],
            context_budget: None,
            scene_id: None,
            location_id: None,
            game_time: None,
        };

        // Build identical phase outputs
        let mut outputs1 = PhaseOutputs::default();
        outputs1.phase0_context = Some(Phase0ContextDigest {
            scene_summary: "Scene".to_string(),
            player_action_digest: "Action".to_string(),
            npc_summary: "NPC".to_string(),
            active_challenge_count: 1,
            active_event_count: 0,
            recent_topics: vec!["topic1".to_string(), "topic2".to_string()],
            directorial_excerpt: Some("Notes".to_string()),
        });

        let mut outputs2 = PhaseOutputs::default();
        outputs2.phase0_context = Some(Phase0ContextDigest {
            scene_summary: "Scene".to_string(),
            player_action_digest: "Action".to_string(),
            npc_summary: "NPC".to_string(),
            active_challenge_count: 1,
            active_event_count: 0,
            recent_topics: vec!["topic1".to_string(), "topic2".to_string()],
            directorial_excerpt: Some("Notes".to_string()),
        });

        let key1 = generate_phase_key(
            LlmPhase::ChallengeTriggers,
            &base_request,
            &prompt,
            Some(&outputs1),
        );

        let key2 = generate_phase_key(
            LlmPhase::ChallengeTriggers,
            &base_request,
            &prompt,
            Some(&outputs2),
        );

        // Keys should be identical for identical outputs
        assert_eq!(
            key1, key2,
            "Phase keys should be identical for identical PhaseOutputs"
        );
    }

    #[test]
    fn test_generate_phase_key_different_phases_produce_different_keys() {
        use crate::queue_types::{Phase0ContextDigest, PhaseOutputs};
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let base_request = LlmRequestData {
            request_type: crate::queue_types::LlmRequestType::NpcResponse {
                action_item_id: wrldbldr_domain::QueueItemId::new(),
            },
            world_id,
            pc_id: None,
            prompt: None,
            suggestion_context: None,
            callback_id: "test".to_string(),
            conversation_id: None,
            temperature: None,
            phase_meta: None,
        };

        let prompt = GamePromptRequest {
            world_id: Some(world_id.to_string()),
            player_action: crate::llm_context::PlayerActionContext {
                action_type: "speak".to_string(),
                target: Some("NPC".to_string()),
                dialogue: Some("Hello".to_string()),
            },
            scene_context: crate::llm_context::SceneContext {
                scene_name: "Test".to_string(),
                location_name: "Tavern".to_string(),
                time_context: "Evening".to_string(),
                present_characters: vec!["Player".to_string(), "NPC".to_string()],
                region_items: vec![],
            },
            directorial_notes: "Test notes".to_string(),
            conversation_history: vec![],
            responding_character: crate::llm_context::CharacterContext {
                character_id: None,
                name: "NPC".to_string(),
                archetype: "Merchant".to_string(),
                current_mood: Some("neutral".to_string()),
                disposition_toward_player: Some("friendly".to_string()),
                motivations: None,
                social_stance: None,
                relationship_to_player: None,
                available_expressions: None,
                available_actions: None,
            },
            active_challenges: vec![],
            active_narrative_events: vec![],
            context_budget: None,
            scene_id: None,
            location_id: None,
            game_time: None,
        };

        let mut outputs = PhaseOutputs::default();
        outputs.phase0_context = Some(Phase0ContextDigest {
            scene_summary: "Scene".to_string(),
            player_action_digest: "Action".to_string(),
            npc_summary: "NPC".to_string(),
            active_challenge_count: 1,
            active_event_count: 0,
            recent_topics: vec![],
            directorial_excerpt: None,
        });

        let key_phase1 = generate_phase_key(
            LlmPhase::ChallengeTriggers,
            &base_request,
            &prompt,
            Some(&outputs),
        );

        let key_phase2 = generate_phase_key(
            LlmPhase::EventTriggers,
            &base_request,
            &prompt,
            Some(&outputs),
        );

        // Keys should be different for different phases even with same outputs
        assert_ne!(
            key_phase1, key_phase2,
            "Phase keys should differ for different phases"
        );
        assert!(key_phase1.starts_with("chal-"));
        assert!(key_phase2.starts_with("evt-"));
    }

    #[test]
    fn test_generate_phase_key_with_hash_produces_deterministic_keys() {
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let base_request = LlmRequestData {
            request_type: crate::queue_types::LlmRequestType::NpcResponse {
                action_item_id: wrldbldr_domain::QueueItemId::new(),
            },
            world_id,
            pc_id: None,
            prompt: None,
            suggestion_context: None,
            callback_id: "test".to_string(),
            conversation_id: None,
            temperature: None,
            phase_meta: None,
        };

        let prompt = GamePromptRequest {
            world_id: Some(world_id.to_string()),
            player_action: crate::llm_context::PlayerActionContext {
                action_type: "speak".to_string(),
                target: Some("NPC".to_string()),
                dialogue: Some("Hello".to_string()),
            },
            scene_context: crate::llm_context::SceneContext {
                scene_name: "Test".to_string(),
                location_name: "Tavern".to_string(),
                time_context: "Evening".to_string(),
                present_characters: vec!["Player".to_string(), "NPC".to_string()],
                region_items: vec![],
            },
            directorial_notes: "Test notes".to_string(),
            conversation_history: vec![],
            responding_character: crate::llm_context::CharacterContext {
                character_id: None,
                name: "NPC".to_string(),
                archetype: "Merchant".to_string(),
                current_mood: Some("neutral".to_string()),
                disposition_toward_player: Some("friendly".to_string()),
                motivations: None,
                social_stance: None,
                relationship_to_player: None,
                available_expressions: None,
                available_actions: None,
            },
            active_challenges: vec![],
            active_narrative_events: vec![],
            context_budget: None,
            scene_id: None,
            location_id: None,
            game_time: None,
        };

        let content_hash = "abc123def456";

        let key1 = generate_phase_key_with_hash(
            LlmPhase::ChallengeTriggers,
            &base_request,
            &prompt,
            content_hash,
        );

        let key2 = generate_phase_key_with_hash(
            LlmPhase::ChallengeTriggers,
            &base_request,
            &prompt,
            content_hash,
        );

        // Same inputs should produce same key
        assert_eq!(
            key1, key2,
            "generate_phase_key_with_hash should be deterministic"
        );

        // Different hash should produce different key
        let key3 = generate_phase_key_with_hash(
            LlmPhase::ChallengeTriggers,
            &base_request,
            &prompt,
            "different_hash",
        );

        assert_ne!(
            key1, key3,
            "Different content hashes should produce different keys"
        );
    }
}
