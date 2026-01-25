use super::*;

#[tokio::test]
async fn when_player_enters_unstaged_region_then_dm_can_approve_and_player_receives_staging_ready()
{
    use wrldbldr_domain::value_objects::CampbellArchetype;
    use wrldbldr_domain::TimeMode;

    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let location_id = LocationId::new();
    let region_id = RegionId::new();
    let pc_id = PlayerCharacterId::new();
    let visible_npc_id = CharacterId::new();
    let hidden_npc_id = CharacterId::new();

    // World (manual time, so movement doesn't generate time suggestions).
    let world_name = wrldbldr_domain::WorldName::new("Test World").unwrap();
    let mut world = wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("desc").unwrap())
        .with_id(world_id);
    let _ = world.set_time_mode(TimeMode::Manual, now);

    // Domain fixtures.
    let location_name = wrldbldr_domain::value_objects::LocationName::new("Test Location").unwrap();
    let location = wrldbldr_domain::Location::new(
        world_id,
        location_name,
        wrldbldr_domain::LocationType::Exterior,
    )
    .with_description(wrldbldr_domain::Description::new("desc").unwrap())
    .with_id(location_id);

    let region = wrldbldr_domain::Region::from_parts(
        region_id,
        location_id,
        wrldbldr_domain::value_objects::RegionName::new("Unstaged Region").unwrap(),
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
    // initial spawn - PC starts with no current_region_id, skip connection validation

    let mut visible_npc = wrldbldr_domain::Character::new(
        world_id,
        wrldbldr_domain::CharacterName::new("Visible NPC").unwrap(),
        CampbellArchetype::Hero,
    );
    visible_npc = visible_npc.with_id(visible_npc_id);
    let mut hidden_npc = wrldbldr_domain::Character::new(
        world_id,
        wrldbldr_domain::CharacterName::new("Hidden NPC").unwrap(),
        CampbellArchetype::Herald,
    );
    hidden_npc = hidden_npc.with_id(hidden_npc_id);

    // Visual state fixtures: minimal states to satisfy fail-fast validation
    let location_state_id = wrldbldr_domain::LocationStateId::new();
    let location_state = wrldbldr_domain::LocationState::from_parts(
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
    let region_state = wrldbldr_domain::RegionState::from_parts(
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

    // World repo: serve the world for both time + visual state resolution.
    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .times(0..) // Allow multiple calls
        .returning(move |_| Ok(Some(world_for_get.clone())));
    world_repo
        .expect_save()
        .times(0..) // Allow multiple calls
        .returning(|_world| Ok(()));

    let mut repos = TestAppRepos::new(world_repo);

    // Movement needs PC + region + location.
    let pc_for_get = pc.clone();
    repos
        .player_character_repo
        .expect_get()
        .times(0..)
        .returning(move |_| Ok(Some(pc_for_get.clone())));

    repos
        .player_character_repo
        .expect_get_inventory()
        .times(0..)
        .returning(|_| Ok(vec![]));

    repos
        .player_character_repo
        .expect_update_position()
        .times(0..)
        .returning(|_, _, _| Ok(()));

    let region_for_get = region.clone();
    repos
        .location_repo
        .expect_get_region()
        .times(0..)
        .returning(move |_| Ok(Some(region_for_get.clone())));

    let location_for_get = location.clone();
    repos
        .location_repo
        .expect_get_location()
        .times(0..)
        .returning(move |_| Ok(Some(location_for_get.clone())));

    repos
        .location_repo
        .expect_get_connections()
        .times(0..)
        .returning(|_, _| Ok(vec![]));

    repos
        .location_repo
        .expect_get_location_exits()
        .times(0..)
        .returning(|_, _| Ok(vec![]));

    // Unstaged region -> pending.
    repos
        .staging_repo
        .expect_get_active_staging()
        .times(0..)
        .returning(|_, _| Ok(None));

    repos
        .staging_repo
        .expect_get_staged_npcs()
        .times(0..)
        .returning(|_| Ok(vec![]));

    // Narrative triggers: keep empty so we don't need deeper narrative deps.
    repos
        .narrative_repo
        .expect_get_triggers_for_region()
        .times(0..)
        .returning(|_, _| Ok(vec![]));

    // Scene resolution: no scenes.
    repos
        .scene_repo
        .expect_get_completed_scenes()
        .times(0..)
        .returning(|_| Ok(vec![]));
    repos
        .scene_repo
        .expect_list_for_region()
        .times(0..)
        .returning(|_| Ok(vec![]));

    // Observations + flags: empty.
    repos
        .observation_repo
        .expect_get_observations()
        .times(0..)
        .returning(|_| Ok(vec![]));

    repos
        .observation_repo
        .expect_has_observed()
        .times(0..)
        .returning(|_, _| Ok(false));

    repos
        .observation_repo
        .expect_save_observation()
        .times(0..)
        .returning(|_| Ok(()));
    repos
        .flag_repo
        .expect_get_world_flags()
        .times(0..)
        .returning(|_| Box::pin(async { Ok(vec![]) }));
    repos
        .flag_repo
        .expect_get_pc_flags()
        .times(0..)
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
    // get_active is called during movement (enter_region) AND during approval
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

    // Items in region: empty.
    repos
        .item_repo
        .expect_list_in_region()
        .times(0..)
        .returning(|_| Ok(vec![]));

    // Settings: return defaults (default_presence_cache_ttl_hours = 3)
    repos
        .settings_repo
        .expect_get_for_world()
        .times(0..)
        .returning(|_| {
            Ok(Some(
                crate::infrastructure::app_settings::AppSettings::default(),
            ))
        });

    // Staging approval persists full per-NPC info (including hidden flags).
    let region_id_for_staging = region_id;
    let location_id_for_staging = location_id;
    let world_id_for_staging = world_id;
    let visible_npc_id_for_staging = visible_npc_id;
    let hidden_npc_id_for_staging = hidden_npc_id;
    let location_state_id_for_assertion = location_state_id;
    let region_state_id_for_assertion = region_state_id;
    repos
        .staging_repo
        .expect_save_and_activate_pending_staging_with_states()
        .withf(move |s, r, loc_state_id, reg_state_id| {
            s.region_id() == region_id_for_staging
                && s.location_id() == location_id_for_staging
                && s.world_id() == world_id_for_staging
                && s.ttl_hours() == 24 // DM-specified TTL (overrides default from settings)
                && s.npcs().iter().any(|n| {
                    n.character_id == visible_npc_id_for_staging
                        && n.is_present()
                        && !n.is_hidden_from_players()
                })
                && s.npcs().iter().any(|n| {
                    n.character_id == hidden_npc_id_for_staging
                        && n.is_present()
                        && n.is_hidden_from_players()
                })
                && *r == region_id_for_staging
                && loc_state_id == &Some(location_state_id_for_assertion) // Resolved from active state
                && reg_state_id == &Some(region_state_id_for_assertion) // Resolved from active state
        })
        .returning(|_, _, _, _| Ok(()));

    // Character details for StagingReady payload.
    let visible_npc_for_get = visible_npc.clone();
    let hidden_npc_for_get = hidden_npc.clone();
    repos
        .character_repo
        .expect_get()
        .times(0..) // Allow any number of calls
        .returning(move |id| {
            if id == visible_npc_for_get.id() {
                Ok(Some(visible_npc_for_get.clone()))
            } else if id == hidden_npc_for_get.id() {
                Ok(Some(hidden_npc_for_get.clone()))
            } else {
                Ok(None)
        }
    });

    repos
        .character_repo
        .expect_get_npcs_for_region()
        .times(0..)
        .returning(|_| Ok(vec![]));

    let app = build_test_app(repos, now);
    let connections = Arc::new(ConnectionManager::new());

    let ws_state = Arc::new(WsState {
        app,
        connections,
        pending_time_suggestions: Arc::new(TimeSuggestionStoreImpl::new()),
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

    // Player moves into region with no active staging.
    ws_send_client(
        &mut player_ws,
        &ClientMessage::MoveToRegion {
            pc_id: pc_id.to_string(),
            region_id: region_id.to_string(),
        },
    )
    .await;

    let _pending = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::StagingPending { .. })
    })
    .await;

    // DM gets staging approval request.
    let approval_required = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::StagingApprovalRequired { .. })
    })
    .await;

    let approval_request_id = match approval_required {
        ServerMessage::StagingApprovalRequired { request_id, .. } => request_id,
        other => panic!("expected StagingApprovalRequired, got: {:?}", other),
    };

    // DM approves: one visible NPC + one hidden NPC.
    ws_send_client(
        &mut dm_ws,
        &ClientMessage::StagingApprovalResponse {
            request_id: approval_request_id,
            approved_npcs: vec![
                wrldbldr_shared::ApprovedNpcInfo {
                    character_id: visible_npc_id.to_string(),
                    is_present: true,
                    reasoning: None,
                    is_hidden_from_players: false,
                    mood: None,
                },
                wrldbldr_shared::ApprovedNpcInfo {
                    character_id: hidden_npc_id.to_string(),
                    is_present: true,
                    reasoning: None,
                    is_hidden_from_players: true,
                    mood: None,
                },
            ],
            ttl_hours: 24,
            source: "test".to_string(),
            location_state_id: None,
            region_state_id: None,
        },
    )
    .await;

    // Player receives StagingReady broadcast, containing only visible NPC.
    let staging_ready = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::StagingReady { .. })
    })
    .await;

    match staging_ready {
        ServerMessage::StagingReady {
            region_id: got_region_id,
            npcs_present,
            visual_state: staging_visual_state,
        } => {
            assert_eq!(got_region_id, region_id.to_string());
            assert!(npcs_present
                .iter()
                .any(|n| n.character_id == visible_npc_id.to_string()));
            assert!(!npcs_present
                .iter()
                .any(|n| n.character_id == hidden_npc_id.to_string()));
            // Visual state should be None since no state IDs were provided in approval
            assert!(staging_visual_state.is_none());
        }
        other => panic!("expected StagingReady, got: {:?}", other),
    }

    // Player receives VisualStateChanged broadcast after staging approval
    let visual_state_changed = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::VisualStateChanged { .. })
    })
    .await;

    match visual_state_changed {
        ServerMessage::VisualStateChanged {
            region_id: got_region_id,
            visual_state,
        } => {
            assert_eq!(got_region_id, region_id.to_string());
            // Visual state should be None since no state IDs were provided in approval
            assert!(visual_state.is_none());
        }
        other => panic!("expected VisualStateChanged, got: {:?}", other),
    }

    server.abort();
}

