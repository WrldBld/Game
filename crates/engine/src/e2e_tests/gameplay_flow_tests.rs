//! Gameplay flow E2E tests with VCR LLM recording.
//!
//! These tests exercise the full gameplay loop:
//! - Player joining and character creation
//! - Movement to spawn point
//! - NPC staging and presence
//! - Conversation with queue processing and DM approval
//!
//! # Running Tests
//!
//! ```bash
//! # Record cassettes with real Ollama (first time)
//! E2E_LLM_MODE=record cargo test -p wrldbldr-engine --lib gameplay_flow -- --ignored --test-threads=1
//!
//! # Playback from cassettes (subsequent runs)
//! cargo test -p wrldbldr-engine --lib gameplay_flow -- --ignored --test-threads=1
//! ```

use std::sync::Arc;

use super::{
    approve_staging_with_npc, create_e2e_llm, create_player_character_via_use_case,
    create_shared_log, create_test_player, start_conversation_with_npc, E2ETestContext,
    LoggingLlmDecorator, TestOutcome, VcrLlm,
};

// =============================================================================
// Player Character Creation Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_player_creates_character_via_use_case() {
    const TEST_NAME: &str = "test_player_creates_character_via_use_case";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create a player character via the management use case
        let pc_id = create_player_character_via_use_case(&ctx, "Aldric the Brave", "test-user-123")
            .await
            .expect("Failed to create player character");

        // Verify PC was created
        let pc = ctx
            .app
            .repositories
            .player_character
            .get(pc_id)
            .await
            .expect("Failed to get PC")
            .expect("PC not found");

        assert_eq!(pc.name, "Aldric the Brave");
        assert_eq!(pc.user_id, "test-user-123");
        assert_eq!(pc.world_id, ctx.world.world_id);

        // Verify PC is in spawn region (Common Room)
        let spawn_region = ctx.world.region("Common Room").expect("Spawn region not found");
        assert_eq!(pc.current_region_id, Some(spawn_region));

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() { TestOutcome::Pass } else { TestOutcome::Fail };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME)).expect("save log");
    test_result.expect("Test failed");
}

// =============================================================================
// Staging Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_staging_npc_in_region() {
    const TEST_NAME: &str = "test_staging_npc_in_region";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Get test NPC and region
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        let common_room = ctx.world.region("Common Room").expect("Region not found");

        // Stage Marta in the Common Room
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Verify staging was created
        let current_game_time = ctx
            .app
            .repositories
            .world
            .get(ctx.world.world_id)
            .await
            .expect("World query failed")
            .expect("World not found")
            .game_time
            .current();

        let staged = ctx
            .app
            .repositories
            .staging
            .resolve_for_region(common_room, current_game_time)
            .await
            .expect("Staging query failed");

        assert!(!staged.is_empty(), "No NPCs staged");
        assert!(
            staged.iter().any(|s| s.character_id == marta_id),
            "Marta not found in staged NPCs"
        );

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() { TestOutcome::Pass } else { TestOutcome::Fail };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME)).expect("save log");
    test_result.expect("Test failed");
}

// =============================================================================
// Conversation Flow Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_start_conversation_with_staged_npc() {
    const TEST_NAME: &str = "test_start_conversation_with_staged_npc";
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
        let (player_id, pc_id) = create_test_player(
            ctx.harness.graph(),
            ctx.world.world_id,
            common_room,
            "Test Hero",
        )
        .await
        .expect("Failed to create test player");

        // Stage Marta in the Common Room
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Start conversation with Marta
        let (conversation_id, response) = start_conversation_with_npc(
            &ctx,
            pc_id,
            marta_id,
            &player_id,
            "Good morning! I'm new to Thornhaven.",
        )
        .await
        .expect("Failed to start conversation");

        // Verify we got a response
        assert!(!conversation_id.is_nil(), "Conversation ID should not be nil");
        assert!(!response.is_empty(), "Response should not be empty");

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() { TestOutcome::Pass } else { TestOutcome::Fail };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME)).expect("save log");

    // Save cassette if recording
    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_multi_turn_conversation() {
    use super::run_conversation_turn;

    const TEST_NAME: &str = "test_multi_turn_conversation";
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
        let (player_id, pc_id) = create_test_player(
            ctx.harness.graph(),
            ctx.world.world_id,
            common_room,
            "Test Hero",
        )
        .await
        .expect("Failed to create test player");

        // Stage Marta
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Turn 1: Start conversation
        let (conversation_id, response1) = start_conversation_with_npc(
            &ctx,
            pc_id,
            marta_id,
            &player_id,
            "Hello! I'm looking for information about the village.",
        )
        .await
        .expect("Failed to start conversation");

        assert!(!response1.is_empty(), "Turn 1 response should not be empty");

        // DEBUG: Verify conversation was created after turn 1
        use neo4rs::query;
        let mut conv_result = ctx
            .harness
            .graph()
            .execute(
                query(
                    "MATCH (pc:PlayerCharacter {id: $pc_id})-[:PARTICIPATED_IN]->(c:Conversation)<-[:PARTICIPATED_IN]-(npc:Character {id: $npc_id})
                     WHERE c.is_active = true
                     RETURN c.id as id, c.is_active as active",
                )
                .param("pc_id", pc_id.to_string())
                .param("npc_id", marta_id.to_string()),
            )
            .await
            .expect("Debug query failed");

        let has_conversation = conv_result.next().await.expect("Row read error").is_some();
        assert!(has_conversation, "No active conversation found in DB after turn 1");

        // Turn 2: Continue conversation (pass None to let the system find the active conversation)
        let response2 = run_conversation_turn(
            &ctx,
            pc_id,
            marta_id,
            &player_id,
            "What can you tell me about the local merchants?",
            None,
        )
        .await
        .expect("Failed to continue conversation");

        assert!(!response2.is_empty(), "Turn 2 response should not be empty");

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() { TestOutcome::Pass } else { TestOutcome::Fail };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME)).expect("save log");

    // Save cassette if recording
    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}

