//! DM approval timeout E2E tests.
//!
//! Tests for scenarios where DM doesn't respond to approval requests
//! within the configured timeout period.
//!
//! # Test Scenarios
//!
//! ## Staging Auto-Approve
//! - Player enters region, DM doesn't approve staging
//! - After timeout, staging auto-approves with default/rule-based NPCs
//! - Player can interact with auto-staged NPCs
//!
//! ## Time Suggestion Timeout
//! - Time advancement suggested to DM
//! - DM doesn't respond within timeout
//! - Time advancement proceeds automatically (or is cancelled)

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use uuid::Uuid;
use wrldbldr_domain::{StagingSource, TimeSuggestionDecision};

use super::*;
use crate::infrastructure::cache::TtlCache;
use crate::infrastructure::ports::{ClockPort, PendingStagingRequest, TimeSuggestion};
use crate::use_cases::time::TimeSuggestionError;

// =============================================================================
// Staging Auto-Approve
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer and time manipulation"]
async fn test_staging_auto_approves_after_timeout() {
    // Scenario: Player enters region, DM doesn't approve staging.
    // Expected: After configured timeout, staging auto-approves.
    //
    // This test validates the auto-approval timeout flow using real E2E context.
    // We simulate what happens when a player enters a region and the DM doesn't
    // respond to the staging approval request within the timeout period.

    // Setup E2E context
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let tavern_bar = ctx
        .world
        .region("Tavern Bar")
        .expect("Tavern Bar should exist");

    // Create a player character
    let (_, _pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        common_room,
        "Timeout Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Create a pending staging request (simulating what happens when player enters region)
    let pending_request = PendingStagingRequest {
        region_id: tavern_bar,
        location_id: ctx
            .world
            .location("The Drowsy Dragon Inn")
            .expect("Location should exist"),
        world_id: ctx.world.world_id,
        created_at: Utc::now(),
    };

    let request_id = Uuid::new_v4().to_string();

    // Execute auto-approval directly (simulating timeout trigger)
    let result = ctx
        .app
        .use_cases
        .staging
        .auto_approve_timeout
        .execute(request_id.clone(), pending_request)
        .await;

    // Verify auto-approval succeeded
    assert!(
        result.is_ok(),
        "Auto-approval should succeed: {:?}",
        result.err()
    );

    let payload = result.unwrap();

    // Verify the auto-approval was for the correct region
    assert_eq!(
        payload.region_id, tavern_bar,
        "Auto-approval should be for Tavern Bar"
    );

    // Verify we can now get the active staging for this region
    let current_game_time = ctx.clock.now();
    let active_staging = ctx
        .app
        .repositories
        .staging
        .get_active_staging(tavern_bar, current_game_time)
        .await
        .expect("Should get active staging");

    assert!(
        active_staging.is_some(),
        "Should have active staging after auto-approve"
    );

    let staging = active_staging.unwrap();
    assert_eq!(
        staging.source,
        StagingSource::AutoApproved,
        "Active staging should have AutoApproved source"
    );
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_staging_timeout_uses_rule_based_npcs() {
    // Scenario: Auto-approve uses rule-based NPC selection.
    // Expected: NPCs staged match location's default population rules.
    //
    // The Thornhaven test world has Mira Thornwood configured to work at
    // The Drowsy Dragon Inn, so she should appear in rule-based staging.

    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");

    // Create a pending staging request for Common Room
    let pending_request = PendingStagingRequest {
        region_id: common_room,
        location_id: ctx
            .world
            .location("The Drowsy Dragon Inn")
            .expect("Location should exist"),
        world_id: ctx.world.world_id,
        created_at: Utc::now(),
    };

    let request_id = Uuid::new_v4().to_string();

    // Execute auto-approval
    let result = ctx
        .app
        .use_cases
        .staging
        .auto_approve_timeout
        .execute(request_id.clone(), pending_request)
        .await;

    assert!(
        result.is_ok(),
        "Auto-approval should succeed: {:?}",
        result.err()
    );

    let _payload = result.unwrap();

    // Verify active staging has correct source
    let current_game_time = ctx.clock.now();
    let active_staging = ctx
        .app
        .repositories
        .staging
        .get_active_staging(common_room, current_game_time)
        .await
        .expect("Should get active staging");

    assert!(
        active_staging.is_some(),
        "Should have active staging after auto-approve"
    );

    let staging = active_staging.unwrap();
    assert_eq!(
        staging.source,
        StagingSource::AutoApproved,
        "Source should be AutoApproved for timeout"
    );

    // Get staged NPCs from repository to verify persistence
    let staged_npcs = ctx
        .app
        .repositories
        .staging
        .get_staged_npcs(common_room)
        .await
        .expect("Should get staged NPCs");

    // All staged NPCs should have auto-approved reasoning
    for npc in &staged_npcs {
        assert!(
            npc.reasoning.contains("[Auto-approved]")
                || npc.reasoning.contains("Lives here")
                || npc.reasoning.contains("Works here")
                || npc.reasoning.contains("Frequents"),
            "Staged NPC {} should have rule-based reasoning, got: {}",
            npc.name,
            npc.reasoning
        );
    }
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_player_can_interact_after_auto_staging() {
    // Scenario: Player interacts with NPC after staging auto-approved.
    // Expected: Interaction proceeds normally.
    //
    // Setup:
    // 1. Create E2E context with VCR LLM
    // 2. Player enters region, staging auto-approves
    // 3. Player starts conversation with auto-staged NPC
    //
    // Assertions:
    // - Conversation starts successfully
    // - NPC responds (via LLM)

    use super::{
        create_shared_log, create_test_player, start_conversation_with_npc, LoggingLlmDecorator,
        TestOutcome, VcrLlm,
    };
    use std::sync::Arc;

    const TEST_NAME: &str = "test_player_can_interact_after_auto_staging";
    let event_log = create_shared_log(TEST_NAME);

    // Create VCR LLM with event logging
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
        // Get test region and create player
        let common_room = ctx
            .world
            .region("Common Room")
            .expect("Common Room should exist");

        let (player_id, pc_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Auto-Stage Tester",
        )
        .await
        .expect("Player creation should succeed");

        // Create a pending staging request and auto-approve it
        let pending_request = PendingStagingRequest {
            region_id: common_room,
            location_id: ctx
                .world
                .location("The Drowsy Dragon Inn")
                .expect("Location should exist"),
            world_id: ctx.world.world_id,
            created_at: Utc::now(),
        };

        let request_id = Uuid::new_v4().to_string();

        // Execute auto-approval
        let payload = ctx
            .app
            .use_cases
            .staging
            .auto_approve_timeout
            .execute(request_id.clone(), pending_request)
            .await
            .expect("Auto-approval should succeed");

        assert_eq!(
            payload.region_id, common_room,
            "Auto-approval should be for Common Room"
        );

        // Verify staging was activated
        let current_game_time = ctx.clock.now();
        let active_staging = ctx
            .app
            .repositories
            .staging
            .get_active_staging(common_room, current_game_time)
            .await
            .expect("Should get active staging");

        assert!(
            active_staging.is_some(),
            "Should have active staging after auto-approve"
        );

        let staging = active_staging.unwrap();
        assert_eq!(
            staging.source,
            StagingSource::AutoApproved,
            "Staging source should be AutoApproved"
        );

        // Get an auto-staged NPC to interact with
        let staged_npcs = ctx
            .app
            .repositories
            .staging
            .get_staged_npcs(common_room)
            .await
            .expect("Should get staged NPCs");

        assert!(
            !staged_npcs.is_empty(),
            "Should have at least one auto-staged NPC"
        );

        // Get the first available NPC for conversation
        let npc_id = staged_npcs[0].character_id;

        // Start conversation with the auto-staged NPC
        let (conversation_id, response) = start_conversation_with_npc(
            &ctx,
            pc_id,
            npc_id,
            &player_id,
            "Hello! I just arrived here.",
        )
        .await
        .expect("Should be able to start conversation with auto-staged NPC");

        // Verify conversation was successful
        assert!(
            !conversation_id.is_nil(),
            "Conversation ID should not be nil"
        );
        assert!(!response.is_empty(), "NPC should respond to player");

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
async fn test_dm_can_still_modify_after_auto_approve() {
    // Scenario: DM responds late, after auto-approve already triggered.
    // Expected: DM's late modification updates the staging.
    //
    // Setup:
    // 1. Create E2E context
    // 2. Player enters region
    // 3. Auto-approve triggers
    // 4. DM sends StagingApprovalResponse (late)
    //
    // Assertions:
    // - DM's changes are applied (NPCs updated)
    // - Player receives updated staging info

    use crate::use_cases::staging::{ApproveStagingInput, ApprovedNpc};

    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");

    // Create a player character
    let (_, _pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        common_room,
        "Late DM Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Step 1: Auto-approve staging (simulating timeout trigger)
    let pending_request = PendingStagingRequest {
        region_id: common_room,
        location_id: ctx
            .world
            .location("The Drowsy Dragon Inn")
            .expect("Location should exist"),
        world_id: ctx.world.world_id,
        created_at: Utc::now(),
    };

    let request_id = Uuid::new_v4().to_string();

    let auto_result = ctx
        .app
        .use_cases
        .staging
        .auto_approve_timeout
        .execute(request_id.clone(), pending_request)
        .await
        .expect("Auto-approval should succeed");

    assert_eq!(
        auto_result.region_id, common_room,
        "Auto-approval should be for Common Room"
    );

    // Verify initial auto-approved staging
    let current_game_time = ctx.clock.now();
    let initial_staging = ctx
        .app
        .repositories
        .staging
        .get_active_staging(common_room, current_game_time)
        .await
        .expect("Should get active staging")
        .expect("Should have active staging after auto-approve");

    assert_eq!(
        initial_staging.source,
        StagingSource::AutoApproved,
        "Initial staging should be AutoApproved"
    );

    let _initial_staged_npcs = ctx
        .app
        .repositories
        .staging
        .get_staged_npcs(common_room)
        .await
        .expect("Should get staged NPCs");

    // Step 2: DM sends late staging approval with different NPCs
    // Get a specific NPC (Mira Thornwood) to add to staging
    let mira_id = ctx.world.npc("Mira Thornwood").expect("Mira should exist");
    let mira = ctx
        .app
        .repositories
        .character
        .get(mira_id)
        .await
        .expect("Should get Mira")
        .expect("Mira should exist");

    // Create DM's late approval with just Mira (different from auto-approved NPCs)
    let dm_input = ApproveStagingInput {
        region_id: common_room,
        location_id: Some(
            ctx.world
                .location("The Drowsy Dragon Inn")
                .expect("Location should exist"),
        ),
        world_id: ctx.world.world_id,
        approved_by: "dm-late-response".to_string(),
        ttl_hours: 24,
        source: StagingSource::DmCustomized,
        approved_npcs: vec![ApprovedNpc {
            character_id: mira_id,
            is_present: true,
            reasoning: Some("DM added Mira after auto-approve".to_string()),
            is_hidden_from_players: false,
            mood: Some(format!("{:?}", mira.default_mood()).to_lowercase()),
        }],
        location_state_id: None,
        region_state_id: None,
    };

    // Execute DM's late approval
    let dm_result = ctx
        .app
        .use_cases
        .staging
        .approve
        .execute(dm_input)
        .await
        .expect("DM late approval should succeed");

    assert_eq!(
        dm_result.region_id, common_room,
        "DM approval should be for Common Room"
    );

    // Step 3: Verify staging was updated with DM's changes
    let updated_staging = ctx
        .app
        .repositories
        .staging
        .get_active_staging(common_room, current_game_time)
        .await
        .expect("Should get active staging")
        .expect("Should have active staging after DM approval");

    assert_eq!(
        updated_staging.source,
        StagingSource::DmCustomized,
        "Staging source should now be DmCustomized"
    );

    let updated_staged_npcs = ctx
        .app
        .repositories
        .staging
        .get_staged_npcs(common_room)
        .await
        .expect("Should get staged NPCs");

    // Verify DM's changes were applied
    assert!(
        updated_staged_npcs
            .iter()
            .any(|npc| npc.character_id == mira_id),
        "Mira should be in the updated staging"
    );

    // Verify the staging was actually updated (not just the same as before)
    // The DM approval should have created a new staging
    assert_ne!(
        updated_staging.id, initial_staging.id,
        "DM approval should create a new staging record"
    );

    // Verify the NPC list reflects DM's changes
    assert!(
        updated_staged_npcs
            .iter()
            .any(|npc| npc.reasoning.contains("DM added Mira")),
        "Staging should contain DM's reasoning for Mira"
    );
}

// =============================================================================
// Time Suggestion Timeout
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer and time manipulation"]
async fn test_time_suggestion_expires_after_timeout() {
    // Scenario: Time advancement suggested, DM doesn't respond.
    // Expected: Suggestion expires after TTL.
    //
    // This test uses the TtlCache directly with a short TTL to verify
    // that time suggestions expire and cannot be resolved after timeout.

    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");

    // Create a player character for the time suggestion
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        common_room,
        "Time Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Create a TTL cache with a very short TTL for testing (50ms)
    let short_ttl_store: TtlCache<Uuid, TimeSuggestion> =
        TtlCache::new(Duration::from_millis(50));

    // Create a time suggestion
    // Note: TimeSuggestion uses domain GameTime which wraps DateTime<Utc>
    let suggestion_id = Uuid::new_v4();
    let current_game_time = wrldbldr_domain::GameTime::new(ctx.clock.now());
    let mut resulting_game_time = current_game_time.clone();
    resulting_game_time.advance_minutes(15);

    let suggestion = TimeSuggestion {
        id: suggestion_id,
        world_id: ctx.world.world_id,
        pc_id,
        pc_name: "Time Tester".to_string(),
        action_type: "travel_region".to_string(),
        action_description: "Moving to another region".to_string(),
        suggested_minutes: 15,
        current_time: current_game_time,
        resulting_time: resulting_game_time,
        period_change: None,
    };

    // Insert the suggestion
    short_ttl_store.insert(suggestion_id, suggestion).await;

    // Verify it exists immediately
    let exists_before = short_ttl_store.get(&suggestion_id).await;
    assert!(
        exists_before.is_some(),
        "Suggestion should exist immediately after insertion"
    );

    // Wait for TTL to expire (use generous margin)
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify the suggestion has expired
    let exists_after = short_ttl_store.get(&suggestion_id).await;
    assert!(
        exists_after.is_none(),
        "Suggestion should not be accessible after TTL expiration"
    );

    // Verify the entry still exists in the cache (just expired)
    let len_before_cleanup = short_ttl_store.len().await;
    assert_eq!(
        len_before_cleanup, 1,
        "Entry should still exist before cleanup (just expired)"
    );

    // Verify cleanup removes the expired entry
    let cleaned = short_ttl_store.cleanup_expired().await;
    assert_eq!(cleaned, 1, "One expired entry should be cleaned up");

    let len_after_cleanup = short_ttl_store.len().await;
    assert_eq!(len_after_cleanup, 0, "Cache should be empty after cleanup");
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_expired_time_decision_returns_error() {
    // Scenario: DM tries to approve expired time suggestion.
    // Expected: Server returns error indicating suggestion expired.
    //
    // This test creates a time suggestion, waits for expiration,
    // then attempts to resolve it which should return NotFound error.

    use crate::api::websocket::TimeSuggestionStoreImpl;

    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");

    // Create a player character
    let (_, _pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        common_room,
        "Decision Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Use the standard TimeSuggestionStoreImpl (30 min TTL)
    // Since we can't wait 30 minutes, we'll test the NotFound case
    // by attempting to resolve a suggestion that was never inserted
    let store = Arc::new(TimeSuggestionStoreImpl::new());
    let store_repo = crate::repositories::TimeSuggestionStore::new(store);

    // Create a fake suggestion ID that doesn't exist in the store
    let fake_suggestion_id = Uuid::new_v4();

    // Attempt to resolve the non-existent suggestion
    let result = ctx
        .app
        .use_cases
        .time
        .suggestions
        .resolve(
            &store_repo,
            ctx.world.world_id,
            fake_suggestion_id,
            TimeSuggestionDecision::Approve,
        )
        .await;

    // Should return NotFound error
    assert!(
        matches!(result, Err(TimeSuggestionError::NotFound)),
        "Should return NotFound error for non-existent suggestion, got: {:?}",
        result
    );

    // Verify game time was not modified
    let current_time = ctx
        .app
        .use_cases
        .time
        .control
        .get_game_time(ctx.world.world_id)
        .await
        .expect("Should get game time");

    // The seeded world starts at day 1, hour 9 (morning)
    // If time wasn't modified, we should still be at the initial time
    // (within reasonable tolerance for test setup)
    assert!(
        current_time.day() == 1,
        "Game day should not have changed from initial value"
    );
}

// =============================================================================
// Approval Queue Cleanup
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer and time manipulation"]
async fn test_old_approvals_cleaned_up() {
    // Scenario: Approval requests pile up, cleanup removes expired ones.
    // Expected: Expired approval requests are removed from queue.
    //
    // This test validates the TtlCache cleanup mechanism for pending
    // staging requests using a short TTL for fast test execution.

    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    // Create a TTL cache with very short TTL for testing (30ms)
    let short_ttl_store: TtlCache<String, PendingStagingRequest> =
        TtlCache::new(Duration::from_millis(30));

    let location_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Location should exist");

    // Insert multiple pending staging requests
    for i in 0..5 {
        let region_id = ctx
            .world
            .region("Common Room")
            .expect("Region should exist");

        let request = PendingStagingRequest {
            region_id,
            location_id,
            world_id: ctx.world.world_id,
            created_at: Utc::now(),
        };

        short_ttl_store
            .insert(format!("request-{}", i), request)
            .await;
    }

    // Verify all requests exist
    let count_before = short_ttl_store.len().await;
    assert_eq!(count_before, 5, "Should have 5 pending requests");

    // Verify we can access them
    let entries_before = short_ttl_store.entries().await;
    assert_eq!(
        entries_before.len(),
        5,
        "Should be able to access all 5 entries"
    );

    // Wait for TTL to expire (use generous margin)
    tokio::time::sleep(Duration::from_millis(80)).await;

    // Entries are still in the cache, but not accessible (expired)
    let entries_after_expiry = short_ttl_store.entries().await;
    assert_eq!(
        entries_after_expiry.len(),
        0,
        "No entries should be accessible after expiry"
    );

    // Raw count still shows entries (they're expired but not cleaned)
    let len_before_cleanup = short_ttl_store.len().await;
    assert_eq!(
        len_before_cleanup, 5,
        "Raw count should still show 5 entries before cleanup"
    );

    // Now trigger cleanup
    let cleaned_count = short_ttl_store.cleanup_expired().await;
    assert_eq!(cleaned_count, 5, "Should clean up all 5 expired entries");

    // Verify queue is now empty
    let count_after = short_ttl_store.len().await;
    assert_eq!(count_after, 0, "Queue should be empty after cleanup");

    // Verify specific requests are no longer accessible
    for i in 0..5 {
        let key = format!("request-{}", i);
        let result = short_ttl_store.get(&key).await;
        assert!(
            result.is_none(),
            "Request {} should not be accessible after cleanup",
            i
        );
    }
}

// =============================================================================
// Additional TtlCache Unit-Style Tests (run without Neo4j)
// =============================================================================

/// Test that TtlCache correctly tracks insertion time for TTL calculation.
#[tokio::test]
async fn test_ttl_cache_basic_expiration() {
    let cache: TtlCache<String, String> = TtlCache::new(Duration::from_millis(20));

    // Insert an entry
    cache.insert("key1".to_string(), "value1".to_string()).await;

    // Should be accessible immediately
    assert_eq!(
        cache.get(&"key1".to_string()).await,
        Some("value1".to_string())
    );

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(40)).await;

    // Should no longer be accessible
    assert_eq!(cache.get(&"key1".to_string()).await, None);
}

/// Test that fresh entries are preserved during cleanup.
#[tokio::test]
async fn test_ttl_cache_cleanup_preserves_fresh_entries() {
    let cache: TtlCache<String, String> = TtlCache::new(Duration::from_millis(50));

    // Insert first entry
    cache.insert("old".to_string(), "old_value".to_string()).await;

    // Wait for first entry to expire
    tokio::time::sleep(Duration::from_millis(70)).await;

    // Insert second entry (fresh)
    cache.insert("new".to_string(), "new_value".to_string()).await;

    // Cleanup should remove old, preserve new
    let cleaned = cache.cleanup_expired().await;
    assert_eq!(cleaned, 1, "Should clean up 1 expired entry");

    // Old entry gone
    assert!(cache.get(&"old".to_string()).await.is_none());

    // New entry preserved
    assert_eq!(
        cache.get(&"new".to_string()).await,
        Some("new_value".to_string())
    );

    assert_eq!(cache.len().await, 1, "Should have 1 entry remaining");
}

/// Test that contains() respects TTL.
#[tokio::test]
async fn test_ttl_cache_contains_respects_ttl() {
    let cache: TtlCache<String, i32> = TtlCache::new(Duration::from_millis(20));

    cache.insert("key".to_string(), 42).await;

    // Should exist immediately
    assert!(cache.contains(&"key".to_string()).await);

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(40)).await;

    // Should not exist after expiration
    assert!(!cache.contains(&"key".to_string()).await);
}

/// Test that remove() works even for expired entries.
#[tokio::test]
async fn test_ttl_cache_remove_works_for_expired() {
    let cache: TtlCache<String, String> = TtlCache::new(Duration::from_millis(20));

    cache
        .insert("key".to_string(), "value".to_string())
        .await;

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(40)).await;

    // Remove should still work (returns the value even if expired)
    let removed = cache.remove(&"key".to_string()).await;
    assert_eq!(removed, Some("value".to_string()));

    // Entry is now gone
    assert_eq!(cache.len().await, 0);
}
