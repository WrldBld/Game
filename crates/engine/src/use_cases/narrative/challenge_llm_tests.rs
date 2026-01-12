//! LLM integration tests for challenge outcome generation.
//!
//! These tests verify that the LLM generates appropriate narrative descriptions
//! for different challenge outcomes (success, failure, critical hits, etc.).
//! Run with: `cargo test -p wrldbldr-engine narrative::challenge_llm -- --ignored`

use crate::infrastructure::ports::{ChatMessage, LlmPort, LlmRequest};
use crate::test_fixtures::llm_integration::*;

// =============================================================================
// Success/Failure Outcomes
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_generates_success_outcome_description() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master. Generate a brief narrative description of a successful skill check outcome. Keep it to 2-3 sentences.";

    let user_prompt = "Stealth check SUCCEEDED. The rogue rolled 18 vs DC 15. \
        Describe them successfully sneaking past the sleeping guards in the barracks.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = client.generate(request).await.expect("LLM request failed");

    let content_lower = response.content.to_lowercase();

    // Should describe successful outcome
    assert!(
        !content_lower.contains("fail")
            && !content_lower.contains("caught")
            && !content_lower.contains("notice")
            && !content_lower.contains("spot"),
        "Success outcome should not describe failure: {}",
        response.content
    );

    // Should be about stealth/sneaking
    assert!(
        content_lower.contains("sneak")
            || content_lower.contains("silent")
            || content_lower.contains("shadow")
            || content_lower.contains("past")
            || content_lower.contains("avoid")
            || content_lower.contains("undetect"),
        "Should describe successful stealth: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_generates_failure_outcome_description() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master. Generate a brief narrative description of a failed skill check outcome. Keep it to 2-3 sentences.";

    let user_prompt = "Stealth check FAILED. The rogue rolled 8 vs DC 15. \
        Describe them failing to sneak past the guards and getting caught.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = client.generate(request).await.expect("LLM request failed");

    let content_lower = response.content.to_lowercase();

    // Should describe failure/getting caught
    assert!(
        content_lower.contains("notice")
            || content_lower.contains("spot")
            || content_lower.contains("caught")
            || content_lower.contains("fail")
            || content_lower.contains("alert")
            || content_lower.contains("hear")
            || content_lower.contains("creak")
            || content_lower.contains("noise"),
        "Failure outcome should describe getting caught: {}",
        response.content
    );
}

// =============================================================================
// Critical Success/Failure
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_generates_critical_success_description() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master. Generate an exciting, dramatic narrative \
        for a CRITICAL SUCCESS (natural 20). This should be more impressive and impactful than a regular success. \
        Keep it to 2-3 sentences.";

    let user_prompt = "CRITICAL SUCCESS! Natural 20 on attack roll. \
        A warrior strikes at a goblin with a greatsword. Describe the devastating critical hit.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.8); // Higher temp for more dramatic response

    let response = client.generate(request).await.expect("LLM request failed");

    let content_lower = response.content.to_lowercase();

    // Should be dramatic/impressive
    assert!(
        content_lower.contains("devastat")
            || content_lower.contains("critical")
            || content_lower.contains("powerful")
            || content_lower.contains("perfect")
            || content_lower.contains("cleave")
            || content_lower.contains("slice")
            || content_lower.contains("fell")
            || content_lower.contains("strike")
            || content_lower.contains("blow"),
        "Critical success should be dramatic: {}",
        response.content
    );

    // Should mention the weapon or attack
    assert!(
        content_lower.contains("sword")
            || content_lower.contains("blade")
            || content_lower.contains("weapon")
            || content_lower.contains("attack")
            || content_lower.contains("strike"),
        "Should reference the attack: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_generates_critical_failure_description() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master. Generate a narrative for a CRITICAL FAILURE \
        (natural 1). This should describe a fumble, mishap, or embarrassing failure. \
        Keep it to 2-3 sentences but make it memorable.";

    let user_prompt = "CRITICAL FAILURE! Natural 1 on attack roll. \
        A warrior swings their greatsword at a goblin. Describe the disastrous fumble.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.8);

    let response = client.generate(request).await.expect("LLM request failed");

    let content_lower = response.content.to_lowercase();

    // Should describe fumble/mishap
    assert!(
        content_lower.contains("fumble")
            || content_lower.contains("miss")
            || content_lower.contains("slip")
            || content_lower.contains("drop")
            || content_lower.contains("stumble")
            || content_lower.contains("fall")
            || content_lower.contains("fail")
            || content_lower.contains("wild")
            || content_lower.contains("lose")
            || content_lower.contains("unfortunate"),
        "Critical failure should describe a fumble: {}",
        response.content
    );
}

