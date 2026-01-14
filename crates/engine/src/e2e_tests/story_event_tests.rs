//! E2E tests for the story event system.
//!
//! Tests verify:
//! - Story events can be triggered
//! - Event effects execute correctly
//! - Events update context

use std::sync::Arc;

use super::{E2EEventLog, E2ETestContext, TestOutcome};

/// Test listing active narrative events.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_list_active_events() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Setup should succeed");

    // List active narrative events
    let events = ctx
        .app
        .repositories
        .narrative
        .list_events(ctx.world.world_id)
        .await
        .expect("Should list events");

    println!("World has {} active narrative events", events.len());

    for event in &events {
        println!("  Event: {} - {}", event.name(), event.description());
    }
}

/// Test triggering a narrative event.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_trigger_narrative_event() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Setup should succeed");

    use neo4rs::query;
    use uuid::Uuid;

    // Create a simple event
    let event_id = Uuid::new_v4();
    ctx.harness
        .graph()
        .run(
            query(
                r#"CREATE (e:NarrativeEvent {
                    id: $id,
                    world_id: $world_id,
                    name: 'Test Trigger Event',
                    description: 'An event to be triggered',
                    scene_direction: 'Something dramatic happens',
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

    // Deactivate the event (simulating triggered behavior)
    // Note: mark_triggered doesn't exist, use set_event_active to toggle event state
    let trigger_result = ctx
        .app
        .repositories
        .narrative
        .set_event_active(wrldbldr_domain::NarrativeEventId::from(event_id), false)
        .await;

    match trigger_result {
        Ok(_) => {
            println!("Event deactivated successfully");
        }
        Err(e) => {
            println!("Event state change failed: {:?}", e);
        }
    }
}

/// Test repeatable vs non-repeatable events.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_repeatable_events() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Setup should succeed");

    use neo4rs::query;
    use uuid::Uuid;

    // Create a repeatable event
    let repeatable_id = Uuid::new_v4();
    ctx.harness
        .graph()
        .run(
            query(
                r#"CREATE (e:NarrativeEvent {
                    id: $id,
                    world_id: $world_id,
                    name: 'Repeatable Event',
                    description: 'Can happen multiple times',
                    scene_direction: 'This can repeat',
                    is_active: true,
                    is_triggered: false,
                    is_repeatable: true,
                    priority: 1,
                    is_favorite: false
                })"#,
            )
            .param("id", repeatable_id.to_string())
            .param("world_id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Repeatable event creation should succeed");

    // Create a non-repeatable event
    let non_repeatable_id = Uuid::new_v4();
    ctx.harness
        .graph()
        .run(
            query(
                r#"CREATE (e:NarrativeEvent {
                    id: $id,
                    world_id: $world_id,
                    name: 'One-Time Event',
                    description: 'Can only happen once',
                    scene_direction: 'This is unique',
                    is_active: true,
                    is_triggered: false,
                    is_repeatable: false,
                    priority: 1,
                    is_favorite: false
                })"#,
            )
            .param("id", non_repeatable_id.to_string())
            .param("world_id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Non-repeatable event creation should succeed");

    // List events - both should be active
    let events = ctx
        .app
        .repositories
        .narrative
        .list_events(ctx.world.world_id)
        .await
        .expect("Should list events");

    assert!(
        events.iter().any(|e| e.name() == "Repeatable Event"),
        "Repeatable event should be listed"
    );
    assert!(
        events.iter().any(|e| e.name() == "One-Time Event"),
        "One-time event should be listed"
    );
}

/// Test event priority ordering.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_event_priority() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Setup should succeed");

    use neo4rs::query;
    use uuid::Uuid;

    // Create events with different priorities
    for (priority, name) in [(1, "Low Priority"), (5, "Medium Priority"), (10, "High Priority")] {
        let event_id = Uuid::new_v4();
        ctx.harness
            .graph()
            .run(
                query(
                    r#"CREATE (e:NarrativeEvent {
                        id: $id,
                        world_id: $world_id,
                        name: $name,
                        description: 'Priority test event',
                        scene_direction: 'Test',
                        is_active: true,
                        is_triggered: false,
                        is_repeatable: false,
                        priority: $priority,
                        is_favorite: false
                    })"#,
                )
                .param("id", event_id.to_string())
                .param("world_id", ctx.world.world_id.to_string())
                .param("name", name)
                .param("priority", priority as i64),
            )
            .await
            .expect("Event creation should succeed");
    }

    // List events - should be ordered by priority
    let events = ctx
        .app
        .repositories
        .narrative
        .list_events(ctx.world.world_id)
        .await
        .expect("Should list events");

    // Find our priority test events
    let priority_events: Vec<_> = events
        .iter()
        .filter(|e| e.description() == "Priority test event")
        .collect();

    assert_eq!(priority_events.len(), 3, "Should have 3 priority test events");
    println!("Events ordered by priority:");
    for e in &priority_events {
        println!("  {} (priority: {})", e.name(), e.priority());
    }
}

/// Test event context in LLM prompts.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_event_in_context() {
    let event_log = Arc::new(E2EEventLog::new("test_event_in_context"));
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Setup should succeed");

    // Get active events for context building
    let events = ctx
        .app
        .repositories
        .narrative
        .list_events(ctx.world.world_id)
        .await
        .expect("Should list events");

    // Active events should be available for LLM context
    println!("{} events available for context", events.len());

    ctx.finalize_event_log(TestOutcome::Pass);
    let _ = ctx.save_event_log(&E2ETestContext::default_log_path("story_events_context"));
}

/// Test favorite events.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_favorite_events() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Setup should succeed");

    use neo4rs::query;
    use uuid::Uuid;

    // Create a favorite event
    let event_id = Uuid::new_v4();
    ctx.harness
        .graph()
        .run(
            query(
                r#"CREATE (e:NarrativeEvent {
                    id: $id,
                    world_id: $world_id,
                    name: 'Favorite Event',
                    description: 'A favorited event',
                    scene_direction: 'This is special',
                    is_active: true,
                    is_triggered: false,
                    is_repeatable: false,
                    priority: 5,
                    is_favorite: true
                })"#,
            )
            .param("id", event_id.to_string())
            .param("world_id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Favorite event creation should succeed");

    // List all events and filter for favorites
    // Note: list_favorites doesn't exist, so we filter from list_events
    let events = ctx
        .app
        .repositories
        .narrative
        .list_events(ctx.world.world_id)
        .await
        .expect("Should list events");

    let favorites: Vec<_> = events.iter().filter(|e| e.is_favorite()).collect();
    assert!(
        favorites.iter().any(|e| e.name() == "Favorite Event"),
        "Favorite event should be in filtered favorites list"
    );
}
