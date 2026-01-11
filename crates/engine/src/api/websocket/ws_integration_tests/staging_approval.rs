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
    let mut world = wrldbldr_domain::World::new("Test World", "desc", now);
    world.id = world_id;
    world.set_time_mode(TimeMode::Manual, now);

    // Domain fixtures.
    let mut location = wrldbldr_domain::Location::new(
        world_id,
        "Test Location",
        wrldbldr_domain::LocationType::Exterior,
    );
    location.id = location_id;

    let mut region = wrldbldr_domain::Region::new(location_id, "Unstaged Region");
    region.id = region_id;

    let mut pc =
        wrldbldr_domain::PlayerCharacter::new("player-1", world_id, "PC", location_id, now);
    pc.id = pc_id;
    pc.current_region_id = None; // initial spawn; skip connection validation

    let mut visible_npc =
        wrldbldr_domain::Character::new(world_id, "Visible NPC", CampbellArchetype::Hero);
    visible_npc.id = visible_npc_id;
    let mut hidden_npc =
        wrldbldr_domain::Character::new(world_id, "Hidden NPC", CampbellArchetype::Herald);
    hidden_npc.id = hidden_npc_id;

    // World repo: serve the world for both time + visual state resolution.
    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));
    world_repo.expect_save().returning(|_world| Ok(()));

    let mut repos = TestAppRepos::new(world_repo);

    // Movement needs PC + region + location.
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

    // Unstaged region -> pending.
    repos
        .staging_repo
        .expect_get_active_staging()
        .returning(|_, _| Ok(None));

    repos
        .staging_repo
        .expect_get_staged_npcs()
        .returning(|_| Ok(vec![]));

    // Narrative triggers: keep empty so we don't need deeper narrative deps.
    repos
        .narrative_repo
        .expect_get_triggers_for_region()
        .returning(|_, _| Ok(vec![]));

    // Scene resolution: no scenes.
    repos
        .scene_repo
        .expect_get_completed_scenes()
        .returning(|_| Ok(vec![]));
    repos
        .scene_repo
        .expect_list_for_region()
        .returning(|_| Ok(vec![]));

    // Observations + flags: empty.
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

    // Items in region: empty.
    repos
        .item_repo
        .expect_list_in_region()
        .returning(|_| Ok(vec![]));

    // Settings: return defaults (default_presence_cache_ttl_hours = 3)
    repos
        .settings_repo
        .expect_get_for_world()
        .returning(|_| Ok(Some(wrldbldr_domain::AppSettings::default())));

    // Staging approval persists full per-NPC info (including hidden flags).
    let region_id_for_staging = region_id;
    let location_id_for_staging = location_id;
    let world_id_for_staging = world_id;
    let visible_npc_id_for_staging = visible_npc_id;
    let hidden_npc_id_for_staging = hidden_npc_id;
    repos
        .staging_repo
        .expect_save_pending_staging()
        .withf(move |s| {
            s.region_id == region_id_for_staging
                && s.location_id == location_id_for_staging
                && s.world_id == world_id_for_staging
                && s.ttl_hours == 24 // DM-specified TTL (overrides default from settings)
                && s.npcs.iter().any(|n| {
                    n.character_id == visible_npc_id_for_staging
                        && n.is_present
                        && !n.is_hidden_from_players
                })
                && s.npcs.iter().any(|n| {
                    n.character_id == hidden_npc_id_for_staging
                        && n.is_present
                        && n.is_hidden_from_players
                })
        })
        .returning(|_| Ok(()));

    repos
        .staging_repo
        .expect_activate_staging()
        .withf(move |_staging_id, r| *r == region_id)
        .returning(|_, _| Ok(()));

    // Character details for StagingReady payload.
    let visible_npc_for_get = visible_npc.clone();
    let hidden_npc_for_get = hidden_npc.clone();
    repos.character_repo.expect_get().returning(move |id| {
        if id == visible_npc_for_get.id {
            Ok(Some(visible_npc_for_get.clone()))
        } else if id == hidden_npc_for_get.id {
            Ok(Some(hidden_npc_for_get.clone()))
        } else {
            Ok(None)
        }
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
        pending_time_suggestions: tokio::sync::RwLock::new(HashMap::new()),
        pending_staging_requests: tokio::sync::RwLock::new(HashMap::new()),
        generation_read_state: tokio::sync::RwLock::new(HashMap::new()),
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
                wrldbldr_protocol::ApprovedNpcInfo {
                    character_id: visible_npc_id.to_string(),
                    is_present: true,
                    reasoning: None,
                    is_hidden_from_players: false,
                    mood: None,
                },
                wrldbldr_protocol::ApprovedNpcInfo {
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
            ..
        } => {
            assert_eq!(got_region_id, region_id.to_string());
            assert!(npcs_present
                .iter()
                .any(|n| n.character_id == visible_npc_id.to_string()));
            assert!(!npcs_present
                .iter()
                .any(|n| n.character_id == hidden_npc_id.to_string()));
        }
        other => panic!("expected StagingReady, got: {:?}", other),
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
    let mut world = wrldbldr_domain::World::new("Test World", "desc", now);
    world.id = world_id;
    world.set_time_mode(TimeMode::Manual, now);

    // Domain fixtures
    let mut location = wrldbldr_domain::Location::new(
        world_id,
        "Test Location",
        wrldbldr_domain::LocationType::Exterior,
    );
    location.id = location_id;

    let mut region = wrldbldr_domain::Region::new(location_id, "Test Region");
    region.id = region_id;

    // Custom settings with non-default TTL (7 hours instead of default 3)
    let mut custom_settings = wrldbldr_domain::AppSettings::default();
    custom_settings.default_presence_cache_ttl_hours = 7;

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

    // Verify that save_pending_staging is called with TTL from settings (7 hours)
    let region_id_for_staging = region_id;
    let location_id_for_staging = location_id;
    let world_id_for_staging = world_id;
    repos
        .staging_repo
        .expect_save_pending_staging()
        .withf(move |s| {
            s.region_id == region_id_for_staging
                && s.location_id == location_id_for_staging
                && s.world_id == world_id_for_staging
                && s.ttl_hours == 7 // Custom TTL from settings (not default 3)
                && s.source == wrldbldr_domain::StagingSource::AutoApproved
        })
        .returning(|_| Ok(()));

    repos
        .staging_repo
        .expect_activate_staging()
        .returning(|_, _| Ok(()));

    // Character repo: no NPCs for rule-based suggestions
    repos
        .character_repo
        .expect_get_npcs_for_region()
        .returning(|_| Ok(vec![]));

    // Location state and region state repos
    repos
        .location_state_repo
        .expect_get_active()
        .returning(|_| Ok(None));
    repos
        .region_state_repo
        .expect_get_active()
        .returning(|_| Ok(None));

    let app = build_test_app(repos, now);

    // Create a pending staging request
    let pending = crate::use_cases::staging::PendingStagingRequest {
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
    let mut world = wrldbldr_domain::World::new("Test World", "desc", now);
    world.id = world_id;
    world.set_time_mode(TimeMode::Manual, now);

    // Domain fixtures
    let mut location = wrldbldr_domain::Location::new(
        world_id,
        "Test Location",
        wrldbldr_domain::LocationType::Exterior,
    );
    location.id = location_id;

    let mut region = wrldbldr_domain::Region::new(location_id, "Test Region");
    region.id = region_id;

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
    repos
        .settings_repo
        .expect_get_for_world()
        .returning(|_| Err(crate::infrastructure::ports::RepoError::NotFound));

    // Staging repo
    repos
        .staging_repo
        .expect_get_active_staging()
        .returning(|_, _| Ok(None));
    repos
        .staging_repo
        .expect_get_staged_npcs()
        .returning(|_| Ok(vec![]));

    // Verify that save_pending_staging is called with DEFAULT TTL (3 hours)
    // since settings fetch failed and we fall back to AppSettings::default()
    let region_id_for_staging = region_id;
    let location_id_for_staging = location_id;
    let world_id_for_staging = world_id;
    repos
        .staging_repo
        .expect_save_pending_staging()
        .withf(move |s| {
            s.region_id == region_id_for_staging
                && s.location_id == location_id_for_staging
                && s.world_id == world_id_for_staging
                && s.ttl_hours == 3 // Default TTL (settings fetch failed)
                && s.source == wrldbldr_domain::StagingSource::AutoApproved
        })
        .returning(|_| Ok(()));

    repos
        .staging_repo
        .expect_activate_staging()
        .returning(|_, _| Ok(()));

    // Character repo: no NPCs
    repos
        .character_repo
        .expect_get_npcs_for_region()
        .returning(|_| Ok(vec![]));

    // Location state and region state repos
    repos
        .location_state_repo
        .expect_get_active()
        .returning(|_| Ok(None));
    repos
        .region_state_repo
        .expect_get_active()
        .returning(|_| Ok(None));

    let app = build_test_app(repos, now);

    // Create a pending staging request
    let pending = crate::use_cases::staging::PendingStagingRequest {
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
