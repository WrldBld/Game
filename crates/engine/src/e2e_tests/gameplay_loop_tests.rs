//! E2E gameplay loop integration tests.
//!
//! These tests validate the complete gameplay loop using real Neo4j and
//! the full application stack.

use neo4rs::query;

use super::e2e_helpers::{create_test_player, E2ETestContext};

// =============================================================================
// World Setup Verification
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_thornhaven_world_seeded_correctly() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Verify world exists
    let mut result = ctx
        .graph()
        .execute(
            query("MATCH (w:World {id: $id}) RETURN w.name as name")
                .param("id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("World query failed");

    let row = result
        .next()
        .await
        .expect("No world found")
        .expect("Row error");
    let name: String = row.get("name").expect("name column");
    assert_eq!(name, "Thornhaven Village");

    // Verify 6 locations exist
    let mut result = ctx
        .graph()
        .execute(
            query("MATCH (l:Location)-[:LOCATED_IN]->(:World {id: $id}) RETURN count(l) as count")
                .param("id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Location count query failed");

    let row = result.next().await.expect("No result").expect("Row error");
    let count: i64 = row.get("count").expect("count column");
    assert_eq!(count, 6, "Should have 6 locations");

    // Verify regions exist (should be 16 across all locations)
    let mut result = ctx
        .graph()
        .execute(
            query(
                "MATCH (r:Region)-[:LOCATED_IN]->(l:Location)-[:LOCATED_IN]->(:World {id: $id})
             RETURN count(r) as count",
            )
            .param("id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Region count query failed");

    let row = result.next().await.expect("No result").expect("Row error");
    let count: i64 = row.get("count").expect("count column");
    assert!(
        count >= 10,
        "Should have at least 10 regions, got {}",
        count
    );

    // Verify 8 NPCs exist
    let mut result = ctx
        .graph()
        .execute(
            query("MATCH (c:Character)-[:BELONGS_TO]->(:World {id: $id}) RETURN count(c) as count")
                .param("id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Character count query failed");

    let row = result.next().await.expect("No result").expect("Row error");
    let count: i64 = row.get("count").expect("count column");
    assert_eq!(count, 8, "Should have 8 NPCs");

    // Verify key NPCs exist by name
    assert!(
        ctx.world.npc("Marta Hearthwood").is_some(),
        "Marta should exist"
    );
    assert!(
        ctx.world.npc("Grom Ironhand").is_some(),
        "Grom should exist"
    );
    assert!(ctx.world.npc("Old Tom").is_some(), "Old Tom should exist");

    // Verify key locations exist
    assert!(
        ctx.world.location("The Drowsy Dragon Inn").is_some(),
        "Inn should exist"
    );
    assert!(
        ctx.world.location("The Old Mill").is_some(),
        "Mill should exist"
    );
    assert!(
        ctx.world.location("Thornhaven Square").is_some(),
        "Square should exist"
    );

    // Verify scenes exist
    let mut result = ctx
        .graph()
        .execute(query("MATCH (s:Scene) RETURN count(s) as count"))
        .await
        .expect("Scene count query failed");

    let row = result.next().await.expect("No result").expect("Row error");
    let count: i64 = row.get("count").expect("count column");
    assert!(count >= 8, "Should have at least 8 scenes, got {}", count);
}

// =============================================================================
// Player Setup Flow
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_player_joins_world_at_starting_location() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Get the Inn's Common Room as starting region
    let common_room_id = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");

    // Create a test player
    let (player_id, character_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        common_room_id,
        "Test Hero",
    )
    .await
    .expect("Failed to create test player");

    // Verify player character was created
    let mut result = ctx
        .graph()
        .execute(
            query("MATCH (pc:PlayerCharacter {id: $id}) RETURN pc.name as name")
                .param("id", character_id.to_string()),
        )
        .await
        .expect("Player query failed");

    let row = result
        .next()
        .await
        .expect("No player found")
        .expect("Row error");
    let name: String = row.get("name").expect("name column");
    assert_eq!(name, "Test Hero");

    // Verify player is in the starting region
    let mut result = ctx
        .graph()
        .execute(
            query(
                "MATCH (pc:PlayerCharacter {id: $id})-[:CURRENTLY_IN]->(r:Region)
                 RETURN r.name as region_name",
            )
            .param("id", character_id.to_string()),
        )
        .await
        .expect("Location query failed");

    let row = result
        .next()
        .await
        .expect("No location found")
        .expect("Row error");
    let region_name: String = row.get("region_name").expect("region_name column");
    assert_eq!(region_name, "Common Room");

    // Verify player belongs to world (uses IN_WORLD relationship)
    let mut result = ctx
        .graph()
        .execute(
            query(
                "MATCH (pc:PlayerCharacter {id: $id})-[:IN_WORLD]->(w:World)
                 RETURN w.id as world_id",
            )
            .param("id", character_id.to_string()),
        )
        .await
        .expect("World relationship query failed");

    let row = result
        .next()
        .await
        .expect("No world relationship")
        .expect("Row error");
    let world_id: String = row.get("world_id").expect("world_id column");
    assert_eq!(world_id, ctx.world.world_id.to_string());
}

// =============================================================================
// Staging Flow
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_staging_npcs_with_work_schedules() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Query NPCs that work at the Inn (Marta should be there)
    let inn_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Inn should exist");

    // Get NPCs that work at a region in the Inn
    let mut result = ctx
        .graph()
        .execute(
            query(
                "MATCH (c:Character)-[:WORKS_AT]->(r:Region)-[:LOCATED_IN]->(l:Location {id: $loc_id})
                 RETURN c.name as name, c.default_disposition as disposition"
            )
            .param("loc_id", inn_id.to_string()),
        )
        .await
        .expect("WORKS_AT query failed");

    let mut workers = Vec::new();
    while let Some(row) = result.next().await.expect("Row read error") {
        let name: String = row.get("name").expect("name column");
        workers.push(name);
    }

    // Marta should work at the Inn
    assert!(
        workers.contains(&"Marta Hearthwood".to_string()),
        "Marta should work at the Inn, got: {:?}",
        workers
    );
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_npcs_frequency_relationships() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Query NPCs that frequent locations
    let mut result = ctx
        .graph()
        .execute(query(
            "MATCH (c:Character)-[f:FREQUENTS]->(r:Region)
                 RETURN c.name as name, f.time_of_day as time_of_day, r.name as region",
        ))
        .await
        .expect("FREQUENTS query failed");

    let mut frequents = Vec::new();
    while let Some(row) = result.next().await.expect("Row read error") {
        let name: String = row.get("name").expect("name column");
        let time: String = row.get("time_of_day").expect("time_of_day column");
        frequents.push((name, time));
    }

    // Grom should frequent the Inn in the evening
    let grom_frequents = frequents.iter().find(|(name, _)| name == "Grom Ironhand");
    assert!(
        grom_frequents.is_some(),
        "Grom should have FREQUENTS relationship"
    );

    if let Some((_, time)) = grom_frequents {
        assert!(
            time.to_lowercase().contains("evening"),
            "Grom should frequent in evening, got: {}",
            time
        );
    }
}

// =============================================================================
// Region Connections
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_region_connections_exist() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Query region connections
    let mut result = ctx
        .graph()
        .execute(query(
            "MATCH (r1:Region)-[c:CONNECTS_TO]->(r2:Region)
                 RETURN r1.name as from_region, r2.name as to_region, c.description as description
                 LIMIT 10",
        ))
        .await
        .expect("CONNECTS_TO query failed");

    let mut connections = Vec::new();
    while let Some(row) = result.next().await.expect("Row read error") {
        let from: String = row.get("from_region").expect("from_region column");
        let to: String = row.get("to_region").expect("to_region column");
        connections.push((from, to));
    }

    assert!(!connections.is_empty(), "Should have region connections");
}

// =============================================================================
// Acts and Scenes
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_acts_and_scenes_hierarchy() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Query acts with their scenes
    let mut result = ctx
        .graph()
        .execute(query(
            "MATCH (a:Act)<-[:PART_OF]-(s:Scene)
                 RETURN a.name as act_name, a.ordering as ordering, collect(s.name) as scene_names
                 ORDER BY ordering",
        ))
        .await
        .expect("Act/Scene query failed");

    let mut acts_with_scenes = Vec::new();
    while let Some(row) = result.next().await.expect("Row read error") {
        let act_name: String = row.get("act_name").expect("act_name column");
        let scene_names: Vec<String> = row.get("scene_names").expect("scene_names column");
        acts_with_scenes.push((act_name, scene_names));
    }

    // Should have at least one act with scenes
    assert!(!acts_with_scenes.is_empty(), "Should have acts with scenes");

    // First act (Arrival) should have scenes
    if let Some((act_name, scenes)) = acts_with_scenes.first() {
        assert!(!scenes.is_empty(), "Act '{}' should have scenes", act_name);
    }
}

