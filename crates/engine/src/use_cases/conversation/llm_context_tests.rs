//! LLM context integration tests for dialogue generation.
//!
//! These tests verify that the LLM generates appropriate dialogue based on:
//! - Scene context (location, time of day, present characters)
//! - Character context (mood, disposition, motivations)
//! - Conversation history
//! - Directorial notes
//!
//! Run with: `cargo test -p wrldbldr-engine conversation::llm_context_tests -- --ignored`

use crate::test_fixtures::llm_integration::*;

// =============================================================================
// Scene Context Affects Dialogue
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_npc_responds_with_location_awareness() {
    let client = create_test_ollama_client();

    let request = GamePromptBuilder::new()
        .with_scene_builder(
            SceneContextBuilder::tavern_evening()
                .with_present_character("Player")
                .with_present_character("Marta Hearthwood"),
        )
        .with_character_builder(CharacterContextBuilder::friendly_innkeeper())
        .with_player_dialogue("Marta", "What's this place like?")
        .build_llm_request();

    let response = generate_and_log(
        &client,
        request,
        "test_npc_responds_with_location_awareness",
        None,
    )
    .await
    .expect("LLM request failed");

    // Validate that the response references tavern elements
    assert_llm_valid(
        &client,
        "Ask an innkeeper about her tavern",
        &response.content,
        "The response should reference elements of a tavern/inn (drinks, food, rooms, guests, fire, bar, etc.)",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_npc_responds_with_time_of_day_awareness() {
    let client = create_test_ollama_client();

    // Morning scene - should be quiet
    let morning_request = GamePromptBuilder::new()
        .with_scene_builder(SceneContextBuilder::tavern_morning().with_present_character("Player"))
        .with_character_builder(CharacterContextBuilder::friendly_innkeeper())
        .with_player_dialogue("Marta", "How is business today?")
        .build_llm_request();

    let morning_response = generate_and_log(
        &client,
        morning_request,
        "test_npc_responds_with_time_of_day_awareness",
        Some("morning"),
    )
    .await
    .expect("Morning request failed");

    // Evening scene - should be busy
    let evening_request = GamePromptBuilder::new()
        .with_scene_builder(
            SceneContextBuilder::tavern_evening()
                .with_present_characters(vec!["Player", "Various patrons"]),
        )
        .with_character_builder(CharacterContextBuilder::friendly_innkeeper())
        .with_player_dialogue("Marta", "How is business today?")
        .build_llm_request();

    let evening_response = generate_and_log(
        &client,
        evening_request,
        "test_npc_responds_with_time_of_day_awareness",
        Some("evening"),
    )
    .await
    .expect("Evening request failed");

    // Morning should suggest quieter atmosphere
    assert_llm_valid(
        &client,
        "Ask innkeeper about business in the morning",
        &morning_response.content,
        "The response should suggest morning/quiet atmosphere (quiet, early, preparing, slow, fresh, etc.)",
    )
    .await;

    // Evening should suggest busier atmosphere
    assert_llm_valid(
        &client,
        "Ask innkeeper about business in the evening",
        &evening_response.content,
        "The response should suggest evening/busy atmosphere (busy, crowded, lively, patrons, etc.)",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_npc_references_present_characters() {
    let client = create_test_ollama_client();

    let request = GamePromptBuilder::new()
        .with_scene_builder(
            SceneContextBuilder::tavern_evening()
                .with_present_character("Player")
                .with_present_character("Marta Hearthwood")
                .with_present_character("Grom Ironhand")
                .with_present_character("Brother Aldric"),
        )
        .with_character_builder(CharacterContextBuilder::friendly_innkeeper())
        .with_player_dialogue("Marta", "Who else is here tonight?")
        .build_llm_request();

    let response = generate_and_log(
        &client,
        request,
        "test_npc_references_present_characters",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should mention at least one of the other present characters
    assert_llm_valid(
        &client,
        "Ask who else is present at the tavern",
        &response.content,
        "The response should acknowledge or mention other people being present (Grom, Aldric, blacksmith, priest, or general references to other patrons)",
    )
    .await;
}

// =============================================================================
// Mood and Disposition Affects Response
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_hostile_disposition_produces_unfriendly_response() {
    let client = create_test_ollama_client();

    let request = GamePromptBuilder::new()
        .with_scene_builder(
            SceneContextBuilder::marketplace_morning()
                .with_present_character("Player")
                .with_present_character("Guard Thorne"),
        )
        .with_character_builder(CharacterContextBuilder::hostile_guard())
        .with_player_dialogue("Guard", "Good morning! How are you today?")
        .build_llm_request();

    let response = generate_and_log(
        &client,
        request,
        "test_hostile_disposition_produces_unfriendly_response",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should be unfriendly despite the friendly greeting
    assert_llm_valid(
        &client,
        "Greet a hostile guard cheerfully",
        &response.content,
        "The response should be unfriendly, suspicious, dismissive, or hostile - NOT warm or welcoming",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_friendly_disposition_produces_helpful_response() {
    let client = create_test_ollama_client();

    let request = GamePromptBuilder::new()
        .with_scene_builder(SceneContextBuilder::tavern_evening().with_present_character("Player"))
        .with_character_builder(CharacterContextBuilder::friendly_innkeeper())
        .with_player_dialogue("Marta", "I'm looking for information about the old mill.")
        .build_llm_request();

    let response = generate_and_log(
        &client,
        request,
        "test_friendly_disposition_produces_helpful_response",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should be helpful and willing to share
    assert_llm_valid(
        &client,
        "Ask a friendly innkeeper for information",
        &response.content,
        "The response should be helpful, willing to share information, or at least acknowledge the question positively",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_anxious_mood_affects_dialogue_style() {
    let client = create_test_ollama_client();

    let request = GamePromptBuilder::new()
        .with_scene_builder(
            SceneContextBuilder::new()
                .with_scene_name("By the Old Well")
                .with_location_name("Thornhaven Square")
                .with_time_context("Evening")
                .with_present_character("Player")
                .with_present_character("Old Tom"),
        )
        .with_character_builder(CharacterContextBuilder::traumatized_witness())
        .with_player_dialogue("Tom", "Hello there. Nice evening, isn't it?")
        .build_llm_request();

    let response = generate_and_log(
        &client,
        request,
        "test_anxious_mood_affects_dialogue_style",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should reflect anxiety/trauma
    assert_llm_valid(
        &client,
        "Greet an anxious, traumatized witness",
        &response.content,
        "The response should reflect anxiety, nervousness, or trauma (distracted, mumbling, nervous, fearful, or evasive)",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_calm_vs_angry_mood_comparison() {
    let client = create_test_ollama_client();

    // Calm blacksmith
    let calm_request = GamePromptBuilder::new()
        .with_scene_builder(
            SceneContextBuilder::new()
                .with_scene_name("The Smithy")
                .with_location_name("Ironforge Smithy")
                .with_time_context("Afternoon"),
        )
        .with_character_builder(CharacterContextBuilder::gruff_blacksmith().with_mood("Calm"))
        .with_player_dialogue("Grom", "I need a sword repaired.")
        .build_llm_request();

    let calm_response = generate_and_log(
        &client,
        calm_request,
        "test_calm_vs_angry_mood_comparison",
        Some("calm"),
    )
    .await
    .expect("Calm request failed");

    // Angry blacksmith
    let angry_request = GamePromptBuilder::new()
        .with_scene_builder(
            SceneContextBuilder::new()
                .with_scene_name("The Smithy")
                .with_location_name("Ironforge Smithy")
                .with_time_context("Afternoon"),
        )
        .with_character_builder(CharacterContextBuilder::gruff_blacksmith().with_mood("Angry"))
        .with_player_dialogue("Grom", "I need a sword repaired.")
        .build_llm_request();

    let angry_response = generate_and_log(
        &client,
        angry_request,
        "test_calm_vs_angry_mood_comparison",
        Some("angry"),
    )
    .await
    .expect("Angry request failed");

    // Calm should be professional
    assert_llm_valid(
        &client,
        "Request service from a calm blacksmith",
        &calm_response.content,
        "The response should be professional, businesslike, or at least not overtly hostile",
    )
    .await;

    // Angry should be more aggressive
    assert_llm_valid(
        &client,
        "Request service from an angry blacksmith",
        &angry_response.content,
        "The response should reflect anger or irritation (gruff, short, dismissive, or aggressive tone)",
    )
    .await;
}

// =============================================================================
// Conversation History Referenced
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_references_previous_conversation_topic() {
    let client = create_test_ollama_client();

    let request = GamePromptBuilder::new()
        .with_scene_builder(
            SceneContextBuilder::tavern_evening().with_present_character("Player"),
        )
        .with_character_builder(CharacterContextBuilder::friendly_innkeeper())
        .with_conversation_history(vec![
            ("Player", "Have you heard anything about strange happenings at the old mill?"),
            ("Marta Hearthwood", "Aye, there's been talk. Old Tom saw something there years ago that changed him forever."),
            ("Player", "What did he see?"),
        ])
        .with_player_dialogue("Marta", "Tell me more about Old Tom.")
        .build_llm_request();

    let response = generate_and_log(
        &client,
        request,
        "test_references_previous_conversation_topic",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should connect to the previous conversation about the mill
    assert_llm_valid(
        &client,
        "Continue a conversation about Old Tom and the mill",
        &response.content,
        "The response should relate to the previous conversation context (the mill, what Tom saw, or the strange happenings)",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_maintains_character_knowledge_from_history() {
    let client = create_test_ollama_client();

    let request = GamePromptBuilder::new()
        .with_scene_builder(
            SceneContextBuilder::tavern_evening().with_present_character("Player"),
        )
        .with_character_builder(CharacterContextBuilder::friendly_innkeeper())
        .with_conversation_history(vec![
            ("Player", "My name is Tharion. I'm a fighter from the northern lands."),
            ("Marta Hearthwood", "Welcome to the Drowsy Dragon, Tharion! We don't get many fighters from the north these days."),
        ])
        .with_player_dialogue("Marta", "What kind of food do you have?")
        .build_llm_request();

    let response = generate_and_log(
        &client,
        request,
        "test_maintains_character_knowledge_from_history",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should ideally remember the player's name or origin (but this is a soft test)
    // At minimum, should maintain the friendly rapport established
    assert_llm_valid(
        &client,
        "Continue conversation with established rapport",
        &response.content,
        "The response should be friendly and welcoming, maintaining the established rapport (may optionally use the player's name or reference their origin)",
    )
    .await;
}

// =============================================================================
// Directorial Notes Followed
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_directorial_notes_affect_response_style() {
    let client = create_test_ollama_client();

    // Evasive directorial notes
    let request = GamePromptBuilder::new()
        .with_scene_builder(
            SceneContextBuilder::marketplace_morning().with_present_character("Player"),
        )
        .with_character_builder(CharacterContextBuilder::mysterious_merchant())
        .with_player_dialogue("Vera", "I heard you're interested in the old mill. What do you know about it?")
        .with_directorial_notes(
            "Be evasive and deflect questions about the mill. Change the subject to your merchandise.",
        )
        .build_llm_request();

    let response = generate_and_log(
        &client,
        request,
        "test_directorial_notes_affect_response_style",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should be evasive per directorial notes
    assert_llm_valid(
        &client,
        "Ask a merchant about the mill with 'be evasive' direction",
        &response.content,
        "The response should be evasive, deflecting, or redirect to merchandise rather than directly answering about the mill",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_directorial_notes_reveal_information() {
    let client = create_test_ollama_client();

    let request = GamePromptBuilder::new()
        .with_scene_builder(
            SceneContextBuilder::temple_morning()
                .with_present_character("Player")
                .with_present_character("Brother Aldric"),
        )
        .with_character_builder(CharacterContextBuilder::quest_giving_priest())
        .with_player_dialogue("Brother Aldric", "What troubles you, holy one?")
        .with_directorial_notes(
            "This is the moment to reveal the quest. Express deep concern about a growing darkness, \
             mention nightmares about the old mill, and ask the player for help investigating.",
        )
        .build_llm_request();

    let response = generate_and_log(
        &client,
        request,
        "test_directorial_notes_reveal_information",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should reveal quest information per directorial notes
    assert_llm_valid(
        &client,
        "Priest reveals quest with 'dramatic revelation' direction",
        &response.content,
        "The response should express concern or worry, mention darkness/danger/the mill/nightmares, and include a request for help or investigation",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_directorial_notes_response_length() {
    let client = create_test_ollama_client();

    let request = GamePromptBuilder::new()
        .with_scene_builder(
            SceneContextBuilder::mill_night().with_present_character("Player"),
        )
        .with_character_builder(
            CharacterContextBuilder::traumatized_witness().with_mood("Terrified"),
        )
        .with_player_dialogue("Tom", "What happened here?")
        .with_directorial_notes(
            "Respond with only a brief, cryptic warning. One or two words at most. You are too terrified to say more.",
        )
        .build_llm_request();

    let response = generate_and_log(
        &client,
        request,
        "test_directorial_notes_response_length",
        None,
    )
    .await
    .expect("LLM request failed");

    // Response should be very brief
    assert_llm_valid(
        &client,
        "Ask terrified witness with 'brief cryptic' direction",
        &response.content,
        "The response should be very brief (a few words or short phrase), cryptic, and suggest terror",
    )
    .await;
}

// =============================================================================
// Featured Characters and Secret Motivations
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_npc_with_secret_motivation_shows_tells() {
    let client = create_test_ollama_client();

    // Ask the merchant about artifacts - should trigger her secret tell
    let request = GamePromptBuilder::new()
        .with_scene_builder(
            SceneContextBuilder::marketplace_morning()
                .with_present_character("Player")
                .with_present_character("Vera Nightshade"),
        )
        .with_character_builder(CharacterContextBuilder::mysterious_merchant())
        .with_player_dialogue("Vera", "Do you deal in magical artifacts or ancient relics?")
        .with_directorial_notes(
            "The player has touched on your secret interest. Show subtle tells but maintain your cover.",
        )
        .build_llm_request();

    let response = generate_and_log(
        &client,
        request,
        "test_npc_with_secret_motivation_shows_tells",
        None,
    )
    .await
    .expect("LLM request failed");

    // Should show interest but maintain cover
    assert_llm_valid(
        &client,
        "Ask merchant with secret agenda about artifacts",
        &response.content,
        "The response should show some interest in the topic (artifacts/relics) while possibly being evasive or redirecting, suggesting hidden knowledge",
    )
    .await;
}
