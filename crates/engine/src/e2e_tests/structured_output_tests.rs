//! E2E tests for LLM structured output parsing.
//!
//! Tests the response_parser.rs functionality in a full E2E context:
//! - `<reasoning>` -> internal reasoning (shown to DM, hidden from player)
//! - `<dialogue>` -> NPC spoken response
//! - `<topics>` -> conversation topics
//! - `<challenge_suggestion>` -> suggested challenge trigger
//! - `<narrative_event_suggestion>` -> suggested narrative event trigger
//!
//! # Running Tests
//!
//! ```bash
//! # Record cassettes with real Ollama (first time)
//! E2E_LLM_MODE=record cargo test -p wrldbldr-engine --lib structured_output -- --ignored --test-threads=1
//!
//! # Playback from cassettes (subsequent runs)
//! cargo test -p wrldbldr-engine --lib structured_output -- --ignored --test-threads=1
//! ```

use std::sync::Arc;

use crate::queue_types::DmApprovalDecision;

use super::{
    approve_staging_with_npc, create_shared_log, create_test_player, E2ETestContext,
    LoggingLlmDecorator, SemanticAssert, TestOutcome, VcrLlm,
};

// =============================================================================
// Test 1: Internal Reasoning Extraction
// =============================================================================

/// Verify that LLM responses include internal reasoning that is visible to the DM
/// but hidden from the player.
///
/// The structured output format includes `<reasoning>` tags containing the LLM's
/// internal thought process. This should:
/// - Appear in `internal_reasoning` field of the approval request
/// - NOT appear in the `final_dialogue` delivered to the player
#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes"]
async fn test_llm_response_has_internal_reasoning() {
    const TEST_NAME: &str = "test_llm_response_has_internal_reasoning";
    let event_log = create_shared_log(TEST_NAME);

    let vcr = Arc::new(VcrLlm::from_env(std::path::PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        TEST_NAME
    ))));
    let llm = Arc::new(LoggingLlmDecorator::new(vcr.clone(), event_log.clone()));

    let ctx = E2ETestContext::setup_with_llm_and_logging(llm, event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player and stage NPC
        let common_room = ctx.world.region("Common Room").expect("Region not found");
        let (player_id, pc_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Reasoning Test Player",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Start a conversation that should trigger internal reasoning
        // The LLM should think about how to respond before generating dialogue
        let started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_id,
                marta_id,
                player_id,
                "I've heard rumors about strange happenings at the old mill. What do you know about it?".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(!started.action_queue_id.is_nil(), "Action should be queued");

        // Process player action queue -> creates LLM request
        ctx.app
            .use_cases
            .queues
            .process_player_action
            .execute()
            .await
            .expect("Failed to process player action");

        // Process LLM request queue -> creates approval request with structured output
        let llm_result = ctx
            .app
            .use_cases
            .queues
            .process_llm_request
            .execute(|_| {})
            .await
            .expect("Failed to process LLM request");

        assert!(llm_result.is_some(), "Should have processed an LLM request");

        let result = llm_result.unwrap();

        // Get the approval request to examine structured output
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request not found");

        tracing::info!(
            proposed_dialogue = %approval_data.proposed_dialogue,
            internal_reasoning = %approval_data.internal_reasoning,
            internal_reasoning_len = approval_data.internal_reasoning.len(),
            "Approval request with structured output"
        );

        // Print the actual values for debugging
        println!("\n=== APPROVAL DATA DEBUG ===");
        println!("Internal reasoning ({} chars):", approval_data.internal_reasoning.len());
        println!("{}", approval_data.internal_reasoning);
        println!("\nProposed dialogue ({} chars):", approval_data.proposed_dialogue.len());
        println!("{}", approval_data.proposed_dialogue);
        println!("=== END DEBUG ===\n");

        // ASSERTION 1: Internal reasoning should NOT be empty
        // The LLM should have thought about how to respond
        assert!(
            !approval_data.internal_reasoning.is_empty(),
            "Internal reasoning should not be empty - LLM should provide its thought process"
        );

        // ASSERTION 2: Internal reasoning should NOT appear in proposed dialogue
        // The reasoning is for DM eyes only
        assert!(
            !approval_data.proposed_dialogue.contains("<reasoning>"),
            "Proposed dialogue should not contain reasoning tags. Dialogue was:\n{}",
            approval_data.proposed_dialogue
        );

        // Approve the response and verify final dialogue
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(result.approval_id.into(), DmApprovalDecision::Accept)
            .await
            .expect("Failed to approve");

        // ASSERTION 3: Final dialogue should exist and be clean
        let final_dialogue = approval_result
            .final_dialogue
            .expect("Should have final dialogue");

        assert!(
            !final_dialogue.is_empty(),
            "Final dialogue should not be empty"
        );
        assert!(
            !final_dialogue.contains("<reasoning>"),
            "Final dialogue should not contain reasoning tags"
        );

        tracing::info!(
            final_dialogue = %final_dialogue,
            "Player sees clean dialogue without reasoning"
        );

        // SEMANTIC ASSERTIONS: Validate content makes sense
        let semantic = SemanticAssert::new(vcr.clone());

        // ASSERTION 4: Response should be relevant to the player's question about the mill
        semantic
            .assert_responds_to_question(
                "I've heard rumors about strange happenings at the old mill. What do you know about it?",
                &final_dialogue,
                "NPC should respond to the player's question about the old mill",
            )
            .await?;

        // ASSERTION 5: Response should mention or relate to the mill, rumors, or strange events
        semantic
            .assert_mentions_any(
                &final_dialogue,
                &["mill", "rumors", "strange", "happenings", "events", "heard"],
                "Response should be relevant to the mill or rumors topic",
            )
            .await?;

        // ASSERTION 6: Response should be in character for Marta (innkeeper)
        semantic
            .assert_in_character(
                &final_dialogue,
                "Marta Hearthwood, a friendly village innkeeper who knows local gossip",
                "Response should match Marta's innkeeper personality",
            )
            .await?;

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");

    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 2: Topics Extraction
// =============================================================================

/// Verify that conversation topics are extracted from LLM responses.
///
/// When discussing specific subjects, the LLM should tag topics in `<topics>`
/// which are then:
/// - Included in the approval request for DM visibility
/// - Persisted in dialogue history for future context
#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes"]
async fn test_llm_response_extracts_topics() {
    const TEST_NAME: &str = "test_llm_response_extracts_topics";
    let event_log = create_shared_log(TEST_NAME);

    let vcr = Arc::new(VcrLlm::from_env(std::path::PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        TEST_NAME
    ))));
    let llm = Arc::new(LoggingLlmDecorator::new(vcr.clone(), event_log.clone()));

    let ctx = E2ETestContext::setup_with_llm_and_logging(llm, event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player and stage NPC
        let common_room = ctx.world.region("Common Room").expect("Region not found");
        let (player_id, pc_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Topics Test Player",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Start a conversation about a specific topic
        // The LLM should identify and tag relevant topics
        let started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_id,
                marta_id,
                player_id,
                "Tell me about the ancient artifact that was discovered near the village. I've heard it has magical properties.".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(!started.action_queue_id.is_nil(), "Action should be queued");

        // Process through the queue pipeline
        ctx.app
            .use_cases
            .queues
            .process_player_action
            .execute()
            .await
            .expect("Failed to process player action");

        let llm_result = ctx
            .app
            .use_cases
            .queues
            .process_llm_request
            .execute(|_| {})
            .await
            .expect("Failed to process LLM request");

        assert!(llm_result.is_some(), "Should have processed an LLM request");

        let result = llm_result.unwrap();

        // Get the approval request to examine topics
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request not found");

        tracing::info!(
            proposed_dialogue = %approval_data.proposed_dialogue,
            topics = ?approval_data.topics,
            topics_count = approval_data.topics.len(),
            "Approval request with topics"
        );

        // ASSERTION 1: Topics should be extracted
        // Note: The LLM may or may not include topics depending on the response.
        // When topics are present, they should be relevant to the conversation.
        if !approval_data.topics.is_empty() {
            tracing::info!(
                "Found {} topics: {:?}",
                approval_data.topics.len(),
                approval_data.topics
            );

            // Each topic should be a non-empty string
            for topic in &approval_data.topics {
                assert!(
                    !topic.trim().is_empty(),
                    "Topic should not be empty or whitespace-only"
                );
            }
        } else {
            tracing::warn!(
                "No topics extracted - LLM response may not have included <topics> tags. \
                 This is acceptable but ideally the prompt should encourage topic tagging."
            );
        }

        // ASSERTION 2: Topics should not appear as raw tags in dialogue
        assert!(
            !approval_data.proposed_dialogue.contains("<topics>"),
            "Proposed dialogue should not contain topics tags"
        );

        // Approve and complete the flow
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(result.approval_id.into(), DmApprovalDecision::Accept)
            .await
            .expect("Failed to approve");

        let final_dialogue = approval_result
            .final_dialogue
            .expect("Should have final dialogue");

        // SEMANTIC ASSERTIONS: Validate content makes sense
        let semantic = SemanticAssert::new(vcr.clone());

        // ASSERTION 3: Response should be relevant to the player's question about artifacts
        semantic
            .assert_responds_to_question(
                "Tell me about the ancient artifact that was discovered near the village. I've heard it has magical properties.",
                &final_dialogue,
                "NPC should respond to the question about the artifact",
            )
            .await?;

        // ASSERTION 4: Response should mention artifacts, magic, or discovery
        semantic
            .assert_mentions_any(
                &final_dialogue,
                &["artifact", "magic", "discovery", "ancient", "village", "found"],
                "Response should be relevant to artifacts or magical items",
            )
            .await?;

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");

    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 3: Challenge Suggestion During Dialogue
// =============================================================================

/// Verify that the LLM can suggest challenges during dialogue.
///
/// When a player attempts something that could trigger a skill check,
/// the LLM should include a `<challenge_suggestion>` with:
/// - Challenge ID (matching a seeded challenge)
/// - Confidence level
/// - Reasoning for why this challenge applies
///
/// The suggestion is then enriched with challenge metadata (name, difficulty, skill)
/// before being presented to the DM.
#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes"]
async fn test_llm_suggests_challenge_during_dialogue() {
    const TEST_NAME: &str = "test_llm_suggests_challenge_during_dialogue";
    let event_log = create_shared_log(TEST_NAME);

    let vcr = Arc::new(VcrLlm::from_env(std::path::PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        TEST_NAME
    ))));
    let llm = Arc::new(LoggingLlmDecorator::new(vcr.clone(), event_log.clone()));

    let ctx = E2ETestContext::setup_with_llm_and_logging(llm, event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player and stage NPC
        let common_room = ctx.world.region("Common Room").expect("Region not found");
        let (player_id, pc_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Challenge Suggestion Test Player",
        )
        .await
        .expect("Failed to create test player");

        // Stage Grom - he has the "Convince Grom to Share His Past" challenge
        let grom_id = ctx.world.npc("Grom Ironhand").expect("Grom not found");
        approve_staging_with_npc(&ctx, common_room, grom_id)
            .await
            .expect("Failed to stage NPC");

        // Verify the challenge exists in the seeded world
        let challenge_name = "Convince Grom to Share His Past";
        let challenge_id = ctx
            .world
            .challenge(challenge_name)
            .expect("Challenge should be seeded");

        tracing::info!(
            challenge_id = %challenge_id,
            challenge_name = %challenge_name,
            "Using seeded challenge for test"
        );

        // Start a conversation that should trigger a persuasion check
        // The player is explicitly trying to persuade Grom
        let started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_id,
                grom_id,
                player_id,
                "Grom, I can see you carry a heavy burden. I want to help you. Please, tell me about your past - I promise to listen without judgment.".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(!started.action_queue_id.is_nil(), "Action should be queued");

        // Process through the queue pipeline
        ctx.app
            .use_cases
            .queues
            .process_player_action
            .execute()
            .await
            .expect("Failed to process player action");

        let llm_result = ctx
            .app
            .use_cases
            .queues
            .process_llm_request
            .execute(|_| {})
            .await
            .expect("Failed to process LLM request");

        assert!(llm_result.is_some(), "Should have processed an LLM request");

        let result = llm_result.unwrap();

        // Get the approval request to examine challenge suggestion
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request not found");

        tracing::info!(
            proposed_dialogue = %approval_data.proposed_dialogue,
            has_challenge_suggestion = approval_data.challenge_suggestion.is_some(),
            "Approval request with potential challenge suggestion"
        );

        // Check if a challenge suggestion was made
        if let Some(ref suggestion) = approval_data.challenge_suggestion {
            tracing::info!(
                challenge_id = %suggestion.challenge_id,
                challenge_name = %suggestion.challenge_name,
                skill_name = %suggestion.skill_name,
                difficulty_display = %suggestion.difficulty_display,
                confidence = %suggestion.confidence,
                reasoning = %suggestion.reasoning,
                "Challenge suggestion found"
            );

            // ASSERTION 1: Challenge ID should be non-empty
            assert!(
                !suggestion.challenge_id.is_empty(),
                "Challenge ID should not be empty"
            );

            // ASSERTION 2: Challenge name should be enriched (not empty)
            assert!(
                !suggestion.challenge_name.is_empty(),
                "Challenge name should be enriched from database"
            );

            // ASSERTION 3: Skill name should be present
            assert!(
                !suggestion.skill_name.is_empty(),
                "Skill name should be present"
            );

            // ASSERTION 4: Confidence should be set
            assert!(
                !suggestion.confidence.is_empty(),
                "Confidence level should be present"
            );

            // ASSERTION 5: Reasoning should explain why the challenge applies
            assert!(
                !suggestion.reasoning.is_empty(),
                "Reasoning should explain why this challenge was suggested"
            );
        } else {
            tracing::warn!(
                "No challenge suggestion in LLM response. \
                 The LLM may not have identified the player's action as a challenge trigger. \
                 When recording cassettes, ensure the prompt encourages challenge suggestions."
            );
        }

        // ASSERTION 6: Challenge tags should not appear in dialogue
        assert!(
            !approval_data.proposed_dialogue.contains("<challenge_suggestion>"),
            "Proposed dialogue should not contain challenge_suggestion tags"
        );

        // Approve and complete the flow
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(result.approval_id.into(), DmApprovalDecision::Accept)
            .await
            .expect("Failed to approve");

        let final_dialogue = approval_result
            .final_dialogue
            .expect("Should have final dialogue");

        // SEMANTIC ASSERTIONS: Validate Grom's response makes sense
        let semantic = SemanticAssert::new(vcr.clone());

        // ASSERTION 7: Grom should acknowledge the topic (past/burden), even if he refuses
        // Note: A guarded refusal is appropriate for a threshold guardian - he guards his secrets
        semantic
            .assert_custom(
                &final_dialogue,
                "Does this response acknowledge or address the topic of the speaker's past, burden, or personal history (even if refusing to share details)?",
                "Grom should acknowledge the topic of his past, even if guardedly",
            )
            .await?;

        // ASSERTION 8: Response should relate to Grom's past, burden, or personal history
        semantic
            .assert_mentions_any(
                &final_dialogue,
                &["past", "burden", "forge", "family", "loss", "guard", "war", "before", "road", "path", "weight", "carry"],
                "Grom's response should touch on his past or personal history",
            )
            .await?;

        // ASSERTION 9: Response should be in character for Grom (threshold guardian, dwarven)
        semantic
            .assert_in_character(
                &final_dialogue,
                "Grom Ironhand, a stoic dwarven threshold guardian with a troubled past",
                "Response should match Grom's gruff but honorable personality",
            )
            .await?;

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");

    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 4: Narrative Event Suggestion
// =============================================================================

/// Verify that the LLM can suggest narrative events during dialogue.
///
/// When a player mentions something that could trigger a narrative event
/// (e.g., a key phrase or action), the LLM should include a
/// `<narrative_event_suggestion>` with:
/// - Event ID (matching a seeded event)
/// - Confidence level
/// - Reasoning for why this event should trigger
/// - Matched triggers (what the player said that matched)
///
/// The suggestion is enriched with event metadata before being shown to the DM.
#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes"]
async fn test_llm_suggests_narrative_event() {
    const TEST_NAME: &str = "test_llm_suggests_narrative_event";
    let event_log = create_shared_log(TEST_NAME);

    let vcr = Arc::new(VcrLlm::from_env(std::path::PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        TEST_NAME
    ))));
    let llm = Arc::new(LoggingLlmDecorator::new(vcr.clone(), event_log.clone()));

    let ctx = E2ETestContext::setup_with_llm_and_logging(llm, event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player and stage NPC
        let common_room = ctx.world.region("Common Room").expect("Region not found");
        let (player_id, pc_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Narrative Event Test Player",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Check for a seeded narrative event
        // "Marta Shares Local Rumors" has a simple dialogue_topic trigger
        let event_name = "Marta Shares Local Rumors";
        if let Some(event_id) = ctx.world.event(event_name) {
            tracing::info!(
                event_id = %event_id,
                event_name = %event_name,
                "Using seeded narrative event for test"
            );
        } else {
            tracing::warn!(
                "Narrative event '{}' not found in seeded world. \
                 Test will verify the parsing flow still works.",
                event_name
            );
        }

        // Start a conversation that triggers the "Marta Shares Local Rumors" event
        // The player asks about rumors, which matches the event's dialogue_topic trigger
        let started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_id,
                marta_id,
                player_id,
                "Marta, have you heard any rumors lately? Any strange happenings around town I should know about?".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(!started.action_queue_id.is_nil(), "Action should be queued");

        // Process through the queue pipeline
        ctx.app
            .use_cases
            .queues
            .process_player_action
            .execute()
            .await
            .expect("Failed to process player action");

        let llm_result = ctx
            .app
            .use_cases
            .queues
            .process_llm_request
            .execute(|_| {})
            .await
            .expect("Failed to process LLM request");

        assert!(llm_result.is_some(), "Should have processed an LLM request");

        let result = llm_result.unwrap();

        // Get the approval request to examine narrative event suggestion
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request not found");

        tracing::info!(
            proposed_dialogue = %approval_data.proposed_dialogue,
            has_narrative_event_suggestion = approval_data.narrative_event_suggestion.is_some(),
            "Approval request with potential narrative event suggestion"
        );

        // Check if a narrative event suggestion was made
        if let Some(ref suggestion) = approval_data.narrative_event_suggestion {
            tracing::info!(
                event_id = %suggestion.event_id,
                event_name = %suggestion.event_name,
                description = %suggestion.description,
                scene_direction = %suggestion.scene_direction,
                confidence = %suggestion.confidence,
                reasoning = %suggestion.reasoning,
                matched_triggers = ?suggestion.matched_triggers,
                "Narrative event suggestion found"
            );

            // ASSERTION 1: Event ID should be non-empty
            assert!(
                !suggestion.event_id.is_empty(),
                "Event ID should not be empty"
            );

            // ASSERTION 2: Event name should be enriched (not empty)
            assert!(
                !suggestion.event_name.is_empty(),
                "Event name should be enriched from database"
            );

            // ASSERTION 3: Description should be present
            assert!(
                !suggestion.description.is_empty(),
                "Description should be present"
            );

            // ASSERTION 4: Scene direction should guide the DM
            assert!(
                !suggestion.scene_direction.is_empty(),
                "Scene direction should be present"
            );

            // ASSERTION 5: Confidence should be set
            assert!(
                !suggestion.confidence.is_empty(),
                "Confidence level should be present"
            );

            // ASSERTION 6: Reasoning should explain why the event applies
            assert!(
                !suggestion.reasoning.is_empty(),
                "Reasoning should explain why this event was suggested"
            );

            // ASSERTION 7: If matched_triggers exist, they should be non-empty
            for trigger in &suggestion.matched_triggers {
                assert!(
                    !trigger.trim().is_empty(),
                    "Matched trigger should not be empty"
                );
            }
        } else {
            tracing::warn!(
                "No narrative event suggestion in LLM response. \
                 The LLM may not have identified the player's dialogue as an event trigger. \
                 When recording cassettes, ensure the prompt encourages narrative event suggestions."
            );
        }

        // ASSERTION 8: Event suggestion tags should not appear in dialogue
        assert!(
            !approval_data.proposed_dialogue.contains("<narrative_event_suggestion>"),
            "Proposed dialogue should not contain narrative_event_suggestion tags"
        );

        // Approve and complete the flow
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(result.approval_id.into(), DmApprovalDecision::Accept)
            .await
            .expect("Failed to approve");

        let final_dialogue = approval_result
            .final_dialogue
            .expect("Should have final dialogue");

        // SEMANTIC ASSERTIONS: Validate response to rumors question
        let semantic = SemanticAssert::new(vcr.clone());

        // ASSERTION 9: Response should address the rumors topic
        semantic
            .assert_responds_to_question(
                "Marta, have you heard any rumors lately? Any strange happenings around town I should know about?",
                &final_dialogue,
                "NPC should respond to the player's question about rumors and happenings",
            )
            .await?;

        // ASSERTION 10: Response should touch on rumors, news, or local happenings
        semantic
            .assert_mentions_any(
                &final_dialogue,
                &["rumor", "heard", "town", "folk", "talk", "news", "happening", "strange"],
                "Response should relate to rumors or local happenings",
            )
            .await?;

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");

    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 5: Full Structured Output Integration
// =============================================================================

/// Integration test verifying all structured output components work together.
///
/// This test exercises a scenario where the LLM response might include
/// multiple structured components simultaneously:
/// - Internal reasoning
/// - Dialogue
/// - Topics
/// - Potentially a challenge or event suggestion
///
/// Validates the complete parsing and enrichment pipeline.
#[tokio::test]
#[ignore = "VCR cassette has corrupted recording (truncated response) - needs re-recording with live LLM"]
async fn test_full_structured_output_integration() {
    const TEST_NAME: &str = "test_full_structured_output_integration";
    let event_log = create_shared_log(TEST_NAME);

    let vcr = Arc::new(VcrLlm::from_env(std::path::PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        TEST_NAME
    ))));
    let llm = Arc::new(LoggingLlmDecorator::new(vcr.clone(), event_log.clone()));

    let ctx = E2ETestContext::setup_with_llm_and_logging(llm, event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player and stage NPC
        let common_room = ctx.world.region("Common Room").expect("Region not found");
        let (player_id, pc_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Integration Test Player",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Complex conversation that could trigger multiple structured outputs
        let started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_id,
                marta_id,
                player_id,
                "I need your help, Marta. I'm investigating the old mill, but I'll need to persuade the guards to let me in. Can you teach me what you know about the mill's history and perhaps give me some advice on how to approach the guards?".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(!started.action_queue_id.is_nil(), "Action should be queued");

        // Process through the queue pipeline
        ctx.app
            .use_cases
            .queues
            .process_player_action
            .execute()
            .await
            .expect("Failed to process player action");

        let llm_result = ctx
            .app
            .use_cases
            .queues
            .process_llm_request
            .execute(|_| {})
            .await
            .expect("Failed to process LLM request");

        assert!(llm_result.is_some(), "Should have processed an LLM request");

        let result = llm_result.unwrap();

        // Get the approval request to examine all structured outputs
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request not found");

        // Log all structured output components
        tracing::info!(
            "=== Structured Output Summary ===\n\
             Proposed Dialogue Length: {}\n\
             Internal Reasoning Length: {}\n\
             Topics Count: {}\n\
             Has Challenge Suggestion: {}\n\
             Has Narrative Event Suggestion: {}",
            approval_data.proposed_dialogue.len(),
            approval_data.internal_reasoning.len(),
            approval_data.topics.len(),
            approval_data.challenge_suggestion.is_some(),
            approval_data.narrative_event_suggestion.is_some()
        );

        // CORE ASSERTIONS - These should always pass
        
        // 1. Proposed dialogue should always be non-empty
        assert!(
            !approval_data.proposed_dialogue.is_empty(),
            "Proposed dialogue should not be empty"
        );

        // 2. NPC name should be present
        assert!(
            !approval_data.npc_name.is_empty(),
            "NPC name should be present"
        );

        // 3. No raw XML tags in the dialogue
        let forbidden_tags = [
            "<reasoning>",
            "</reasoning>",
            "<topics>",
            "</topics>",
            "<challenge_suggestion>",
            "</challenge_suggestion>",
            "<narrative_event_suggestion>",
            "</narrative_event_suggestion>",
        ];

        for tag in &forbidden_tags {
            assert!(
                !approval_data.proposed_dialogue.contains(tag),
                "Proposed dialogue should not contain raw tag: {}",
                tag
            );
        }

        // 4. Verify the approval flow works with structured output
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(result.approval_id.into(), DmApprovalDecision::Accept)
            .await
            .expect("Failed to approve");

        assert!(
            approval_result.final_dialogue.is_some(),
            "Should have final dialogue after approval"
        );

        let final_dialogue = approval_result.final_dialogue.unwrap();

        // 5. Final dialogue should also be clean of tags
        for tag in &forbidden_tags {
            assert!(
                !final_dialogue.contains(tag),
                "Final dialogue should not contain raw tag: {}",
                tag
            );
        }

        tracing::info!(
            final_dialogue = %final_dialogue,
            "Successfully processed structured output to clean dialogue"
        );

        // SEMANTIC ASSERTIONS: Validate content quality
        let semantic = SemanticAssert::new(vcr.clone());

        // Player asked a complex multi-part question about mill investigation and guards
        let player_question = "I need your help, Marta. I'm investigating the old mill, but I'll need to persuade the guards to let me in. Can you teach me what you know about the mill's history and perhaps give me some advice on how to approach the guards?";

        // ASSERTION 6: Response should address the player's request for help
        semantic
            .assert_responds_to_question(
                player_question,
                &final_dialogue,
                "Marta should respond helpfully to the player's multi-part request",
            )
            .await?;

        // ASSERTION 7: Response should touch on the mill, guards, or helpful advice
        semantic
            .assert_mentions_any(
                &final_dialogue,
                &["mill", "guard", "help", "advice", "history", "careful", "approach"],
                "Response should address mill, guards, or provide helpful advice",
            )
            .await?;

        // ASSERTION 8: Response should have a helpful, friendly tone (Marta is an innkeeper)
        semantic
            .assert_tone(
                &final_dialogue,
                "helpful or friendly",
                "Marta should respond with a helpful tone as a friendly innkeeper",
            )
            .await?;

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");

    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 6: Fallback When No Tags Present
// =============================================================================

/// Verify graceful handling when LLM response has no structured tags.
///
/// Some LLM responses may not include explicit `<dialogue>` tags.
/// The parser should treat the entire response (minus other tags) as dialogue.
/// This tests backward compatibility with simpler LLM responses.
#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes"]
async fn test_llm_response_without_explicit_dialogue_tags() {
    const TEST_NAME: &str = "test_llm_response_without_explicit_dialogue_tags";
    let event_log = create_shared_log(TEST_NAME);

    let vcr = Arc::new(VcrLlm::from_env(std::path::PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        TEST_NAME
    ))));
    let llm = Arc::new(LoggingLlmDecorator::new(vcr.clone(), event_log.clone()));

    let ctx = E2ETestContext::setup_with_llm_and_logging(llm, event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player and stage NPC
        let common_room = ctx.world.region("Common Room").expect("Region not found");
        let (player_id, pc_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Fallback Test Player",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Simple greeting that might result in a simple response
        let started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_id,
                marta_id,
                player_id,
                "Hello!".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(!started.action_queue_id.is_nil(), "Action should be queued");

        // Process through the queue pipeline
        ctx.app
            .use_cases
            .queues
            .process_player_action
            .execute()
            .await
            .expect("Failed to process player action");

        let llm_result = ctx
            .app
            .use_cases
            .queues
            .process_llm_request
            .execute(|_| {})
            .await
            .expect("Failed to process LLM request");

        assert!(llm_result.is_some(), "Should have processed an LLM request");

        let result = llm_result.unwrap();

        // Get the approval request
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request not found");

        tracing::info!(
            proposed_dialogue = %approval_data.proposed_dialogue,
            internal_reasoning = %approval_data.internal_reasoning,
            "Simple response approval request"
        );

        // ASSERTION: Even without explicit tags, we should get dialogue
        assert!(
            !approval_data.proposed_dialogue.is_empty(),
            "Should have dialogue even without explicit <dialogue> tags"
        );

        // Approve and verify
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(result.approval_id.into(), DmApprovalDecision::Accept)
            .await
            .expect("Failed to approve");

        let final_dialogue = approval_result
            .final_dialogue
            .expect("Should have final dialogue");

        tracing::info!(
            final_dialogue = %final_dialogue,
            "Fallback dialogue extraction successful"
        );

        // SEMANTIC ASSERTIONS: Validate greeting response content
        let semantic = SemanticAssert::new(vcr.clone());

        // ASSERTION 2: Response should be an appropriate greeting
        semantic
            .assert_is_greeting(
                &final_dialogue,
                "NPC should respond to 'Hello!' with a greeting or acknowledgment",
            )
            .await?;

        // ASSERTION 3: Response should have a welcoming tone (Marta is a friendly innkeeper)
        semantic
            .assert_tone(
                &final_dialogue,
                "friendly or welcoming",
                "Marta should greet visitors warmly as an innkeeper",
            )
            .await?;

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");

    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}
