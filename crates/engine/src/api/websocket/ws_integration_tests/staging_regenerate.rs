use super::*;

#[tokio::test]
async fn when_dm_requests_staging_regenerate_then_returns_llm_suggestions_and_does_not_mutate_staging(
) {
    use crate::infrastructure::ports::{NpcRegionRelationType, NpcWithRegionInfo};
    use wrldbldr_domain::RegionFrequency;

    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let location_id = LocationId::new();
    let region_id = RegionId::new();
    let npc_id = CharacterId::new();

    let world_name = wrldbldr_domain::WorldName::new("Test World").unwrap();
    let world = wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("desc").unwrap())
        .with_id(world_id);

    let location_name = wrldbldr_domain::value_objects::LocationName::new("Test Location").unwrap();
    let location = wrldbldr_domain::Location::new(
        world_id,
        location_name,
        wrldbldr_domain::LocationType::Exterior,
    )
    .with_description(wrldbldr_domain::Description::new("desc").unwrap())
    .with_id(location_id);

    let region = wrldbldr_domain::Region::from_storage(
        region_id,
        location_id,
        wrldbldr_domain::value_objects::RegionName::new("Test Region").unwrap(),
        wrldbldr_domain::Description::default(),
        None,
        None,
        None,
        false,
        0,
    );

    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));
    world_repo.expect_save().returning(|_world| Ok(()));

    let mut repos = TestAppRepos::new(world_repo);

    let region_for_get = region.clone();
    repos
        .location_repo
        .expect_get_region()
        .returning(move |_| Ok(Some(region_for_get.clone())));

    let location_for_get = location.clone();
    repos
        .location_repo
        .expect_get_location()
        .returning(move |_| Ok(Some(location_for_get.clone())));

    // Candidates for LLM suggestions.
    repos
        .character_repo
        .expect_get_npcs_for_region()
        .returning(move |_| {
            Ok(vec![NpcWithRegionInfo {
                character_id: npc_id,
                name: "Alice".to_string(),
                sprite_asset: None,
                portrait_asset: None,
                relationship_type: NpcRegionRelationType::Frequents,
                shift: None,
                frequency: Some(RegionFrequency::Often),
                time_of_day: None,
                reason: None,
                default_mood: wrldbldr_domain::MoodState::default(),
            }])
        });

    // Regenerate should not touch staging persistence.
    repos.staging_repo.expect_save_pending_staging().times(0);
    repos.staging_repo.expect_activate_staging().times(0);

    let llm = Arc::new(FixedLlm {
        content: r#"[{"name":"Alice","reason":"She is here"}]"#.to_string(),
    });

    let app = build_test_app_with_ports(repos, now, Arc::new(NoopQueue), llm);
    let connections = Arc::new(ConnectionManager::new());

    let ws_state = Arc::new(WsState {
        app,
        connections,
        pending_time_suggestions: Arc::new(TimeSuggestionStore::new()),
        pending_staging_requests: Arc::new(PendingStagingStoreImpl::new()),
        generation_read_state: GenerationStateStoreImpl::new(),
    });

    // Seed a pending staging request correlation.
    let request_id = "req-123".to_string();
    ws_state
        .pending_staging_requests
        .insert(
            request_id.clone(),
            PendingStagingRequest {
                region_id,
                location_id,
                world_id,
                created_at: now,
            },
        )
        .await;

    let (addr, server) = spawn_ws_server(ws_state.clone()).await;
    let mut dm_ws = ws_connect(addr).await;

    // DM joins.
    ws_send_client(
        &mut dm_ws,
        &ClientMessage::JoinWorld {
            world_id: *world_id.as_uuid(),
            role: ProtoWorldRole::Dm,
            user_id: "dm-user".to_string(),
            pc_id: None,
            spectate_pc_id: None,
        },
    )
    .await;
    let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::WorldJoined { .. })
    })
    .await;

    // DM requests regeneration.
    ws_send_client(
        &mut dm_ws,
        &ClientMessage::StagingRegenerateRequest {
            request_id: request_id.clone(),
            guidance: "more drama".to_string(),
        },
    )
    .await;

    let regenerated = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::StagingRegenerated { .. })
    })
    .await;

    match regenerated {
        ServerMessage::StagingRegenerated {
            request_id: got_id,
            llm_based_npcs,
        } => {
            assert_eq!(got_id, request_id);
            assert_eq!(llm_based_npcs.len(), 1);
            assert_eq!(llm_based_npcs[0].character_id, npc_id.to_string());
            assert!(llm_based_npcs[0].reasoning.contains("[LLM]"));
        }
        other => panic!("expected StagingRegenerated, got: {:?}", other),
    }

    server.abort();
}
