//! LLM integration tests for staging suggestions.
//!
//! These tests verify that the LLM generates appropriate NPC staging suggestions.
//! Run with: `cargo test -p wrldbldr-engine staging::llm_integration -- --ignored`

use crate::infrastructure::openai_compatible::OpenAICompatibleClient;
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

    let response = generate_and_log(
        &client,
        request,
        "test_llm_generates_staging_npc_suggestions",
        None,
    )
    .await
    .expect("LLM request failed");

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

    let response = generate_and_log(&client, request, "test_llm_staging_respects_guidance", None)
        .await
        .expect("LLM request failed");

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

    let system_prompt =
        "You are a creative TTRPG assistant. Generate evocative scene descriptions.";
    let user_prompt = "Describe the atmosphere in a busy medieval tavern at evening time.";

    let request1 = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.8); // Higher temperature for variety

    let request2 = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.8);

    let response1 = generate_and_log(
        &client,
        request1,
        "test_llm_regeneration_produces_different_results_1",
        None,
    )
    .await
    .expect("First request failed");
    let response2 = generate_and_log(
        &client,
        request2,
        "test_llm_regeneration_produces_different_results_2",
        None,
    )
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

    let response = generate_and_log(
        &client,
        request,
        "test_llm_staging_handles_empty_npc_list",
        None,
    )
    .await
    .expect("LLM request failed");

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

    let response = generate_and_log(
        &client,
        request,
        "test_llm_staging_respects_fantasy_setting",
        None,
    )
    .await
    .expect("LLM request failed");

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

    let system_prompt =
        "You are a TTRPG game master. Consider the time of day when describing scenes.";
    let user_prompt = "It is midnight. Describe the town square.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = generate_and_log(
        &client,
        request,
        "test_llm_staging_maintains_time_context",
        None,
    )
    .await
    .expect("LLM request failed");

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

// =============================================================================
// Time-of-Day Staging Tests (Thornhaven)
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_morning_suggests_workers_not_patrons() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a helpful TTRPG assistant helping decide which NPCs should be present in a scene. \
        Respond with a JSON array of objects, each with 'name' (exact name from the list) and 'reason' (brief explanation). \
        Select 1-4 NPCs that would logically be present based on time of day and their roles.";

    // Using Thornhaven NPCs
    let user_prompt = "Region: The Drowsy Dragon Inn - Common Room (in Thornhaven Village)\n\
        Time: Early Morning (7 AM)\n\n\
        Available NPCs:\n\
        1. Marta Hearthwood - Innkeeper (works here, day shift)\n\
        2. Grom Ironhand - Blacksmith (works at smithy, frequents inn in evening)\n\
        3. Pip Quickfingers - Street urchin (frequents inn in evening)\n\
        4. Brother Aldric - Priest (frequents inn occasionally)\n\n\
        Which NPCs should be present at this time? Consider their work schedules. Respond with JSON only.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.5);

    let response = generate_and_log(
        &client,
        request,
        "test_morning_suggests_workers_not_patrons",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should include Marta (innkeeper works mornings)
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("marta"),
        "Morning should include innkeeper Marta: {}",
        response.content
    );

    // Should NOT suggest evening frequenters like Pip
    if let (Some(start), Some(end)) = (response.content.find('['), response.content.rfind(']')) {
        let json_str = &response.content[start..=end];
        if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
            let has_evening_only = parsed.iter().any(|npc| {
                npc.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.to_lowercase().contains("pip"))
                    .unwrap_or(false)
            });
            // Pip should NOT be present in the morning
            assert!(
                !has_evening_only,
                "Evening frequenters like Pip should not be suggested for morning: {:?}",
                parsed
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_evening_suggests_patrons_present() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a helpful TTRPG assistant helping decide which NPCs should be present in a scene. \
        Respond with a JSON array of objects, each with 'name' (exact name from the list) and 'reason' (brief explanation). \
        Select 1-4 NPCs that would logically be present based on time of day and their roles.";

    let user_prompt = "Region: The Drowsy Dragon Inn - Common Room (in Thornhaven Village)\n\
        Time: Evening (8 PM) - The busy time when locals gather\n\n\
        Available NPCs:\n\
        1. Marta Hearthwood - Innkeeper (works here, day shift)\n\
        2. Grom Ironhand - Blacksmith (frequents inn in evening after work)\n\
        3. Pip Quickfingers - Street urchin (frequents inn in evening)\n\
        4. Captain Elena Stone - Town guard (frequents inn in evening)\n\n\
        Which NPCs should be present at this time? Respond with JSON only.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.5);

    let response = generate_and_log(
        &client,
        request,
        "test_evening_suggests_patrons_present",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should suggest multiple people including evening frequenters
    if let (Some(start), Some(end)) = (response.content.find('['), response.content.rfind(']')) {
        let json_str = &response.content[start..=end];
        if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
            // Evening should have multiple NPCs
            assert!(
                parsed.len() >= 2,
                "Evening should have multiple NPCs present: {:?}",
                parsed
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_midnight_suggests_reduced_presence() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a helpful TTRPG assistant helping decide which NPCs should be present in a scene. \
        Respond with a JSON array of objects, each with 'name' (exact name from the list) and 'reason' (brief explanation). \
        Consider that most people are asleep at midnight. Very few NPCs should be present.";

    let user_prompt = "Region: Thornhaven Square - The Well (in Thornhaven Village)\n\
        Time: Midnight (12 AM) - Most villagers are asleep\n\n\
        Available NPCs:\n\
        1. Marta Hearthwood - Innkeeper (asleep at this hour)\n\
        2. Captain Elena Stone - Town guard (may patrol at night)\n\
        3. Pip Quickfingers - Street urchin (sleeps rough, might be around)\n\
        4. Brother Aldric - Priest (asleep at temple)\n\n\
        Which NPCs should be present at this late hour? Most should be absent. Respond with JSON only.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.5);

    let response = generate_and_log(
        &client,
        request,
        "test_midnight_suggests_reduced_presence",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should have very few (0-2) NPCs at midnight
    if let (Some(start), Some(end)) = (response.content.find('['), response.content.rfind(']')) {
        let json_str = &response.content[start..=end];
        if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
            assert!(
                parsed.len() <= 2,
                "Midnight should have very few NPCs: {:?}",
                parsed
            );
        }
    }
}

// =============================================================================
// Work Schedule Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_shopkeeper_present_during_business_hours() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a helpful TTRPG assistant helping decide which NPCs should be present in a scene. \
        Respond with a JSON array of objects, each with 'name' (exact name from the list) and 'reason' (brief explanation). \
        NPCs who work at a location should definitely be present during their work hours.";

    let user_prompt = "Region: Ironforge Smithy - Forge Floor (in Thornhaven Village)\n\
        Time: Afternoon (2 PM) - Normal business hours\n\n\
        Available NPCs:\n\
        1. Grom Ironhand - Blacksmith (works here, day shift)\n\
        2. Marta Hearthwood - Innkeeper (works at inn)\n\
        3. Pip Quickfingers - Street urchin (frequents market)\n\n\
        Which NPCs should be present at the smithy during work hours? Respond with JSON only.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_shopkeeper_present_during_business_hours",
        None,
    )
    .await
    .expect("LLM request failed");

    // Grom should definitely be present
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("grom"),
        "Blacksmith Grom should be present at smithy during work hours: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_dm_guidance_overrides_time_rules() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a helpful TTRPG assistant helping decide which NPCs should be present in a scene. \
        Respond with a JSON array of objects, each with 'name' (exact name from the list) and 'reason' (brief explanation). \
        DM guidance takes priority over normal time-based rules.";

    let user_prompt = "Region: The Drowsy Dragon Inn - Common Room (in Thornhaven Village)\n\
        Time: Early Morning (6 AM)\n\n\
        Available NPCs:\n\
        1. Marta Hearthwood - Innkeeper (works here)\n\
        2. Grom Ironhand - Blacksmith (normally at smithy at this hour)\n\
        3. Brother Aldric - Priest (normally at temple at this hour)\n\
        4. Captain Elena Stone - Guard (normally patrolling)\n\n\
        DM's Guidance: An emergency meeting has been called! All the village leaders have gathered \
        at the inn at dawn to discuss a pressing matter. Everyone is present despite the early hour.\n\n\
        Which NPCs should be present? Follow the DM's guidance. Respond with JSON only.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_dm_guidance_overrides_time_rules",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should include most/all NPCs despite early hour
    if let (Some(start), Some(end)) = (response.content.find('['), response.content.rfind(']')) {
        let json_str = &response.content[start..=end];
        if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
            assert!(
                parsed.len() >= 3,
                "DM guidance for emergency meeting should override time rules: {:?}",
                parsed
            );
        }
    }
}

