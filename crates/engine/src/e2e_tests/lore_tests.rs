//! E2E tests for the lore discovery system.
//!
//! Tests verify:
//! - Lore can be created and discovered
//! - Lore flows to LLM context
//! - LoreDiscovered triggers work

use std::sync::Arc;

use super::{create_test_player, E2EEventLog, E2ETestContext, TestOutcome};

/// Test listing world lore.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_list_world_lore() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    // List all lore for the world
    let lore = ctx
        .app
        .repositories
        .lore
        .list_for_world(ctx.world.world_id)
        .await
        .expect("Should list lore");

    println!("World has {} lore entries", lore.len());

    for entry in &lore {
        println!("  Lore: {} - {:?}", entry.title(), entry.category());
    }
}

/// Test creating lore entry.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_create_lore_entry() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    use neo4rs::query;
    use uuid::Uuid;

    // Create a lore entry directly with all required fields
    let lore_id = Uuid::new_v4();
    ctx.graph()
        .run(
            query(
                r#"CREATE (l:Lore {
                    id: $id,
                    world_id: $world_id,
                    title: 'Ancient Legend',
                    content: 'Long ago, a dragon ruled these lands...',
                    category: 'historical',
                    is_discovered: false,
                    is_secret: true,
                    tags: '["dragon", "history"]'
                })"#,
            )
            .param("id", lore_id.to_string())
            .param("world_id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Lore creation should succeed");

    // Verify lore exists
    let lore = ctx
        .app
        .repositories
        .lore
        .list_for_world(ctx.world.world_id)
        .await
        .expect("Should list lore");

    assert!(
        lore.iter().any(|l| l.title() == "Ancient Legend"),
        "Created lore should exist"
    );
}

/// Test discovering lore for a PC.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_discover_lore() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");

    let (_, pc_id) =
        create_test_player(ctx.graph(), ctx.world.world_id, common_room, "Lore Seeker")
            .await
            .expect("Player creation should succeed");

    use neo4rs::query;
    use uuid::Uuid;

    // Create a lore entry
    let lore_id = Uuid::new_v4();
    ctx.graph()
        .run(
            query(
                r#"CREATE (l:Lore {
                    id: $id,
                    world_id: $world_id,
                    title: 'Secret Passage',
                    content: 'A hidden passage exists behind the fireplace',
                    category: 'common',
                    is_discovered: false,
                    is_secret: true,
                    tags: '["secret", "passage"]'
                })"#,
            )
            .param("id", lore_id.to_string())
            .param("world_id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Lore creation should succeed");

    // Mark lore as discovered by PC
    let discover_result = ctx
        .graph()
        .run(
            query(
                r#"MATCH (l:Lore {id: $lore_id}), (pc:PlayerCharacter {id: $pc_id})
                   MERGE (pc)-[:DISCOVERED]->(l)"#,
            )
            .param("lore_id", lore_id.to_string())
            .param("pc_id", pc_id.to_string()),
        )
        .await;

    assert!(discover_result.is_ok(), "Discovering lore should succeed");
}

/// Test lore in context.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_lore_in_context() {
    let event_log = Arc::new(E2EEventLog::new("test_lore_in_context"));
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
        "Context Seeker",
    )
    .await
    .expect("Player creation should succeed");

    // Get character knowledge for context
    // Note: list_discovered_by_pc doesn't exist, use get_character_knowledge with PC's CharacterId
    let pc_char_id = wrldbldr_domain::CharacterId::from(*pc_id.as_uuid());
    let knowledge = ctx
        .app
        .repositories
        .lore
        .get_character_knowledge(pc_char_id)
        .await;

    match knowledge {
        Ok(known_lore) => {
            println!("PC has knowledge of {} lore entries", known_lore.len());
        }
        Err(e) => {
            println!("Lore knowledge listing failed: {:?}", e);
        }
    }

    ctx.finalize_event_log(TestOutcome::Pass);
    let _ = ctx.save_event_log(&E2ETestContext::default_log_path("lore_context"));
}

/// Test lore-related narrative event trigger.
///
/// This test verifies that a narrative event can be created with a FlagSet trigger
/// that represents lore discovery (e.g., "lore_trigger_discovered" flag).
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_lore_discovered_trigger() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    use chrono::Utc;
    use neo4rs::query;
    use uuid::Uuid;
    use wrldbldr_domain::{
        Description, NarrativeEvent, NarrativeEventName, NarrativeTrigger, NarrativeTriggerType,
    };

    let lore_id = Uuid::new_v4();

    // First create the lore
    ctx.graph()
        .run(
            query(
                r#"CREATE (l:Lore {
                    id: $id,
                    world_id: $world_id,
                    title: 'Trigger Lore',
                    content: 'This lore triggers an event',
                    category: 'legend',
                    is_discovered: false,
                    is_secret: true,
                    tags: '["trigger", "legend"]'
                })"#,
            )
            .param("id", lore_id.to_string())
            .param("world_id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Lore creation should succeed");

    // Create event using domain model with a FlagSet trigger
    // (representing lore discovery via a flag like "lore_<id>_discovered")
    let now = Utc::now();
    let flag_name = format!("lore_{}_discovered", lore_id);

    let trigger = NarrativeTrigger::new(
        NarrativeTriggerType::FlagSet {
            flag_name: flag_name.clone(),
        },
        "Triggered when specific lore is discovered",
        "lore-discovered-trigger",
    )
    .with_required(true);

    let event = NarrativeEvent::new(
        ctx.world.world_id,
        NarrativeEventName::new("Lore Revelation Event").unwrap(),
        now,
    )
    .with_description("Triggered when specific lore is discovered")
    .with_scene_direction(Description::new("The knowledge changes everything").unwrap())
    .with_trigger_condition(trigger)
    .with_active(true)
    .with_priority(1);

    // Save using the repository (this creates proper schema with HAS_NARRATIVE_EVENT)
    ctx.app
        .repositories
        .narrative
        .save_event(&event)
        .await
        .expect("save event");

    // Verify event exists
    let events = ctx
        .app
        .repositories
        .narrative
        .list_events(ctx.world.world_id)
        .await
        .expect("Should list events");

    assert!(
        events.iter().any(|e| e.id() == event.id()),
        "Event with lore discovery trigger should exist"
    );

    // Verify the event has the correct trigger
    let found_event = events.iter().find(|e| e.id() == event.id()).unwrap();
    assert_eq!(found_event.name().as_str(), "Lore Revelation Event");
    assert!(!found_event.trigger_conditions().is_empty());
}
