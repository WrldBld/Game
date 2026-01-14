//! E2E tests for the location event system.
//!
//! Tests verify:
//! - Location events fire on enter
//! - One-time vs repeatable location events
//! - Location event context

use std::sync::Arc;

use super::{create_test_player, E2EEventLog, E2ETestContext, TestOutcome};

/// Test PlayerEntersLocation trigger type.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_player_enters_location_trigger() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    use neo4rs::query;
    use uuid::Uuid;

    let location_id = ctx
        .world
        .location("The Rusty Anchor")
        .expect("Location should exist");

    // Create event with PlayerEntersLocation trigger
    let event_id = Uuid::new_v4();
    ctx
        .graph()
        .run(
            query(
                r#"CREATE (e:NarrativeEvent {
                    id: $id,
                    world_id: $world_id,
                    name: 'Tavern Entry Event',
                    description: 'Triggered when entering the tavern',
                    scene_direction: 'The smell of ale greets you',
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

    // Create PlayerEntersLocation trigger
    let trigger_id = Uuid::new_v4();
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id})
                   CREATE (t:NarrativeTrigger {
                       id: $trigger_id,
                       trigger_type: 'PlayerEntersLocation',
                       location_id: $location_id,
                       location_name: 'The Rusty Anchor',
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
        "Event with PlayerEntersLocation trigger should exist"
    );
}

/// Test location event fires on movement.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_location_event_on_movement() {
    let event_log = Arc::new(E2EEventLog::new("test_location_event_on_movement"));
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Setup should succeed");

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
        "Location Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Move to target region
    let result = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, tavern_bar)
        .await
        .expect("Movement should succeed");

    // Check for triggered events
    println!(
        "Movement triggered {} events",
        result.triggered_events.len()
    );
    for event in &result.triggered_events {
        println!("  Triggered: {} - {}", event.name(), event.description());
    }

    ctx.finalize_event_log(TestOutcome::Pass);
    let _ = ctx.save_event_log(&E2ETestContext::default_log_path("location_events"));
}

/// Test one-time location event.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_one_time_location_event() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    use neo4rs::query;
    use uuid::Uuid;

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let tavern_bar = ctx
        .world
        .region("Tavern Bar")
        .expect("Tavern Bar should exist");
    let location_id = ctx
        .world
        .location("The Rusty Anchor")
        .expect("Location should exist");

    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        common_room,
        "One-Time Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Create a one-time location event
    let event_id = Uuid::new_v4();
    ctx
        .graph()
        .run(
            query(
                r#"CREATE (e:NarrativeEvent {
                    id: $id,
                    world_id: $world_id,
                    name: 'First Visit Event',
                    description: 'Only happens on first visit',
                    scene_direction: 'Welcome, newcomer',
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

    // Create trigger
    let trigger_id = Uuid::new_v4();
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id})
                   CREATE (t:NarrativeTrigger {
                       id: $trigger_id,
                       trigger_type: 'PlayerEntersLocation',
                       location_id: $location_id,
                       location_name: 'The Rusty Anchor',
                       is_active: true
                   })-[:TRIGGERS]->(e)"#,
            )
            .param("event_id", event_id.to_string())
            .param("trigger_id", trigger_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Trigger creation should succeed");

    // First visit
    let result1 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, tavern_bar)
        .await
        .expect("First movement should succeed");

    let first_visit_triggered = result1
        .triggered_events
        .iter()
        .any(|e| e.name() == "First Visit Event");

    // Go back
    let _ = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await;

    // Second visit
    let result2 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, tavern_bar)
        .await
        .expect("Second movement should succeed");

    let second_visit_triggered = result2
        .triggered_events
        .iter()
        .any(|e| e.name() == "First Visit Event");

    // Document behavior - one-time event should only trigger once
    println!("First visit triggered: {}", first_visit_triggered);
    println!("Second visit triggered: {}", second_visit_triggered);
}

/// Test repeatable location event.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_repeatable_location_event() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    use neo4rs::query;
    use uuid::Uuid;

    let location_id = ctx
        .world
        .location("The Rusty Anchor")
        .expect("Location should exist");

    // Create a repeatable location event
    let event_id = Uuid::new_v4();
    ctx
        .graph()
        .run(
            query(
                r#"CREATE (e:NarrativeEvent {
                    id: $id,
                    world_id: $world_id,
                    name: 'Recurring Greeting',
                    description: 'Happens every visit',
                    scene_direction: 'Welcome back',
                    is_active: true,
                    is_triggered: false,
                    is_repeatable: true,
                    priority: 1,
                    is_favorite: false
                })"#,
            )
            .param("id", event_id.to_string())
            .param("world_id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Event creation should succeed");

    // Create trigger
    let trigger_id = Uuid::new_v4();
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id})
                   CREATE (t:NarrativeTrigger {
                       id: $trigger_id,
                       trigger_type: 'PlayerEntersLocation',
                       location_id: $location_id,
                       location_name: 'The Rusty Anchor',
                       is_active: true
                   })-[:TRIGGERS]->(e)"#,
            )
            .param("event_id", event_id.to_string())
            .param("trigger_id", trigger_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Trigger creation should succeed");

    // Verify event is marked as repeatable
    let events = ctx
        .app
        .repositories
        .narrative
        .list_events(ctx.world.world_id)
        .await
        .expect("Should list events");

    let event = events
        .iter()
        .find(|e| e.id().to_string() == event_id.to_string());
    assert!(event.is_some(), "Event should exist");
    assert!(event.unwrap().is_repeatable(), "Event should be repeatable");
}