// =============================================================================
// Relationship-Based Staging Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_avoiding_npc_not_suggested() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a helpful TTRPG assistant helping decide which NPCs should be present in a scene. \
        Respond with a JSON array of objects, each with 'name' (exact name from the list) and 'reason' (brief explanation). \
        NPCs who AVOID a location should NOT be suggested for that location.";

    let user_prompt = "Region: The Old Mill - Mill Floor (in Thornhaven Village)\n\
        Time: Afternoon\n\n\
        Available NPCs:\n\
        1. Old Tom - Former miller (AVOIDS this location - traumatic memories)\n\
        2. Captain Elena Stone - Guard (may investigate the area)\n\
        3. Vera Nightshade - Merchant (interested in exploring)\n\n\
        Which NPCs should be present? Do NOT include NPCs who avoid this location. Respond with JSON only.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(&client, request, "test_avoiding_npc_not_suggested", None)
        .await
        .expect("LLM request failed");

    // Old Tom should NOT be suggested
    if let (Some(start), Some(end)) = (response.content.find('['), response.content.rfind(']')) {
        let json_str = &response.content[start..=end];
        if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
            let has_tom = parsed.iter().any(|npc| {
                npc.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.to_lowercase().contains("tom"))
                    .unwrap_or(false)
            });
            assert!(
                !has_tom,
                "NPCs who avoid a location should not be suggested: {:?}",
                parsed
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_home_region_npc_prioritized() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a helpful TTRPG assistant helping decide which NPCs should be present in a scene. \
        Respond with a JSON array of objects, each with 'name' (exact name from the list) and 'reason' (brief explanation). \
        NPCs whose HOME is at a location should have highest priority for being present.";

    let user_prompt = "Region: Temple of the Dawn - Sanctuary (in Thornhaven Village)\n\
        Time: Morning\n\n\
        Available NPCs:\n\
        1. Brother Aldric - Priest (HOME REGION - lives and works here)\n\
        2. Rosie Bramblefoot - Herbalist (works here, healing chamber)\n\
        3. Marta Hearthwood - Innkeeper (might visit for prayers)\n\
        4. Grom Ironhand - Blacksmith (rarely visits temple)\n\n\
        Which NPCs should be present? Prioritize those whose home is here. Respond with JSON only.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(&client, request, "test_home_region_npc_prioritized", None)
        .await
        .expect("LLM request failed");

    // Brother Aldric should be present (home region)
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("aldric"),
        "NPCs with home region should be prioritized: {}",
        response.content
    );
}
