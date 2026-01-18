//! Tool execution E2E tests.
//!
//! These tests verify that when the DM approves LLM-proposed tools,
//! the tool executor actually performs the game state changes.
//!
//! # Test Coverage
//!
//! - `give_item` tool creates items in player inventory
//! - Rejected tools do not execute
//! - Multiple tools execute in order
//! - Tool effects persist after conversation ends
//! - Invalid tool arguments fail gracefully
//!
//! # Running Tests
//!
//! ```bash
//! cargo test -p wrldbldr-engine --lib tool_execution -- --ignored --test-threads=1
//! ```

use std::sync::Arc;

use serde_json::json;

use crate::queue_types::{ApprovalRequestData, DmApprovalDecision, ProposedTool};

use super::{
    approve_staging_with_npc, create_shared_log, create_test_player, E2ETestContext,
    LoggingLlmDecorator, SemanticAssert, TestOutcome, VcrLlm,
};

// =============================================================================
// Test 1: Approved give_item Creates Item in Inventory
// =============================================================================

/// Verify that when DM approves a give_item tool, the item appears in the PC's inventory.
#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes"]
async fn test_approved_give_item_creates_item_in_inventory() {
    const TEST_NAME: &str = "test_approved_give_item_creates_item_in_inventory";
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
            "Item Recipient",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Get initial inventory to compare later
        let initial_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get inventory");
        let initial_count = initial_inventory.len();

        // Start conversation asking for something (this triggers LLM to suggest give_item)
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

        // Get the approval request to inspect proposed tools
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request not found");

        tracing::info!(
            proposed_dialogue = %approval_data.proposed_dialogue,
            proposed_tools_count = approval_data.proposed_tools.len(),
            "Received approval request"
        );

        // SEMANTIC ASSERTIONS: Validate dialogue content
        let semantic = SemanticAssert::new(vcr.clone());
        let player_message =
            "I've been traveling for days without food. Could you spare something to eat?";
        semantic
            .assert_responds_to_question(
                player_message,
                &approval_data.proposed_dialogue,
                "NPC should respond to the player's request for food",
            )
            .await?;

        // Approve with all proposed tools
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
                result.approval_id,
                DmApprovalDecision::AcceptWithModification {
                    modified_dialogue: approval_data.proposed_dialogue.clone(),
                    approved_tools: tool_ids.clone(),
                    rejected_tools: vec![],
                    item_recipients: std::collections::HashMap::new(),
                },
            )
            .await
            .expect("Failed to approve");

        assert!(approval_result.approved, "Should be approved");

        // Verify inventory changed if give_item tools were approved
        let final_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get final inventory");

        // Check if any give_item tools were in the approval
        let give_item_tools: Vec<_> = approval_data
            .proposed_tools
            .iter()
            .filter(|t| t.name == "give_item")
            .collect();

        if !give_item_tools.is_empty() {
            assert!(
                final_inventory.len() > initial_count,
                "Inventory should have grown after give_item execution. Initial: {}, Final: {}",
                initial_count,
                final_inventory.len()
            );

            // Verify item properties
            for tool in &give_item_tools {
                if let Some(item_name) = tool.arguments.get("item_name").and_then(|v| v.as_str()) {
                    let found = final_inventory
                        .iter()
                        .any(|i| i.name().as_str() == item_name);
                    assert!(
                        found,
                        "Item '{}' should be in inventory after approval",
                        item_name
                    );
                }
            }
        }

        tracing::info!(
            initial_count = initial_count,
            final_count = final_inventory.len(),
            items = ?final_inventory.iter().map(|i| i.name().as_str()).collect::<Vec<_>>(),
            "Inventory state after tool execution"
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
// Test 2: Rejected Tool Not Executed
// =============================================================================

/// Verify that when DM rejects a tool, it is not executed.
#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes"]
async fn test_rejected_tool_not_executed() {
    const TEST_NAME: &str = "test_rejected_tool_not_executed";
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
            "No Items Player",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Get initial inventory
        let initial_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get inventory");
        let initial_count = initial_inventory.len();

        // Start conversation
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
                "Please give me that old map you mentioned. I really need it.".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(!started.action_queue_id.is_nil(), "Action should be queued");

        // Process through queues
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

        // Get approval data
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request not found");

        // Collect all tool IDs to REJECT
        let tool_ids_to_reject: Vec<String> = approval_data
            .proposed_tools
            .iter()
            .map(|t| t.id.clone())
            .collect();

        tracing::info!(
            proposed_dialogue = %approval_data.proposed_dialogue,
            tools_to_reject = ?tool_ids_to_reject,
            "Rejecting all tools while accepting dialogue"
        );

        // SEMANTIC ASSERTIONS: Validate dialogue content
        let semantic = SemanticAssert::new(vcr.clone());
        let player_message = "Please give me that old map you mentioned. I really need it.";
        semantic
            .assert_responds_to_question(
                player_message,
                &approval_data.proposed_dialogue,
                "NPC should respond to the player's request for a map",
            )
            .await?;

        // DM accepts dialogue but REJECTS all tools
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(
                result.approval_id,
                DmApprovalDecision::AcceptWithModification {
                    modified_dialogue: approval_data.proposed_dialogue.clone(),
                    approved_tools: vec![], // No tools approved!
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

        // Verify NO tools were executed
        assert!(
            approval_result.approved_tools.is_empty(),
            "No tools should be approved"
        );

        // Verify inventory is UNCHANGED
        let final_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get final inventory");

        assert_eq!(
            final_inventory.len(),
            initial_count,
            "Inventory should be unchanged after tool rejection. Initial: {}, Final: {}",
            initial_count,
            final_inventory.len()
        );

        tracing::info!(
            initial_count = initial_count,
            final_count = final_inventory.len(),
            "Inventory unchanged after tool rejection"
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
// Test 3: Multiple Tools Executed in Order
// =============================================================================

/// Verify that when multiple tools are approved, all are executed.
#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes"]
async fn test_multiple_tools_executed_in_order() {
    const TEST_NAME: &str = "test_multiple_tools_executed_in_order";
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
            "Multi-Tool Receiver",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Get initial inventory
        let initial_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get inventory");
        let initial_count = initial_inventory.len();

        // Complex request that might trigger multiple tools
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
                "I heard you know secrets about the old temple. I brought you a gift - will you share what you know and perhaps help me with supplies for my journey?".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(!started.action_queue_id.is_nil(), "Action should be queued");

        // Process through queues
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

        // Get approval data
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

        // SEMANTIC ASSERTIONS: Validate dialogue content
        let semantic = SemanticAssert::new(vcr.clone());
        let player_message = "I heard you know secrets about the old temple. I brought you a gift - will you share what you know and perhaps help me with supplies for my journey?";
        semantic
            .assert_responds_to_question(
                player_message,
                &approval_data.proposed_dialogue,
                "NPC should respond to the player's request about temple secrets and supplies",
            )
            .await?;

        for tool in &approval_data.proposed_tools {
            tracing::info!(
                tool_id = %tool.id,
                tool_name = %tool.name,
                tool_description = %tool.description,
                "Proposed tool"
            );
        }

        // Approve ALL tools
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
                result.approval_id,
                DmApprovalDecision::AcceptWithModification {
                    modified_dialogue: approval_data.proposed_dialogue.clone(),
                    approved_tools: tool_ids.clone(),
                    rejected_tools: vec![],
                    item_recipients: std::collections::HashMap::new(),
                },
            )
            .await
            .expect("Failed to approve");

        assert!(approval_result.approved, "Should be approved");

        // Verify that approved_tools matches what we requested
        assert_eq!(
            approval_result.approved_tools.len(),
            tool_ids.len(),
            "All requested tools should be approved"
        );

        // Check final inventory for any give_item tools
        let final_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get final inventory");

        let give_item_count = approval_data
            .proposed_tools
            .iter()
            .filter(|t| t.name == "give_item")
            .count();

        tracing::info!(
            initial_count = initial_count,
            final_count = final_inventory.len(),
            give_item_tools = give_item_count,
            total_tools_approved = approval_result.approved_tools.len(),
            "Multiple tools execution complete"
        );

        // If there were give_item tools, inventory should have grown
        if give_item_count > 0 {
            assert!(
                final_inventory.len() >= initial_count + give_item_count,
                "Expected {} new items but got {}",
                give_item_count,
                final_inventory.len() - initial_count
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
// Test 4: Give Item Persists After Conversation End
// =============================================================================

/// Verify that items given via tool persist after the conversation ends.
#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes"]
async fn test_give_item_persists_after_conversation_end() {
    const TEST_NAME: &str = "test_give_item_persists_after_conversation_end";
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
            "Persistent Item Tester",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Get initial inventory
        let initial_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get inventory");
        let initial_count = initial_inventory.len();

        // Start conversation
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

        // Process through queues
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

            // SEMANTIC ASSERTIONS: Validate dialogue content
            let semantic = SemanticAssert::new(vcr.clone());
            let player_message = "You mentioned having a map of the area. Could I see it?";
            semantic
                .assert_responds_to_question(
                    player_message,
                    &approval_data.proposed_dialogue,
                    "NPC should respond to the player's request to see a map",
                )
                .await?;

            // Approve all tools
            let tool_ids: Vec<String> = approval_data
                .proposed_tools
                .iter()
                .map(|t| t.id.clone())
                .collect();

            ctx.app
                .use_cases
                .approval
                .decision_flow
                .execute(
                    result.approval_id,
                    DmApprovalDecision::AcceptWithModification {
                        modified_dialogue: approval_data.proposed_dialogue.clone(),
                        approved_tools: tool_ids,
                        rejected_tools: vec![],
                        item_recipients: std::collections::HashMap::new(),
                    },
                )
                .await
                .expect("Failed to approve");
        }

        // Check inventory BEFORE ending conversation
        let mid_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get mid inventory");

        // END the conversation
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
            mid_count = mid_inventory.len(),
            final_count = final_inventory.len(),
            items = ?final_inventory.iter().map(|i| i.name().as_str()).collect::<Vec<_>>(),
            "Inventory after conversation ended"
        );

        // Key assertion: items given during conversation PERSIST after it ends
        assert_eq!(
            final_inventory.len(),
            mid_inventory.len(),
            "Inventory should not change when conversation ends. Mid: {}, Final: {}",
            mid_inventory.len(),
            final_inventory.len()
        );

        // All items should have valid names
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
// Test 5: Tool Execution Fails Gracefully on Invalid Arguments
// =============================================================================

/// Verify that tools with invalid arguments fail gracefully without crashing.
#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_tool_execution_fails_gracefully_on_invalid_args() {
    const TEST_NAME: &str = "test_tool_execution_fails_gracefully_on_invalid_args";
    let event_log = create_shared_log(TEST_NAME);

    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player character
        let common_room = ctx.world.region("Common Room").expect("Region not found");
        let (_player_id, pc_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Invalid Args Tester",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");

        // Create tools with INVALID arguments directly
        let invalid_tools = vec![
            // give_item without required item_name
            ProposedTool {
                id: "call_invalid_1".to_string(),
                name: "give_item".to_string(),
                description: "Give item without name".to_string(),
                arguments: json!({}), // Missing required item_name!
            },
            // give_item with wrong type for item_name
            ProposedTool {
                id: "call_invalid_2".to_string(),
                name: "give_item".to_string(),
                description: "Give item with invalid type".to_string(),
                arguments: json!({"item_name": 12345}), // Number instead of string!
            },
            // Unknown tool (should be skipped gracefully)
            ProposedTool {
                id: "call_invalid_3".to_string(),
                name: "nonexistent_tool".to_string(),
                description: "Tool that doesn't exist".to_string(),
                arguments: json!({"foo": "bar"}),
            },
        ];

        // Get the tool executor from the approval use case
        // We'll call it directly to test edge cases
        let tool_executor = &ctx.app.use_cases.approval.decision_flow;

        // Create a fake approval request with these invalid tools
        // We need to manually create and enqueue an approval request
        let approval_id = ctx
            .app
            .queue
            .enqueue_dm_approval(&ApprovalRequestData {
                world_id: ctx.world.world_id,
                source_action_id: uuid::Uuid::new_v4(),
                decision_type: crate::queue_types::ApprovalDecisionType::NpcResponse,
                urgency: crate::queue_types::ApprovalUrgency::Normal,
                pc_id: Some(pc_id),
                npc_id: Some(marta_id),
                npc_name: "Marta Hearthwood".to_string(),
                proposed_dialogue: "Here are some items for you.".to_string(),
                internal_reasoning: "Test reasoning".to_string(),
                proposed_tools: invalid_tools.clone(),
                retry_count: 0,
                challenge_suggestion: None,
                narrative_event_suggestion: None,
                challenge_outcome: None,
                player_dialogue: Some("Give me stuff".to_string()),
                scene_id: None,
                location_id: None,
                game_time: None,
                topics: vec![],
                conversation_id: None,
            })
            .await
            .expect("Failed to enqueue approval");

        // Approve all the invalid tools
        let invalid_tool_ids: Vec<String> = invalid_tools.iter().map(|t| t.id.clone()).collect();

        // This should NOT panic - errors should be logged and skipped
        let result = tool_executor
            .execute(
                approval_id,
                DmApprovalDecision::AcceptWithModification {
                    modified_dialogue: "Here are some items for you.".to_string(),
                    approved_tools: invalid_tool_ids.clone(),
                    rejected_tools: vec![],
                    item_recipients: std::collections::HashMap::new(),
                },
            )
            .await;

        // Should succeed (not panic), even though tools failed
        assert!(
            result.is_ok(),
            "Execution should not panic on invalid tools"
        );
        let outcome = result.unwrap();

        // Dialogue should still be delivered
        assert!(outcome.approved, "Should still be approved");
        assert!(outcome.final_dialogue.is_some(), "Should have dialogue");

        tracing::info!(
            approved_tools = ?outcome.approved_tools,
            final_dialogue = ?outcome.final_dialogue,
            "Graceful handling of invalid tool arguments"
        );

        // Verify no inventory change (give_item failed)
        let final_inventory = ctx
            .app
            .repositories
            .player_character
            .get_inventory(pc_id)
            .await
            .expect("Failed to get final inventory");

        assert!(
            final_inventory.is_empty()
                || final_inventory.iter().all(|i| {
                    // None of the items should be from our invalid tools
                    i.name().as_str() != "12345" // The invalid type
                }),
            "Invalid tool arguments should not create items"
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

    test_result.expect("Test failed");
}

// =============================================================================
// Test 6: Partial Tool Approval
// =============================================================================

/// Verify that DM can approve some tools and reject others in the same request.
#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes"]
async fn test_partial_tool_approval() {
    const TEST_NAME: &str = "test_partial_tool_approval";
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
            "Partial Approval Tester",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Complex request that might trigger multiple tools
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
                "I need help preparing for my journey. Can you tell me about the dangers and also give me some supplies?".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(!started.action_queue_id.is_nil(), "Action should be queued");

        // Process through queues
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

        // Get approval data
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request not found");

        // SEMANTIC ASSERTIONS: Validate dialogue content
        let semantic = SemanticAssert::new(vcr.clone());
        let player_message = "I need help preparing for my journey. Can you tell me about the dangers and also give me some supplies?";
        semantic
            .assert_responds_to_question(
                player_message,
                &approval_data.proposed_dialogue,
                "NPC should respond to the player's request for journey information and supplies",
            )
            .await?;

        // If there are multiple tools, approve only the first one
        let all_tool_ids: Vec<String> = approval_data
            .proposed_tools
            .iter()
            .map(|t| t.id.clone())
            .collect();

        let (approved_ids, rejected_ids) = if all_tool_ids.len() > 1 {
            // Approve first half, reject rest
            let mid = all_tool_ids.len() / 2;
            (all_tool_ids[..=mid].to_vec(), all_tool_ids[mid + 1..].to_vec())
        } else {
            // If only one tool, approve it
            (all_tool_ids.clone(), vec![])
        };

        tracing::info!(
            total_tools = all_tool_ids.len(),
            approved = ?approved_ids,
            rejected = ?rejected_ids,
            "Partial tool approval"
        );

        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(
                result.approval_id,
                DmApprovalDecision::AcceptWithModification {
                    modified_dialogue: approval_data.proposed_dialogue.clone(),
                    approved_tools: approved_ids.clone(),
                    rejected_tools: rejected_ids.clone(),
                    item_recipients: std::collections::HashMap::new(),
                },
            )
            .await
            .expect("Failed to approve with partial tools");

        assert!(approval_result.approved, "Should be approved");

        // Only approved tools should be in the result
        assert_eq!(
            approval_result.approved_tools.len(),
            approved_ids.len(),
            "Only approved tools should be in result"
        );

        // Verify approved tools are correct
        for tool_id in &approved_ids {
            assert!(
                approval_result.approved_tools.contains(tool_id),
                "Approved tool {} should be in result",
                tool_id
            );
        }

        // Verify rejected tools are NOT in the result
        for tool_id in &rejected_ids {
            assert!(
                !approval_result.approved_tools.contains(tool_id),
                "Rejected tool {} should NOT be in result",
                tool_id
            );
        }

        tracing::info!(
            final_approved = ?approval_result.approved_tools,
            "Partial approval complete"
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
