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
    let mut world = wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("desc").unwrap())
        .with_id(world_id);
    let _ = world.set_time_mode(TimeMode::Manual, now);

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
        wrldbldr_domain::value_objects::RegionName::new("Region").unwrap(),
        wrldbldr_domain::Description::default(),
        None,
        None,
        None,
        false,
        0,
    );

    let pc = wrldbldr_domain::PlayerCharacter::new(
        wrldbldr_domain::UserId::new("player-1").unwrap(),
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

    // Visual state fixtures: minimal states to satisfy fail-fast validation
    let location_state_id = wrldbldr_domain::LocationStateId::new();
    let location_state = wrldbldr_domain::LocationState::from_storage(
        location_state_id,
        location_id,
        world_id,
        wrldbldr_domain::StateName::new("Test Location State").unwrap(),
        wrldbldr_domain::Description::default(),
        None, // backdrop_override
        None, // atmosphere_override
        None, // ambient_sound
        None, // map_overlay
        vec![wrldbldr_domain::ActivationRule::Always],
        wrldbldr_domain::ActivationLogic::All,
        0, // priority
        true, // is_default
        None, // generation_prompt
        None, // workflow_id
        now,
        now,
    );

    let region_state_id = wrldbldr_domain::RegionStateId::new();
    let region_state = wrldbldr_domain::RegionState::from_storage(
        region_state_id,
        region_id,
        location_id,
        world_id,
        wrldbldr_domain::StateName::new("Test Region State").unwrap(),
        wrldbldr_domain::Description::default(),
        None, // backdrop_override
        None, // atmosphere_override
        None, // ambient_sound
        vec![wrldbldr_domain::ActivationRule::Always],
        wrldbldr_domain::ActivationLogic::All,
        0, // priority
        true, // is_default
        None, // generation_prompt
        None, // workflow_id
        now,
        now,
    );

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
        .returning(|_, _| Ok(vec![]));

    repos
        .location_repo
        .expect_get_location_exits()
        .returning(|_, _| Ok(vec![]));

    repos
        .location_repo
        .expect_get_region_exits()
        .returning(|_, _| Ok(vec![]));

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

    // Visual state resolution: return active states to satisfy fail-fast validation
    let location_state_for_list = vec![location_state.clone()];
    let location_state_for_active = location_state.clone();
    let region_state_for_list = vec![region_state.clone()];
    let region_state_for_active = region_state.clone();

    repos
        .location_state_repo
        .expect_list_for_location()
        .times(0..)
        .returning(move |_| Ok(location_state_for_list.clone()));
    repos
        .region_state_repo
        .expect_list_for_region()
        .times(0..)
        .returning(move |_| Ok(region_state_for_list.clone()));
    repos
        .location_state_repo
        .expect_get_active()
        .times(0..)
        .returning(move |_| Ok(Some(location_state_for_active.clone())));
    repos
        .region_state_repo
        .expect_get_active()
        .times(0..)
        .returning(move |_| Ok(Some(region_state_for_active.clone())));
    // get() is called to build visual state response
    let location_state_for_get = location_state.clone();
    let location_state_id_for_get = location_state_id;
    repos
        .location_state_repo
        .expect_get()
        .times(0..)
        .returning(move |id| {
            if id == location_state_id_for_get {
                Ok(Some(location_state_for_get.clone()))
            } else {
                Ok(None)
            }
        });
    let region_state_for_get = region_state.clone();
    let region_state_id_for_get = region_state_id;
    repos
        .region_state_repo
        .expect_get()
        .times(0..)
        .returning(move |id| {
            if id == region_state_id_for_get {
                Ok(Some(region_state_for_get.clone()))
            } else {
                Ok(None)
            }
        });

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
        active: Option<wrldbldr_domain::Staging>,
    }

    let shared = Arc::new(Mutex::new(SharedStaging::default()));

    let shared_for_save_and_activate = shared.clone();
    let location_state_id_for_assertion = location_state_id;
    let region_state_id_for_assertion = region_state_id;
    repos
        .staging_repo
        .expect_save_and_activate_pending_staging_with_states()
        .withf(move |s, r, loc_state_id, reg_state_id| {
            *r == region_id
                && loc_state_id == &Some(location_state_id_for_assertion)
                && reg_state_id == &Some(region_state_id_for_assertion)
        })
        .returning(move |s, _region, _loc_state_id, _reg_state_id| {
            let mut guard = shared_for_save_and_activate.lock().unwrap();
            guard.active = Some(s.clone());
            Ok(())
        });

    let shared_for_get_active = shared.clone();
    repos
        .staging_repo
        .expect_get_active_staging()
        .returning(move |rid, _now| {
            let guard = shared_for_get_active.lock().unwrap();
            Ok(guard.active.clone().filter(|s| s.region_id() == rid))
        });

    repos
        .staging_repo
        .expect_get_staged_npcs()
        .returning(|_| Ok(vec![]));

    // Settings: return defaults
    repos.settings_repo.expect_get_for_world().returning(|_| {
        Ok(Some(
            crate::infrastructure::app_settings::AppSettings::default(),
        ))
    });

    repos
        .character_repo
        .expect_get_npcs_for_region()
        .returning(|_| Ok(vec![]));

    let app = build_test_app(repos, now);
    let connections = Arc::new(ConnectionManager::new());

    let ws_state = Arc::new(WsState {
        app,
        connections,
        pending_time_suggestions: Arc::new(TimeSuggestionStore::new()),
        pending_staging_requests: Arc::new(PendingStagingStoreImpl::new()),
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
            npcs: vec![wrldbldr_shared::ApprovedNpcInfo {
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
