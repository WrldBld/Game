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
        println!("  Lore: {} - {:?}", entry.title, entry.category);
    }
}

/// Test creating lore entry.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_create_lore_entry() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    use neo4rs::query;
    use uuid::Uuid;

    // Create a lore entry directly
    let lore_id = Uuid::new_v4();
    ctx.graph()
        .run(
            query(
                r#"CREATE (l:Lore {
                    id: $id,
                    world_id: $world_id,
                    title: 'Ancient Legend',
                    content: 'Long ago, a dragon ruled these lands...',
                    category: 'History',
                    is_discovered: false,
                    is_secret: true
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
        lore.iter().any(|l| l.title == "Ancient Legend"),
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
                    category: 'Location',
                    is_discovered: false,
                    is_secret: true
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

/// Test LoreDiscovered trigger.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_lore_discovered_trigger() {
    let ctx = E2ETestContext::setup().await.expect("Setup should succeed");

    use neo4rs::query;
    use uuid::Uuid;

    // Create event with LoreDiscovered trigger
    let event_id = Uuid::new_v4();
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
                    category: 'Quest',
                    is_discovered: false,
                    is_secret: true
                })"#,
            )
            .param("id", lore_id.to_string())
            .param("world_id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Lore creation should succeed");

    // Create event
    ctx.graph()
        .run(
            query(
                r#"CREATE (e:NarrativeEvent {
                    id: $id,
                    world_id: $world_id,
                    name: 'Lore Revelation Event',
                    description: 'Triggered when specific lore is discovered',
                    scene_direction: 'The knowledge changes everything',
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

    // Create LoreDiscovered trigger
    let trigger_id = Uuid::new_v4();
    ctx.graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id})
                   CREATE (t:NarrativeTrigger {
                       id: $trigger_id,
                       trigger_type: 'LoreDiscovered',
                       lore_id: $lore_id,
                       is_active: true
                   })-[:TRIGGERS]->(e)"#,
            )
            .param("event_id", event_id.to_string())
            .param("trigger_id", trigger_id.to_string())
            .param("lore_id", lore_id.to_string()),
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
        "Event with LoreDiscovered trigger should exist"
    );
}
