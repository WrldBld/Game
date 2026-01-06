use std::{sync::Arc, time::Duration};

use chrono::{TimeZone, Utc};
use neo4rs::query;
use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage};
use tokio::time::sleep;
use uuid::Uuid;
use wrldbldr_domain::{
    EventOutcome, NarrativeEvent, NarrativeEventId, NarrativeTrigger, NarrativeTriggerType,
    LocationId, MoodState, RegionId, StagedNpc, Staging, StagingSource, TriggerLogic, WorldId,
};

use crate::infrastructure::{clock::FixedClock, ports::{NarrativeRepo, StagingRepo}};

fn neo4j_image(password: &str) -> GenericImage {
    GenericImage::new("neo4j", "5")
        .with_env_var("NEO4J_AUTH", format!("neo4j/{password}"))
        .with_env_var("NEO4J_dbms_connector_bolt_advertised__address", "localhost:7687")
        .with_exposed_port(7687)
        .with_wait_for(WaitFor::message_on_stdout("Started."))
}

async fn connect_with_retry(uri: &str, user: &str, pass: &str) -> neo4rs::Graph {
    let mut last_err: Option<anyhow::Error> = None;
    for _ in 0..60 {
        match neo4rs::Graph::new(uri, user, pass).await {
            Ok(graph) => return graph,
            Err(e) => {
                last_err = Some(anyhow::anyhow!(e));
                sleep(Duration::from_millis(250)).await;
            }
        }
    }

    panic!(
        "Failed to connect to Neo4j at {uri} after retries: {:?}",
        last_err
    );
}

async fn clean_db(graph: &neo4rs::Graph) {
    graph
        .run(query("MATCH (n) DETACH DELETE n"))
        .await
        .expect("clean db");
}

#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn narrative_triggers_fallback_is_bounded_to_500() {
    let password = "password";
    let container = neo4j_image(password).start().await;
    let bolt_port = container.get_host_port_ipv4(7687).await;
    let uri = format!("bolt://127.0.0.1:{bolt_port}");

    let graph = connect_with_retry(&uri, "neo4j", password).await;
    clean_db(&graph).await;

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
    let repo = super::Neo4jNarrativeRepo::new(graph.clone(), clock);

    let trigger = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::PlayerEntersLocation {
            location_id,
            location_name: "Test Region".to_string(),
        },
        description: "enter test region".to_string(),
        is_required: true,
        trigger_id: "t1".to_string(),
    };

    let outcome = EventOutcome {
        name: "default".to_string(),
        label: "Default".to_string(),
        description: "noop".to_string(),
        condition: None,
        effects: vec![],
        chain_events: vec![],
        timeline_summary: None,
    };

    for i in 0..600 {
        let event = NarrativeEvent {
            id: NarrativeEventId::new(),
            world_id,
            name: format!("Event {i}"),
            description: "test".to_string(),
            tags: vec![],
            trigger_conditions: vec![trigger.clone()],
            trigger_logic: TriggerLogic::All,
            scene_direction: "sd".to_string(),
            suggested_opening: None,
            outcomes: vec![outcome.clone()],
            default_outcome: Some("default".to_string()),
            is_active: true,
            is_triggered: false,
            triggered_at: None,
            selected_outcome: None,
            is_repeatable: false,
            trigger_count: 0,
            delay_turns: 0,
            expires_after_turns: None,
            priority: i as i32,
            is_favorite: false,
            created_at: now,
            updated_at: now,
        };

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
    let password = "password";
    let container = neo4j_image(password).start().await;
    let bolt_port = container.get_host_port_ipv4(7687).await;
    let uri = format!("bolt://127.0.0.1:{bolt_port}");

    let graph = connect_with_retry(&uri, "neo4j", password).await;
    clean_db(&graph).await;

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));
    let repo = super::Neo4jStagingRepo::new(graph.clone(), clock);

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
        now,
        "dm",
        StagingSource::RuleBased,
        24,
        now,
    );
    staging.is_active = false;
    staging.npcs = npc_ids
        .iter()
        .enumerate()
        .map(|(i, id)| StagedNpc {
            character_id: (*id).into(),
            name: format!("NPC {i}"),
            sprite_asset: None,
            portrait_asset: None,
            is_present: true,
            is_hidden_from_players: false,
            reasoning: "test".to_string(),
            mood: MoodState::Calm,
        })
        .collect();

    repo.save_pending_staging(&staging)
        .await
        .expect("save pending staging");

    let mut result = graph
        .execute(
            query(
                "MATCH (s:Staging {id: $id})-[r:INCLUDES_NPC]->(:Character) RETURN COUNT(r) as cnt",
            )
            .param("id", staging.id.to_string()),
        )
        .await
        .expect("query includes_npc count");

    let row = result.next().await.expect("row read").expect("row");
    let count: i64 = row.get("cnt").expect("cnt");
    assert_eq!(count, 3);
}
