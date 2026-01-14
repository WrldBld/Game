//! E2E tests for the flag system.
//!
//! Tests verify:
//! - Flags can be set and retrieved
//! - World vs PC flag scope works correctly
//! - Flags flow to LLM context
//! - Flag changes can trigger events

use std::sync::Arc;

use super::{create_test_player, E2EEventLog, E2ETestContext, TestOutcome};

/// Test setting and getting world-scoped flags.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_set_and_get_world_flag() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    // Set a world flag
    ctx.app
        .repositories
        .flag
        .set_world_flag(ctx.world.world_id, "dragon_awakened")
        .await
        .expect("Setting world flag should succeed");

    // Verify flag is set
    let is_set = ctx
        .app
        .repositories
        .flag
        .is_world_flag_set(ctx.world.world_id, "dragon_awakened")
        .await
        .expect("Checking world flag should succeed");
    assert!(is_set, "Flag should be set");

    // Get all world flags
    let flags = ctx
        .app
        .repositories
        .flag
        .get_world_flags(ctx.world.world_id)
        .await
        .expect("Getting world flags should succeed");
    assert!(
        flags.contains(&"dragon_awakened".to_string()),
        "Flags should contain dragon_awakened"
    );
}

/// Test setting and getting PC-scoped flags.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_set_and_get_pc_flag() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");

    // Create a player character
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        common_room,
        "Flag Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Set a PC flag
    ctx.app
        .repositories
        .flag
        .set_pc_flag(pc_id, "completed_tutorial")
        .await
        .expect("Setting PC flag should succeed");

    // Verify flag is set
    let is_set = ctx
        .app
        .repositories
        .flag
        .is_pc_flag_set(pc_id, "completed_tutorial")
        .await
        .expect("Checking PC flag should succeed");
    assert!(is_set, "PC flag should be set");

    // Get all PC flags
    let flags = ctx
        .app
        .repositories
        .flag
        .get_pc_flags(pc_id)
        .await
        .expect("Getting PC flags should succeed");
    assert!(
        flags.contains(&"completed_tutorial".to_string()),
        "PC flags should contain completed_tutorial"
    );
}

/// Test that world and PC flags are distinct.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_world_vs_pc_flag_scope() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");

    // Create a player character
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        common_room,
        "Scope Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Set a world flag
    ctx.app
        .repositories
        .flag
        .set_world_flag(ctx.world.world_id, "global_event")
        .await
        .expect("Setting world flag should succeed");

    // Set a PC flag with different name
    ctx.app
        .repositories
        .flag
        .set_pc_flag(pc_id, "personal_quest")
        .await
        .expect("Setting PC flag should succeed");

    // PC flag should NOT be visible at world scope
    let is_world_flag = ctx
        .app
        .repositories
        .flag
        .is_world_flag_set(ctx.world.world_id, "personal_quest")
        .await
        .expect("Checking should succeed");
    assert!(
        !is_world_flag,
        "PC flag should not be visible as world flag"
    );

    // World flag should NOT be visible at PC scope
    let is_pc_flag = ctx
        .app
        .repositories
        .flag
        .is_pc_flag_set(pc_id, "global_event")
        .await
        .expect("Checking should succeed");
    assert!(!is_pc_flag, "World flag should not be visible as PC flag");

    // Combined flags should include both
    let all_flags = ctx
        .app
        .repositories
        .flag
        .get_all_flags_for_pc(ctx.world.world_id, pc_id)
        .await
        .expect("Getting all flags should succeed");
    assert!(
        all_flags.contains(&"global_event".to_string()),
        "Combined flags should include world flag"
    );
    assert!(
        all_flags.contains(&"personal_quest".to_string()),
        "Combined flags should include PC flag"
    );
}

/// Test unsetting flags.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_unset_flag() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    // Set a world flag
    ctx.app
        .repositories
        .flag
        .set_world_flag(ctx.world.world_id, "temporary_effect")
        .await
        .expect("Setting flag should succeed");

    // Verify it's set
    let is_set = ctx
        .app
        .repositories
        .flag
        .is_world_flag_set(ctx.world.world_id, "temporary_effect")
        .await
        .expect("Check should succeed");
    assert!(is_set, "Flag should be set initially");

    // Unset the flag
    ctx.app
        .repositories
        .flag
        .unset_world_flag(ctx.world.world_id, "temporary_effect")
        .await
        .expect("Unsetting flag should succeed");

    // Verify it's no longer set
    let is_still_set = ctx
        .app
        .repositories
        .flag
        .is_world_flag_set(ctx.world.world_id, "temporary_effect")
        .await
        .expect("Check should succeed");
    assert!(!is_still_set, "Flag should be unset");
}

/// Test that flags with same name at different scopes are independent.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_same_flag_name_different_scopes() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");

    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        common_room,
        "Dual Scope Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Set same flag name at both scopes
    ctx.app
        .repositories
        .flag
        .set_world_flag(ctx.world.world_id, "quest_started")
        .await
        .expect("Setting world flag should succeed");

    ctx.app
        .repositories
        .flag
        .set_pc_flag(pc_id, "quest_started")
        .await
        .expect("Setting PC flag should succeed");

    // Unset world flag only
    ctx.app
        .repositories
        .flag
        .unset_world_flag(ctx.world.world_id, "quest_started")
        .await
        .expect("Unsetting should succeed");

    // World flag should be unset
    let world_set = ctx
        .app
        .repositories
        .flag
        .is_world_flag_set(ctx.world.world_id, "quest_started")
        .await
        .expect("Check should succeed");
    assert!(!world_set, "World flag should be unset");

    // PC flag should still be set
    let pc_set = ctx
        .app
        .repositories
        .flag
        .is_pc_flag_set(pc_id, "quest_started")
        .await
        .expect("Check should succeed");
    assert!(pc_set, "PC flag should still be set");
}

/// Test that flags flow to LLM context.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_flags_in_character_context() {
    let event_log = Arc::new(E2EEventLog::new("test_flags_in_character_context"));
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");

    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        common_room,
        "Context Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Set flags
    ctx.app
        .repositories
        .flag
        .set_world_flag(ctx.world.world_id, "main_quest_active")
        .await
        .expect("Setting flag should succeed");

    ctx.app
        .repositories
        .flag
        .set_pc_flag(pc_id, "has_magic_item")
        .await
        .expect("Setting flag should succeed");

    // Get all flags for context
    let flags = ctx
        .app
        .repositories
        .flag
        .get_all_flags_for_pc(ctx.world.world_id, pc_id)
        .await
        .expect("Getting flags should succeed");

    // Verify flags are present for context building
    assert!(
        flags.len() >= 2,
        "Should have at least 2 flags for context: {:?}",
        flags
    );

    ctx.finalize_event_log(TestOutcome::Pass);
    let _ = ctx.save_event_log(&E2ETestContext::default_log_path("flags_context"));
}