/// Tests that AutoApproveStagingTimeout uses world settings for TTL
/// and falls back to defaults when settings cannot be loaded.
#[tokio::test]
async fn auto_approve_staging_timeout_uses_world_settings_for_ttl() {
    use wrldbldr_domain::TimeMode;

    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let location_id = LocationId::new();
    let region_id = RegionId::new();

    // World (manual time)
    let world_name = wrldbldr_domain::WorldName::new("Test World").unwrap();
    let mut world = wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("desc").unwrap())
        .with_id(world_id);
    let _ = world.set_time_mode(TimeMode::Manual, now);

    // Domain fixtures
    let location_name = wrldbldr_domain::value_objects::LocationName::new("Test Location").unwrap();
    let location = wrldbldr_domain::Location::new(
        world_id,
        location_name,
        wrldbldr_domain::LocationType::Exterior,
    )
        .with_description(wrldbldr_domain::Description::new("desc").unwrap())
        .with_id(location_id);

    // Visual state fixtures: minimal states to satisfy fail-fast validation
    let location_state_id = wrldbldr_domain::LocationStateId::new();
    let location_state = wrldbldr_domain::LocationState::from_parts(
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
    let region_state = wrldbldr_domain::RegionState::from_parts(
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

    let region = wrldbldr_domain::Region::from_parts(
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

    // Custom settings with non-default TTL (7 hours instead of default 3)
    let custom_settings = crate::infrastructure::app_settings::AppSettings::default()
        .with_default_presence_cache_ttl_hours(7);

    // World repo
    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));
    world_repo.expect_save().returning(|_world| Ok(()));

    let mut repos = TestAppRepos::new(world_repo);

    // Location repo
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

    // Settings repo: return custom settings with TTL of 7 hours
    let custom_settings_for_get = custom_settings.clone();
    repos
        .settings_repo
        .expect_get_for_world()
        .returning(move |_| Ok(Some(custom_settings_for_get.clone())));

    // Staging repo: no active staging, no staged NPCs
    repos
        .staging_repo
        .expect_get_active_staging()
        .returning(|_, _| Ok(None));
    repos
        .staging_repo
        .expect_get_staged_npcs()
        .returning(|_| Ok(vec![]));

    // Verify that save_and_activate_pending_staging_with_states is called with TTL from settings (7 hours)
    let region_id_for_staging = region_id;
    let location_id_for_staging = location_id;
    let world_id_for_staging = world_id;
    let location_state_id_for_assertion = location_state_id;
    let region_state_id_for_assertion = region_state_id;
    repos
        .staging_repo
        .expect_save_and_activate_pending_staging_with_states()
        .withf(move |s, r, loc_state_id, reg_state_id| {
            s.region_id() == region_id_for_staging
                && s.location_id() == location_id_for_staging
                && s.world_id() == world_id_for_staging
                && s.ttl_hours() == 7 // Custom TTL from settings (not default 3)
                && s.source() == wrldbldr_domain::StagingSource::AutoApproved
                && *r == region_id_for_staging
                && loc_state_id == &Some(location_state_id_for_assertion) // Resolved from active state
                && reg_state_id == &Some(region_state_id_for_assertion) // Resolved from active state
        })
        .returning(|_, _, _, _| Ok(()));

    // Character repo: no NPCs for rule-based suggestions
    repos
        .character_repo
        .expect_get_npcs_for_region()
        .returning(|_| Ok(vec![]));

    // Location state and region state repos: return active states to satisfy fail-fast validation
    let location_state_for_active = location_state.clone();
    let region_state_for_active = region_state.clone();
    repos
        .location_state_repo
        .expect_get_active()
        .returning(move |_| Ok(Some(location_state_for_active.clone())));
    repos
        .region_state_repo
        .expect_get_active()
        .returning(move |_| Ok(Some(region_state_for_active.clone())));
    // get() is called to build visual state response
    let location_state_for_get = location_state.clone();
    let region_state_for_get = region_state.clone();
    repos
        .location_state_repo
        .expect_get()
        .times(0..)
        .returning(move |id| {
            if id == location_state_id {
                Ok(Some(location_state_for_get.clone()))
            } else {
                Ok(None)
            }
        });
    repos
        .region_state_repo
        .expect_get()
        .times(0..)
        .returning(move |id| {
            if id == region_state_id {
                Ok(Some(region_state_for_get.clone()))
            } else {
                Ok(None)
            }
        });

    let app = build_test_app(repos, now);

    // Create a pending staging request
    let pending = crate::infrastructure::ports::PendingStagingRequest {
        region_id,
        location_id,
        world_id,
        created_at: now,
    };

    // Execute auto-approval timeout
    let result = app
        .use_cases
        .staging
        .auto_approve_timeout
        .execute("test-request-id".to_string(), pending)
        .await;

    assert!(result.is_ok(), "Auto-approval should succeed");

    let payload = result.unwrap();
    assert_eq!(payload.region_id, region_id);
    // No NPCs should be present since we didn't set up any rule-based NPCs
    assert!(payload.npcs_present.is_empty());
}