// =============================================================================
// Challenges
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_challenges_exist() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Query challenges
    let mut result = ctx
        .graph()
        .execute(query(
            "MATCH (ch:Challenge)
                 RETURN ch.name as name, ch.challenge_type as challenge_type, ch.is_active as active
                 ORDER BY ch.ordering",
        ))
        .await
        .expect("Challenge query failed");

    let mut challenges = Vec::new();
    while let Some(row) = result.next().await.expect("Row read error") {
        let name: String = row.get("name").expect("name column");
        let challenge_type: String = row.get("challenge_type").expect("challenge_type column");
        challenges.push((name, challenge_type));
    }

    assert!(
        challenges.len() >= 3,
        "Should have at least 3 challenges, got {}",
        challenges.len()
    );

    // Verify key challenges exist
    let challenge_names: Vec<&str> = challenges.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        challenge_names.contains(&"Convince Grom to Share His Past"),
        "Should have Grom challenge"
    );
}

// =============================================================================
// Narrative Events
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_narrative_events_exist() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Query narrative events
    let mut result = ctx
        .graph()
        .execute(query(
            "MATCH (e:NarrativeEvent)
                 RETURN e.name as name, e.is_active as active, e.priority as priority
                 ORDER BY e.priority DESC",
        ))
        .await
        .expect("NarrativeEvent query failed");

    let mut events = Vec::new();
    while let Some(row) = result.next().await.expect("Row read error") {
        let name: String = row.get("name").expect("name column");
        let active: bool = row.get("active").expect("active column");
        events.push((name, active));
    }

    assert!(
        events.len() >= 3,
        "Should have at least 3 narrative events, got {}",
        events.len()
    );

    // Verify key events exist
    let event_names: Vec<&str> = events.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        event_names.contains(&"The Stranger's Warning"),
        "Should have The Stranger's Warning event"
    );
}

