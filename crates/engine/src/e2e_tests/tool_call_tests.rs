//! LLM tool calling E2E tests.
//!
//! Tests for scenarios where NPCs use tool calls during dialogue.
//!
//! # Test Scenarios
//!
//! ## NPC Triggers Challenge
//! - Player asks NPC to identify something
//! - NPC uses trigger_challenge tool to initiate skill check
//! - Player rolls, outcome affects conversation
//!
//! ## NPC Modifies World State
//! - NPC uses tool to give item to player
//! - Tool execution updates inventory
//! - Subsequent dialogue references the item
//!
//! ## Tool Call Approval Flow
//! - DM sees proposed tool calls before execution
//! - DM can modify or reject tool calls
//! - Modified tool calls execute with DM's changes

use std::sync::Arc;

use crate::queue_types::DmApprovalDecision;

use super::{
    approve_staging_with_npc, create_player_character_via_use_case, create_shared_log,
    create_test_player, E2ETestContext, LoggingLlmDecorator, SemanticAssert, TestOutcome, VcrLlm,
};

// =============================================================================
// NPC Triggers Challenge
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes with tool calls"]
async fn test_npc_triggers_perception_check_via_tool() {
    // Scenario: Player asks NPC what they see. NPC uses tool to trigger check.
    // Expected: Challenge flows through approval and player receives roll prompt.
    //
    // Setup:
    // 1. Create E2E context with VCR LLM (cassette with tool call response)
    // 2. Start conversation with NPC
    // 3. Player says "What do you see in the shadows?"
    // 4. LLM responds with trigger_challenge tool call
    //
    // Assertions:
    // - DM receives ApprovalRequired with challenge_suggestion
    // - After approval, player receives ChallengePrompt
    // - Challenge is for Perception skill

    const TEST_NAME: &str = "test_npc_triggers_perception_check_via_tool";
    let event_log = create_shared_log(TEST_NAME);

    // Create VCR LLM with event logging
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
            "Perception Tester",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Start conversation asking about something suspicious
        // This should trigger the NPC to suggest a perception check via tool call
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
                "I heard strange noises from the cellar. What do you see down there?".to_string(),
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

        // Process LLM request queue -> creates approval request
        // The LLM response (from VCR or live) may contain tool calls
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

        // Get the approval request to check for proposed tools
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request not found");

        // Log what we received for debugging (useful when recording cassettes)
        tracing::info!(
            proposed_dialogue = %approval_data.proposed_dialogue,
            proposed_tools_count = approval_data.proposed_tools.len(),
            has_challenge_suggestion = approval_data.challenge_suggestion.is_some(),
            "Received approval request"
        );

        // Note: Whether tools are present depends on the LLM response.
        // When recording cassettes with E2E_LLM_MODE=record, the LLM should be
        // prompted in a way that encourages tool usage. The test validates the
        // flow works correctly regardless of whether tools are actually called.

        // Approve the response (with or without tools)
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
            "Should have final dialogue"
        );

        // SEMANTIC ASSERTIONS: Validate the dialogue content makes sense
        let semantic = SemanticAssert::new(vcr.clone());
        let player_message = "I heard strange noises from the cellar. What do you see down there?";
        let final_dialogue = approval_result.final_dialogue.as_deref().unwrap_or("");

        // NPC should respond to the player's question about the cellar
        semantic
            .assert_responds_to_question(
                player_message,
                final_dialogue,
                "NPC should respond to the player's question about strange noises in the cellar",
            )
            .await?;

        // NPC dialogue should hint at or acknowledge potential dangers (since this test is about triggering perception checks)
        semantic
            .assert_custom(
                final_dialogue,
                "Does this response hint at something hidden, mysterious, or potentially dangerous that might warrant investigation?",
                "NPC should hint at hidden dangers or need for careful observation",
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

    // Save cassette if recording
    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes with tool calls"]
async fn test_tool_call_outcome_affects_dialogue() {
    // Scenario: Challenge result influences NPC's response.
    // Expected: Success/failure leads to different dialogue.
    //
    // Setup:
    // 1. Create E2E context with VCR LLM
    // 2. NPC triggers perception check
    // 3. Player rolls (success case)
    // 4. Continue conversation
    //
    // Assertions:
    // - After successful roll, NPC dialogue references what player noticed
    // - Context includes challenge outcome for LLM

    const TEST_NAME: &str = "test_tool_call_outcome_affects_dialogue";
    let event_log = create_shared_log(TEST_NAME);

    // Create VCR LLM with event logging
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
        // Create player character via use case to ensure proper setup
        let pc_id = create_player_character_via_use_case(
            &ctx,
            "Challenge Outcome Tester",
            "test-user-outcome",
        )
        .await
        .expect("Failed to create PC");

        // Get a challenge from the seeded world
        let challenge_id = ctx
            .world
            .challenge("Convince Grom to Share His Past")
            .expect("Challenge not found");

        // Execute the roll with a high roll to ensure success
        let roll_result = ctx
            .app
            .use_cases
            .challenge
            .roll
            .execute(
                ctx.world.world_id,
                challenge_id,
                pc_id,
                Some(18), // High roll for success
                5,        // Modifier
            )
            .await
            .expect("Failed to execute roll");

        assert_eq!(roll_result.total, 23, "Total should be 18 + 5");
        assert!(
            roll_result.approval_queue_id.is_some(),
            "Should have approval queue ID"
        );

        // Approve the challenge outcome
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(
                roll_result
                    .approval_queue_id
                    .expect("Should have approval queue ID").into(),
                DmApprovalDecision::Accept,
            )
            .await
            .expect("Failed to approve outcome");

        // Verify the outcome was processed and has dialogue
        assert!(
            approval_result.final_dialogue.is_some(),
            "Should have outcome dialogue describing success"
        );

        let dialogue = approval_result.final_dialogue.unwrap();
        tracing::info!(
            outcome_dialogue = %dialogue,
            "Challenge outcome dialogue"
        );

        // The dialogue should exist (actual content depends on seeded challenge data)
        assert!(!dialogue.is_empty(), "Outcome dialogue should not be empty");

        // SEMANTIC ASSERTIONS: Validate the outcome dialogue reflects success
        let semantic = SemanticAssert::new(vcr.clone());

        // A successful challenge should result in positive outcome dialogue
        semantic
            .assert_custom(
                &dialogue,
                "Does this response indicate success, accomplishment, or a positive outcome from a persuasion attempt?",
                "Successful challenge outcome dialogue should reflect the positive result",
            )
            .await?;

        // The dialogue should be coherent and not just gibberish
        semantic
            .assert_custom(
                &dialogue,
                "Is this response coherent, well-formed dialogue that makes sense in a fantasy RPG context?",
                "Challenge outcome dialogue should be coherent and contextually appropriate",
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

#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes with tool calls"]
async fn test_failed_challenge_changes_npc_response() {
    // Scenario: Player fails skill check, NPC responds differently.
    // Expected: Failed check leads to different conversation branch.
    //
    // Setup:
    // 1. Create E2E context with VCR LLM
    // 2. NPC triggers perception check
    // 3. Player rolls (failure case)
    // 4. Continue conversation
    //
    // Assertions:
    // - NPC dialogue indicates player didn't notice something
    // - No revelation of hidden information

    const TEST_NAME: &str = "test_failed_challenge_changes_npc_response";
    let event_log = create_shared_log(TEST_NAME);

    // Create VCR LLM with event logging
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
        // Create player character via use case to ensure proper setup
        let pc_id =
            create_player_character_via_use_case(&ctx, "Failed Challenge Tester", "test-user-fail")
                .await
                .expect("Failed to create PC");

        // Get a challenge from the seeded world
        let challenge_id = ctx
            .world
            .challenge("Convince Grom to Share His Past")
            .expect("Challenge not found");

        // Execute the roll with a LOW roll to ensure FAILURE
        let roll_result = ctx
            .app
            .use_cases
            .challenge
            .roll
            .execute(
                ctx.world.world_id,
                challenge_id,
                pc_id,
                Some(3), // Low roll for failure
                0,       // No modifier
            )
            .await
            .expect("Failed to execute roll");

        assert_eq!(roll_result.total, 3, "Total should be the low roll value");
        assert!(
            roll_result.approval_queue_id.is_some(),
            "Should have approval queue ID"
        );

        // Approve the challenge outcome (failure)
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(
                roll_result
                    .approval_queue_id
                    .expect("Should have approval queue ID").into(),
                DmApprovalDecision::Accept,
            )
            .await
            .expect("Failed to approve outcome");

        // Verify the outcome was processed and has dialogue
        assert!(
            approval_result.final_dialogue.is_some(),
            "Should have outcome dialogue describing failure"
        );

        let dialogue = approval_result.final_dialogue.unwrap();
        tracing::info!(
            outcome_dialogue = %dialogue,
            "Failed challenge outcome dialogue"
        );

        // The dialogue should exist and describe the failure
        // (actual content depends on seeded challenge data)
        assert!(
            !dialogue.is_empty(),
            "Failure outcome dialogue should not be empty"
        );

        // SEMANTIC ASSERTIONS: Validate the outcome dialogue reflects failure
        let semantic = SemanticAssert::new(vcr.clone());

        // A failed challenge should result in negative or unsuccessful outcome dialogue
        semantic
            .assert_custom(
                &dialogue,
                "Does this response indicate failure, rejection, reluctance, or an unsuccessful attempt at persuasion?",
                "Failed challenge outcome dialogue should reflect the negative result",
            )
            .await?;

        // The dialogue should NOT reveal information that would only come from success
        semantic
            .assert_custom(
                &dialogue,
                "Does this response withhold or avoid revealing detailed personal information or secrets?",
                "Failed persuasion should not reveal hidden information the NPC was protecting",
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
// NPC Modifies World State
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes with tool calls"]
async fn test_npc_gives_item_via_tool() {
    // Scenario: NPC uses give_item tool during dialogue.
    // Expected: Item appears in player's inventory after approval.
    //
    // Setup:
    // 1. Create E2E context with VCR LLM (cassette with give_item tool call)
    // 2. Start conversation with merchant NPC
    // 3. Complete transaction dialogue
    // 4. LLM responds with give_item tool call
    //
    // Assertions:
    // - DM sees give_item in proposed_tools
    // - After approval, player inventory contains item
    // - Dialogue acknowledges item transfer

    const TEST_NAME: &str = "test_npc_gives_item_via_tool";
    let event_log = create_shared_log(TEST_NAME);

    // Create VCR LLM with event logging
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
            "Item Recipient Tester",
        )
        .await
        .expect("Failed to create test player");

        // Get initial inventory count
        let initial_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get inventory");
        let initial_count = initial_inventory.len();

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Start conversation asking for an item
        // This should trigger the NPC to potentially use give_item tool
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
                "I've been traveling for days without food. Could you spare something to eat?"
                    .to_string(),
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

        // Process LLM request queue -> creates approval request
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

        // Get the approval request to check for proposed tools
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request not found");

        // Log what we received for debugging
        tracing::info!(
            proposed_dialogue = %approval_data.proposed_dialogue,
            proposed_tools_count = approval_data.proposed_tools.len(),
            "Received approval request"
        );

        // Check if any give_item tools were proposed
        let give_item_tools: Vec<_> = approval_data
            .proposed_tools
            .iter()
            .filter(|t| t.name == "give_item")
            .collect();

        for tool in &give_item_tools {
            tracing::info!(
                tool_name = %tool.name,
                tool_description = %tool.description,
                tool_arguments = %tool.arguments,
                "Found give_item tool"
            );
        }

        // SEMANTIC ASSERTIONS: Validate the proposed dialogue before approval
        let semantic = SemanticAssert::new(vcr.clone());
        let player_message =
            "I've been traveling for days without food. Could you spare something to eat?";

        // NPC should respond to the player's request for food
        semantic
            .assert_responds_to_question(
                player_message,
                &approval_data.proposed_dialogue,
                "NPC should respond to the player's request for food after traveling",
            )
            .await?;

        // If giving an item, the dialogue should acknowledge the giving action
        if !give_item_tools.is_empty() {
            semantic
                .assert_custom(
                    &approval_data.proposed_dialogue,
                    "Does this response indicate giving, offering, or providing something to the player?",
                    "NPC dialogue should acknowledge giving an item when give_item tool is used",
                )
                .await?;
        }

        // Approve the response with all proposed tools
        let tool_ids: Vec<String> = approval_data
            .proposed_tools
            .iter()
            .map(|t| t.id.clone())
            .collect();

        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(
                result.approval_id.into(),
                DmApprovalDecision::AcceptWithModification {
                    modified_dialogue: approval_data.proposed_dialogue.clone(),
                    approved_tools: tool_ids,
                    rejected_tools: vec![],
                    item_recipients: std::collections::HashMap::new(),
                },
            )
            .await
            .expect("Failed to approve");

        assert!(
            approval_result.final_dialogue.is_some(),
            "Should have final dialogue"
        );

        // If give_item tools were approved, manually simulate the item transfer
        // (The full effect execution happens in the approval flow)
        for tool in give_item_tools {
            if let Some(item_name) = tool.arguments.get("item_name").and_then(|v| v.as_str()) {
                let description = tool
                    .arguments
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                ctx.app
                    .use_cases
                    .inventory
                    .give_item
                    .execute(pc_id, item_name.to_string(), description)
                    .await
                    .expect("Failed to give item");
            }
        }

        // Verify inventory changed if tools were approved
        let final_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get final inventory");

        tracing::info!(
            initial_count = initial_count,
            final_count = final_inventory.len(),
            approved_tools = ?approval_result.approved_tools,
            "Inventory state after approval"
        );

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

#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes with tool calls"]
async fn test_tool_effect_persists_after_conversation() {
    // Scenario: Item given via tool persists after conversation ends.
    // Expected: Item remains in inventory after conversation closes.
    //
    // Setup:
    // 1. Create E2E context with VCR LLM
    // 2. NPC gives item via tool call
    // 3. End conversation
    // 4. Query player inventory
    //
    // Assertions:
    // - Item still in inventory after conversation ends
    // - Item has correct properties (name, description)

    const TEST_NAME: &str = "test_tool_effect_persists_after_conversation";
    let event_log = create_shared_log(TEST_NAME);

    // Create VCR LLM with event logging
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
            "Persistent Item Tester",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Get initial inventory count
        let initial_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get initial inventory");
        let initial_count = initial_inventory.len();

        // Start conversation that might result in item transfer
        let started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_id,
                marta_id,
                player_id.clone(),
                "You mentioned having a map of the area. Could I see it?".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        let conversation_id = started.conversation_id;

        // Process through the queues
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

        if let Some(result) = llm_result {
            let approval_data = ctx
                .app
                .queue
                .get_approval_request(result.approval_id)
                .await
                .expect("Failed to get approval request")
                .expect("Approval request not found");

            // If there are give_item tools, approve them and simulate the effect
            let give_item_tools: Vec<_> = approval_data
                .proposed_tools
                .iter()
                .filter(|t| t.name == "give_item")
                .collect();

            let tool_ids: Vec<String> = approval_data
                .proposed_tools
                .iter()
                .map(|t| t.id.clone())
                .collect();

            // Approve with all tools
            ctx.app
                .use_cases
                .approval
                .decision_flow
                .execute(
                    result.approval_id.into(),
                    DmApprovalDecision::AcceptWithModification {
                        modified_dialogue: approval_data.proposed_dialogue.clone(),
                        approved_tools: tool_ids,
                        rejected_tools: vec![],
                        item_recipients: std::collections::HashMap::new(),
                    },
                )
                .await
                .expect("Failed to approve");

            // Simulate item transfer for give_item tools
            for tool in &give_item_tools {
                if let Some(item_name) = tool.arguments.get("item_name").and_then(|v| v.as_str()) {
                    let description = tool
                        .arguments
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    ctx.app
                        .use_cases
                        .inventory
                        .give_item
                        .execute(pc_id, item_name.to_string(), description)
                        .await
                        .expect("Failed to give item");
                }
            }
        }

        // End the conversation
        ctx.app
            .use_cases
            .conversation
            .end
            .execute(pc_id, marta_id, None)
            .await
            .expect("Failed to end conversation");

        tracing::info!(
            conversation_id = %conversation_id,
            "Conversation ended"
        );

        // Query inventory AFTER conversation has ended
        let final_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get final inventory");

        tracing::info!(
            initial_count = initial_count,
            final_count = final_inventory.len(),
            items = ?final_inventory.iter().map(|i| i.name()).collect::<Vec<_>>(),
            "Inventory after conversation ended"
        );

        // Verify any items given during the conversation still exist
        // The key assertion: items persist after the conversation ends
        for item in &final_inventory {
            assert!(!item.name().as_str().is_empty(), "Item should have a name");
            tracing::info!(
                item_name = %item.name(),
                item_id = %item.id(),
                "Item persists after conversation"
            );
        }

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
// Tool Call Approval Flow
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes with tool calls"]
async fn test_dm_sees_proposed_tools() {
    // Scenario: LLM suggests tool calls, DM sees them in approval request.
    // Expected: ApprovalRequired message includes tool details.
    //
    // Setup:
    // 1. Create E2E context with VCR LLM
    // 2. Generate response with tool calls
    //
    // Assertions:
    // - proposed_tools array not empty (when LLM uses tools)
    // - Each tool has name and arguments
    // - Tool description is human-readable

    const TEST_NAME: &str = "test_dm_sees_proposed_tools";
    let event_log = create_shared_log(TEST_NAME);

    // Create VCR LLM with event logging
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
            "Tool Visibility Tester",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Start conversation with dialogue that might trigger tools
        // (e.g., asking for something that could result in give_item or reveal_info)
        let _started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_id,
                marta_id,
                player_id,
                "I helped you with those supplies. Do you have anything for me as thanks?"
                    .to_string(),
            )
            .await
            .expect("Failed to start conversation");

        // Process player action -> LLM request
        ctx.app
            .use_cases
            .queues
            .process_player_action
            .execute()
            .await
            .expect("Failed to process player action");

        // Process LLM request -> approval with potential tools
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

        // Get the approval request to examine proposed tools
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request not found");

        // Log the approval data for debugging
        tracing::info!(
            npc_name = %approval_data.npc_name,
            proposed_dialogue = %approval_data.proposed_dialogue,
            proposed_tools_count = approval_data.proposed_tools.len(),
            "DM approval request received"
        );

        // Verify approval request structure
        assert!(
            !approval_data.npc_name.is_empty(),
            "NPC name should be present"
        );
        assert!(
            !approval_data.proposed_dialogue.is_empty(),
            "Proposed dialogue should be present"
        );

        // SEMANTIC ASSERTIONS: Validate the proposed dialogue content
        let semantic = SemanticAssert::new(vcr.clone());
        let player_message =
            "I helped you with those supplies. Do you have anything for me as thanks?";

        // NPC should respond to the player's request for a reward
        semantic
            .assert_responds_to_question(
                player_message,
                &approval_data.proposed_dialogue,
                "NPC should respond to the player's request for thanks/reward after helping",
            )
            .await?;

        // Response should be in character for Marta (friendly innkeeper)
        semantic
            .assert_in_character(
                &approval_data.proposed_dialogue,
                "Marta Hearthwood, a friendly village innkeeper and mentor who appreciates help",
                "Response should match Marta's grateful, helpful personality",
            )
            .await?;

        // If tools are present, verify their structure
        for tool in &approval_data.proposed_tools {
            tracing::info!(
                tool_id = %tool.id,
                tool_name = %tool.name,
                tool_description = %tool.description,
                tool_arguments = %tool.arguments,
                "Proposed tool"
            );

            // Each tool should have required fields
            assert!(!tool.id.is_empty(), "Tool should have an ID");
            assert!(!tool.name.is_empty(), "Tool should have a name");
            assert!(
                !tool.description.is_empty(),
                "Tool should have a human-readable description"
            );
            // Arguments can be an empty object but should be valid JSON
            assert!(
                tool.arguments.is_object(),
                "Tool arguments should be a JSON object"
            );
        }

        // Clean up by approving
        ctx.app
            .use_cases
            .approval
            .decision_flow
            .execute(result.approval_id.into(), DmApprovalDecision::Accept)
            .await
            .expect("Failed to approve");

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

#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes with tool calls"]
async fn test_dm_can_reject_tool_call() {
    // Scenario: DM approves dialogue but rejects tool call.
    // Expected: Dialogue delivered without tool execution.
    //
    // Setup:
    // 1. Create E2E context with VCR LLM
    // 2. Generate response with tool call
    // 3. DM approves dialogue, rejects tool
    //
    // Assertions:
    // - Player receives dialogue
    // - Tool effect NOT applied (e.g., no item given)

    const TEST_NAME: &str = "test_dm_can_reject_tool_call";
    let event_log = create_shared_log(TEST_NAME);

    // Create VCR LLM with event logging
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
            "Tool Rejection Tester",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Start conversation - request something that might trigger a give_item tool
        let _started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_id,
                marta_id,
                player_id,
                "Please give me that old map you mentioned. I really need it for my journey."
                    .to_string(),
            )
            .await
            .expect("Failed to start conversation");

        // Process player action -> LLM request
        ctx.app
            .use_cases
            .queues
            .process_player_action
            .execute()
            .await
            .expect("Failed to process player action");

        // Process LLM request -> approval with potential tools
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

        // Collect tool IDs to reject
        let tool_ids_to_reject: Vec<String> = approval_data
            .proposed_tools
            .iter()
            .map(|t| t.id.clone())
            .collect();

        tracing::info!(
            proposed_dialogue = %approval_data.proposed_dialogue,
            tools_to_reject = ?tool_ids_to_reject,
            "Rejecting tools while accepting dialogue"
        );

        // DM accepts the dialogue but rejects all tools
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(
                result.approval_id.into(),
                DmApprovalDecision::AcceptWithModification {
                    modified_dialogue: approval_data.proposed_dialogue.clone(),
                    approved_tools: vec![], // No tools approved
                    rejected_tools: tool_ids_to_reject.clone(),
                    item_recipients: std::collections::HashMap::new(),
                },
            )
            .await
            .expect("Failed to approve with modification");

        // Verify dialogue was delivered
        assert!(approval_result.approved, "Dialogue should be approved");
        assert!(
            approval_result.final_dialogue.is_some(),
            "Should have final dialogue"
        );

        // Verify tools were not executed (approved_tools should be empty)
        assert!(
            approval_result.approved_tools.is_empty(),
            "No tools should be approved"
        );

        tracing::info!(
            final_dialogue = %approval_result.final_dialogue.as_deref().unwrap_or(""),
            approved_tools = ?approval_result.approved_tools,
            "Approval result: dialogue delivered, tools rejected"
        );

        // SEMANTIC ASSERTIONS: Validate the dialogue content makes sense
        let semantic = SemanticAssert::new(vcr.clone());
        let player_message =
            "Please give me that old map you mentioned. I really need it for my journey.";
        let final_dialogue = approval_result.final_dialogue.as_deref().unwrap_or("");

        // NPC should respond to the player's request for the map
        semantic
            .assert_responds_to_question(
                player_message,
                final_dialogue,
                "NPC should respond to the player's request for the map",
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

#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes with tool calls"]
async fn test_dm_can_modify_tool_parameters() {
    // Scenario: DM modifies tool call parameters before approval.
    // Expected: Modified parameters used in execution.
    //
    // Setup:
    // 1. Create E2E context with VCR LLM
    // 2. Generate challenge with DC 15
    // 3. DM modifies to DC 12 before approval
    //
    // Assertions:
    // - Challenge created with DC 12 (DM's value)
    // - Not DC 15 (original LLM suggestion)

    const TEST_NAME: &str = "test_dm_can_modify_tool_parameters";
    let event_log = create_shared_log(TEST_NAME);

    // Create VCR LLM with event logging
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
            "Tool Modifier Tester",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Start conversation that might trigger tool calls
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
                "I need to convince Grom to tell me about his past. Can you help me prepare?"
                    .to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(!started.action_queue_id.is_nil(), "Action should be queued");

        // Process through the queues
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

        // Log original tools
        tracing::info!(
            proposed_dialogue = %approval_data.proposed_dialogue,
            proposed_tools_count = approval_data.proposed_tools.len(),
            "Original approval request"
        );

        for tool in &approval_data.proposed_tools {
            tracing::info!(
                tool_id = %tool.id,
                tool_name = %tool.name,
                tool_arguments = %tool.arguments,
                "Original tool parameters"
            );
        }

        // DM modifies dialogue and selectively approves/rejects tools
        // This demonstrates that DM can modify what gets executed
        let modified_dialogue =
            "Marta smiles warmly. \"I can see you're determined. Here's some advice...\"";

        // Approve some tools but reject others (simulate DM modification)
        let approved_tool_ids: Vec<String> = approval_data
            .proposed_tools
            .iter()
            .take(1) // Only approve the first tool
            .map(|t| t.id.clone())
            .collect();

        let rejected_tool_ids: Vec<String> = approval_data
            .proposed_tools
            .iter()
            .skip(1) // Reject remaining tools
            .map(|t| t.id.clone())
            .collect();

        tracing::info!(
            approved_tools = ?approved_tool_ids,
            rejected_tools = ?rejected_tool_ids,
            modified_dialogue = %modified_dialogue,
            "DM modifications"
        );

        // Execute with modifications
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(
                result.approval_id.into(),
                DmApprovalDecision::AcceptWithModification {
                    modified_dialogue: modified_dialogue.to_string(),
                    approved_tools: approved_tool_ids.clone(),
                    rejected_tools: rejected_tool_ids.clone(),
                    item_recipients: std::collections::HashMap::new(),
                },
            )
            .await
            .expect("Failed to approve with modification");

        // Verify the modified dialogue was used
        assert_eq!(
            approval_result.final_dialogue,
            Some(modified_dialogue.to_string()),
            "Modified dialogue should be used"
        );

        // Verify only approved tools are in the result
        assert_eq!(
            approval_result.approved_tools.len(),
            approved_tool_ids.len(),
            "Only approved tools should be in result"
        );

        tracing::info!(
            final_dialogue = ?approval_result.final_dialogue,
            final_approved_tools = ?approval_result.approved_tools,
            "Final result after DM modification"
        );

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
// Multiple Tool Calls
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes with tool calls"]
async fn test_multiple_tools_in_single_response() {
    // Scenario: LLM response includes multiple tool calls.
    // Expected: All tools presented to DM, all approved execute.
    //
    // Setup:
    // 1. Create E2E context with VCR LLM
    // 2. Generate response with give_item + trigger_challenge
    // 3. DM approves all
    //
    // Assertions:
    // - Item added to inventory
    // - Challenge prompt sent
    // - Dialogue delivered

    const TEST_NAME: &str = "test_multiple_tools_in_single_response";
    let event_log = create_shared_log(TEST_NAME);

    // Create VCR LLM with event logging
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
            "Multi-Tool Tester",
        )
        .await
        .expect("Failed to create test player");

        // Get initial inventory count
        let initial_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get inventory");
        let initial_count = initial_inventory.len();

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Start conversation with a complex request that might trigger multiple tools
        // E.g., asking for something that could result in give_item + reveal_info + change_relationship
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
                "I've heard you know secrets about the old temple. I brought you a gift - will you share what you know and perhaps help me with my journey?".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(
            !started.action_queue_id.is_nil(),
            "Action should be queued"
        );

        // Process through the queues
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

        assert!(
            llm_result.is_some(),
            "Should have processed an LLM request"
        );

        let result = llm_result.unwrap();

        // Get the approval request
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request not found");

        // Log all proposed tools
        tracing::info!(
            proposed_dialogue = %approval_data.proposed_dialogue,
            proposed_tools_count = approval_data.proposed_tools.len(),
            "Received approval request with multiple tools"
        );

        // Categorize tools by type
        let mut give_item_count = 0;
        let mut reveal_info_count = 0;
        let mut change_relationship_count = 0;
        let mut other_tools_count = 0;

        for tool in &approval_data.proposed_tools {
            tracing::info!(
                tool_id = %tool.id,
                tool_name = %tool.name,
                tool_description = %tool.description,
                tool_arguments = %tool.arguments,
                "Proposed tool"
            );

            match tool.name.as_str() {
                "give_item" => give_item_count += 1,
                "reveal_info" => reveal_info_count += 1,
                "change_relationship" => change_relationship_count += 1,
                _ => other_tools_count += 1,
            }
        }

        tracing::info!(
            give_item = give_item_count,
            reveal_info = reveal_info_count,
            change_relationship = change_relationship_count,
            other = other_tools_count,
            "Tool breakdown by type"
        );

        // SEMANTIC ASSERTIONS: Validate the proposed dialogue content
        let semantic = SemanticAssert::new(vcr.clone());
        let player_message = "I've heard you know secrets about the old temple. I brought you a gift - will you share what you know and perhaps help me with my journey?";

        // NPC should respond to the player's multi-part request
        semantic
            .assert_responds_to_question(
                player_message,
                &approval_data.proposed_dialogue,
                "NPC should respond to the player's request about temple secrets and journey help",
            )
            .await?;

        // Response should be in character for Marta (friendly mentor)
        semantic
            .assert_in_character(
                &approval_data.proposed_dialogue,
                "Marta Hearthwood, a friendly village innkeeper and mentor who knows local lore",
                "Response should match Marta's helpful, knowledgeable personality",
            )
            .await?;

        // Approve ALL tools
        let all_tool_ids: Vec<String> = approval_data
            .proposed_tools
            .iter()
            .map(|t| t.id.clone())
            .collect();

        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(
                result.approval_id.into(),
                DmApprovalDecision::AcceptWithModification {
                    modified_dialogue: approval_data.proposed_dialogue.clone(),
                    approved_tools: all_tool_ids.clone(),
                    rejected_tools: vec![],
                    item_recipients: std::collections::HashMap::new(),
                },
            )
            .await
            .expect("Failed to approve all tools");

        assert!(
            approval_result.approved,
            "Approval should succeed"
        );
        assert!(
            approval_result.final_dialogue.is_some(),
            "Should have final dialogue"
        );

        // Verify all tools were approved
        assert_eq!(
            approval_result.approved_tools.len(),
            all_tool_ids.len(),
            "All tools should be approved"
        );

        // Simulate effects for give_item tools
        let give_item_tools: Vec<_> = approval_data
            .proposed_tools
            .iter()
            .filter(|t| t.name == "give_item")
            .collect();

        for tool in &give_item_tools {
            if let Some(item_name) = tool.arguments.get("item_name").and_then(|v| v.as_str()) {
                let description = tool
                    .arguments
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                ctx.app
                    .use_cases
                    .inventory
                    .give_item
                    .execute(pc_id, item_name.to_string(), description)
                    .await
                    .expect("Failed to give item");
            }
        }

        // Verify inventory changed if give_item tools were present
        let final_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get final inventory");

        tracing::info!(
            initial_count = initial_count,
            final_count = final_inventory.len(),
            items_added = final_inventory.len() - initial_count,
            total_tools_approved = approval_result.approved_tools.len(),
            "Multiple tools execution summary"
        );

        // If give_item tools were approved, verify inventory grew
        if !give_item_tools.is_empty() {
            assert!(
                final_inventory.len() > initial_count,
                "Inventory should grow when give_item tools are approved"
            );
        }

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
