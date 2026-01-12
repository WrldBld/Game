//! LLM integration tests using Ollama.
//!
//! These tests require a running Ollama instance with the configured model.
//! Run with: `cargo test -p wrldbldr-engine llm_integration -- --ignored`

use crate::infrastructure::ports::{ChatMessage, LlmPort, LlmRequest, ToolDefinition};
use crate::test_fixtures::llm_integration::*;

// =============================================================================
// Basic Narrative Generation
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_generates_narrative_response() {
    let client = create_test_ollama_client();

    let request = build_request_with_system(
        DM_SYSTEM_PROMPT,
        "I look around the tavern and take in my surroundings.",
    );

    let response = generate_and_log(
        &client,
        request,
        "test_llm_generates_narrative_response",
        None,
    )
    .await
    .expect("LLM request failed");

    // Verify we got a non-empty narrative response
    assert!(
        !response.content.is_empty(),
        "Response should not be empty"
    );
    assert!(
        response.content.len() > 20,
        "Response should be descriptive (got {} chars)",
        response.content.len()
    );

    // The response should be parseable text (not binary or corrupted)
    assert!(
        response.content.chars().all(|c| !c.is_control() || c.is_whitespace()),
        "Response should be readable text"
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_responds_to_simple_action() {
    let client = create_test_ollama_client();

    let task = "I approach the bartender and wave.";
    let request = build_request_with_system(DM_SYSTEM_PROMPT, task);

    let response = generate_and_log(
        &client,
        request,
        "test_llm_responds_to_simple_action",
        None,
    )
    .await
    .expect("LLM request failed");

    assert!(!response.content.is_empty());

    // Use semantic validation instead of keyword matching
    assert_llm_valid(
        &client,
        task,
        &response.content,
        "The response should describe a bartender acknowledging or reacting to someone approaching and waving at them in a tavern setting"
    ).await;
}

// =============================================================================
// Skill Check Suggestions
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_suggests_skill_check_for_lockpicking() {
    let client = create_test_ollama_client();

    let task = "The player says: 'I want to pick the lock on this door.' What skill check should they make?";
    let request = build_request_with_system(MECHANICS_SYSTEM_PROMPT, task);

    let response = generate_and_log(
        &client,
        request,
        "test_llm_suggests_skill_check_for_lockpicking",
        None,
    )
    .await
    .expect("LLM request failed");

    // Use semantic validation - should suggest an appropriate skill for lockpicking
    assert_llm_valid(
        &client,
        task,
        &response.content,
        "The response should suggest a Dexterity-based check, Sleight of Hand, or mention thieves' tools - any skill appropriate for picking a lock in D&D 5e"
    ).await;
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_suggests_skill_check_for_persuasion() {
    let client = create_test_ollama_client();

    let task = "The player says: 'I try to convince the guard to let us through.' What skill check?";
    let request = build_request_with_system(MECHANICS_SYSTEM_PROMPT, task);

    let response = generate_and_log(
        &client,
        request,
        "test_llm_suggests_skill_check_for_persuasion",
        None,
    )
    .await
    .expect("LLM request failed");

    // Use semantic validation - should suggest a social/Charisma skill
    assert_llm_valid(
        &client,
        task,
        &response.content,
        "The response should suggest a Charisma-based check like Persuasion, Deception, or Intimidation - any social skill appropriate for convincing someone in D&D 5e"
    ).await;
}

// =============================================================================
// Context and History
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_handles_conversation_history() {
    let client = create_test_ollama_client();

    let request = build_request_with_history(
        Some(DM_SYSTEM_PROMPT),
        vec![
            ("user", "I enter the tavern. What do I see?"),
            ("assistant", "The tavern is dimly lit with flickering candles. A gruff bartender polishes glasses behind the counter, while a hooded figure sits alone in the corner."),
            ("user", "I approach the hooded figure cautiously."),
        ],
    );

    let response = generate_and_log(
        &client,
        request,
        "test_llm_handles_conversation_history",
        None,
    )
    .await
    .expect("LLM request failed");

    let content_lower = response.content.to_lowercase();

    // Should reference the hooded figure from context
    assert!(
        content_lower.contains("hooded")
            || content_lower.contains("figure")
            || content_lower.contains("approach")
            || content_lower.contains("corner"),
        "Response should reference conversation context: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_maintains_character_context() {
    let client = create_test_ollama_client();

    let system = "You are a DM for a D&D 5e game. The player is Elara, a level 3 High Elf Wizard \
                  with high Intelligence. Respond narratively to their actions.";

    let request = build_request_with_system(system, "I examine the magical runes on the wall.");

    let response = generate_and_log(
        &client,
        request,
        "test_llm_maintains_character_context",
        None,
    )
    .await
    .expect("LLM request failed");

    // Response should be relevant to a wizard examining magic
    assert!(!response.content.is_empty());
    // Wizard context should influence the response
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("rune")
            || content_lower.contains("magic")
            || content_lower.contains("arcane")
            || content_lower.contains("symbol")
            || content_lower.contains("glyph"),
        "Response should be about magical examination"
    );
}

// =============================================================================
// Tool Calling
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_tool_call_for_skill_check() {
    let client = create_test_ollama_client();

    let tools = vec![ToolDefinition {
        name: "request_skill_check".to_string(),
        description: "Request a skill check from a player".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "skill": {
                    "type": "string",
                    "description": "The skill to check (e.g., 'Stealth', 'Perception', 'Athletics')"
                },
                "dc": {
                    "type": "integer",
                    "description": "Difficulty Class for the check"
                }
            },
            "required": ["skill"]
        }),
    }];

    let request = build_request_with_system(
        "You are a D&D game master. When players attempt actions that require checks, \
         use the request_skill_check tool.",
        "I try to sneak past the sleeping guards.",
    );

    let response = client
        .generate_with_tools(request, tools)
        .await
        .expect("LLM request failed");

    // The model should either return tool calls or narrative
    // Not all models support tool calling reliably, so we accept either
    if !response.tool_calls.is_empty() {
        let tool_call = &response.tool_calls[0];
        assert_eq!(tool_call.name, "request_skill_check");

        // Verify the arguments are valid JSON
        assert!(
            tool_call.arguments.is_object(),
            "Tool arguments should be an object"
        );

        // If skill is provided, it should be stealth-related
        if let Some(skill) = tool_call.arguments.get("skill") {
            let skill_str = skill.as_str().unwrap_or("").to_lowercase();
            assert!(
                skill_str.contains("stealth") || skill_str.contains("dexterity"),
                "Skill should be stealth-related, got: {}",
                skill_str
            );
        }
    } else {
        // Model responded with narrative instead of tool call
        // This is acceptable - verify it's a valid response
        assert!(
            !response.content.is_empty(),
            "Should have either tool calls or content"
        );
    }
}

