use std::sync::Arc;

use chrono::{TimeZone, Utc};
use neo4rs::query;
use uuid::Uuid;
use wrldbldr_domain::{
    CharacterId, Description, EventOutcome, LocationId, Lore, LoreCategory, LoreDiscoverySource,
    LoreId, LoreKnowledge, MoodState, NarrativeEvent, NarrativeEventName, NarrativeTrigger,
    NarrativeTriggerType, RegionId, StagedNpc, Staging, StagingSource, Tag, TriggerLogic, WorldId,
};

use crate::e2e_tests::{clean_db, SharedNeo4jHarness};
use crate::infrastructure::{
    clock::FixedClock,
    ports::{LoreRepo, NarrativeRepo, RepoError, StagingRepo},
};

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn narrative_triggers_fallback_is_bounded_to_500() {
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let world_id = WorldId::new();
    graph
        .run(query("CREATE (:World {id: $id})").param("id", world_id.to_string()))
        .await
        .expect("create world");

    // Repo compares trigger location_id string to region_id string.
    // For this test we intentionally re-use the same UUID so they match.
    let region_uuid = Uuid::new_v4();
    let region_id = wrldbldr_domain::RegionId::from(region_uuid);
    let location_id = wrldbldr_domain::LocationId::from(region_uuid);

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jNarrativeRepo::new(test_graph, clock);

    let trigger = NarrativeTrigger::new(
        NarrativeTriggerType::PlayerEntersLocation {
            location_id,
            location_name: "Test Region".to_string(),
        },
        "enter test region",
        "t1",
    )
    .with_required(true);

    let outcome = EventOutcome::new("default", "Default", "noop");

    for i in 0..600 {
        let event = NarrativeEvent::new(
            world_id,
            NarrativeEventName::new(format!("Event {i}")).expect("valid event name"),
            now,
        )
        .with_description("test")
        .with_trigger_conditions(vec![trigger.clone()])
        .with_trigger_logic(TriggerLogic::All)
        .with_scene_direction(Description::new("sd").expect("valid description"))
        .with_outcomes(vec![outcome.clone()])
        .with_default_outcome("default")
        .with_priority(i);

        repo.save_event(&event).await.expect("save event");
    }

    let triggers = repo
        .get_triggers_for_region(world_id, region_id)
        .await
        .expect("get triggers");

    assert_eq!(triggers.len(), 500);
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn save_pending_staging_creates_includes_npc_edges_for_all_npcs() {
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jStagingRepo::new(test_graph, clock);

    let region_id = RegionId::new();
    let location_id = LocationId::new();
    let world_id = WorldId::new();

    graph
        .run(query("CREATE (:Region {id: $id})").param("id", region_id.to_string()))
        .await
        .expect("create region");

    let npc_ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
    for (i, id) in npc_ids.iter().enumerate() {
        graph
            .run(
                query("CREATE (:Character {id: $id, name: $name})")
                    .param("id", id.to_string())
                    .param("name", format!("NPC {i}")),
            )
            .await
            .expect("create character");
    }

    let staging = Staging::new(
        region_id,
        location_id,
        world_id,
        0, // game_time_seconds at epoch
        "dm",
        StagingSource::RuleBased,
        24,
        now,
    );
    let mut npc0 = StagedNpc::new(npc_ids[0].into(), "NPC 0", true, "r0");
    npc0.mood = MoodState::Calm;
    let mut npc1 =
        StagedNpc::new(npc_ids[1].into(), "NPC 1", true, "r1").with_hidden_from_players(true);
    npc1.mood = MoodState::Nervous;
    let mut npc2 = StagedNpc::new(npc_ids[2].into(), "NPC 2", false, "r2");
    npc2.mood = MoodState::Happy;
    let staging = staging.with_active(false).with_npcs(vec![npc0, npc1, npc2]);

    repo.save_pending_staging(&staging)
        .await
        .expect("save pending staging");

    let mut result = graph
        .execute(
            query(
                "MATCH (s:Staging {id: $id})-[r:INCLUDES_NPC]->(c:Character)\
                 RETURN c.name as name,\
                        r.is_present as is_present,\
                        COALESCE(r.is_hidden_from_players, false) as hidden,\
                        r.reasoning as reasoning,\
                        r.mood as mood\
                 ORDER BY name",
            )
            .param("id", staging.id().to_string()),
        )
        .await
        .expect("query includes_npc edges");

    let mut rows = Vec::new();
    while let Some(row) = result.next().await.expect("row read") {
        rows.push(row);
    }
    assert_eq!(rows.len(), 3);

    let r0_name: String = rows[0].get("name").expect("name");
    let r0_present: bool = rows[0].get("is_present").expect("is_present");
    let r0_hidden: bool = rows[0].get("hidden").expect("hidden");
    let r0_reasoning: String = rows[0].get("reasoning").expect("reasoning");
    let r0_mood: String = rows[0].get("mood").expect("mood");
    assert_eq!(r0_name, "NPC 0");
    assert!(r0_present);
    assert!(!r0_hidden);
    assert_eq!(r0_reasoning, "r0");
    assert_eq!(r0_mood, "calm");

    let r1_name: String = rows[1].get("name").expect("name");
    let r1_present: bool = rows[1].get("is_present").expect("is_present");
    let r1_hidden: bool = rows[1].get("hidden").expect("hidden");
    let r1_reasoning: String = rows[1].get("reasoning").expect("reasoning");
    let r1_mood: String = rows[1].get("mood").expect("mood");
    assert_eq!(r1_name, "NPC 1");
    assert!(r1_present);
    assert!(r1_hidden);
    assert_eq!(r1_reasoning, "r1");
    assert_eq!(r1_mood, "nervous");

    let r2_name: String = rows[2].get("name").expect("name");
    let r2_present: bool = rows[2].get("is_present").expect("is_present");
    let r2_hidden: bool = rows[2].get("hidden").expect("hidden");
    let r2_reasoning: String = rows[2].get("reasoning").expect("reasoning");
    let r2_mood: String = rows[2].get("mood").expect("mood");
    assert_eq!(r2_name, "NPC 2");
    assert!(!r2_present);
    assert!(!r2_hidden);
    assert_eq!(r2_reasoning, "r2");
    assert_eq!(r2_mood, "happy");
}

// =============================================================================
// Phase 1 Narrative Event Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn set_event_active_updates_activation_in_neo4j() {
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let world_id = WorldId::new();
    graph
        .run(query("CREATE (:World {id: $id})").param("id", world_id.to_string()))
        .await
        .expect("create world");

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jNarrativeRepo::new(test_graph, clock);

    // Create an active event
    let event = NarrativeEvent::new(
        world_id,
        NarrativeEventName::new("Test Event").expect("valid event name"),
        now,
    );
    repo.save_event(&event).await.expect("save event");

    // Deactivate the event
    repo.set_event_active(event.id(), false)
        .await
        .expect("deactivate event");

    // Reload and verify it's inactive
    let loaded = repo.get_event(event.id()).await.expect("load event");
    let loaded = loaded.expect("event should exist");
    assert!(!loaded.is_active());
    assert_eq!(loaded.activation(), wrldbldr_domain::EventActivation::Inactive);

    // Reactivate the event
    repo.set_event_active(event.id(), true)
        .await
        .expect("activate event");

    // Reload and verify it's active
    let loaded = repo.get_event(event.id()).await.expect("load event");
    let loaded = loaded.expect("event should exist");
    assert!(loaded.is_active());
    assert_eq!(loaded.activation(), wrldbldr_domain::EventActivation::Active);
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn get_completed_events_returns_events_from_all_chains() {
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let world_id = WorldId::new();
    graph
        .run(query("CREATE (:World {id: $id})").param("id", world_id.to_string()))
        .await
        .expect("create world");

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jNarrativeRepo::new(test_graph, clock);

    // Create two event chains
    let chain1_id = wrldbldr_domain::EventChainId::new();
    let _chain2_id = wrldbldr_domain::EventChainId::new();

    let mut chain1 = wrldbldr_domain::EventChain::new(
        world_id,
        wrldbldr_domain::EventChainName::new("Chain 1").expect("valid chain name"),
        now,
    ).expect("create chain");
    chain1.add_event(wrldbldr_domain::NarrativeEventId::new(), now);
    chain1.add_event(wrldbldr_domain::NarrativeEventId::new(), now);
    repo.save_chain(&chain1).await.expect("save chain 1");

    let mut chain2 = wrldbldr_domain::EventChain::new(
        world_id,
        wrldbldr_domain::EventChainName::new("Chain 2").expect("valid chain name"),
        now,
    ).expect("create chain");
    chain2.add_event(wrldbldr_domain::NarrativeEventId::new(), now);
    repo.save_chain(&chain2).await.expect("save chain 2");

    // Mark events as completed in chain 1
    let mut chain1_reloaded = repo.get_chain(chain1_id).await.expect("load chain")
        .expect("chain should exist");
    let event1 = *chain1_reloaded.events().first().expect("event exists");
    let event2 = *chain1_reloaded.events().get(1).expect("event exists");
    chain1_reloaded.complete_event(event1, now);
    chain1_reloaded.complete_event(event2, now);
    repo.save_chain(&chain1_reloaded).await.expect("save chain with completed events");

    // Get completed events
    let completed = repo.get_completed_events(world_id).await.expect("get completed events");

    // Should return all 3 events (2 from chain1, 1 from chain2 marked completed)
    // Note: get_completed_events returns events from chain.completed_events, not chain.events
    assert_eq!(completed.len(), 2);
    assert!(completed.contains(&event1));
    assert!(completed.contains(&event2));
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn get_chains_for_event_returns_containing_chains() {
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let world_id = WorldId::new();
    graph
        .run(query("CREATE (:World {id: $id})").param("id", world_id.to_string()))
        .await
        .expect("create world");

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jNarrativeRepo::new(test_graph, clock);

    // Create event
    let event_id = wrldbldr_domain::NarrativeEventId::new();

    // Create two chains containing the same event
    let chain1_id = wrldbldr_domain::EventChainId::new();
    let mut chain1 = wrldbldr_domain::EventChain::new(
        world_id,
        wrldbldr_domain::EventChainName::new("Chain 1").expect("valid chain name"),
        now,
    ).expect("create chain");
    chain1.add_event(event_id, now);
    repo.save_chain(&chain1).await.expect("save chain 1");

    let chain2_id = wrldbldr_domain::EventChainId::new();
    let mut chain2 = wrldbldr_domain::EventChain::new(
        world_id,
        wrldbldr_domain::EventChainName::new("Chain 2").expect("valid chain name"),
        now,
    ).expect("create chain");
    chain2.add_event(event_id, now);
    repo.save_chain(&chain2).await.expect("save chain 2");

    // Get chains for the event
    let chains = repo.get_chains_for_event(event_id).await.expect("get chains for event");

    // Should return both chains
    assert_eq!(chains.len(), 2);
    let chain_ids: Vec<_> = chains.iter().map(|c| c.id()).collect();
    assert!(chain_ids.contains(&chain1_id));
    assert!(chain_ids.contains(&chain2_id));
}

// =============================================================================
// Lore Repository Tests - Fail-Fast Parsing and Tag Search Semantics
// =============================================================================

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn lore_search_by_tags_uses_exact_matching_not_substring() {
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jLoreRepo::new(test_graph, clock);

    let world_id = WorldId::new();
    graph
        .run(query("CREATE (:World {id: $id})").param("id", world_id.to_string()))
        .await
        .expect("create world");

    // Create lore with "martial" tag (NOT "art")
    let lore_martial = Lore::new(
        world_id,
        "Martial Combat".to_string(),
        LoreCategory::Historical,
        now,
    )
    .with_tag(Tag::new("martial").expect("valid tag"))
    .with_tag(Tag::new("combat").expect("valid tag"));
    repo.save(&lore_martial).await.expect("save martial lore");

    // Create lore with "art" tag
    let lore_art = Lore::new(
        world_id,
        "Fine Arts".to_string(),
        LoreCategory::Common,
        now,
    )
    .with_tag(Tag::new("art").expect("valid tag"))
    .with_tag(Tag::new("culture").expect("valid tag"));
    repo.save(&lore_art).await.expect("save art lore");

    // Create lore with "martial-art" tag (compound)
    let lore_martial_art = Lore::new(
        world_id,
        "Kung Fu Styles".to_string(),
        LoreCategory::Common,
        now,
    )
    .with_tag(Tag::new("martial-art").expect("valid tag"))
    .with_tag(Tag::new("history").expect("valid tag"));
    repo.save(&lore_martial_art)
        .await
        .expect("save martial-art lore");

    // Search for "art" - should ONLY return lore with "art" tag, not "martial"
    let results = repo
        .search_by_tags(world_id, &["art".to_string()])
        .await
        .expect("search by tags");

    // Should find exactly 1 result: "Fine Arts" (has "art" tag)
    // Should NOT find "Martial Combat" (only has "martial" tag)
    assert_eq!(results.len(), 1, "Expected exactly 1 result for 'art' search, got {}: {:?}", 
        results.len(), results.iter().map(|l| l.title()).collect::<Vec<_>>());
    assert_eq!(results[0].title(), "Fine Arts");

    // Search for "martial" - should NOT match "martial-art"
    let results = repo
        .search_by_tags(world_id, &["martial".to_string()])
        .await
        .expect("search by martial tag");

    // Should find exactly 1 result: "Martial Combat" (has "martial" tag)
    // Should NOT find "Kung Fu Styles" (only has "martial-art" tag)
    assert_eq!(results.len(), 1, "Expected exactly 1 result for 'martial' search, got {}",
        results.len());
    assert_eq!(results[0].title(), "Martial Combat");

    // Search for "martial-art" - should find only the compound tag
    let results = repo
        .search_by_tags(world_id, &["martial-art".to_string()])
        .await
        .expect("search by martial-art tag");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title(), "Kung Fu Styles");
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn lore_search_by_tags_returns_multiple_matches_for_or_semantics() {
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jLoreRepo::new(test_graph, clock);

    let world_id = WorldId::new();
    graph
        .run(query("CREATE (:World {id: $id})").param("id", world_id.to_string()))
        .await
        .expect("create world");

    // Create lore entries with different tags
    let lore1 = Lore::new(
        world_id,
        "History of Magic".to_string(),
        LoreCategory::Historical,
        now,
    )
    .with_tag(Tag::new("magic").expect("valid tag"))
    .with_tag(Tag::new("history").expect("valid tag"));
    repo.save(&lore1).await.expect("save lore1");

    let lore2 = Lore::new(
        world_id,
        "Ancient Relics".to_string(),
        LoreCategory::Historical,
        now,
    )
    .with_tag(Tag::new("history").expect("valid tag"))
    .with_tag(Tag::new("artifacts").expect("valid tag"));
    repo.save(&lore2).await.expect("save lore2");

    let lore3 = Lore::new(
        world_id,
        "Dragon Lore".to_string(),
        LoreCategory::Legend,
        now,
    )
    .with_tag(Tag::new("dragons").expect("valid tag"));
    repo.save(&lore3).await.expect("save lore3");

    // Search for "magic" OR "dragons" - should return lore1 and lore3
    let results = repo
        .search_by_tags(world_id, &["magic".to_string(), "dragons".to_string()])
        .await
        .expect("search by multiple tags");

    assert_eq!(results.len(), 2);
    let titles: Vec<_> = results.iter().map(|l| l.title()).collect();
    assert!(titles.contains(&"History of Magic"));
    assert!(titles.contains(&"Dragon Lore"));
    assert!(!titles.contains(&"Ancient Relics"));
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn lore_repo_fail_fast_on_malformed_tags_json() {
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jLoreRepo::new(test_graph, clock);

    let world_id = WorldId::new();
    let lore_id = LoreId::new();

    graph
        .run(query("CREATE (:World {id: $id})").param("id", world_id.to_string()))
        .await
        .expect("create world");

    // Insert a lore entry with malformed JSON in the tags field directly via Cypher
    // This simulates data corruption
    graph
        .run(
            query(
                "CREATE (l:Lore {
                    id: $lore_id,
                    world_id: $world_id,
                    title: $title,
                    category: $category,
                    tags: $malformed_tags,
                    created_at: $created_at,
                    updated_at: $updated_at
                })"
            )
            .param("lore_id", lore_id.to_string())
            .param("world_id", world_id.to_string())
            .param("title", "Corrupted Lore")
            .param("category", "History")
            .param("malformed_tags", "not-valid-json[{")
            .param("created_at", now.to_rfc3339())
            .param("updated_at", now.to_rfc3339()),
        )
        .await
        .expect("create corrupted lore");

    // Attempting to read the lore should fail with Serialization error
    let result = repo.get(lore_id).await;

    assert!(
        result.is_err(),
        "Expected error when reading lore with malformed tags JSON, got: {:?}",
        result
    );

    let err = result.unwrap_err();
    assert!(
        matches!(err, RepoError::Serialization(_)),
        "Expected Serialization error, got: {:?}",
        err
    );

    // Verify the error message contains context
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Invalid tags JSON") || err_msg.contains("tags"),
        "Error message should mention tags JSON issue, got: {}",
        err_msg
    );
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn lore_repo_fail_fast_on_missing_chunk_fields() {
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jLoreRepo::new(test_graph, clock);

    let world_id = WorldId::new();
    let lore_id = LoreId::new();
    let chunk_id = wrldbldr_domain::LoreChunkId::new();

    graph
        .run(query("CREATE (:World {id: $id})").param("id", world_id.to_string()))
        .await
        .expect("create world");

    // Create a lore entry
    let lore = Lore::new(
        world_id,
        "Test Lore".to_string(),
        LoreCategory::Historical,
        now,
    );
    repo.save(&lore).await.expect("save lore");

    // Insert a chunk with missing content field directly via Cypher
    // This simulates data corruption
    graph
        .run(
            query(
                "MATCH (l:Lore {id: $lore_id})
                 CREATE (c:LoreChunk {
                    id: $chunk_id,
                    lore_id: $lore_id,
                    order: 0,
                    lore_order_key: $order_key
                 })
                 CREATE (l)-[:HAS_CHUNK]->(c)"
            )
            .param("lore_id", lore_id.to_string())
            .param("chunk_id", chunk_id.to_string())
            .param("order_key", format!("{}_0", lore_id)),
        )
        .await
        .expect("create corrupted chunk");

    // Attempting to read the lore should fail due to missing chunk content
    let result = repo.get(lore_id).await;

    assert!(
        result.is_err(),
        "Expected error when reading lore with missing chunk content, got: {:?}",
        result
    );

    let err = result.unwrap_err();
    assert!(
        matches!(err, RepoError::Database { .. }),
        "Expected Database error, got: {:?}",
        err
    );

    // Verify the error message contains context about the missing field
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("content") || err_msg.contains("Missing required field"),
        "Error message should mention missing content field, got: {}",
        err_msg
    );
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn lore_repo_journal_entry_fail_fast_on_malformed_tags() {
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jLoreRepo::new(test_graph, clock);

    let world_id = WorldId::new();
    let character_id = wrldbldr_domain::CharacterId::new();
    let lore_id = LoreId::new();

    // Create world, character, and lore with malformed tags
    graph
        .run(query("CREATE (:World {id: $id})").param("id", world_id.to_string()))
        .await
        .expect("create world");

    graph
        .run(
            query("CREATE (:Character {id: $id, name: 'Test Character'})")
                .param("id", character_id.to_string()),
        )
        .await
        .expect("create character");

    // Insert lore with malformed tags
    graph
        .run(
            query(
                "CREATE (l:Lore {
                    id: $lore_id,
                    world_id: $world_id,
                    title: 'Corrupted Lore',
                    category: 'History',
                    tags: 'invalid-json',
                    created_at: $created_at,
                    updated_at: $updated_at
                })"
            )
            .param("lore_id", lore_id.to_string())
            .param("world_id", world_id.to_string())
            .param("created_at", now.to_rfc3339())
            .param("updated_at", now.to_rfc3339()),
        )
        .await
        .expect("create corrupted lore");

    // Create knowledge relationship
    graph
        .run(
            query(
                "MATCH (c:Character {id: $char_id})
                 MATCH (l:Lore {id: $lore_id})
                 CREATE (c)-[:KNOWS_LORE {
                    lore_id: $lore_id,
                    character_id: $char_id,
                    known_chunk_ids: '[]',
                    discovery_source: '{}',
                    discovered_at: $discovered_at
                 }]->(l)"
            )
            .param("char_id", character_id.to_string())
            .param("lore_id", lore_id.to_string())
            .param("discovered_at", now.to_rfc3339()),
        )
        .await
        .expect("create knowledge");

    // Attempting to get the journal should fail with Serialization error
    let result = repo.get_character_lore_journal(character_id).await;

    assert!(
        result.is_err(),
        "Expected error when reading journal with malformed tags JSON, got: {:?}",
        result
    );

    let err = result.unwrap_err();
    assert!(
        matches!(err, RepoError::Serialization(_)),
        "Expected Serialization error, got: {:?}",
        err
    );
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn lore_repo_handles_empty_tags_gracefully() {
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jLoreRepo::new(test_graph, clock);

    let world_id = WorldId::new();

    graph
        .run(query("CREATE (:World {id: $id})").param("id", world_id.to_string()))
        .await
        .expect("create world");

    // Create lore with no tags (empty array JSON)
    let lore = Lore::new(
        world_id,
        "Untagged Lore".to_string(),
        LoreCategory::Historical,
        now,
    );
    repo.save(&lore).await.expect("save lore");

    // Should be able to read it back
    let loaded = repo.get(lore.id()).await.expect("load lore");
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert!(loaded.tags().is_empty());

    // Searching for any tag should return empty results
    let results = repo
        .search_by_tags(world_id, &["nonexistent".to_string()])
        .await
        .expect("search by nonexistent tag");
    assert!(results.is_empty());
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn lore_repo_roundtrip_preserves_tags_exactly() {
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jLoreRepo::new(test_graph, clock);

    let world_id = WorldId::new();

    graph
        .run(query("CREATE (:World {id: $id})").param("id", world_id.to_string()))
        .await
        .expect("create world");

    // Create lore with various tag patterns that could cause false positives
    let lore = Lore::new(
        world_id,
        "Complex Tags".to_string(),
        LoreCategory::Historical,
        now,
    )
    .with_tag(Tag::new("a").expect("valid tag"))
    .with_tag(Tag::new("art").expect("valid tag"))
    .with_tag(Tag::new("martial").expect("valid tag"))
    .with_tag(Tag::new("martial-art").expect("valid tag"))
    .with_tag(Tag::new("art-history").expect("valid tag"));
    repo.save(&lore).await.expect("save lore");

    // Load and verify all tags are preserved
    let loaded = repo.get(lore.id()).await.expect("load lore").unwrap();
    let tags: Vec<_> = loaded.tags().iter().map(|t| t.to_string()).collect();

    assert_eq!(tags.len(), 5);
    assert!(tags.contains(&"a".to_string()));
    assert!(tags.contains(&"art".to_string()));
    assert!(tags.contains(&"martial".to_string()));
    assert!(tags.contains(&"martial-art".to_string()));
    assert!(tags.contains(&"art-history".to_string()));

    // Verify each tag search returns correct results
    for tag in ["a", "art", "martial", "martial-art", "art-history"] {
        let results = repo
            .search_by_tags(world_id, &[tag.to_string()])
            .await
            .expect("search by tag");

        assert_eq!(
            results.len(),
            1,
            "Search for '{}' should return exactly 1 result, got {}: {:?}",
            tag,
            results.len(),
            results.iter().map(|l| l.title()).collect::<Vec<_>>()
        );
        assert_eq!(results[0].title(), "Complex Tags");
    }
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn lore_repo_handles_null_tags_as_empty() {
    // Tests that lore entries with null/missing tags property are treated as having empty tags
    // This ensures consistent behavior between row_to_lore_without_chunks and node_to_lore_detail_repo
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jLoreRepo::new(test_graph, clock);

    let world_id = WorldId::new();
    let lore_id = LoreId::new();

    graph
        .run(query("CREATE (:World {id: $id})").param("id", world_id.to_string()))
        .await
        .expect("create world");

    // Create lore node directly with NULL tags (simulating legacy data)
    // This tests the path where tags property is missing entirely
    graph
        .run(
            query(
                "CREATE (l:Lore {
                    id: $lore_id,
                    world_id: $world_id,
                    title: 'Legacy Lore Without Tags',
                    category: 'historical',
                    is_common_knowledge: false,
                    created_at: datetime(),
                    updated_at: datetime()
                })"
            )
            .param("lore_id", lore_id.to_string())
            .param("world_id", world_id.to_string()),
        )
        .await
        .expect("create lore with null tags");

    // Should be able to read it back (treating null tags as empty)
    let loaded = repo.get(lore_id).await.expect("load lore");
    assert!(loaded.is_some(), "Should be able to load lore with null tags");
    let loaded = loaded.unwrap();
    assert!(loaded.tags().is_empty(), "Null tags should be treated as empty");

    // Searching should not match any tags (empty list)
    let results = repo
        .search_by_tags(world_id, &["any".to_string()])
        .await
        .expect("search by tag");
    assert!(results.is_empty(), "Lore with null tags should not match any search");

    // Also test via journal entry path (uses node_to_lore_detail_repo)
    let character_id = CharacterId::new();
    graph
        .run(query("CREATE (:Character {id: $id})").param("id", character_id.to_string()))
        .await
        .expect("create character");

    // Grant knowledge to create a path through row_to_journal_entry
    let knowledge = LoreKnowledge::full(
        lore_id,
        character_id,
        LoreDiscoverySource::DmGranted { reason: Some("Test DM grant".to_string()) },
        now,
    );
    repo.grant_knowledge(&knowledge).await.expect("grant knowledge");

    // Load via journal entry path
    let journal = repo
        .get_character_lore_journal(character_id)
        .await
        .expect("get journal");
    assert_eq!(journal.len(), 1, "Should have one journal entry");
    assert!(
        journal[0].lore.tags.is_empty(),
        "Journal entry should also have empty tags for null tags"
    );
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn lore_repo_search_by_tags_handles_empty_string_tags() {
    // Tests that lore entries with empty-string tags are treated as having empty tags
    // This ensures the nullIf(l.tags, "") fix works correctly in search_by_tags
    let harness = SharedNeo4jHarness::shared()
        .await
        .expect("Failed to get shared Neo4j harness");
    let graph = harness
        .create_graph()
        .await
        .expect("Failed to create graph");
    clean_db(&graph).await.expect("Failed to clean db");

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let test_graph = super::Neo4jGraph::new(graph.clone());
    let repo = super::Neo4jLoreRepo::new(test_graph, clock);

    let world_id = WorldId::new();
    let lore_id = LoreId::new();

    graph
        .run(query("CREATE (:World {id: $id})").param("id", world_id.to_string()))
        .await
        .expect("create world");

    // Create lore node directly with empty-string tags (simulating legacy data issue)
    // This tests the path where tags property is an empty string instead of valid JSON
    graph
        .run(
            query(
                "CREATE (l:Lore {
                    id: $lore_id,
                    world_id: $world_id,
                    title: 'Legacy Lore With Empty String Tags',
                    category: 'historical',
                    is_common_knowledge: false,
                    tags: '',
                    created_at: datetime(),
                    updated_at: datetime()
                })"
            )
            .param("lore_id", lore_id.to_string())
            .param("world_id", world_id.to_string()),
        )
        .await
        .expect("create lore with empty-string tags");

    // Should be able to read it back (treating empty-string tags as empty)
    let loaded = repo.get(lore_id).await.expect("load lore");
    assert!(loaded.is_some(), "Should be able to load lore with empty-string tags");
    let loaded = loaded.unwrap();
    assert!(loaded.tags().is_empty(), "Empty-string tags should be treated as empty");

    // Searching should not crash and should return empty results (empty tags don't match)
    let results = repo
        .search_by_tags(world_id, &["any".to_string()])
        .await
        .expect("search by tag should not crash with empty-string tags");
    assert!(results.is_empty(), "Lore with empty-string tags should not match any search");

    // Also create lore with valid tags to ensure search still works for valid entries
    let lore_with_tags = Lore::new(
        world_id,
        "Tagged Lore".to_string(),
        LoreCategory::Historical,
        now,
    )
    .with_tag(Tag::new("magic").expect("valid tag"));
    repo.save(&lore_with_tags).await.expect("save lore with tags");

    // Search should find the lore with valid tags
    let results = repo
        .search_by_tags(world_id, &["magic".to_string()])
        .await
        .expect("search by valid tag");
    assert_eq!(results.len(), 1, "Should find lore with valid tags");
    assert_eq!(results[0].title(), "Tagged Lore");
}
