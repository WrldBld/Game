//! E2E tests for conversation ending.
//!
//! Tests that starting and ending conversations properly updates the conversation state
//! (is_active=false, ended_at set).
//!
//! # Running Tests
//!
//! ```bash
//! # Run with VCR playback
//! cargo test -p wrldbldr-engine --lib e2e_conversation_end -- --ignored --test-threads=1
//! ```

use std::sync::Arc;

use super::*;

// =============================================================================
// Conversation End E2E Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_start_and_end_conversation_updates_state() {
    const TEST_NAME: &str = "test_start_and_end_conversation_updates_state";
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
        // Create player character
        let common_room = ctx.world.region("Common Room").expect("Region not found");
        let (player_id, pc_id) =
            create_test_player(ctx.graph(), ctx.world.world_id, common_room, "Test Hero")
                .await
                .expect("Failed to create test player");

        // Stage Marta in the Common Room
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Start conversation with Marta
        let (conversation_id, _response) = start_conversation_with_npc(
            &ctx,
            pc_id,
            marta_id,
            &player_id,
            "Good morning! I'm looking for information about the town.",
        )
        .await
        .expect("Failed to start conversation");

        // Verify conversation was created and is active
        let active_conversation_id = ctx
            .app
            .repositories
            .narrative_repo
            .get_active_conversation_id(pc_id, marta_id)
            .await
            .expect("Failed to get active conversation ID");

        assert!(active_conversation_id.is_some(), "Active conversation should exist");
        assert_eq!(active_conversation_id, Some(conversation_id));

        let is_active = ctx
            .app
            .repositories
            .narrative_repo
            .is_conversation_active(conversation_id)
            .await
            .expect("Failed to check if conversation is active");

        assert!(is_active, "Conversation should be active");

        // End the conversation
        let end_result = ctx
            .app
            .use_cases
            .conversation
            .end
            .execute(pc_id, marta_id, Some("Goodbye, thanks for the information.".to_string()))
            .await
            .expect("Failed to end conversation");

        // Verify end result
        assert_eq!(end_result.pc_name, "Test Hero");
        assert_eq!(end_result.npc_name, "Marta Hearthwood");
        assert_eq!(
            end_result.summary,
            Some("Goodbye, thanks for the information.".to_string())
        );
        assert_eq!(
            end_result.conversation_id,
            Some(conversation_id),
            "Conversation ID should match the active conversation that was ended"
        );

        // Verify conversation is now inactive
        let is_still_active = ctx
            .app
            .repositories
            .narrative_repo
            .is_conversation_active(conversation_id)
            .await
            .expect("Failed to check if conversation is still active");

        assert!(!is_still_active, "Conversation should be inactive after ending");

        // Verify no active conversation exists now
        let no_active_id = ctx
            .app
            .repositories
            .narrative_repo
            .get_active_conversation_id(pc_id, marta_id)
            .await
            .expect("Failed to check active conversation");

        assert!(
            no_active_id.is_none(),
            "No active conversation should exist after ending"
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

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_end_conversation_without_summary() {
    const TEST_NAME: &str = "test_end_conversation_without_summary";
    let event_log = create_shared_log(TEST_NAME);

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
        let common_room = ctx.world.region("Common Room").expect("Region not found");
        let (player_id, pc_id) =
            create_test_player(ctx.graph(), ctx.world.world_id, common_room, "Test Hero")
                .await
                .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Start conversation
        let (conversation_id, _response) = start_conversation_with_npc(
            &ctx,
            pc_id,
            marta_id,
            &player_id,
            "Hello!",
        )
        .await
        .expect("Failed to start conversation");

        // End conversation without a summary
        let end_result = ctx
            .app
            .use_cases
            .conversation
            .end
            .execute(pc_id, marta_id, None)
            .await
            .expect("Failed to end conversation");

        assert!(
            end_result.conversation_id.is_some(),
            "Conversation ID should be returned"
        );

        // Verify conversation is inactive
        let is_active = ctx
            .app
            .repositories
            .narrative_repo
            .is_conversation_active(end_result.conversation_id.unwrap())
            .await
            .expect("Failed to check if conversation is active");

        assert!(!is_active, "Conversation should be inactive");

        // Verify no active conversation exists
        let no_active = ctx
            .app
            .repositories
            .narrative_repo
            .get_active_conversation_id(pc_id, marta_id)
            .await
            .expect("Failed to check active conversation");

        assert!(no_active.is_none(), "No active conversation should exist after ending");

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

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_multiple_conversation_cycles() {
    const TEST_NAME: &str = "test_multiple_conversation_cycles";
    let event_log = create_shared_log(TEST_NAME);

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
        let common_room = ctx.world.region("Common Room").expect("Region not found");
        let (player_id, pc_id) =
            create_test_player(ctx.graph(), ctx.world.world_id, common_room, "Test Hero")
                .await
                .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // First conversation cycle
        let (conv1_id, _response1) = start_conversation_with_npc(
            &ctx,
            pc_id,
            marta_id,
            &player_id,
            "First conversation",
        )
        .await
        .expect("Failed to start first conversation");

        // Verify first conversation is active
        let conv1_active = ctx
            .app
            .repositories
            .narrative_repo
            .is_conversation_active(conv1_id)
            .await
            .expect("Failed to check if conversation is active");
        assert!(conv1_active, "First conversation should be active");

        // End first conversation
        let end_result1 = ctx
            .app
            .use_cases
            .conversation
            .end
            .execute(pc_id, marta_id, Some("Ending first talk".to_string()))
            .await
            .expect("Failed to end first conversation");

        assert_eq!(end_result1.conversation_id, Some(conv1_id));

        // Verify first conversation is inactive
        let conv1_inactive = ctx
            .app
            .repositories
            .narrative_repo
            .is_conversation_active(conv1_id)
            .await
            .expect("Failed to check if conversation is active");
        assert!(!conv1_inactive, "First conversation should be inactive");

        // Second conversation cycle with same NPC
        let (conv2_id, _response2) = start_conversation_with_npc(
            &ctx,
            pc_id,
            marta_id,
            &player_id,
            "Second conversation",
        )
        .await
        .expect("Failed to start second conversation");

        // Verify new conversation is active and has different ID
        let conv2_active = ctx
            .app
            .repositories
            .narrative_repo
            .is_conversation_active(conv2_id)
            .await
            .expect("Failed to check if conversation is active");
        assert!(conv2_active, "Second conversation should be active");
        assert_ne!(conv2_id, conv1_id, "Conversation IDs should be different");

        // End second conversation
        let end_result2 = ctx
            .app
            .use_cases
            .conversation
            .end
            .execute(pc_id, marta_id, Some("Ending second talk".to_string()))
            .await
            .expect("Failed to end second conversation");

        assert_eq!(end_result2.conversation_id, Some(conv2_id));

        // Both conversations should now be inactive
        let conv1_final = ctx
            .app
            .repositories
            .narrative_repo
            .is_conversation_active(conv1_id)
            .await
            .expect("Failed to check if conversation is active");
        assert!(!conv1_final, "First conversation should be inactive");

        let conv2_final = ctx
            .app
            .repositories
            .narrative_repo
            .is_conversation_active(conv2_id)
            .await
            .expect("Failed to check if conversation is active");
        assert!(!conv2_final, "Second conversation should be inactive");

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
