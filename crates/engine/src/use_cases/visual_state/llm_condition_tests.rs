//! LLM condition integration tests for visual state soft rules.
//!
//! These tests verify that the LLM correctly evaluates "soft rules" -
//! custom conditions that require LLM interpretation to determine if
//! a visual state should be active.
//!
//! Run with: `cargo test -p wrldbldr-engine visual_state::llm_condition_tests -- --ignored`

use crate::infrastructure::ports::{ChatMessage, LlmPort, LlmRequest};
use crate::test_fixtures::llm_integration::*;

// =============================================================================
// Soft Rule Evaluation
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_evaluates_weather_condition() {
    let client = create_test_ollama_client();

    let system_prompt = "You are evaluating a visual state condition for a TTRPG game. \
        Determine if the condition is met based on the game context. \
        Respond with JSON: {\"condition_met\": true/false, \"confidence\": 0.0-1.0, \"reasoning\": \"explanation\"}";

    let user_prompt = "Visual State: 'Rainy Day' for Thornhaven Square\n\
        Condition: 'The weather should feel rainy and dreary'\n\n\
        Current Context:\n\
        - Location: Thornhaven Square\n\
        - Time: Afternoon\n\
        - Recent events: Storm clouds have been building all morning\n\
        - DM notes: A light rain has started falling\n\n\
        Is this condition met?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_evaluates_weather_condition",
        None,
    )
    .await
    .expect("LLM request failed");

    // With rain mentioned, condition should be met
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("true") || content_lower.contains("met"),
        "Weather condition should be met when rain is present: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_evaluates_crowd_condition() {
    let client = create_test_ollama_client();

    let system_prompt = "You are evaluating a visual state condition for a TTRPG game. \
        Determine if the condition is met based on the game context. \
        Respond with JSON: {\"condition_met\": true/false, \"confidence\": 0.0-1.0, \"reasoning\": \"explanation\"}";

    let user_prompt = "Visual State: 'Busy Market' for Thornhaven Square\n\
        Condition: 'The market should feel crowded and busy with merchants and shoppers'\n\n\
        Current Context:\n\
        - Location: Thornhaven Square - Market Stalls\n\
        - Time: Mid-morning\n\
        - Day: Market day (weekly)\n\
        - Present NPCs: 5 merchants, many unnamed villagers\n\
        - Scene notes: Merchants are calling out their wares, villagers haggling\n\n\
        Is this condition met?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_evaluates_crowd_condition",
        None,
    )
    .await
    .expect("LLM request failed");

    // Market day with merchants present should be busy
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("true") || content_lower.contains("met"),
        "Crowd condition should be met on busy market day: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_low_confidence_uses_default_state() {
    let client = create_test_ollama_client();

    let system_prompt = "You are evaluating a visual state condition for a TTRPG game. \
        Determine if the condition is met based on the game context. \
        If you're uncertain, indicate low confidence. \
        Respond with JSON: {\"condition_met\": true/false, \"confidence\": 0.0-1.0, \"reasoning\": \"explanation\"}";

    let user_prompt = "Visual State: 'Magical Disturbance' for Temple of the Dawn\n\
        Condition: 'There should be signs of magical instability or divine interference'\n\n\
        Current Context:\n\
        - Location: Temple of the Dawn - Sanctuary\n\
        - Time: Afternoon\n\
        - No specific events mentioned\n\
        - No DM notes about magic\n\
        - Normal day at the temple\n\n\
        Is this condition met? Be honest about uncertainty.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_low_confidence_uses_default_state",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should show low confidence or false due to lack of evidence
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("false")
            || content_lower.contains("uncertain")
            || content_lower.contains("not met")
            || content_lower.contains("0.")
            || content_lower.contains("low"),
        "Ambiguous condition should show low confidence or be false: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_item_possession_condition() {
    let client = create_test_ollama_client();

    let system_prompt = "You are evaluating a visual state condition for a TTRPG game. \
        Determine if the condition is met based on the game context. \
        Respond with JSON: {\"condition_met\": true/false, \"confidence\": 0.0-1.0, \"reasoning\": \"explanation\"}";

    let user_prompt = "Visual State: 'Locket Glow' for The Old Mill - Basement\n\
        Condition: 'The player possesses Sarah's Locket, which glows in the presence of shadow creatures'\n\n\
        Current Context:\n\
        - Location: The Old Mill - Basement\n\
        - Player inventory includes: Sarah's Locket, torch, rope, sword\n\
        - Locket description: 'A tarnished silver locket that faintly glows in the presence of shadow creatures'\n\
        - Scene: Player is exploring the dark basement\n\n\
        Is this condition met (does the player have the locket)?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_item_possession_condition",
        None,
    )
    .await
    .expect("LLM request failed");

    // Player has the locket, condition should be met
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("true") || content_lower.contains("met"),
        "Item possession condition should be met when player has item: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_relationship_condition() {
    let client = create_test_ollama_client();

    let system_prompt = "You are evaluating a visual state condition for a TTRPG game. \
        Determine if the condition is met based on the game context. \
        Respond with JSON: {\"condition_met\": true/false, \"confidence\": 0.0-1.0, \"reasoning\": \"explanation\"}";

    let user_prompt = "Visual State: 'Grom's Warm Welcome' for Ironforge Smithy\n\
        Condition: 'Grom considers the player a friend and is willing to share stories'\n\n\
        Current Context:\n\
        - Location: Ironforge Smithy - Forge Floor\n\
        - NPC: Grom Ironhand\n\
        - Current relationship with player: Friend (was previously Stranger, then Acquaintance)\n\
        - Disposition: Friendly\n\
        - Recent events: Player helped defend the smithy from bandits\n\n\
        Is this relationship condition met?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_relationship_condition",
        None,
    )
    .await
    .expect("LLM request failed");

    // Friend relationship should meet condition
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("true") || content_lower.contains("met"),
        "Relationship condition should be met when NPC is a friend: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_narrative_progress_condition() {
    let client = create_test_ollama_client();

    let system_prompt = "You are evaluating a visual state condition for a TTRPG game. \
        Determine if the condition is met based on the game context. \
        Respond with JSON: {\"condition_met\": true/false, \"confidence\": 0.0-1.0, \"reasoning\": \"explanation\"}";

    let user_prompt = "Visual State: 'Village Under Threat' for Thornhaven Square\n\
        Condition: 'The village feels tense because the shadow threat has become public knowledge'\n\n\
        Current Context:\n\
        - Location: Thornhaven Square\n\
        - Story progress: Act 2 - Into the Darkness\n\
        - World flags set: 'shadow_threat_public', 'mill_investigation_started'\n\
        - Recent events: Brother Aldric warned the villagers about the danger\n\
        - Villager mood: Worried, speaking in hushed tones\n\n\
        Is this narrative condition met?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_narrative_progress_condition",
        None,
    )
    .await
    .expect("LLM request failed");

    // With shadow_threat_public flag and worried villagers, condition should be met
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("true") || content_lower.contains("met"),
        "Narrative progress condition should be met when story flags are set: {}",
        response.content
    );
}

