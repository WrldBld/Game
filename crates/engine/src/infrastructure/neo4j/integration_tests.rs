use std::sync::Arc;

use chrono::{TimeZone, Utc};
use neo4rs::query;
use uuid::Uuid;
use wrldbldr_domain::{
    Description, EventOutcome, LocationId, MoodState, NarrativeEvent, NarrativeEventName,
    NarrativeTrigger, NarrativeTriggerType, RegionId, StagedNpc, Staging, StagingSource,
    TriggerLogic, WorldId,
};

use crate::e2e_tests::{clean_db, SharedNeo4jHarness};
use crate::infrastructure::{
    clock::FixedClock,
    ports::{NarrativeRepo, StagingRepo},
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
            NarrativeEventName::new(format!("Event {i}")).unwrap(),
            now,
        )
        .with_description("test")
        .with_trigger_conditions(vec![trigger.clone()])
        .with_trigger_logic(TriggerLogic::All)
        .with_scene_direction(Description::new("sd").unwrap())
        .with_outcomes(vec![outcome.clone()])
        .with_default_outcome("default")
        .with_priority(i as i32);

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

    let mut staging = Staging::new(
        region_id,
        location_id,
        world_id,
        0, // game_time_minutes at epoch
        "dm",
        StagingSource::RuleBased,
        24,
        now,
    );
    let staging = staging.with_active(false).with_npcs(vec![
        StagedNpc::new(npc_ids[0].into(), "NPC 0", true, "r0").with_mood(MoodState::Calm),
        StagedNpc::new(npc_ids[1].into(), "NPC 1", true, "r1")
            .with_hidden_from_players(true)
            .with_mood(MoodState::Nervous),
        StagedNpc::new(npc_ids[2].into(), "NPC 2", false, "r2").with_mood(MoodState::Happy),
    ]);

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
