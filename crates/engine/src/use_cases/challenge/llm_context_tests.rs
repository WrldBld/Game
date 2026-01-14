//! LLM context integration tests for challenge outcomes.
//!
//! These tests verify that the LLM generates appropriate:
//! - Skill check suggestions based on player actions
//! - Outcome narratives that reference scene context
//! - DC suggestions based on difficulty
//!
//! Run with: `cargo test -p wrldbldr-engine challenge::llm_context_tests -- --ignored`

use crate::infrastructure::ports::{ChatMessage, LlmPort, LlmRequest};
use crate::test_fixtures::llm_integration::*;

// =============================================================================
// Context-Appropriate Skill Suggestions
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_suggests_stealth_for_sneaking() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master assistant. When a player describes an action, \
        suggest the most appropriate skill check. Respond with JSON: {\"skill\": \"SkillName\", \"reason\": \"why\"}";

    let user_prompt =
        "The player says: 'I want to quietly sneak past the guards at the mill entrance.'\n\
        Scene: The Old Mill at night, guards are patrolling.\n\n\
        What skill check should be called for?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(&client, request, "test_suggests_stealth_for_sneaking", None)
        .await
        .expect("LLM request failed");

    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("stealth")
            || content_lower.contains("sneak")
            || content_lower.contains("dexterity"),
        "Sneaking should suggest stealth/dexterity check: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_suggests_persuasion_for_negotiation() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master assistant. When a player describes an action, \
        suggest the most appropriate skill check. Respond with JSON: {\"skill\": \"SkillName\", \"reason\": \"why\"}";

    let user_prompt =
        "The player says: 'I want to convince Grom the blacksmith to share what he knows \
        about his adventuring past.'\n\
        Scene: Ironforge Smithy, afternoon. Grom is working at his forge.\n\n\
        What skill check should be called for?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_suggests_persuasion_for_negotiation",
        None,
    )
    .await
    .expect("LLM request failed");

    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("persuasion")
            || content_lower.contains("charisma")
            || content_lower.contains("diplomacy")
            || content_lower.contains("intimidation"),
        "Convincing someone should suggest persuasion/charisma: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_suggests_investigation_for_searching() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master assistant. When a player describes an action, \
        suggest the most appropriate skill check. Respond with JSON: {\"skill\": \"SkillName\", \"reason\": \"why\"}";

    let user_prompt =
        "The player says: 'I carefully search the mill basement for any hidden compartments \
        or clues about the ritual that was performed here.'\n\
        Scene: The Old Mill basement, dark and dusty with old symbols on the floor.\n\n\
        What skill check should be called for?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_suggests_investigation_for_searching",
        None,
    )
    .await
    .expect("LLM request failed");

    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("investigation")
            || content_lower.contains("perception")
            || content_lower.contains("intelligence")
            || content_lower.contains("search"),
        "Searching for clues should suggest investigation: {}",
        response.content
    );
}

