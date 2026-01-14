use super::*;

#[tokio::test]
async fn when_dm_prestages_region_then_player_entering_gets_scene_changed_without_staging_pending()
{
    use wrldbldr_domain::value_objects::CampbellArchetype;
    use wrldbldr_domain::TimeMode;

    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let location_id = LocationId::new();
    let region_id = RegionId::new();
    let pc_id = PlayerCharacterId::new();
    let npc_id = CharacterId::new();

    let world_name = wrldbldr_domain::WorldName::new("Test World").unwrap();
    let mut world = wrldbldr_domain::World::new(world_name)
        .with_description(wrldbldr_domain::Description::new("desc").unwrap())
        .with_id(world_id);
    world.set_time_mode(TimeMode::Manual);

    let location_name = wrldbldr_domain::value_objects::LocationName::new("Test Location").unwrap();
    let location = wrldbldr_domain::Location::new(
        world_id,
        location_name,
        wrldbldr_domain::LocationType::Exterior,
    )
    .with_description(wrldbldr_domain::Description::new("desc").unwrap())
    .with_id(location_id);

    let mut region = wrldbldr_domain::Region::new(location_id, "Region");
    region.id = region_id;

    let pc = wrldbldr_domain::PlayerCharacter::new(
        "player-1",
        world_id,
        wrldbldr_domain::CharacterName::new("PC").unwrap(),
        location_id,
        now,
    )
    .with_id(pc_id);
    // initial spawn - PC starts with no current_region_id

    let mut npc = wrldbldr_domain::Character::new(
        world_id,
        wrldbldr_domain::CharacterName::new("NPC").unwrap(),
        CampbellArchetype::Hero,
    );
    npc = npc.with_id(npc_id);

    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));
    world_repo.expect_save().returning(|_world| Ok(()));

    let mut repos = TestAppRepos::new(world_repo);

    // Join+movement needs PC+region+location.
    let pc_for_get = pc.clone();
    repos
        .player_character_repo
        .expect_get()
        .returning(move |_| Ok(Some(pc_for_get.clone())));

    repos
        .player_character_repo
        .expect_get_inventory()
        .returning(|_| Ok(vec![]));

    repos
        .player_character_repo
        .expect_update_position()
        .returning(|_, _, _| Ok(()));

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

    repos
        .location_repo
        .expect_get_connections()
        .returning(|_| Ok(vec![]));

    repos
        .location_repo
        .expect_get_location_exits()
        .returning(|_| Ok(vec![]));

    repos
        .location_repo
        .expect_get_region_exits()
        .returning(|_| Ok(vec![]));

    repos
        .item_repo
        .expect_list_in_region()
        .returning(|_| Ok(vec![]));

    // Narrative triggers/scene/flags/observations: empty.
    repos
        .narrative_repo
        .expect_get_triggers_for_region()
        .returning(|_, _| Ok(vec![]));
    repos
        .scene_repo
        .expect_get_completed_scenes()
        .returning(|_| Ok(vec![]));
    repos
        .scene_repo
        .expect_list_for_region()
        .returning(|_| Ok(vec![]));
    repos
        .observation_repo
        .expect_get_observations()
        .returning(|_| Ok(vec![]));
    repos
        .observation_repo
        .expect_has_observed()
        .returning(|_, _| Ok(false));
    repos
        .observation_repo
        .expect_save_observation()
        .returning(|_| Ok(()));
    repos
        .flag_repo
        .expect_get_world_flags()
        .returning(|_| Box::pin(async { Ok(vec![]) }));
    repos
        .flag_repo
        .expect_get_pc_flags()
        .returning(|_| Box::pin(async { Ok(vec![]) }));

    // Visual state resolution: no states.
    repos
        .location_state_repo
        .expect_list_for_location()
        .returning(|_| Ok(vec![]));
    repos
        .region_state_repo
        .expect_list_for_region()
        .returning(|_| Ok(vec![]));
    repos
        .location_state_repo
        .expect_get_active()
        .returning(|_| Ok(None));
    repos
        .region_state_repo
        .expect_get_active()
        .returning(|_| Ok(None));

    // Character details used by PreStageRegion.
    let npc_for_get = npc.clone();
    repos.character_repo.expect_get().returning(move |id| {
        if id == npc_for_get.id() {
            Ok(Some(npc_for_get.clone()))
        } else {
            Ok(None)
        }
    });

    // Stage activation should influence subsequent get_active_staging.
    #[derive(Default)]
    struct SharedStaging {
        pending: Option<wrldbldr_domain::Staging>,
        activated: bool,
    }

    let shared = Arc::new(Mutex::new(SharedStaging::default()));

    let shared_for_save = shared.clone();
    repos
        .staging_repo
        .expect_save_pending_staging()
        .returning(move |s| {
            let mut guard = shared_for_save.lock().unwrap();
            guard.pending = Some(s.clone());
            Ok(())
        });

    let shared_for_activate = shared.clone();
    repos
        .staging_repo
        .expect_activate_staging()
        .withf(move |_id, r| *r == region_id)
        .returning(move |_id, _region| {
            let mut guard = shared_for_activate.lock().unwrap();
            guard.activated = true;
            Ok(())
        });

    let shared_for_get_active = shared.clone();
    repos
        .staging_repo
        .expect_get_active_staging()
        .returning(move |rid, _now| {
            let guard = shared_for_get_active.lock().unwrap();
            if guard.activated {
                Ok(guard.pending.clone().filter(|s| s.region_id == rid))
            } else {
                Ok(None)
            }
        });

    repos
        .staging_repo
        .expect_get_staged_npcs()
        .returning(|_| Ok(vec![]));

    repos
        .character_repo
        .expect_get_npcs_for_region()
        .returning(|_| Ok(vec![]));

    let app = build_test_app(repos, now);
    let connections = Arc::new(ConnectionManager::new());

    let ws_state = Arc::new(WsState {
        app,
        connections,
        pending_time_suggestions: TimeSuggestionStoreImpl::new(),
        pending_staging_requests: PendingStagingStoreImpl::new(),
        generation_read_state: GenerationStateStoreImpl::new(),
    });

    let (addr, server) = spawn_ws_server(ws_state.clone()).await;
    let mut dm_ws = ws_connect(addr).await;
    let mut player_ws = ws_connect(addr).await;

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

    // Player joins with PC.
    ws_send_client(
        &mut player_ws,
        &ClientMessage::JoinWorld {
            world_id: *world_id.as_uuid(),
            role: ProtoWorldRole::Player,
            user_id: "player-user".to_string(),
            pc_id: Some(*pc_id.as_uuid()),
            spectate_pc_id: None,
        },
    )
    .await;
    let _ = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::WorldJoined { .. })
    })
    .await;

    // DM receives UserJoined broadcast.
    let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::UserJoined { .. })
    })
    .await;

    // DM pre-stages the region.
    ws_send_client(
        &mut dm_ws,
        &ClientMessage::PreStageRegion {
            region_id: region_id.to_string(),
            npcs: vec![wrldbldr_protocol::ApprovedNpcInfo {
                character_id: npc_id.to_string(),
                is_present: true,
                reasoning: Some("pre-staged".to_string()),
                is_hidden_from_players: false,
                mood: None,
            }],
            ttl_hours: 24,
            location_state_id: None,
            region_state_id: None,
        },
    )
    .await;

    // Player moves into region and should immediately receive SceneChanged (not StagingPending).
    ws_send_client(
        &mut player_ws,
        &ClientMessage::MoveToRegion {
            pc_id: pc_id.to_string(),
            region_id: region_id.to_string(),
        },
    )
    .await;

    let scene_changed = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::SceneChanged { .. })
    })
    .await;
    match scene_changed {
        ServerMessage::SceneChanged { npcs_present, .. } => {
            assert!(npcs_present
                .iter()
                .any(|n| n.character_id == npc_id.to_string()));
        }
        other => panic!("expected SceneChanged, got: {:?}", other),
    }

    // DM should not receive a staging approval request as a result of the move.
    ws_expect_no_message_matching(&mut dm_ws, Duration::from_millis(250), |m| {
        matches!(m, ServerMessage::StagingApprovalRequired { .. })
    })
    .await;

    server.abort();
}
