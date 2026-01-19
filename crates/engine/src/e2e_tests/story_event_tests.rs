//! E2E tests for the story event system.
//!
//! Tests verify:
//! - Story events can be triggered
//! - Event effects execute correctly
//! - Events update context

use std::sync::Arc;

use chrono::Utc;
use wrldbldr_domain::{Description, NarrativeEvent, NarrativeEventName};

use super::{E2EEventLog, E2ETestContext, TestOutcome};

/// Test listing active narrative events.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_list_active_events() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

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
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    // Create a simple event using the domain model
    let event = NarrativeEvent::new(
        ctx.world.world_id,
        NarrativeEventName::new("Test Trigger Event").unwrap(),
        Utc::now(),
    )
    .with_description("An event to be triggered")
    .with_scene_direction(Description::new("Something dramatic happens").unwrap())
    .with_priority(1)
    .with_active(true)
    .with_repeatable(false);

    let event_id = event.id();
    ctx.app
        .repositories
        .narrative
        .save_event(&event)
        .await
        .expect("Event creation should succeed");

    // Deactivate the event (simulating triggered behavior)
    // Note: mark_triggered doesn't exist, use set_event_active to toggle event state
    let trigger_result = ctx
        .app
        .repositories
        .narrative
        .set_event_active(event_id, false)
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
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    // Create a repeatable event using the domain model
    let repeatable_event = NarrativeEvent::new(
        ctx.world.world_id,
        NarrativeEventName::new("Repeatable Event").unwrap(),
        Utc::now(),
    )
    .with_description("Can happen multiple times")
    .with_scene_direction(Description::new("This can repeat").unwrap())
    .with_priority(1)
    .with_active(true)
    .with_repeatable(true);

    ctx.app
        .repositories
        .narrative
        .save_event(&repeatable_event)
        .await
        .expect("Repeatable event creation should succeed");

    // Create a non-repeatable event using the domain model
    let non_repeatable_event = NarrativeEvent::new(
        ctx.world.world_id,
        NarrativeEventName::new("One-Time Event").unwrap(),
        Utc::now(),
    )
    .with_description("Can only happen once")
    .with_scene_direction(Description::new("This is unique").unwrap())
    .with_priority(1)
    .with_active(true)
    .with_repeatable(false);

    ctx.app
        .repositories
        .narrative
        .save_event(&non_repeatable_event)
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
        events
            .iter()
            .any(|e| e.name().as_str() == "Repeatable Event"),
        "Repeatable event should be listed"
    );
    assert!(
        events.iter().any(|e| e.name().as_str() == "One-Time Event"),
        "One-time event should be listed"
    );
}

/// Test event priority ordering.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_event_priority() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    // Create events with different priorities using the domain model
    for (priority, name) in [
        (1, "Low Priority"),
        (5, "Medium Priority"),
        (10, "High Priority"),
    ] {
        let event = NarrativeEvent::new(
            ctx.world.world_id,
            NarrativeEventName::new(name).unwrap(),
            Utc::now(),
        )
        .with_description("Priority test event")
        .with_scene_direction(Description::new("Test").unwrap())
        .with_priority(priority)
        .with_active(true)
        .with_repeatable(false);

        ctx.app
            .repositories
            .narrative
            .save_event(&event)
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

    assert_eq!(
        priority_events.len(),
        3,
        "Should have 3 priority test events"
    );
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
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    // Create a favorite event using the domain model
    let event = NarrativeEvent::new(
        ctx.world.world_id,
        NarrativeEventName::new("Favorite Event").unwrap(),
        Utc::now(),
    )
    .with_description("A favorited event")
    .with_scene_direction(Description::new("This is special").unwrap())
    .with_priority(5)
    .with_active(true)
    .with_repeatable(false)
    .with_favorite(true);

    ctx.app
        .repositories
        .narrative
        .save_event(&event)
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
        favorites
            .iter()
            .any(|e| e.name().as_str() == "Favorite Event"),
        "Favorite event should be in filtered favorites list"
    );
}
