//! LLM tool call integration tests.
//!
//! These tests verify that the LLM makes appropriate decisions about:
//! - When to suggest giving items (trust-based)
//! - When to reveal information (trust-based)
//! - When to suggest relationship changes
//!
//! Run with: `cargo test -p wrldbldr-engine narrative::llm_tool_tests -- --ignored`

use crate::infrastructure::ports::{ChatMessage, LlmPort, LlmRequest};
use crate::test_fixtures::llm_integration::*;

// =============================================================================
// Trust-Based Item Giving
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_friendly_npc_may_give_item() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG assistant helping decide NPC actions. \
        NPCs may give items to players they trust. Friendly NPCs with good relationships \
        are more likely to give items. \
        Respond with JSON: {\"action\": \"give_item\" or \"withhold\", \"reason\": \"why\", \"item\": \"item name if giving\"}";

    let user_prompt = "NPC: Marta Hearthwood (Innkeeper)\n\
        Disposition toward player: Friendly\n\
        Relationship: Acquaintance (positive interactions)\n\
        Scene: Player has just helped save a villager from bandits\n\n\
        Player asks: 'I need supplies for my journey. Can you help me?'\n\n\
        Should Marta offer an item to the player?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_friendly_npc_may_give_item",
        None,
    )
    .await
    .expect("LLM request failed");

    // Friendly NPC with positive relationship should be willing to help
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("give")
            || content_lower.contains("offer")
            || content_lower.contains("provide")
            || content_lower.contains("help"),
        "Friendly NPC should be willing to give items: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_hostile_npc_withholds_items() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG assistant helping decide NPC actions. \
        NPCs may give items to players they trust. Hostile or suspicious NPCs \
        are unlikely to give items to strangers. \
        Respond with JSON: {\"action\": \"give_item\" or \"withhold\", \"reason\": \"why\"}";

    let user_prompt = "NPC: Guard Thorne (Town Guard)\n\
        Disposition toward player: Hostile\n\
        Relationship: Stranger (suspicious of outsiders)\n\
        Scene: The guard confronted the player at the town gate\n\n\
        Player asks: 'I need a weapon to defend myself on the road. Can you spare one?'\n\n\
        Should the guard give an item to the player?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_hostile_npc_withholds_items",
        None,
    )
    .await
    .expect("LLM request failed");

    // Hostile NPC should withhold
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("withhold")
            || content_lower.contains("refuse")
            || content_lower.contains("deny")
            || content_lower.contains("no")
            || content_lower.contains("suspicious"),
        "Hostile NPC should withhold items: {}",
        response.content
    );
}

// =============================================================================
// Information Revelation
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_trusted_npc_reveals_information() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG assistant helping decide if an NPC should reveal information. \
        NPCs reveal information based on trust level. \
        Respond with JSON: {\"action\": \"reveal\" or \"conceal\", \"reason\": \"why\", \"info_level\": \"full/partial/none\"}";

    let user_prompt = "NPC: Marta Hearthwood (Innkeeper)\n\
        Disposition: Friendly\n\
        Relationship: Friend (helped her many times)\n\
        Knowledge: Knows about the old mill's dark history\n\n\
        Player asks: 'Marta, what really happened at the old mill all those years ago?'\n\n\
        Should Marta reveal what she knows?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_trusted_npc_reveals_information",
        None,
    )
    .await
    .expect("LLM request failed");

    // Trusted NPC should be willing to reveal
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("reveal")
            || content_lower.contains("tell")
            || content_lower.contains("share")
            || content_lower.contains("full"),
        "Trusted NPC should reveal information: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_secret_information_protected() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG assistant helping decide if an NPC should reveal information. \
        NPCs protect SECRET information even from friendly players unless there's a strong reason. \
        Secret motivations should be concealed with deflection behavior. \
        Respond with JSON: {\"action\": \"reveal\" or \"conceal\", \"reason\": \"why\", \"deflection\": \"how they avoid the topic\"}";

    let user_prompt = "NPC: Vera Nightshade (Mysterious Merchant)\n\
        Disposition: Friendly (surface level)\n\
        Relationship: Acquaintance\n\
        SECRET: She's searching for the Shadowheart Stone for her employers\n\
        Deflection behavior: Redirects conversation to merchandise\n\n\
        Player asks: 'You seem very interested in the old mill. What's your real interest in this village?'\n\n\
        Should Vera reveal her secret motivation?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_secret_information_protected",
        None,
    )
    .await
    .expect("LLM request failed");

    // Secret should be protected
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("conceal")
            || content_lower.contains("deflect")
            || content_lower.contains("avoid")
            || content_lower.contains("redirect")
            || content_lower.contains("secret"),
        "Secret information should be protected: {}",
        response.content
    );
}

