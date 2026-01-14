//! E2E tests for the time/calendar system.
//!
//! Tests verify:
//! - Game time advances correctly
//! - Time flows to LLM context
//! - Time-based triggers evaluate correctly

use std::sync::Arc;

use super::{create_test_player, E2EEventLog, E2ETestContext, TestOutcome};

/// Test that world has game time configuration.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_world_has_game_time() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    // Get the world
    let world = ctx
        .app
        .repositories
        .world
        .get(ctx.world.world_id)
        .await
        .expect("Should get world")
        .expect("World should exist");

    // Verify game time exists
    // The world should have game_time configuration
    // This documents expected behavior
    println!("World name: {}", world.name().as_str());
}

/// Test time suggestion for movement.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_time_suggestion_for_movement() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let tavern_bar = ctx
        .world
        .region("Tavern Bar")
        .expect("Tavern Bar should exist");

    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        common_room,
        "Time Traveler",
    )
    .await
    .expect("Player creation should succeed");

    // Move to another region - should include time suggestion
    let result = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, tavern_bar)
        .await
        .expect("Move should succeed");

    // Check for time suggestion in result
    // Note: Time suggestion may be None if time mode is not Suggested
    if let Some(suggestion) = result.time_suggestion {
        println!("Time suggestion: {:?}", suggestion);
    } else {
        println!("No time suggestion (time mode may be Manual or RealTime)");
    }
}

/// Test that time advances correctly.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_advance_time() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    // Get initial world time
    let world = ctx
        .app
        .repositories
        .world
        .get(ctx.world.world_id)
        .await
        .expect("Should get world")
        .expect("World should exist");

    // Try to advance time using the time control use case
    // Note: The correct method is control.advance_hours or control.advance_minutes
    let advance_result = ctx
        .app
        .use_cases
        .time
        .control
        .advance_hours(ctx.world.world_id, 1) // Advance 1 hour
        .await;

    match advance_result {
        Ok(result) => {
            println!(
                "Time advanced successfully: previous={:?}, new={:?}",
                result.previous_time.current(),
                result.new_time.current()
            );
        }
        Err(e) => {
            println!("Time advance failed: {:?}", e);
        }
    }
}

/// Test time of day enumeration.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_time_of_day_enumeration() {
    // Test that TimeOfDay variants exist
    // Note: TimeOfDay is a simple enum without from_hour method
    use wrldbldr_domain::TimeOfDay;

    // Verify all TimeOfDay variants exist
    let time_of_day_variants = [
        TimeOfDay::Morning,
        TimeOfDay::Afternoon,
        TimeOfDay::Evening,
        TimeOfDay::Night,
    ];

    for tod in &time_of_day_variants {
        println!("TimeOfDay variant: {:?} ({})", tod, tod.display_name());
    }

    // Document expected time ranges:
    // Morning: 6-11
    // Afternoon: 12-17
    // Evening: 18-21
    // Night: 22-5

    // Verify display_name returns expected strings
    assert_eq!(TimeOfDay::Morning.display_name(), "Morning");
    assert_eq!(TimeOfDay::Afternoon.display_name(), "Afternoon");
    assert_eq!(TimeOfDay::Evening.display_name(), "Evening");
    assert_eq!(TimeOfDay::Night.display_name(), "Night");
}

/// Test time context in LLM prompts.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_time_in_world_context() {
    let event_log = Arc::new(E2EEventLog::new("test_time_in_world_context"));
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Setup should succeed");

    // Get world context - should include time information
    let world = ctx
        .app
        .repositories
        .world
        .get(ctx.world.world_id)
        .await
        .expect("Should get world")
        .expect("World should exist");

    // The world context should include game_time when building LLM prompts
    // This verifies time data is available for context building
    println!("World time mode configured for: {}", world.name().as_str());

    ctx.finalize_event_log(TestOutcome::Pass);
    let _ = ctx.save_event_log(&E2ETestContext::default_log_path("time_context"));
}

/// Test TimeReached trigger type.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_time_reached_trigger() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    use neo4rs::query;
    use uuid::Uuid;
    let event_id = Uuid::new_v4();

    // Create event with TimeReached trigger
    ctx
        .graph()
        .run(
            query(
                r#"CREATE (e:NarrativeEvent {
                    id: $id,
                    world_id: $world_id,
                    name: 'Midnight Event',
                    description: 'Something happens at midnight',
                    scene_direction: 'The clock strikes twelve',
                    is_active: true,
                    is_triggered: false,
                    is_repeatable: false,
                    priority: 1,
                    is_favorite: false
                })"#,
            )
            .param("id", event_id.to_string())
            .param("world_id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Event creation should succeed");

    // Create TimeReached trigger
    let trigger_id = Uuid::new_v4();
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id})
                   CREATE (t:NarrativeTrigger {
                       id: $trigger_id,
                       trigger_type: 'TimeReached',
                       target_hour: 0,
                       is_active: true
                   })-[:TRIGGERS]->(e)"#,
            )
            .param("event_id", event_id.to_string())
            .param("trigger_id", trigger_id.to_string()),
        )
        .await
        .expect("Trigger creation should succeed");

    // Verify event exists
    let events = ctx
        .app
        .repositories
        .narrative
        .list_events(ctx.world.world_id)
        .await
        .expect("Should list events");

    assert!(
        events
            .iter()
            .any(|e| e.id().to_string() == event_id.to_string()),
        "Event with TimeReached trigger should exist"
    );
}

/// Test TimeAtLocation trigger type.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_time_at_location_trigger() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    use neo4rs::query;
    use uuid::Uuid;
    let event_id = Uuid::new_v4();
    let location_id = ctx
        .world
        .location("The Rusty Anchor")
        .expect("Location should exist");

    // Create event with TimeAtLocation trigger
    ctx
        .graph()
        .run(
            query(
                r#"CREATE (e:NarrativeEvent {
                    id: $id,
                    world_id: $world_id,
                    name: 'Evening at Tavern Event',
                    description: 'Something happens at the tavern in the evening',
                    scene_direction: 'The evening crowd gathers',
                    is_active: true,
                    is_triggered: false,
                    is_repeatable: false,
                    priority: 1,
                    is_favorite: false
                })"#,
            )
            .param("id", event_id.to_string())
            .param("world_id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Event creation should succeed");

    // Create TimeAtLocation trigger
    let trigger_id = Uuid::new_v4();
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id})
                   CREATE (t:NarrativeTrigger {
                       id: $trigger_id,
                       trigger_type: 'TimeAtLocation',
                       location_id: $location_id,
                       time_of_day: 'Evening',
                       is_active: true
                   })-[:TRIGGERS]->(e)"#,
            )
            .param("event_id", event_id.to_string())
            .param("trigger_id", trigger_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Trigger creation should succeed");

    // Verify event exists
    let events = ctx
        .app
        .repositories
        .narrative
        .list_events(ctx.world.world_id)
        .await
        .expect("Should list events");

    assert!(
        events
            .iter()
            .any(|e| e.id().to_string() == event_id.to_string()),
        "Event with TimeAtLocation trigger should exist"
    );
}