// =============================================================================
// Scene Affects DC Suggestion
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_scene_affects_dc_suggestion() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master assistant. Suggest a difficulty class (DC) for \
        skill checks based on the context. Normal tasks are DC 10-12, moderate are DC 13-15, \
        hard are DC 16-18, very hard are DC 19-22. Respond with JSON: {\"dc\": number, \"reason\": \"why\"}";

    // Easy context - calm NPC, friendly relationship
    let easy_prompt =
        "The player wants to ask Marta the innkeeper (friendly, talkative) for general \
        information about the village. She's been welcoming to the party.\n\
        Scene: The Drowsy Dragon Inn, evening, relaxed atmosphere.\n\n\
        What DC would you suggest for this Persuasion check?";

    let easy_request = LlmRequest::new(vec![ChatMessage::user(easy_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let easy_response = generate_and_log(
        &client,
        easy_request,
        "test_scene_affects_dc_suggestion",
        Some("easy"),
    )
    .await
    .expect("Easy request failed");

    // Hard context - hostile guard, night, suspicious
    let hard_prompt = "The player wants to convince a suspicious night guard to let them pass into \
        a restricted area of town. The guard is hostile to strangers and takes their duty seriously.\n\
        Scene: Town gate at night, guard is suspicious.\n\n\
        What DC would you suggest for this Persuasion check?";

    let hard_request = LlmRequest::new(vec![ChatMessage::user(hard_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let hard_response = generate_and_log(
        &client,
        hard_request,
        "test_scene_affects_dc_suggestion",
        Some("hard"),
    )
    .await
    .expect("Hard request failed");

    // Parse DCs from responses
    let easy_dc = extract_dc_from_response(&easy_response.content);
    let hard_dc = extract_dc_from_response(&hard_response.content);

    // Hard scenario should have higher DC than easy
    if let (Some(easy), Some(hard)) = (easy_dc, hard_dc) {
        assert!(
            hard > easy,
            "Hard context DC ({}) should be higher than easy context DC ({})",
            hard,
            easy
        );
    } else {
        // If we can't parse DCs, at least verify the responses contain reasonable numbers
        assert!(
            easy_response.content.contains("10")
                || easy_response.content.contains("11")
                || easy_response.content.contains("12"),
            "Easy context should suggest low DC: {}",
            easy_response.content
        );
    }
}

// =============================================================================
// Outcome Narratives Reference Location
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_success_narrative_references_location() {
    let client = create_test_ollama_client();

    let system_prompt =
        "You are a TTRPG game master. Generate a brief, evocative success narrative \
        for the challenge outcome. Reference specific elements of the scene and location.";

    let user_prompt = "Challenge: 'Investigate the Mill Basement'\n\
        Location: The Old Mill - Basement (dark, dusty, old ritual symbols on floor)\n\
        Check: Intelligence (Investigation) DC 14\n\
        Result: SUCCESS (rolled 18)\n\n\
        Generate the success outcome narrative.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = generate_and_log(
        &client,
        request,
        "test_success_narrative_references_location",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should reference location elements
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("basement")
            || content_lower.contains("mill")
            || content_lower.contains("dust")
            || content_lower.contains("symbol")
            || content_lower.contains("ritual")
            || content_lower.contains("floor"),
        "Success narrative should reference location elements: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_failure_narrative_uses_scene_hazards() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master. Generate a brief failure narrative for the \
        challenge outcome. Use elements of the scene to explain what went wrong.";

    let user_prompt = "Challenge: 'Sneak Past the Guards'\n\
        Location: The Old Mill - Mill Floor (creaky floorboards, broken machinery, moonlight through gaps)\n\
        Check: Dexterity (Stealth) DC 15\n\
        Result: FAILURE (rolled 8)\n\n\
        Generate the failure outcome narrative using scene elements.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = generate_and_log(
        &client,
        request,
        "test_failure_narrative_uses_scene_hazards",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should reference scene hazards that caused failure
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("creak")
            || content_lower.contains("floor")
            || content_lower.contains("noise")
            || content_lower.contains("machinery")
            || content_lower.contains("moonlight")
            || content_lower.contains("shadow")
            || content_lower.contains("step"),
        "Failure narrative should use scene hazards: {}",
        response.content
    );
}

// =============================================================================
// Critical Outcomes Are Dramatic
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_critical_success_is_dramatically_appropriate() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master. Generate a CRITICAL SUCCESS narrative. \
        This should be exceptionally dramatic and rewarding - the character exceeded expectations \
        in a spectacular way. Include bonus effects or revelations.";

    let user_prompt = "Challenge: 'Convince Grom to Share His Past'\n\
        Location: Ironforge Smithy - Forge Floor\n\
        Check: Charisma (Persuasion) DC 18\n\
        Result: CRITICAL SUCCESS (natural 20, total 25)\n\n\
        Generate a dramatic critical success narrative with bonus rewards.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = generate_and_log(
        &client,
        request,
        "test_critical_success_is_dramatically_appropriate",
        None,
    )
    .await
    .expect("LLM request failed");

    // Validate dramatic qualities
    assert_llm_valid(
        &client,
        "Critical success on convincing a blacksmith to share his past",
        &response.content,
        "The response should be dramatic and include bonus rewards or exceptional revelations beyond the normal success",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_critical_failure_causes_scene_appropriate_mishap() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master. Generate a CRITICAL FAILURE narrative. \
        This should be dramatically bad - something went very wrong in a way fitting the scene. \
        Include consequences but keep it fun.";

    let user_prompt = "Challenge: 'Investigate the Mill Basement'\n\
        Location: The Old Mill - Basement (dark, ancient wards, ritual circle)\n\
        Check: Intelligence (Investigation) DC 14\n\
        Result: CRITICAL FAILURE (natural 1, total 5)\n\n\
        Generate a dramatic critical failure narrative with scene-appropriate consequences.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = generate_and_log(
        &client,
        request,
        "test_critical_failure_causes_scene_appropriate_mishap",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should include scene-appropriate mishap
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("ward")
            || content_lower.contains("trigger")
            || content_lower.contains("shadow")
            || content_lower.contains("ritual")
            || content_lower.contains("trap")
            || content_lower.contains("magic")
            || content_lower.contains("stumble")
            || content_lower.contains("disturb"),
        "Critical failure should include scene-appropriate mishap: {}",
        response.content
    );
}

// =============================================================================
// Thornhaven Challenge Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_calming_tom_uses_wisdom_check() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master assistant. When a player describes an action, \
        suggest the most appropriate skill check. Consider the nature of the task - calming someone \
        traumatized requires insight and empathy. \
        Respond with JSON: {\"skill\": \"SkillName\", \"ability\": \"AbilityScore\", \"reason\": \"why\"}";

    let user_prompt = "The player says: 'I try to calm Old Tom down and get him to focus enough \
        to tell me what happened at the mill twenty years ago.'\n\
        Scene: Thornhaven Square by the well. Old Tom is mumbling and agitated.\n\
        Context: Tom is traumatized by what he witnessed. He needs someone to help him focus.\n\n\
        What skill check should be called for?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(&client, request, "test_calming_tom_uses_wisdom_check", None)
        .await
        .expect("LLM request failed");

    let content_lower = response.content.to_lowercase();
    // Should suggest wisdom-based check (Insight, Medicine, or general Wisdom)
    assert!(
        content_lower.contains("wisdom")
            || content_lower.contains("insight")
            || content_lower.contains("medicine")
            || content_lower.contains("empathy")
            || content_lower.contains("wis"),
        "Calming traumatized person should suggest wisdom check: {}",
        response.content
    );
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Extract DC number from LLM response
fn extract_dc_from_response(response: &str) -> Option<i32> {
    // Try to parse JSON first
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
        if let Some(dc) = json.get("dc").and_then(|v| v.as_i64()) {
            return Some(dc as i32);
        }
    }

    // Try to find JSON embedded in the response
    if let (Some(start), Some(end)) = (response.find('{'), response.rfind('}')) {
        let json_str = &response[start..=end];
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(dc) = json.get("dc").and_then(|v| v.as_i64()) {
                return Some(dc as i32);
            }
        }
    }

    // Fallback: look for "DC" followed by a number
    for word in response.split_whitespace() {
        if let Some(stripped) = word.strip_prefix("DC") {
            if let Ok(dc) = stripped.trim_matches(|c: char| !c.is_ascii_digit()).parse() {
                return Some(dc);
            }
        }
        if word.starts_with(|c: char| c.is_ascii_digit()) {
            if let Ok(dc) = word
                .trim_matches(|c: char| !c.is_ascii_digit())
                .parse::<i32>()
            {
                if (5..=30).contains(&dc) {
                    return Some(dc);
                }
            }
        }
    }

    None
}