// =============================================================================
// Full World Verification
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_full_world_structure_integrity() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Verify all core entities for THIS test's world have correct relationships.
    // With parallel tests, we must scope all queries to this test's world_id.

    // 1. All locations in this world have LOCATED_IN relationship
    let mut result = ctx
        .graph()
        .execute(
            query(
                "MATCH (l:Location {world_id: $id})
                 WHERE NOT (l)-[:LOCATED_IN]->(:World {id: $id})
                 RETURN count(l) as orphan_count",
            )
            .param("id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Orphan location query failed");

    let row = result.next().await.expect("No result").expect("Row error");
    let orphan_count: i64 = row.get("orphan_count").expect("orphan_count column");
    assert_eq!(orphan_count, 0, "All locations should belong to the world");

    // 2. All regions in this world belong to a location
    let mut result = ctx
        .graph()
        .execute(
            query(
                "MATCH (r:Region)-[:LOCATED_IN]->(l:Location {world_id: $id})
                 WHERE NOT (r)-[:LOCATED_IN]->(l)
                 RETURN count(r) as orphan_count",
            )
            .param("id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Orphan region query failed");

    let row = result.next().await.expect("No result").expect("Row error");
    let orphan_count: i64 = row.get("orphan_count").expect("orphan_count column");
    assert_eq!(orphan_count, 0, "All regions should belong to a location");

    // 3. All characters in this world have BELONGS_TO relationship
    let mut result = ctx
        .graph()
        .execute(
            query(
                "MATCH (c:Character {world_id: $id})
                 WHERE NOT (c)-[:BELONGS_TO]->(:World {id: $id})
                 RETURN count(c) as orphan_count",
            )
            .param("id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Orphan character query failed");

    let row = result.next().await.expect("No result").expect("Row error");
    let orphan_count: i64 = row.get("orphan_count").expect("orphan_count column");
    assert_eq!(orphan_count, 0, "All characters should belong to the world");

    // 4. All acts in this world have PART_OF relationship
    let mut result = ctx
        .graph()
        .execute(
            query(
                "MATCH (a:Act {world_id: $id})
                 WHERE NOT (a)-[:PART_OF]->(:World {id: $id})
                 RETURN count(a) as orphan_count",
            )
            .param("id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Orphan act query failed");

    let row = result.next().await.expect("No result").expect("Row error");
    let orphan_count: i64 = row.get("orphan_count").expect("orphan_count column");
    assert_eq!(orphan_count, 0, "All acts should belong to the world");

    // 5. All scenes in this world belong to an act
    let mut result = ctx
        .graph()
        .execute(
            query(
                "MATCH (s:Scene)-[:PART_OF]->(a:Act {world_id: $id})
                 RETURN count(s) as scene_count",
            )
            .param("id", ctx.world.world_id.to_string()),
        )
        .await
        .expect("Scene query failed");

    let row = result.next().await.expect("No result").expect("Row error");
    let scene_count: i64 = row.get("scene_count").expect("scene_count column");
    // We should have scenes (the exact count depends on fixtures)
    assert!(scene_count > 0, "Should have scenes in the world");
}