// =============================================================================
// Error Handling
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_handles_empty_prompt() {
    let client = create_test_ollama_client();

    let request = LlmRequest::new(vec![ChatMessage::user("")])
        .with_temperature(0.7)
        .with_max_tokens(Some(500));

    // Should handle empty prompt gracefully (either error or empty response)
    let result = generate_and_log(
        &client,
        request,
        "test_llm_handles_empty_prompt",
        None,
    )
    .await;

    // Either succeeds with some response or fails with an error
    // Both are acceptable behaviors
    match result {
        Ok(_response) => {
            // Empty response is fine for empty input
            // Or model might say something like "I didn't receive a message"
        }
        Err(e) => {
            // Error is also acceptable for invalid input
            println!("Expected error for empty prompt: {}", e);
        }
    }
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_timeout_with_short_duration() {
    // Create client with very short timeout
    let client = create_test_ollama_client_with_timeout(1); // 1 second

    let request = build_request_with_system(
        "You are a creative writer. Write a detailed story.",
        "Write a 1000-word story about a dragon.",
    );

    // This should likely timeout given the short duration
    let result = generate_and_log(
        &client,
        request,
        "test_llm_timeout_with_short_duration",
        None,
    )
    .await;

    // Either times out (error) or somehow completes (unlikely but valid)
    match result {
        Ok(_) => {
            // Model was unexpectedly fast - still valid
        }
        Err(e) => {
            // Expected timeout error
            let error_str = e.to_string().to_lowercase();
            assert!(
                error_str.contains("timeout")
                    || error_str.contains("timed out")
                    || error_str.contains("deadline")
                    || error_str.contains("request failed"),
                "Error should indicate timeout: {}",
                e
            );
        }
    }
}

// =============================================================================
// Challenge Outcome Generation
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_generates_success_outcome() {
    let client = create_test_ollama_client();

    let system = "You are a TTRPG game master. Generate a brief narrative description of a successful skill check outcome.";
    let user = "Stealth check succeeded. The rogue rolled 18 vs DC 15. Describe sneaking past guards.";

    let request = build_request_with_system(system, user);
    let response = generate_and_log(
        &client,
        request,
        "test_llm_generates_success_outcome",
        None,
    )
    .await
    .expect("LLM request failed");

    let content_lower = response.content.to_lowercase();

    // Should describe successful sneaking
    assert!(
        content_lower.contains("sneak")
            || content_lower.contains("silent")
            || content_lower.contains("unnoticed")
            || content_lower.contains("past")
            || content_lower.contains("shadow"),
        "Response should describe successful stealth: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_generates_failure_outcome() {
    let client = create_test_ollama_client();

    let system = "You are a TTRPG game master. Generate a brief narrative description of a failed skill check outcome.";
    let user = "Stealth check failed. The rogue rolled 8 vs DC 15. Describe getting caught by guards.";

    let request = build_request_with_system(system, user);
    let response = generate_and_log(
        &client,
        request,
        "test_llm_generates_failure_outcome",
        None,
    )
    .await
    .expect("LLM request failed");

    let content_lower = response.content.to_lowercase();

    // Should describe failure/getting caught
    assert!(
        content_lower.contains("notice")
            || content_lower.contains("spot")
            || content_lower.contains("caught")
            || content_lower.contains("alert")
            || content_lower.contains("guard")
            || content_lower.contains("fail"),
        "Response should describe failed stealth: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_generates_critical_success_outcome() {
    let client = create_test_ollama_client();

    let task = "CRITICAL SUCCESS! Natural 20 on attack roll. A warrior strikes at a goblin with a greatsword.";
    let system = "You are a TTRPG game master. Generate an exciting narrative for a critical success (natural 20).";

    let request = build_request_with_system(system, task);
    let response = generate_and_log(
        &client,
        request,
        "test_llm_generates_critical_success_outcome",
        None,
    )
    .await
    .expect("LLM request failed");

    // Use semantic validation for dramatic content
    assert_llm_valid(
        &client,
        task,
        &response.content,
        "The response should be a dramatic, exciting description of a critical hit - conveying power, impact, or devastation of a perfectly executed attack"
    ).await;
}

// =============================================================================
// Staging/Suggestion Regeneration
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_generates_different_suggestions() {
    let client = create_test_ollama_client();

    let system = "You are a creative TTRPG game master. Generate evocative scene descriptions.";
    let user = "Describe the party entering a mysterious forest clearing at twilight.";

    // Generate two responses with same prompt
    let request1 = build_request_with_system(system, user);
    let request2 = build_request_with_system(system, user);

    let response1 = generate_and_log(
        &client,
        request1,
        "test_llm_generates_different_suggestions_1",
        None,
    )
    .await
    .expect("First LLM request failed");
    let response2 = generate_and_log(
        &client,
        request2,
        "test_llm_generates_different_suggestions_2",
        None,
    )
    .await
    .expect("Second LLM request failed");

    // With temperature > 0, responses should usually be different
    // (This is probabilistic, but with temp 0.7 should be quite different)
    assert!(
        response1.content != response2.content,
        "Two generations should produce different text (for variety)"
    );
}