/// Tests that AutoApproveStagingTimeout falls back to default settings
/// when settings fetch fails (verifies graceful degradation).
#[tokio::test]
async fn auto_approve_staging_timeout_falls_back_to_defaults_on_settings_error() {
    use wrldbldr_domain::TimeMode;

    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let location_id = LocationId::new();
    let region_id = RegionId::new();

    // World (manual time)
    let world_name = wrldbldr_domain::WorldName::new("Test World").unwrap();
    let mut world = wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("desc").unwrap())
        .with_id(world_id);
     let _ = world.set_time_mode(TimeMode::Manual, now);

    // Domain fixtures
    let location_name = wrldbldr_domain::value_objects::LocationName::new("Test Location").unwrap();
    let location = wrldbldr_domain::Location::new(
        world_id,
        location_name,
        wrldbldr_domain::LocationType::Exterior,
    )
    .with_description(wrldbldr_domain::Description::new("desc").unwrap())
    .with_id(location_id);

    // Visual state fixtures: minimal states to satisfy fail-fast validation
    let location_state_id = wrldbldr_domain::LocationStateId::new();
    let location_state = wrldbldr_domain::LocationState::from_parts(
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
    let region_state = wrldbldr_domain::RegionState::from_parts(
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

    let region = wrldbldr_domain::Region::from_parts(
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

    // World repo
    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));
    world_repo.expect_save().returning(|_world| Ok(()));

    let mut repos = TestAppRepos::new(world_repo);

    // Location repo
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

    // Settings repo: simulate error (settings unavailable)
    repos.settings_repo.expect_get_for_world().returning(|_| {
        Err(crate::infrastructure::ports::RepoError::not_found(
            "Entity", "unknown",
        ))
    });

    // Staging repo
    repos
        .staging_repo
        .expect_get_active_staging()
        .returning(|_, _| Ok(None));
    repos
        .staging_repo
        .expect_get_staged_npcs()
        .returning(|_| Ok(vec![]));

    // Verify that save_and_activate_pending_staging_with_states is called with DEFAULT TTL (3 hours)
    // since settings fetch failed and we fall back to AppSettings::default()
    let region_id_for_staging = region_id;
    let location_id_for_staging = location_id;
    let world_id_for_staging = world_id;
    let location_state_id_for_assertion = location_state_id;
    let region_state_id_for_assertion = region_state_id;
    repos
        .staging_repo
        .expect_save_and_activate_pending_staging_with_states()
        .withf(move |s, r, loc_state_id, reg_state_id| {
            s.region_id() == region_id_for_staging
                && s.location_id() == location_id_for_staging
                && s.world_id() == world_id_for_staging
                && s.ttl_hours() == 3 // Default TTL (settings fetch failed)
                && s.source() == wrldbldr_domain::StagingSource::AutoApproved
                && *r == region_id_for_staging
                && loc_state_id == &Some(location_state_id_for_assertion) // Resolved from active state
                && reg_state_id == &Some(region_state_id_for_assertion) // Resolved from active state
        })
        .returning(|_, _, _, _| Ok(()));

    // Character repo: no NPCs
    repos
        .character_repo
        .expect_get_npcs_for_region()
        .returning(|_| Ok(vec![]));

    // Location state and region state repos: return active states to satisfy fail-fast validation
    let location_state_for_active = location_state.clone();
    let region_state_for_active = region_state.clone();
    repos
        .location_state_repo
        .expect_get_active()
        .returning(move |_| Ok(Some(location_state_for_active.clone())));
    repos
        .region_state_repo
        .expect_get_active()
        .returning(move |_| Ok(Some(region_state_for_active.clone())));
    // get() is called to build visual state response
    let location_state_for_get = location_state.clone();
    let region_state_for_get = region_state.clone();
    repos
        .location_state_repo
        .expect_get()
        .times(0..)
        .returning(move |id| {
            if id == location_state_id {
                Ok(Some(location_state_for_get.clone()))
            } else {
                Ok(None)
            }
        });
    repos
        .region_state_repo
        .expect_get()
        .times(0..)
        .returning(move |id| {
            if id == region_state_id {
                Ok(Some(region_state_for_get.clone()))
            } else {
                Ok(None)
            }
        });

    let app = build_test_app(repos, now);

    // Create a pending staging request
    let pending = crate::infrastructure::ports::PendingStagingRequest {
        region_id,
        location_id,
        world_id,
        created_at: now,
    };

    // Execute auto-approval timeout - should succeed despite settings error
    let result = app
        .use_cases
        .staging
        .auto_approve_timeout
        .execute("test-request-id".to_string(), pending)
        .await;

    // Auto-approval should succeed even when settings fetch fails
    assert!(
        result.is_ok(),
        "Auto-approval should succeed with default settings fallback"
    );
}