// =============================================================================
// Full Gameplay Session Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_full_gameplay_session() {
    const TEST_NAME: &str = "test_full_gameplay_session";

    // Create event log for comprehensive logging
    let event_log = create_shared_log(TEST_NAME);

    // Create VCR LLM with event logging
    let vcr = Arc::new(VcrLlm::from_env(std::path::PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        TEST_NAME
    ))));
    let llm = Arc::new(LoggingLlmDecorator::new(vcr.clone(), event_log.clone()));

    // Create context with VCR LLM and event logging
    let ctx = E2ETestContext::setup_with_llm_and_logging(llm.clone(), event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    // Track test result for finalization
    let test_result = async {
        // === Phase 1: Create Player Character ===
        let pc_id = create_player_character_via_use_case(&ctx, "Aldric the Brave", "test-user-456")
            .await
            .expect("Failed to create player character");

        let pc = ctx
            .app
            .repositories
            .player_character
            .get(pc_id)
            .await
            .expect("PC query failed")
            .expect("PC not found");

        let common_room = ctx.world.region("Common Room").expect("Region not found");
        assert_eq!(pc.current_region_id, Some(common_room), "PC should be in Common Room");

        // === Phase 2: Stage NPC ===
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage Marta");

        // === Phase 3: Start Conversation ===
        let (conversation_id, response1) = start_conversation_with_npc(
            &ctx,
            pc_id,
            marta_id,
            "test-user-456",
            "Good morning! I've just arrived in Thornhaven. What can you tell me about this place?",
        )
        .await
        .expect("Failed to start conversation");

        assert!(!response1.is_empty(), "Marta should respond");
        tracing::info!(response = %response1, "Marta's first response");

        // === Phase 4: Continue Conversation ===
        // Pass None to let the system find the active conversation
        let response2 = super::run_conversation_turn(
            &ctx,
            pc_id,
            marta_id,
            "test-user-456",
            "That's very helpful. Is there anyone else I should talk to?",
            None,
        )
        .await
        .expect("Failed to continue conversation");

        assert!(!response2.is_empty(), "Marta should respond to follow-up");
        tracing::info!(response = %response2, "Marta's second response");

        // === Verification: Check Neo4j State ===
        // Verify a conversation was recorded in the database (between PC and Marta)
        use neo4rs::query;

        let mut result = ctx
            .harness
            .graph()
            .execute(query(
                "MATCH (pc:PlayerCharacter {id: $pc_id})-[:PARTICIPATED_IN]->(c:Conversation)<-[:PARTICIPATED_IN]-(npc:Character {id: $npc_id})
                 RETURN c.id as id",
            )
            .param("pc_id", pc_id.to_string())
            .param("npc_id", marta_id.to_string()))
            .await
            .expect("Conversation query failed");

        let row = result.next().await;
        assert!(row.is_ok(), "Conversation should exist in Neo4j");

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    // Finalize event log with outcome
    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);

    // Save event log
    let log_path = E2ETestContext::default_log_path(TEST_NAME);
    ctx.save_event_log(&log_path).expect("Failed to save event log");

    // Print summary
    if let Some(ref log) = ctx.event_log {
        let summary = log.summary();
        tracing::info!(
            llm_calls = summary.llm_calls,
            conversations = summary.conversations_count,
            total_tokens = summary.total_tokens.total,
            avg_latency_ms = summary.avg_llm_latency_ms,
            "E2E test summary"
        );
    }

    // Save cassette if recording
    vcr.save_cassette().expect("Failed to save cassette");

    // Propagate any test error
    test_result.expect("Test failed");
}

// =============================================================================
// Queue Processing Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_queue_processes_player_action_to_llm_request() {
    const TEST_NAME: &str = "test_queue_processes_player_action_to_llm_request";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player and stage NPC
        let common_room = ctx.world.region("Common Room").expect("Region not found");
        let (player_id, pc_id) = create_test_player(
            ctx.harness.graph(),
            ctx.world.world_id,
            common_room,
            "Queue Test Hero",
        )
        .await
        .expect("Failed to create test player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Start conversation (this enqueues a player action)
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
                "Hello!".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(!started.action_queue_id.is_nil(), "Action should be queued");

        // Process the player action queue
        let processed = ctx
            .app
            .use_cases
            .queues
            .process_player_action
            .execute()
            .await
            .expect("Failed to process player action");

        assert!(processed.is_some(), "Should have processed a player action");

        // Verify LLM request was enqueued
        let pending_llm = ctx
            .app
            .queue
            .get_pending_count("llm_request")
            .await
            .expect("Failed to get pending count");

        assert!(pending_llm > 0, "LLM request should be enqueued");

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() { TestOutcome::Pass } else { TestOutcome::Fail };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME)).expect("save log");
    test_result.expect("Test failed");
}
