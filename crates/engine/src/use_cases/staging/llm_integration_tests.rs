//! LLM integration tests for staging suggestions.
//!
//! These tests verify that the LLM generates appropriate NPC staging suggestions.
//! Run with: `cargo test -p wrldbldr-engine staging::llm_integration -- --ignored`

use crate::infrastructure::ollama::OllamaClient;
use crate::infrastructure::ports::{ChatMessage, LlmPort, LlmRequest};
use crate::test_fixtures::llm_integration::*;

// =============================================================================
// Staging Suggestion Generation
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_generates_staging_npc_suggestions() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a helpful TTRPG assistant helping decide which NPCs should be present in a scene. \
        Respond with a JSON array of objects, each with 'name' (exact name from the list) and 'reason' (brief explanation). \
        Select 1-4 NPCs that would logically be present. Only include NPCs from the provided list.";

    let user_prompt = "Region: The Tipsy Dragon Tavern (in Riverside District)\n\n\
        Available NPCs:\n\
        1. Marcus the Bartender (works here)\n\
        2. Old Tom (frequents this area)\n\
        3. Sera the Barmaid (works here)\n\
        4. Garrick the Drunk (frequents this area)\n\n\
        Which NPCs should be present? Respond with JSON only.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = client.generate(request).await.expect("LLM request failed");

    // Response should contain JSON array
    assert!(
        response.content.contains('[') && response.content.contains(']'),
        "Response should be a JSON array: {}",
        response.content
    );

    // Should mention at least one of the NPCs
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("marcus")
            || content_lower.contains("tom")
            || content_lower.contains("sera")
            || content_lower.contains("garrick"),
        "Response should include NPC names: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_staging_respects_guidance() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a helpful TTRPG assistant helping decide which NPCs should be present in a scene. \
        Respond with a JSON array of objects, each with 'name' (exact name from the list) and 'reason' (brief explanation). \
        Select 1-4 NPCs that would logically be present. Only include NPCs from the provided list.";

    let user_prompt = "Region: The Tipsy Dragon Tavern (in Riverside District)\n\n\
        Available NPCs:\n\
        1. Marcus the Bartender (works here)\n\
        2. Old Tom (frequents this area)\n\
        3. Sera the Barmaid (works here)\n\
        4. Garrick the Drunk (frequents this area)\n\n\
        DM's guidance: The tavern is empty except for staff - it's early morning.\n\n\
        Which NPCs should be present? Respond with JSON only.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = client.generate(request).await.expect("LLM request failed");

    // With "staff only" guidance, should prefer Marcus and Sera
    let content_lower = response.content.to_lowercase();

    // Extract JSON for parsing
    if let (Some(start), Some(end)) = (response.content.find('['), response.content.rfind(']')) {
        let json_str = &response.content[start..=end];
        if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
            // Should have fewer than all 4 NPCs (early morning, staff only)
            assert!(
                parsed.len() <= 3,
                "Early morning should have fewer NPCs: {:?}",
                parsed
            );
        }
    }

    // At minimum, bartender should likely be present (staff)
    assert!(
        content_lower.contains("marcus") || content_lower.contains("sera"),
        "Staff should be present: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_regeneration_produces_different_results() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a creative TTRPG assistant. Generate evocative scene descriptions.";
    let user_prompt = "Describe the atmosphere in a busy medieval tavern at evening time.";

    let request1 = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.8); // Higher temperature for variety

    let request2 = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.8);

    let response1 = client
        .generate(request1)
        .await
        .expect("First request failed");
    let response2 = client
        .generate(request2)
        .await
        .expect("Second request failed");

    // With temperature > 0, responses should usually differ
    assert!(
        response1.content != response2.content,
        "Regeneration should produce different results"
    );

    // Both should still be about the tavern
    let content1_lower = response1.content.to_lowercase();
    let content2_lower = response2.content.to_lowercase();

    assert!(
        content1_lower.contains("tavern")
            || content1_lower.contains("bar")
            || content1_lower.contains("evening"),
        "First response should be about the tavern"
    );
    assert!(
        content2_lower.contains("tavern")
            || content2_lower.contains("bar")
            || content2_lower.contains("evening"),
        "Second response should be about the tavern"
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_staging_handles_empty_npc_list() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a helpful TTRPG assistant helping decide which NPCs should be present in a scene. \
        Respond with a JSON array of objects, each with 'name' (exact name from the list) and 'reason' (brief explanation). \
        Only include NPCs from the provided list. If no NPCs are available, return an empty array.";

    let user_prompt = "Region: The Abandoned Mine (in Mountain Pass)\n\n\
        Available NPCs:\n(none)\n\n\
        Which NPCs should be present? Respond with JSON only.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = client.generate(request).await.expect("LLM request failed");

    // Should return empty array
    let content_trimmed = response.content.trim();
    assert!(
        content_trimmed.contains("[]") || content_trimmed == "[]",
        "Should return empty array for no NPCs: {}",
        response.content
    );
}

// =============================================================================
// World Setting Consistency
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_staging_respects_fantasy_setting() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master for a high fantasy D&D campaign. \
        When describing scenes, include magical and fantastical elements.";

    let user_prompt = "Describe what the party sees as they enter a wizard's tower study.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = client.generate(request).await.expect("LLM request failed");

    let content_lower = response.content.to_lowercase();

    // Should include magical elements
    assert!(
        content_lower.contains("magic")
            || content_lower.contains("arcane")
            || content_lower.contains("spell")
            || content_lower.contains("tome")
            || content_lower.contains("potion")
            || content_lower.contains("scroll")
            || content_lower.contains("mystical")
            || content_lower.contains("enchant"),
        "Fantasy setting should include magical elements: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_staging_maintains_time_context() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master. Consider the time of day when describing scenes.";
    let user_prompt = "It is midnight. Describe the town square.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = client.generate(request).await.expect("LLM request failed");

    let content_lower = response.content.to_lowercase();

    // Should reflect nighttime
    assert!(
        content_lower.contains("dark")
            || content_lower.contains("night")
            || content_lower.contains("moon")
            || content_lower.contains("torch")
            || content_lower.contains("lantern")
            || content_lower.contains("shadow")
            || content_lower.contains("quiet")
            || content_lower.contains("empty"),
        "Midnight scene should reflect nighttime: {}",
        response.content
    );
}