/// Tests that staging approval with visual state IDs broadcasts VisualStateChanged
/// with resolved visual state data in the payload
#[tokio::test]
async fn when_dm_approves_staging_with_visual_state_ids_then_broadcast_includes_resolved_state()
{
    use wrldbldr_domain::TimeMode;

    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let location_id = LocationId::new();
    let region_id = RegionId::new();
    let pc_id = PlayerCharacterId::new();

    // World (manual time)
    let world_name = wrldbldr_domain::WorldName::new("Test World").unwrap();
    let mut world = wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("desc").unwrap())
        .with_id(world_id);
    let _ = world.set_time_mode(TimeMode::Manual, now);

    // Domain fixtures
    let location_name = wrldbldr_domain::value_objects::LocationName::new("Test Location").unwrap();
    let _location = wrldbldr_domain::Location::new(
        world_id,
        location_name,
        wrldbldr_domain::LocationType::Exterior,
    )
    .with_description(wrldbldr_domain::Description::new("desc").unwrap())
    .with_id(location_id);

    let region = wrldbldr_domain::Region::from_parts(
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

    let _pc = wrldbldr_domain::PlayerCharacter::new(
        wrldbldr_domain::UserId::new("player-1").unwrap(),
        world_id,
        wrldbldr_domain::CharacterName::new("PC").unwrap(),
        location_id,
        now,
    )
    .with_id(pc_id);

    // Visual state fixtures with overrides
    let location_state_id = wrldbldr_domain::LocationStateId::new();
    let backdrop_override_path = wrldbldr_domain::AssetPath::new("/backdrop_night.jpg").ok();
    let backdrop_override = backdrop_override_path.as_ref().map(|s| s.to_string());
    let atmosphere_override_atm = wrldbldr_domain::Atmosphere::new("Atmosphere: Nighttime fog").ok();
    let atmosphere_override = atmosphere_override_atm.as_ref().map(|s| s.to_string());
    let _location_state = wrldbldr_domain::LocationState::from_parts(
        location_state_id,
        location_id,
        world_id,
        wrldbldr_domain::StateName::new("Night State").unwrap(),
        wrldbldr_domain::Description::default(),
        backdrop_override_path.clone(),
        atmosphere_override_atm.clone(),
        None, // ambient_sound
        None, // map_overlay
        vec![wrldbldr_domain::ActivationRule::Always],
        wrldbldr_domain::ActivationLogic::All,
        0,
        true, // is_default
        None,
        None,
        now,
        now,
    );

    let region_state_id = wrldbldr_domain::RegionStateId::new();
    let region_state = wrldbldr_domain::RegionState::from_parts(
        region_state_id,
        region_id,
        location_id,
        world_id,
        wrldbldr_domain::StateName::new("Battle Damaged").unwrap(),
        wrldbldr_domain::Description::default(),
        backdrop_override_path.clone(),
        atmosphere_override_atm.clone(),
        None,
        vec![wrldbldr_domain::ActivationRule::Always],
        wrldbldr_domain::ActivationLogic::All,
        0,
        true, // is_default
        None,
        None,
        now,
        now,
    );

    // Clone world for use in multiple closures
    let world_for_mock = world.clone();
    let world_for_repos = world.clone();

    let mut world_repo_for_mock = MockWorldRepo::new();
    world_repo_for_mock.expect_get().returning(move |_| Ok(Some(world_for_mock.clone())));
    world_repo_for_mock.expect_save().returning(|_world| Ok(()));
    let mut repos = TestAppRepos::new(world_repo_for_mock);

    // Setup mocks
    repos.world_repo.expect_get().returning(move |_| Ok(Some(world_for_repos.clone())));
    repos.location_repo.expect_get_region().returning(move |_| Ok(Some(region.clone())));
    repos
        .character_repo
        .expect_get()
        .times(0..)
        .returning(|_| Ok(None)); // No NPCs for this test

    // Mock repos for state resolution and saving
    let region_state_for_mock = region_state.clone();

    repos
        .region_state_repo
        .expect_get()
        .times(0..)
        .returning(move |id| {
            if id == region_state_id {
                Ok(Some(region_state_for_mock.clone()))
            } else {
                Ok(None)
            }
        });

    let app = build_test_app(repos, now);
    let connections = Arc::new(ConnectionManager::new());

    let ws_state = Arc::new(WsState {
        app,
        connections,
        pending_time_suggestions: Arc::new(TimeSuggestionStoreImpl::new()),
        pending_staging_requests: Arc::new(PendingStagingStoreImpl::new()),
        generation_read_state: GenerationStateStoreImpl::new(),
    });

    let (addr, server) = spawn_ws_server(ws_state.clone()).await;
    let mut dm_ws = ws_connect(addr).await;
    let mut player_ws = ws_connect(addr).await;

    // DM joins
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

    // Player joins with PC
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

    // DM pre-stages region with visual state IDs
    ws_send_client(
        &mut dm_ws,
        &        ClientMessage::PreStageRegion {
            region_id: region_id.to_string(),
            npcs: vec![],
            ttl_hours: 6,
            location_state_id: Some(location_state_id.to_string()),
            region_state_id: Some(region_state_id.to_string()),
        },
    )
    .await;

    // Player should receive StagingReady with visual state data
    let staging_ready = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::StagingReady { .. })
    })
    .await;

    match staging_ready {
        ServerMessage::StagingReady {
            region_id: got_region_id,
            npcs_present,
            visual_state,
        } => {
            assert_eq!(got_region_id, region_id.to_string());
            assert!(npcs_present.is_empty());
            // Verify visual state data is present and correct
            assert!(visual_state.is_some());
            let vs = visual_state.as_ref().unwrap();
            assert!(vs.location_state.is_some());
            assert!(vs.region_state.is_some());
            let loc_state = vs.location_state.as_ref().unwrap();
            assert_eq!(loc_state.id, location_state_id.to_string());
            assert_eq!(loc_state.name, "Night State");
            assert_eq!(loc_state.backdrop_override, backdrop_override);
            assert_eq!(loc_state.atmosphere_override, atmosphere_override);
            let reg_state = vs.region_state.as_ref().unwrap();
            assert_eq!(reg_state.id, region_state_id.to_string());
            assert_eq!(reg_state.name, "Battle Damaged");
            assert_eq!(reg_state.backdrop_override, backdrop_override);
            assert_eq!(reg_state.atmosphere_override, atmosphere_override);
        }
        other => panic!("expected StagingReady, got: {:?}", other),
    }

    // Player should receive VisualStateChanged with same visual state data
    let visual_state_changed = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::VisualStateChanged { .. })
    })
    .await;

    match visual_state_changed {
        ServerMessage::VisualStateChanged {
            region_id: got_region_id,
            visual_state,
        } => {
            assert_eq!(got_region_id, region_id.to_string());
            // Visual state data should match StagingReady
            assert!(visual_state.is_some());
            let vs = visual_state.as_ref().unwrap();
            assert!(vs.location_state.is_some());
            assert!(vs.region_state.is_some());
            let loc_state = vs.location_state.as_ref().unwrap();
            assert_eq!(loc_state.id, location_state_id.to_string());
            assert_eq!(loc_state.backdrop_override, backdrop_override);
            let reg_state = vs.region_state.as_ref().unwrap();
            assert_eq!(reg_state.id, region_state_id.to_string());
        }
        other => panic!("expected VisualStateChanged, got: {:?}", other),
    }

    server.abort();
}