// =============================================================================
// Complex Condition Combinations
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_combined_time_and_flag_condition() {
    let client = create_test_ollama_client();

    let system_prompt = "You are evaluating a visual state condition for a TTRPG game. \
        The state requires BOTH conditions to be met (AND logic). \
        Respond with JSON: {\"condition_met\": true/false, \"confidence\": 0.0-1.0, \"reasoning\": \"explanation\"}";

    // Shadow Stirring requires: night time AND shadow_awakened flag
    let user_prompt = "Visual State: 'Shadow Stirring' for The Old Mill\n\
        Conditions (ALL must be met):\n\
        1. Time must be Night\n\
        2. 'shadow_awakened' flag must be set\n\n\
        Current Context:\n\
        - Location: The Old Mill - Mill Floor\n\
        - Time: Night (11 PM)\n\
        - World flags set: 'shadow_awakened', 'mill_basement_discovered'\n\n\
        Are BOTH conditions met?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_combined_time_and_flag_condition",
        None,
    )
    .await
    .expect("LLM request failed");

    // Both conditions are met
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("true") || content_lower.contains("met"),
        "Combined conditions should be met when all requirements satisfied: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_partial_condition_fails_all_logic() {
    let client = create_test_ollama_client();

    let system_prompt = "You are evaluating a visual state condition for a TTRPG game. \
        The state requires ALL conditions to be met (AND logic). \
        If any condition is not met, the overall evaluation should be false. \
        Respond with JSON: {\"condition_met\": true/false, \"confidence\": 0.0-1.0, \"reasoning\": \"explanation\"}";

    // Shadow Stirring requires: night time AND shadow_awakened flag
    let user_prompt = "Visual State: 'Shadow Stirring' for The Old Mill\n\
        Conditions (ALL must be met):\n\
        1. Time must be Night\n\
        2. 'shadow_awakened' flag must be set\n\n\
        Current Context:\n\
        - Location: The Old Mill - Mill Floor\n\
        - Time: Afternoon (2 PM) - NOT night!\n\
        - World flags set: 'shadow_awakened', 'mill_basement_discovered'\n\n\
        Are BOTH conditions met?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_partial_condition_fails_all_logic",
        None,
    )
    .await
    .expect("LLM request failed");

    // One condition not met, should fail
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("false") || content_lower.contains("not met"),
        "Partial conditions should fail when using ALL logic: {}",
        response.content
    );
}
