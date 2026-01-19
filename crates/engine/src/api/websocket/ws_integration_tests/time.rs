use super::*;

#[tokio::test]
async fn when_dm_approves_time_suggestion_then_time_advances_and_broadcasts() {
    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let world_name = wrldbldr_domain::WorldName::new("Test World").unwrap();
    let world = wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("desc").unwrap())
        .with_id(world_id);

    // World repo mock: always returns the same world and accepts saves.
    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));

    world_repo.expect_save().returning(|_world| Ok(()));

    let repos = TestAppRepos::new(world_repo);
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
    let mut spectator_ws = ws_connect(addr).await;

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

    // Spectator joins (so we can assert broadcast reaches others too).
    ws_send_client(
        &mut spectator_ws,
        &ClientMessage::JoinWorld {
            world_id: *world_id.as_uuid(),
            role: ProtoWorldRole::Spectator,
            user_id: "spectator-user".to_string(),
            pc_id: None,
            spectate_pc_id: None,
        },
    )
    .await;

    let _ = ws_expect_message(&mut spectator_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::WorldJoined { .. })
    })
    .await;

    // DM will receive a UserJoined broadcast for the spectator.
    let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::UserJoined { .. })
    })
    .await;

    // Seed a pending time suggestion.
    let suggestion_id = Uuid::new_v4();
    let pc_id = PlayerCharacterId::new();

    let current_time = world.game_time().clone();
    let mut resulting_time = current_time.clone();
    resulting_time.advance_minutes(15);

    let suggestion = crate::use_cases::time::TimeSuggestion {
        id: suggestion_id.into(),
        world_id,
        pc_id,
        pc_name: "PC".to_string(),
        action_type: "travel_region".to_string(),
        action_description: "to somewhere".to_string(),
        suggested_minutes: 15,
        current_time: current_time.clone(),
        resulting_time: resulting_time.clone(),
        period_change: None,
    };

    ws_state
        .pending_time_suggestions
        .insert(suggestion_id, suggestion)
        .await;

    // DM approves the suggestion (no direct response; only broadcast).
    ws_send_client(
        &mut dm_ws,
        &ClientMessage::RespondToTimeSuggestion {
            suggestion_id: suggestion_id.to_string(),
            decision: wrldbldr_shared::types::TimeSuggestionDecision::Approve,
        },
    )
    .await;

    let dm_broadcast = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::GameTimeAdvanced { .. })
    })
    .await;

    let spectator_broadcast = ws_expect_message(&mut spectator_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::GameTimeAdvanced { .. })
    })
    .await;

    // Basic sanity: both received the same broadcast variant.
    assert!(matches!(
        dm_broadcast,
        ServerMessage::GameTimeAdvanced { .. }
    ));
    assert!(matches!(
        spectator_broadcast,
        ServerMessage::GameTimeAdvanced { .. }
    ));

    server.abort();
}
