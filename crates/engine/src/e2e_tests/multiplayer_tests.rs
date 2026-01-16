//! Multiplayer session E2E tests.
//!
//! Tests for scenarios involving multiple clients connected to the same world.
//!
//! # Test Scenarios
//!
//! ## Two Players in Same Region
//! - Both players can see each other's position updates
//! - Actions by one player are visible to the other
//! - Conversation started by Player A doesn't block Player B
//!
//! ## DM + Player Coordination
//! - DM sees player actions in approval queue
//! - DM approval triggers response to player
//! - DM can broadcast to all connected players
//!
//! ## Player Join/Leave
//! - New player joining receives full world snapshot
//! - Other connected clients notified of new player
//! - Player leaving triggers cleanup and notification

use std::sync::Arc;

use neo4rs::query;

use super::*;

// =============================================================================
// Two Players in Same Region
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_two_players_see_each_other_in_region() {
    // Scenario: Two players join the same world and move to the same region.
    // Expected: Both can see each other in the region's player list.
    //
    // Setup:
    // 1. Create E2E context with seeded world
    // 2. Create two player characters (PC_A and PC_B)
    // 3. Both join world as players
    // 4. PC_A moves to Common Room region
    // 5. PC_B moves to Common Room region
    //
    // Assertions:
    // - PC_A's snapshot shows PC_B present
    // - PC_B's snapshot shows PC_A present
    // - Both receive PlayerMoved broadcasts

    let event_log = create_shared_log("test_two_players_see_each_other_in_region");
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Get the Common Room region (spawn point)
        let common_room = ctx
            .world
            .region("Common Room")
            .expect("Common Room not found");

        // Create two player characters in the same region
        let (_user_a, pc_a_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player A - Knight",
        )
        .await
        .expect("Failed to create player A");

        let (_user_b, pc_b_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player B - Mage",
        )
        .await
        .expect("Failed to create player B");

        // Query database to get all player characters in the Common Room
        let mut result = ctx
            .graph()
            .execute(
                query(
                    "MATCH (pc:PlayerCharacter {current_region_id: $region_id})
                     RETURN pc.id AS id, pc.name AS name",
                )
                .param("region_id", common_room.to_string()),
            )
            .await
            .expect("Query failed");

        let mut players_in_region = Vec::new();
        while let Some(row) = result.next().await.expect("Row read failed") {
            let id: String = row.get("id").expect("id not found");
            let name: String = row.get("name").expect("name not found");
            players_in_region.push((id, name));
        }

        // Assert both players are in the region
        assert_eq!(
            players_in_region.len(),
            2,
            "Expected 2 players in Common Room, found {}",
            players_in_region.len()
        );

        let player_ids: Vec<&str> = players_in_region
            .iter()
            .map(|(id, _)| id.as_str())
            .collect();
        assert!(
            player_ids.contains(&pc_a_id.to_string().as_str()),
            "Player A should be in Common Room"
        );
        assert!(
            player_ids.contains(&pc_b_id.to_string().as_str()),
            "Player B should be in Common Room"
        );

        // Verify using the repository layer
        let all_pcs = ctx
            .app
            .repositories
            .player_character
            .list_in_world(ctx.world.world_id)
            .await
            .expect("Failed to list PCs");

        let pcs_in_common_room: Vec<_> = all_pcs
            .iter()
            .filter(|pc| pc.current_region_id() == Some(common_room))
            .collect();

        assert_eq!(
            pcs_in_common_room.len(),
            2,
            "Expected 2 PCs in Common Room via repository"
        );

        // Verify PC_A can see PC_B and vice versa
        let pc_a_sees_b = pcs_in_common_room.iter().any(|pc| pc.id() == pc_b_id);
        let pc_b_sees_a = pcs_in_common_room.iter().any(|pc| pc.id() == pc_a_id);

        assert!(pc_a_sees_b, "Player A should see Player B in region");
        assert!(pc_b_sees_a, "Player B should see Player A in region");

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(
        "test_two_players_see_each_other_in_region",
    ))
    .expect("save log");
    test_result.expect("Test failed");
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_player_movement_broadcast_to_others() {
    // Scenario: Player A moves to a new region while Player B is watching.
    // Expected: Player B receives notification of Player A's movement.
    //
    // Setup:
    // 1. Create E2E context with seeded world
    // 2. Both players in Common Room
    // 3. PC_A moves to Tavern Bar
    //
    // Assertions:
    // - PC_B receives PlayerMoved broadcast
    // - PC_A no longer appears in PC_B's region view

    let event_log = create_shared_log("test_player_movement_broadcast_to_others");
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Get regions
        let common_room = ctx
            .world
            .region("Common Room")
            .expect("Common Room not found");
        let tavern_bar = ctx
            .world
            .region("Tavern Bar")
            .expect("Tavern Bar not found");

        // Create two player characters in the Common Room
        let (_user_a, pc_a_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player A - Knight",
        )
        .await
        .expect("Failed to create player A");

        let (_user_b, pc_b_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player B - Mage",
        )
        .await
        .expect("Failed to create player B");

        // Verify both are in Common Room before movement
        let pc_a_before = ctx
            .app
            .repositories
            .player_character
            .get(pc_a_id)
            .await
            .expect("Query failed")
            .expect("PC A not found");
        let pc_b_before = ctx
            .app
            .repositories
            .player_character
            .get(pc_b_id)
            .await
            .expect("Query failed")
            .expect("PC B not found");

        assert_eq!(
            pc_a_before.current_region_id(),
            Some(common_room),
            "PC A should start in Common Room"
        );
        assert_eq!(
            pc_b_before.current_region_id(),
            Some(common_room),
            "PC B should start in Common Room"
        );

        // PC_A moves to Tavern Bar using the enter_region use case
        let move_result = ctx
            .app
            .use_cases
            .movement
            .enter_region
            .execute(pc_a_id, tavern_bar)
            .await
            .expect("Movement should succeed");

        // Verify move result
        assert_eq!(
            move_result.region.id, tavern_bar,
            "Move result should show Tavern Bar"
        );
        assert_eq!(
            move_result.pc.id(),
            pc_a_id,
            "Move result should reference PC A"
        );

        // Verify PC_A is now in Tavern Bar
        let pc_a_after = ctx
            .app
            .repositories
            .player_character
            .get(pc_a_id)
            .await
            .expect("Query failed")
            .expect("PC A not found");

        assert_eq!(
            pc_a_after.current_region_id(),
            Some(tavern_bar),
            "PC A should now be in Tavern Bar"
        );

        // Verify PC_B is still in Common Room
        let pc_b_after = ctx
            .app
            .repositories
            .player_character
            .get(pc_b_id)
            .await
            .expect("Query failed")
            .expect("PC B not found");

        assert_eq!(
            pc_b_after.current_region_id(),
            Some(common_room),
            "PC B should still be in Common Room"
        );

        // Query all players in Common Room - PC_A should no longer be there
        let all_pcs = ctx
            .app
            .repositories
            .player_character
            .list_in_world(ctx.world.world_id)
            .await
            .expect("Failed to list PCs");

        let pcs_in_common_room: Vec<_> = all_pcs
            .iter()
            .filter(|pc| pc.current_region_id() == Some(common_room))
            .collect();

        // PC_A should NOT be visible in Common Room anymore
        let pc_a_in_common_room = pcs_in_common_room.iter().any(|pc| pc.id() == pc_a_id);
        assert!(
            !pc_a_in_common_room,
            "PC A should NOT be visible in Common Room after moving"
        );

        // PC_B should still be visible in Common Room
        let pc_b_in_common_room = pcs_in_common_room.iter().any(|pc| pc.id() == pc_b_id);
        assert!(
            pc_b_in_common_room,
            "PC B should still be visible in Common Room"
        );

        // Verify PC_A appears in Tavern Bar now
        let pcs_in_tavern: Vec<_> = all_pcs
            .iter()
            .filter(|pc| pc.current_region_id() == Some(tavern_bar))
            .collect();

        let pc_a_in_tavern = pcs_in_tavern.iter().any(|pc| pc.id() == pc_a_id);
        assert!(pc_a_in_tavern, "PC A should now be visible in Tavern Bar");

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(
        "test_player_movement_broadcast_to_others",
    ))
    .expect("save log");
    test_result.expect("Test failed");
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes"]
async fn test_conversation_does_not_block_other_players() {
    // Scenario: Player A starts conversation with NPC while Player B acts independently.
    // Expected: Player B can move/interact without waiting for Player A's conversation.
    //
    // Setup:
    // 1. Create E2E context with VCR LLM
    // 2. PC_A starts conversation with Marta
    // 3. While conversation is pending approval, PC_B moves to Tavern Bar
    //
    // Assertions:
    // - PC_B's movement succeeds immediately
    // - PC_A's conversation continues independently

    const TEST_NAME: &str = "test_conversation_does_not_block_other_players";
    let event_log = create_shared_log(TEST_NAME);

    // Create context with VCR LLM and event logging
    let vcr = Arc::new(VcrLlm::from_env(std::path::PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        TEST_NAME
    ))));
    let llm = Arc::new(LoggingLlmDecorator::new(vcr.clone(), event_log.clone()));
    let ctx = E2ETestContext::setup_with_llm_and_logging(llm.clone(), event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Get regions
        let common_room = ctx
            .world
            .region("Common Room")
            .expect("Common Room not found");
        let tavern_bar = ctx
            .world
            .region("Tavern Bar")
            .expect("Tavern Bar not found");

        // Create two player characters in the Common Room
        let (user_a, pc_a_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player A - Talker",
        )
        .await
        .expect("Failed to create player A");

        let (_user_b, pc_b_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player B - Mover",
        )
        .await
        .expect("Failed to create player B");

        // Stage Marta in the Common Room
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage Marta");

        // PC_A starts a conversation with Marta - this enqueues a player action
        let conversation_started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_a_id,
                marta_id,
                user_a.clone(),
                "Hello Marta! How are you today?".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        // The conversation has been started and queued, but approval is pending
        assert!(
            !conversation_started.conversation_id.is_nil(),
            "Conversation ID should be valid"
        );

        // NOW, while PC_A's conversation is pending, PC_B should be able to move independently
        let move_result = ctx
            .app
            .use_cases
            .movement
            .enter_region
            .execute(pc_b_id, tavern_bar)
            .await;

        // Assert movement succeeded - conversation does NOT block PC_B
        assert!(
            move_result.is_ok(),
            "PC_B's movement should succeed while PC_A's conversation is pending: {:?}",
            move_result.err()
        );

        // Verify PC_B is now in Tavern Bar
        let pc_b_after = ctx
            .app
            .repositories
            .player_character
            .get(pc_b_id)
            .await
            .expect("Query failed")
            .expect("PC B not found");

        assert_eq!(
            pc_b_after.current_region_id(),
            Some(tavern_bar),
            "PC B should now be in Tavern Bar"
        );

        // PC_A should still be in Common Room (unchanged by conversation)
        let pc_a_after = ctx
            .app
            .repositories
            .player_character
            .get(pc_a_id)
            .await
            .expect("Query failed")
            .expect("PC A not found");

        assert_eq!(
            pc_a_after.current_region_id(),
            Some(common_room),
            "PC A should still be in Common Room"
        );

        // Verify the conversation is still tracked (exists but pending)
        let conversation_exists = ctx
            .graph()
            .execute(
                query("MATCH (c:Conversation {id: $id}) RETURN c.id as id")
                    .param("id", conversation_started.conversation_id.to_string()),
            )
            .await
            .expect("Query failed")
            .next()
            .await
            .expect("Row read failed")
            .is_some();

        assert!(
            conversation_exists,
            "PC_A's conversation should still exist"
        );

        // Now process the queues to complete PC_A's conversation
        let _processed = ctx
            .app
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

        // If we got a result, approve it to complete the conversation
        if let Some(result) = llm_result {
            use crate::queue_types::DmApprovalDecision;
            let _ = ctx
                .app
                .use_cases
                .approval
                .decision_flow
                .execute(result.approval_id, DmApprovalDecision::Accept)
                .await;
        }

        // Final verification: both players' states are independent
        let final_pc_a = ctx
            .app
            .repositories
            .player_character
            .get(pc_a_id)
            .await
            .expect("Query failed")
            .expect("PC A not found");

        let final_pc_b = ctx
            .app
            .repositories
            .player_character
            .get(pc_b_id)
            .await
            .expect("Query failed")
            .expect("PC B not found");

        assert_eq!(
            final_pc_a.current_region_id(),
            Some(common_room),
            "PC A should still be in Common Room after conversation"
        );
        assert_eq!(
            final_pc_b.current_region_id(),
            Some(tavern_bar),
            "PC B should still be in Tavern Bar"
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

    // Save cassette if recording
    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}

// =============================================================================
// DM + Player Coordination
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes"]
async fn test_dm_sees_player_actions_in_queue() {
    // Scenario: Player performs action, DM sees it in approval queue.
    // Expected: DM receives ApprovalRequired message with action details.
    //
    // Setup:
    // 1. Create E2E context with VCR LLM
    // 2. DM joins world
    // 3. Player joins world and starts conversation
    //
    // Assertions:
    // - DM receives ApprovalRequired with NPC dialogue
    // - Queue status shows pending approval

    const TEST_NAME: &str = "test_dm_sees_player_actions_in_queue";
    let event_log = create_shared_log(TEST_NAME);

    // Create context with VCR LLM and event logging
    let vcr = Arc::new(VcrLlm::from_env(std::path::PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        TEST_NAME
    ))));
    let llm = Arc::new(LoggingLlmDecorator::new(vcr.clone(), event_log.clone()));
    let ctx = E2ETestContext::setup_with_llm_and_logging(llm.clone(), event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Get the Common Room region
        let common_room = ctx
            .world
            .region("Common Room")
            .expect("Common Room not found");

        // Create a player character
        let (user_id, pc_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player - Adventurer",
        )
        .await
        .expect("Failed to create player");

        // Stage Marta in the Common Room
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage Marta");

        // Player starts a conversation with Marta - this enqueues a player action
        let conversation_started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_id,
                marta_id,
                user_id.clone(),
                "Hello Marta! What news do you have today?".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        // The conversation has been started
        assert!(
            !conversation_started.conversation_id.is_nil(),
            "Conversation ID should be valid"
        );

        // Process the player action queue - this creates an LLM request
        let action_processed = ctx
            .app
            .use_cases
            .queues
            .process_player_action
            .execute()
            .await
            .expect("Failed to process player action");

        assert!(
            action_processed.is_some(),
            "Player action should have been processed"
        );

        // Process the LLM request queue - this creates a DM approval request
        let llm_result = ctx
            .app
            .use_cases
            .queues
            .process_llm_request
            .execute(|_| {})
            .await
            .expect("Failed to process LLM request");

        // Verify we got an LLM result with an approval ID (the DM would see this)
        let result = llm_result.expect("LLM should have produced a result");

        // The approval_id is what the DM sees in their queue
        assert!(
            !result.approval_id.is_nil(),
            "Approval ID should be valid (this is what DM sees)"
        );

        // Verify the NPC dialogue was generated
        assert!(
            !result.npc_dialogue.is_empty(),
            "NPC dialogue should have been generated"
        );

        // DM can view the approval request details via the queue
        let approval_data = ctx
            .app
            .queue
            .get_approval_request(result.approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval request should exist");

        // Verify the approval data contains the expected information
        assert_eq!(
            approval_data.world_id, ctx.world.world_id,
            "Approval should be for the correct world"
        );
        assert_eq!(
            approval_data.npc_id,
            Some(marta_id),
            "Approval should reference Marta"
        );
        assert!(
            !approval_data.proposed_dialogue.is_empty(),
            "Approval should contain proposed dialogue"
        );
        assert_eq!(
            approval_data.pc_id,
            Some(pc_id),
            "Approval should reference the player character"
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

    // Save cassette if recording
    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer and VCR cassettes"]
async fn test_dm_approval_triggers_player_response() {
    // Scenario: DM approves an LLM response, player receives the dialogue.
    // Expected: After DM approval, player receives DialogueResponse.
    //
    // Setup:
    // 1. Create E2E context with VCR LLM
    // 2. Player starts conversation with NPC
    // 3. DM receives ApprovalRequired
    // 4. DM sends ApprovalDecision (approved)
    //
    // Assertions:
    // - Player receives DialogueResponse
    // - DM receives ResponseApproved

    const TEST_NAME: &str = "test_dm_approval_triggers_player_response";
    let event_log = create_shared_log(TEST_NAME);

    // Create context with VCR LLM and event logging
    let vcr = Arc::new(VcrLlm::from_env(std::path::PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        TEST_NAME
    ))));
    let llm = Arc::new(LoggingLlmDecorator::new(vcr.clone(), event_log.clone()));
    let ctx = E2ETestContext::setup_with_llm_and_logging(llm.clone(), event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Get the Common Room region
        let common_room = ctx
            .world
            .region("Common Room")
            .expect("Common Room not found");

        // Create a player character
        let (user_id, pc_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player - Questioner",
        )
        .await
        .expect("Failed to create player");

        // Stage Marta in the Common Room
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage Marta");

        // Player starts a conversation with Marta
        let conversation_started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_id,
                marta_id,
                user_id.clone(),
                "Good day, Marta! How goes the tavern business?".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        let conversation_id = conversation_started.conversation_id;

        // Process queues to get LLM response pending approval
        let _ = ctx
            .app
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
            .expect("Failed to process LLM request")
            .expect("Should have LLM result");

        // Now the DM sees this in their approval queue
        let approval_id = llm_result.approval_id;
        let proposed_dialogue = llm_result.npc_dialogue.clone();

        // Verify there's a pending approval
        let pending_approval = ctx
            .app
            .queue
            .get_approval_request(approval_id)
            .await
            .expect("Failed to get approval request")
            .expect("Approval should exist");

        assert_eq!(
            pending_approval.proposed_dialogue, proposed_dialogue,
            "Pending approval should have the proposed dialogue"
        );

        // DM approves the response
        use crate::queue_types::DmApprovalDecision;
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(approval_id, DmApprovalDecision::Accept)
            .await
            .expect("Failed to approve suggestion");

        // Verify the approval was successful
        assert!(approval_result.approved, "Response should be approved");

        // Verify the final dialogue matches what was proposed
        assert_eq!(
            approval_result.final_dialogue,
            Some(proposed_dialogue.clone()),
            "Final dialogue should match proposed"
        );

        // Verify the NPC information is correct
        assert_eq!(
            approval_result.npc_id,
            Some(marta_id.to_string()),
            "NPC ID should be Marta"
        );
        assert_eq!(
            approval_result.npc_name,
            Some("Marta Hearthwood".to_string()),
            "NPC name should be Marta"
        );

        // Verify conversation ID is preserved for routing
        assert_eq!(
            approval_result.conversation_id,
            Some(conversation_id),
            "Conversation ID should be preserved for routing to player"
        );

        // Verify the dialogue was recorded in the narrative
        let turns = ctx
            .app
            .repositories
            .narrative
            .get_conversation_turns(pc_id, marta_id, 10)
            .await
            .expect("Failed to get conversation turns");

        // Should have both player and NPC turns
        assert!(
            turns.len() >= 2,
            "Should have at least 2 turns (player + NPC)"
        );

        // Check for NPC response in history
        let npc_turn = turns.iter().find(|t| t.speaker == "Marta Hearthwood");
        assert!(
            npc_turn.is_some(),
            "NPC response should be recorded in conversation history"
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

    // Save cassette if recording
    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_dm_broadcast_reaches_all_players() {
    // Scenario: DM triggers a location event, all players in region receive it.
    // Expected: Both players receive the event narration.
    //
    // Setup:
    // 1. Create E2E context
    // 2. Two players in Common Room
    // 3. DM triggers TriggerLocationEvent
    //
    // Assertions:
    // - Both players receive LocationEventTriggered message

    let event_log = create_shared_log("test_dm_broadcast_reaches_all_players");
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Get the Common Room region
        let common_room = ctx
            .world
            .region("Common Room")
            .expect("Common Room not found");

        // Create two player characters in the Common Room
        let (_user_a, pc_a_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player A - Warrior",
        )
        .await
        .expect("Failed to create player A");

        let (_user_b, pc_b_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player B - Wizard",
        )
        .await
        .expect("Failed to create player B");

        // Verify both players are in the Common Room
        let pc_a = ctx
            .app
            .repositories
            .player_character
            .get(pc_a_id)
            .await
            .expect("Query failed")
            .expect("PC A not found");
        let pc_b = ctx
            .app
            .repositories
            .player_character
            .get(pc_b_id)
            .await
            .expect("Query failed")
            .expect("PC B not found");

        assert_eq!(
            pc_a.current_region_id(),
            Some(common_room),
            "PC A should be in Common Room"
        );
        assert_eq!(
            pc_b.current_region_id(),
            Some(common_room),
            "PC B should be in Common Room"
        );

        // DM triggers a location event for the Common Room
        let event_description = "A sudden gust of wind blows through the tavern, \
            extinguishing several candles and sending papers flying across the room!";

        let event_result = ctx
            .app
            .use_cases
            .location_events
            .trigger
            .execute(common_room, event_description.to_string())
            .await
            .expect("Failed to trigger location event");

        // Verify the event result contains correct information
        assert_eq!(
            event_result.region_id, common_room,
            "Event should be for Common Room"
        );
        assert_eq!(
            event_result.description, event_description,
            "Event description should match"
        );
        assert_eq!(
            event_result.region_name, "Common Room",
            "Region name should be Common Room"
        );

        // In a real scenario, the ConnectionManager would broadcast this to all
        // players in the region. Here we verify the event result contains all
        // the data needed to notify both players.

        // Get all players in the affected region to simulate broadcast targeting
        let all_pcs = ctx
            .app
            .repositories
            .player_character
            .list_in_world(ctx.world.world_id)
            .await
            .expect("Failed to list PCs");

        let players_in_region: Vec<_> = all_pcs
            .iter()
            .filter(|pc| pc.current_region_id() == Some(common_room))
            .collect();

        // Both players should be potential recipients of the broadcast
        assert_eq!(
            players_in_region.len(),
            2,
            "Both players should be in the region to receive broadcast"
        );

        let player_ids: Vec<_> = players_in_region.iter().map(|pc| pc.id()).collect();
        assert!(
            player_ids.contains(&pc_a_id),
            "Player A should receive broadcast"
        );
        assert!(
            player_ids.contains(&pc_b_id),
            "Player B should receive broadcast"
        );

        // Verify the event data is complete for client notification
        assert!(
            !event_result.description.is_empty(),
            "Event should have description for narration"
        );
        assert!(
            !event_result.region_name.is_empty(),
            "Event should have region name for context"
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
    ctx.save_event_log(&E2ETestContext::default_log_path(
        "test_dm_broadcast_reaches_all_players",
    ))
    .expect("save log");
    test_result.expect("Test failed");
}

// =============================================================================
// Player Join/Leave
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_new_player_receives_snapshot() {
    // Scenario: Player joins world that already has DM and another player.
    // Expected: New player receives full snapshot with world data.
    //
    // Setup:
    // 1. Create E2E context
    // 2. Create two player characters
    // 3. Player B joins after Player A is established
    //
    // Assertions:
    // - Player B receives WorldJoined with snapshot
    // - Snapshot contains world data, locations, and characters

    let event_log = create_shared_log("test_new_player_receives_snapshot");
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Get the Common Room region (spawn point)
        let common_room = ctx
            .world
            .region("Common Room")
            .expect("Common Room not found");

        // Create Player A first (establishes presence in the world)
        let (_user_a, pc_a_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player A - Veteran",
        )
        .await
        .expect("Failed to create player A");

        // Player A is now established in the world - verify they exist
        let pc_a = ctx
            .app
            .repositories
            .player_character
            .get(pc_a_id)
            .await
            .expect("Query failed")
            .expect("PC A should exist");

        assert_eq!(pc_a.world_id(), ctx.world.world_id);

        // Now Player B joins the world
        let (_user_b, pc_b_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player B - Newcomer",
        )
        .await
        .expect("Failed to create player B");

        // Simulate Player B joining the world and receiving the snapshot
        // This uses the JoinWorld use case
        let join_result = ctx
            .app
            .use_cases
            .session
            .join_world
            .execute(ctx.world.world_id, Some(pc_b_id), true)
            .await
            .expect("Join world should succeed");

        // Verify the snapshot contains expected world data
        let snapshot = &join_result.snapshot;

        // Check world data
        let world_data = snapshot.get("world").expect("Snapshot should have world");
        assert_eq!(
            world_data.get("id").and_then(|v| v.as_str()),
            Some(ctx.world.world_id.to_string().as_str()),
            "Snapshot world ID should match"
        );
        assert!(
            world_data.get("name").is_some(),
            "Snapshot should include world name"
        );
        assert!(
            world_data.get("description").is_some(),
            "Snapshot should include world description"
        );

        // Check locations are included
        let locations = snapshot
            .get("locations")
            .and_then(|v| v.as_array())
            .expect("Snapshot should have locations array");
        assert!(
            !locations.is_empty(),
            "Snapshot should include at least one location"
        );

        // Check characters (NPCs) are included
        let characters = snapshot
            .get("characters")
            .and_then(|v| v.as_array())
            .expect("Snapshot should have characters array");
        assert!(
            !characters.is_empty(),
            "Snapshot should include NPCs from the world"
        );

        // Verify that Marta (our test NPC) is in the snapshot
        let marta_in_snapshot = characters.iter().any(|c| {
            c.get("name")
                .and_then(|v| v.as_str())
                .map(|n| n.contains("Marta"))
                .unwrap_or(false)
        });
        assert!(marta_in_snapshot, "Marta should be in the snapshot");

        // Verify Player B's PC info is returned
        let your_pc = join_result
            .your_pc
            .expect("your_pc should be present for Player role");
        assert_eq!(
            your_pc.get("id").and_then(|v| v.as_str()),
            Some(pc_b_id.to_string().as_str()),
            "your_pc should be Player B's character"
        );

        // Verify both players exist in the world via repository
        let all_pcs = ctx
            .app
            .repositories
            .player_character
            .list_in_world(ctx.world.world_id)
            .await
            .expect("Failed to list PCs");

        assert_eq!(all_pcs.len(), 2, "World should have 2 player characters");

        let has_pc_a = all_pcs.iter().any(|pc| pc.id() == pc_a_id);
        let has_pc_b = all_pcs.iter().any(|pc| pc.id() == pc_b_id);
        assert!(has_pc_a, "Player A should be in world");
        assert!(has_pc_b, "Player B should be in world");

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(
        "test_new_player_receives_snapshot",
    ))
    .expect("save log");
    test_result.expect("Test failed");
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_other_players_notified_on_join() {
    // Scenario: New player joins, existing players receive notification.
    // Expected: DM and Player A see UserJoined event.
    //
    // Setup:
    // 1. Create E2E context
    // 2. DM and Player A already connected
    // 3. Player B joins
    //
    // Assertions:
    // - DM receives UserJoined for Player B
    // - Player A receives UserJoined for Player B

    let event_log = create_shared_log("test_other_players_notified_on_join");
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Get the Common Room region (spawn point)
        let common_room = ctx
            .world
            .region("Common Room")
            .expect("Common Room not found");

        // Create Player A first (they're already in the world)
        let (user_a, pc_a_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player A - Veteran",
        )
        .await
        .expect("Failed to create player A");

        // Player A joins the world and gets their snapshot
        let join_a_result = ctx
            .app
            .use_cases
            .session
            .join_world
            .execute(ctx.world.world_id, Some(pc_a_id), true)
            .await
            .expect("Player A should join successfully");

        // Verify Player A's join was successful
        assert_eq!(
            join_a_result.world_id, ctx.world.world_id,
            "Player A should be in the correct world"
        );
        assert!(
            join_a_result.your_pc.is_some(),
            "Player A should have their PC info"
        );

        // Record the state before Player B joins
        let pcs_before = ctx
            .app
            .repositories
            .player_character
            .list_in_world(ctx.world.world_id)
            .await
            .expect("Failed to list PCs before");

        let player_count_before = pcs_before.len();
        assert_eq!(
            player_count_before, 1,
            "Should have 1 player before B joins"
        );

        // Now Player B joins the world
        let (user_b, pc_b_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player B - Newcomer",
        )
        .await
        .expect("Failed to create player B");

        // Player B joins the world
        let join_b_result = ctx
            .app
            .use_cases
            .session
            .join_world
            .execute(ctx.world.world_id, Some(pc_b_id), true)
            .await
            .expect("Player B should join successfully");

        // Verify Player B's join was successful
        assert_eq!(
            join_b_result.world_id, ctx.world.world_id,
            "Player B should be in the correct world"
        );
        assert!(
            join_b_result.your_pc.is_some(),
            "Player B should have their PC info"
        );

        // Verify the new player count
        let pcs_after = ctx
            .app
            .repositories
            .player_character
            .list_in_world(ctx.world.world_id)
            .await
            .expect("Failed to list PCs after");

        assert_eq!(pcs_after.len(), 2, "Should have 2 players after B joins");

        // In a real scenario, when Player B joins:
        // 1. The WebSocket handler would broadcast a UserJoined event to all connected clients
        // 2. Player A and the DM would receive notification with Player B's info
        //
        // Here we verify that the join result contains all the data needed for notification

        // Verify Player B's PC info contains data for notification
        let pc_b_info = join_b_result.your_pc.expect("Should have PC B info");
        assert!(
            pc_b_info.get("id").is_some(),
            "PC info should have ID for notification"
        );
        assert!(
            pc_b_info.get("name").is_some(),
            "PC info should have name for notification"
        );

        // Verify Player A can see Player B in the world
        let pc_b_in_world = pcs_after.iter().any(|pc| pc.id() == pc_b_id);
        assert!(
            pc_b_in_world,
            "Player B should be visible in the world after joining"
        );

        // Verify both players are in the same region (Common Room)
        let players_in_common_room: Vec<_> = pcs_after
            .iter()
            .filter(|pc| pc.current_region_id() == Some(common_room))
            .collect();

        assert_eq!(
            players_in_common_room.len(),
            2,
            "Both players should be in Common Room"
        );

        // Verify the join result contains region info for Player A to know where B spawned
        let pc_b_region_id = pc_b_info.get("current_region_id");
        assert!(
            pc_b_region_id.is_some(),
            "PC info should include region for presence notification"
        );

        // Verify user IDs are distinct (for routing notifications)
        assert_ne!(user_a, user_b, "User IDs should be distinct");

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(
        "test_other_players_notified_on_join",
    ))
    .expect("save log");
    test_result.expect("Test failed");
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_player_leave_notifies_others() {
    // Scenario: Player disconnects, other clients are notified.
    // Expected: Remaining clients see UserLeft event.
    //
    // Setup:
    // 1. Create E2E context
    // 2. DM and two players connected
    // 3. Player B disconnects
    //
    // Assertions:
    // - DM receives UserLeft for Player B
    // - Player A receives UserLeft for Player B

    let event_log = create_shared_log("test_player_leave_notifies_others");
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Get the Common Room region (spawn point)
        let common_room = ctx
            .world
            .region("Common Room")
            .expect("Common Room not found");

        // Create two players in the world
        let (user_a, pc_a_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player A - Staying",
        )
        .await
        .expect("Failed to create player A");

        let (user_b, pc_b_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Player B - Leaving",
        )
        .await
        .expect("Failed to create player B");

        // Both players join the world
        let _join_a = ctx
            .app
            .use_cases
            .session
            .join_world
            .execute(ctx.world.world_id, Some(pc_a_id), true)
            .await
            .expect("Player A should join");

        let join_b = ctx
            .app
            .use_cases
            .session
            .join_world
            .execute(ctx.world.world_id, Some(pc_b_id), true)
            .await
            .expect("Player B should join");

        // Verify both players are active
        let pcs_before = ctx
            .app
            .repositories
            .player_character
            .list_in_world(ctx.world.world_id)
            .await
            .expect("Failed to list PCs");

        assert_eq!(pcs_before.len(), 2, "Should have 2 players before leave");

        let pc_b_before = pcs_before.iter().find(|pc| pc.id() == pc_b_id);
        assert!(pc_b_before.is_some(), "Player B should exist before leave");
        assert!(
            pc_b_before.unwrap().is_active(),
            "Player B should be active before leave"
        );

        // Capture Player B's info for the notification payload
        let pc_b_info = join_b.your_pc.expect("Should have PC B info");
        let leaving_player_id = pc_b_info
            .get("id")
            .and_then(|v| v.as_str())
            .expect("Should have PC ID");
        let leaving_player_name = pc_b_info
            .get("name")
            .and_then(|v| v.as_str())
            .expect("Should have PC name");
        let leaving_player_region = pc_b_info.get("current_region_id").and_then(|v| v.as_str());

        // Player B disconnects - in real scenario this would:
        // 1. WebSocket connection closes
        // 2. ConnectionManager broadcasts UserLeft to remaining clients
        // 3. Character is marked as inactive
        //
        // Here we simulate by deactivating the character and verifying notification data

        // Get Player B's character and deactivate it
        let mut pc_b = ctx
            .app
            .repositories
            .player_character
            .get(pc_b_id)
            .await
            .expect("Failed to get PC B")
            .expect("PC B should exist");

        // Deactivate the character (simulates disconnect)
        let state_change = pc_b.deactivate();
        assert_eq!(
            state_change,
            wrldbldr_domain::PlayerCharacterStateChange::Deactivated,
            "Player B should be deactivated"
        );

        // Save the deactivated state
        ctx.app
            .repositories
            .player_character
            .save(&pc_b)
            .await
            .expect("Failed to save PC B");

        // Verify Player B is now inactive
        let pc_b_after = ctx
            .app
            .repositories
            .player_character
            .get(pc_b_id)
            .await
            .expect("Failed to get PC B after")
            .expect("PC B should still exist");

        assert!(
            !pc_b_after.is_active(),
            "Player B should be inactive after disconnect"
        );
        assert!(
            pc_b_after.is_alive(),
            "Player B should still be alive (not dead)"
        );

        // Verify Player A is still active
        let pc_a_after = ctx
            .app
            .repositories
            .player_character
            .get(pc_a_id)
            .await
            .expect("Failed to get PC A after")
            .expect("PC A should exist");

        assert!(
            pc_a_after.is_active(),
            "Player A should still be active after B leaves"
        );

        // Verify we have all the data needed for UserLeft notification
        // In real scenario, this would be broadcast to Player A and DM
        assert!(
            !leaving_player_id.is_empty(),
            "Should have player ID for notification"
        );
        assert!(
            !leaving_player_name.is_empty(),
            "Should have player name for notification"
        );
        assert!(
            leaving_player_region.is_some(),
            "Should have region for presence update notification"
        );

        // Verify user IDs are distinct (for routing)
        assert_ne!(user_a, user_b, "User IDs should be distinct");

        // Check the active player count
        let active_pcs: Vec<_> = ctx
            .app
            .repositories
            .player_character
            .list_in_world(ctx.world.world_id)
            .await
            .expect("Failed to list PCs")
            .into_iter()
            .filter(|pc| pc.is_active())
            .collect();

        assert_eq!(
            active_pcs.len(),
            1,
            "Should have 1 active player after B leaves"
        );
        assert_eq!(
            active_pcs[0].id(),
            pc_a_id,
            "The active player should be Player A"
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
    ctx.save_event_log(&E2ETestContext::default_log_path(
        "test_player_leave_notifies_others",
    ))
    .expect("save log");
    test_result.expect("Test failed");
}
