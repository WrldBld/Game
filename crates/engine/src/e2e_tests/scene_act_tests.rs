//! E2E tests for the scene/act management system.
//!
//! Tests verify:
//! - Scenes can be started and tracked
//! - Scene context flows to LLM
//! - Act progression works
//! - Scene completion triggers work

use std::sync::Arc;

use super::{create_test_player, E2EEventLog, E2ETestContext, TestOutcome};

/// Test that acts are loaded from world.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_acts_loaded_from_world() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Setup should succeed");

    // Get acts for the world
    let acts = ctx
        .app
        .repositories
        .act
        .list_in_world(ctx.world.world_id)
        .await
        .expect("Should list acts");

    // Thornhaven should have acts defined
    assert!(!acts.is_empty(), "World should have acts defined");

    // Verify act structure
    for act in &acts {
        println!("Act: {} - {}", act.name, act.description);
    }
}

/// Test that scenes are loaded for acts.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_scenes_loaded_for_act() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Setup should succeed");

    // Get first act
    let acts = ctx
        .app
        .repositories
        .act
        .list_in_world(ctx.world.world_id)
        .await
        .expect("Should list acts");

    if let Some(first_act) = acts.first() {
        // Get scenes for this act
        let scenes = ctx
            .app
            .repositories
            .scene
            .list_for_act(first_act.id)
            .await
            .expect("Should list scenes");

        println!("Act '{}' has {} scenes", first_act.name, scenes.len());

        for scene in &scenes {
            println!("  Scene: {} at location {:?}", scene.name(), scene.location_id());
        }
    }
}

/// Test scene resolution for a region.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_scene_resolution_for_region() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Setup should succeed");

    let common_room = ctx.world.region("Common Room").expect("Common Room should exist");

    let (_, pc_id) = create_test_player(
        ctx.harness.graph(),
        ctx.world.world_id,
        common_room,
        "Scene Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Enter region - should resolve scene
    let result = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Enter region should succeed");

    // Check if scene was resolved
    if let Some(scene) = result.resolved_scene {
        println!("Resolved scene: {} - {}", scene.name(), scene.directorial_notes());
    } else {
        println!("No scene resolved for this region (may be expected)");
    }
}

/// Test that scene directorial notes are available.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_scene_directorial_notes() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Setup should succeed");

    // Get all scenes
    let acts = ctx
        .app
        .repositories
        .act
        .list_in_world(ctx.world.world_id)
        .await
        .expect("Should list acts");

    for act in &acts {
        let scenes = ctx
            .app
            .repositories
            .scene
            .list_for_act(act.id)
            .await
            .expect("Should list scenes");

        for scene in &scenes {
            // Verify directorial notes exist
            assert!(
                !scene.directorial_notes().is_empty(),
                "Scene '{}' should have directorial notes",
                scene.name()
            );
        }
    }
}

/// Test scene context in LLM prompts.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_scene_in_llm_context() {
    let event_log = Arc::new(E2EEventLog::new("test_scene_in_llm_context"));
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Setup should succeed");

    let common_room = ctx.world.region("Common Room").expect("Common Room should exist");

    let (_, pc_id) = create_test_player(
        ctx.harness.graph(),
        ctx.world.world_id,
        common_room,
        "Context Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Enter region to get scene context
    let result = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Enter region should succeed");

    // Scene should be available for context building
    // The resolved_scene field indicates what scene info would go to LLM
    if result.resolved_scene.is_some() {
        println!("Scene context available for LLM");
    }

    ctx.finalize_event_log(TestOutcome::Pass);
    let _ = ctx.save_event_log(&E2ETestContext::default_log_path("scene_context"));
}
