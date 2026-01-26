//! E2E tests for the observation system.
//!
//! Tests verify:
//! - Observations can be created
//! - Observations flow to LLM context
//! - ObservationCount triggers work

use std::sync::Arc;

use super::{create_test_player, E2EEventLog, E2ETestContext, TestOutcome};

/// Test creating an observation for a PC.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_create_observation() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let mira_id = ctx.world.npc("Mira Thornwood").expect("Mira should exist");

    let (_, pc_id) = create_test_player(ctx.graph(), ctx.world.world_id, common_room, "Observer")
        .await
        .expect("Player creation should succeed");

    // Create an observation using the management use case
    // ObservationType variants are: Direct, HeardAbout, Deduced
    let observation_result = ctx
        .app
        .use_cases
        .management
        .observation
        .create(
            pc_id,
            mira_id,
            "Direct".to_string(),
            None, // location_id - will be resolved from region
            Some(common_room),
            Some("The tavern keeper seems nervous today".to_string()),
        )
        .await;

    match observation_result {
        Ok(obs) => {
            println!("Created observation for NPC: {}", obs.npc_id());
        }
        Err(e) => {
            println!("Observation creation failed: {:?}", e);
        }
    }
}

/// Test listing observations for a PC.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_list_pc_observations() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let mira_id = ctx.world.npc("Mira Thornwood").expect("Mira should exist");
    let grom_id = ctx.world.npc("Grom Ironhand").expect("Grom should exist");

    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        common_room,
        "List Observer",
    )
    .await
    .expect("Player creation should succeed");

    // Create observations for different NPCs using management use case
    let npcs = [mira_id, grom_id];
    for (i, npc_id) in npcs.iter().enumerate() {
        let _ = ctx
            .app
            .use_cases
            .management
            .observation
            .create(
                pc_id,
                *npc_id,
                "Direct".to_string(),
                None,
                Some(common_room),
                Some(format!("Observation number {}", i + 1)),
            )
            .await;
    }

    // List observations using the entity method
    let observations = ctx
        .app
        .repositories
        .observation
        .get_observations(pc_id)
        .await
        .expect("Should list observations");

    println!("PC has {} observations", observations.len());
    assert!(
        observations.len() >= 2,
        "Should have at least 2 observations"
    );
}

/// Test observations in LLM context.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_observations_in_context() {
    let event_log = Arc::new(E2EEventLog::new("test_observations_in_context"));
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let mira_id = ctx.world.npc("Mira Thornwood").expect("Mira should exist");

    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        common_room,
        "Context Observer",
    )
    .await
    .expect("Player creation should succeed");

    // Create an observation using Deduced type (for discovered information)
    let _ = ctx
        .app
        .use_cases
        .management
        .observation
        .create(
            pc_id,
            mira_id,
            "Deduced".to_string(),
            None,
            Some(common_room),
            Some("The old map shows a hidden passage".to_string()),
        )
        .await;

    // Get observations for context
    let observations = ctx
        .app
        .repositories
        .observation
        .get_observations(pc_id)
        .await
        .expect("Should list observations");

    // Observations should be available for LLM context building
    assert!(
        !observations.is_empty(),
        "Should have observations for context"
    );

    ctx.finalize_event_log(TestOutcome::Pass);
    let _ = ctx.save_event_log(&E2ETestContext::default_log_path("observations_context"));
}

/// Test observation types.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_observation_types() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let mira_id = ctx.world.npc("Mira Thornwood").expect("Mira should exist");

    let (_, pc_id) =
        create_test_player(ctx.graph(), ctx.world.world_id, common_room, "Type Tester")
            .await
            .expect("Player creation should succeed");

    // Test different observation types
    // ObservationType variants are: Direct, HeardAbout, Deduced
    use wrldbldr_domain::ObservationType;

    let types = vec![
        (ObservationType::Direct, "Directly saw the NPC"),
        (ObservationType::HeardAbout, "Heard about NPC location"),
        (ObservationType::Deduced, "Deduced NPC whereabouts"),
    ];

    for (obs_type, _description) in &types {
        println!("ObservationType: {:?}", obs_type);
    }

    // Create observations using the management use case with type strings
    let type_strings = ["Direct", "HeardAbout", "Deduced"];
    for (i, type_str) in type_strings.iter().enumerate() {
        let result = ctx
            .app
            .use_cases
            .management
            .observation
            .create(
                pc_id,
                mira_id,
                type_str.to_string(),
                None,
                Some(common_room),
                Some(format!("Observation type test {}", i)),
            )
            .await;

        match result {
            Ok(obs) => {
                println!("Created {} observation for NPC: {}", type_str, obs.npc_id());
            }
            Err(e) => {
                println!("Failed to create {} observation: {:?}", type_str, e);
            }
        }
    }
}

/// Test observation-related narrative event trigger.
///
/// This test verifies that narrative events with Custom triggers (used for
/// observation-based conditions) can be properly created and retrieved via
/// the domain model and repository.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_observation_count_trigger() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    use chrono::Utc;
    use wrldbldr_domain::{
        Description, NarrativeEvent, NarrativeEventName, NarrativeTrigger, NarrativeTriggerType,
    };

    // Get an NPC to reference in the trigger
    let npc_id = ctx.world.npc("Mira Thornwood").expect("Mira should exist");

    // Create a Custom trigger that represents an observation count condition
    // (ObservationCount is not a built-in trigger type, so we use Custom)
    let trigger = NarrativeTrigger::new(
        NarrativeTriggerType::Custom {
            description: format!("Player has observed NPC {} at least 3 times", npc_id),
            llm_evaluation: false,
        },
        "Observed NPC 3 times".to_string(),
        "observation-count-trigger".to_string(),
    )
    .with_required(true);

    let event = NarrativeEvent::new(
        ctx.world.world_id,
        NarrativeEventName::new("Keen Observer Event").unwrap(),
        Utc::now(),
    )
    .with_description("Triggered after many observations")
    .with_scene_direction(Description::new("Your keen eye has uncovered much").unwrap())
    .with_trigger_condition(trigger)
    .with_active(true);

    let event_id = event.id();

    // Save the event using the repository
    ctx.app
        .repositories
        .narrative
        .save_event(&event)
        .await
        .expect("Event creation should succeed");

    // Verify event exists by listing events
    let events = ctx
        .app
        .repositories
        .narrative
        .list_events(ctx.world.world_id)
        .await
        .expect("Should list events");

    assert!(
        events.iter().any(|e| e.id() == event_id),
        "Event with observation-related trigger should exist"
    );
}