// =============================================================================
// Different Skill Types
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_generates_persuasion_success() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master. Generate a brief narrative for a successful social interaction.";

    let user_prompt = "Persuasion check SUCCEEDED. Rolled 17 vs DC 14. \
        The bard is trying to convince the guard captain to let them into the castle after hours.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = client.generate(request).await.expect("LLM request failed");

    let content_lower = response.content.to_lowercase();

    // Should describe successful persuasion
    assert!(
        content_lower.contains("convinc")
            || content_lower.contains("agree")
            || content_lower.contains("nod")
            || content_lower.contains("allow")
            || content_lower.contains("let")
            || content_lower.contains("open")
            || content_lower.contains("gesture"),
        "Should describe successful persuasion: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_generates_athletics_success() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a TTRPG game master. Generate a brief narrative for a successful physical challenge.";

    let user_prompt = "Athletics check SUCCEEDED. Rolled 19 vs DC 16. \
        The fighter is attempting to leap across a 15-foot chasm in a dungeon.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = client.generate(request).await.expect("LLM request failed");

    let content_lower = response.content.to_lowercase();

    // Should describe successful jump
    assert!(
        content_lower.contains("leap")
            || content_lower.contains("jump")
            || content_lower.contains("land")
            || content_lower.contains("cross")
            || content_lower.contains("clear")
            || content_lower.contains("reach")
            || content_lower.contains("safe"),
        "Should describe successful jump: {}",
        response.content
    );
}

// =============================================================================
// Outcome Suggestion Regeneration
// =============================================================================

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_generates_multiple_outcome_suggestions() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a creative TTRPG game master assistant. \
        Generate 3 alternative narrative descriptions for a challenge outcome. \
        Each suggestion should be evocative and fit the fantasy setting. \
        Return each suggestion on a separate line, numbered 1-3.";

    let user_prompt = "Challenge: Stealth check to sneak past guards\n\
        Current outcome description: \"You successfully slip past the guards unnoticed.\"\n\
        Generate 3 alternative descriptions.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.8);

    let response = client.generate(request).await.expect("LLM request failed");

    // Should have multiple suggestions (numbered or on separate lines)
    let lines: Vec<&str> = response
        .content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    assert!(
        lines.len() >= 2,
        "Should have multiple suggestions, got {}: {}",
        lines.len(),
        response.content
    );

    // Each suggestion should be about stealth
    for line in &lines {
        let line_lower = line.to_lowercase();
        // Skip if it's just a number like "1." or "2."
        if line.trim().len() <= 3 {
            continue;
        }
        assert!(
            line_lower.contains("sneak")
                || line_lower.contains("guard")
                || line_lower.contains("silent")
                || line_lower.contains("shadow")
                || line_lower.contains("slip")
                || line_lower.contains("move")
                || line_lower.contains("past")
                || line_lower.contains("stealth"),
            "Suggestion should be about stealth: {}",
            line
        );
    }
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_respects_dm_guidance_for_outcome() {
    let client = create_test_ollama_client();

    let system_prompt = "You are a creative TTRPG game master assistant. \
        Generate alternative narrative descriptions. Consider the DM's guidance.";

    let user_prompt = "Challenge: Athletics check to climb the tower\n\
        Current outcome description: \"You climb the tower successfully.\"\n\
        DM guidance: Make it more dramatic with near-misses and tension.\n\
        Generate an alternative description.";

    let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.8);

    let response = client.generate(request).await.expect("LLM request failed");

    let content_lower = response.content.to_lowercase();

    // Should include dramatic elements as guided
    assert!(
        content_lower.contains("slip")
            || content_lower.contains("grip")
            || content_lower.contains("almost")
            || content_lower.contains("catch")
            || content_lower.contains("moment")
            || content_lower.contains("heart")
            || content_lower.contains("tense")
            || content_lower.contains("barely"),
        "Should include dramatic elements per guidance: {}",
        response.content
    );
}