// =============================================================================
// Relationship Changes
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_positive_interaction_improves_relationship() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG assistant helping track relationship changes. \
        When players have positive interactions with NPCs, relationships may improve. \
        Respond with JSON: {\"relationship_change\": \"improve/worsen/unchanged\", \"new_level\": \"level\", \"reason\": \"why\"}";

    let user_prompt = "NPC: Brother Aldric (Priest)\n\
        Current relationship: Acquaintance\n\
        Current disposition: Respectful\n\n\
        The player just completed a quest for the temple, defeating a threat to the village \
        and bringing back a sacred artifact. Brother Aldric is deeply grateful.\n\n\
        How should this affect the relationship?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_positive_interaction_improves_relationship",
        None,
    )
    .await
    .expect("LLM request failed");

    // Positive action should improve relationship
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("improve")
            || content_lower.contains("friend")
            || content_lower.contains("ally")
            || content_lower.contains("better")
            || content_lower.contains("increase"),
        "Positive interaction should improve relationship: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_insulting_player_worsens_relationship() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG assistant helping track relationship changes. \
        When players have negative interactions with NPCs, relationships may worsen. \
        Respond with JSON: {\"relationship_change\": \"improve/worsen/unchanged\", \"new_level\": \"level\", \"reason\": \"why\"}";

    let user_prompt = "NPC: Grom Ironhand (Blacksmith)\n\
        Current relationship: Acquaintance\n\
        Current disposition: Neutral\n\n\
        The player just publicly accused Grom of being a coward, mocking his refusal to \
        talk about his adventuring past. Other villagers witnessed this insult.\n\n\
        How should this affect the relationship?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.3);

    let response = generate_and_log(
        &client,
        request,
        "test_insulting_player_worsens_relationship",
        None,
    )
    .await
    .expect("LLM request failed");

    // Insulting action should worsen relationship
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("worsen")
            || content_lower.contains("worse")
            || content_lower.contains("hostile")
            || content_lower.contains("enemy")
            || content_lower.contains("decrease")
            || content_lower.contains("angry"),
        "Insulting interaction should worsen relationship: {}",
        response.content
    );
}

// =============================================================================
// Context-Aware Tool Decisions
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_npc_tool_decision_considers_scene() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG assistant helping decide what tools/actions an NPC should use \
        in response to a player. Consider the scene context when making decisions. \
        Available tools: give_item, reveal_information, change_disposition, trigger_challenge, custom_action. \
        Respond with JSON: {\"tools\": [list of tool names], \"reasoning\": \"why these tools\"}";

    let user_prompt = "NPC: Brother Aldric (Herald archetype, quest-giver)\n\
        Scene: Temple of the Dawn at morning, during prayers\n\
        Context: A darkness is threatening the village, Aldric has been having nightmares\n\
        Disposition: Respectful\n\n\
        Player says: 'Brother Aldric, you look troubled. Is something wrong?'\n\n\
        This is a dramatic moment where Aldric should reveal the quest. \
        What tools should be suggested for his response?";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.5);

    let response = generate_and_log(
        &client,
        request,
        "test_npc_tool_decision_considers_scene",
        None,
    )
    .await
    .expect("LLM request failed");

    // Quest-giver in dramatic moment should reveal information
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("reveal")
            || content_lower.contains("information")
            || content_lower.contains("quest"),
        "Quest-giver in dramatic moment should reveal information: {}",
        response.content
    );
}
