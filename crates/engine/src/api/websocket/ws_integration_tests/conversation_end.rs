//! WebSocket integration tests for conversation ending.
//!
//! Tests the EndConversation flow through WebSocket handlers.

use super::*;
use wrldbldr_domain::ConversationId;

#[tokio::test]
async fn when_player_ends_conversation_then_broadcasts_to_world() {
    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let world_name = wrldbldr_domain::WorldName::new("Test World").unwrap();
    let world = wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("desc").unwrap())
        .with_id(world_id);

    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));
    world_repo.expect_save().returning(|_world| Ok(()));

    let mut repos = TestAppRepos::new(world_repo);

    // Setup repos for conversation ending
    let location_id = LocationId::new();
    let _region_id = RegionId::new();
    let pc_id = PlayerCharacterId::new();
    let npc_id = CharacterId::new();
    let conversation_id = ConversationId::new();

    let pc = wrldbldr_domain::PlayerCharacter::new(
        wrldbldr_domain::UserId::new("player-1").unwrap(),
        world_id,
        wrldbldr_domain::CharacterName::new("PC").unwrap(),
        location_id,
        now,
    )
    .with_id(pc_id);

    let mut npc = wrldbldr_domain::Character::new(
        world_id,
        wrldbldr_domain::CharacterName::new("NPC").unwrap(),
        wrldbldr_domain::value_objects::CampbellArchetype::Hero,
    );
    npc = npc.with_id(npc_id);

    // PC repo
    let pc_for_get = pc.clone();
    repos
        .player_character_repo
        .expect_get()
        .times(0..)
        .returning(move |_| Ok(Some(pc_for_get.clone())));

    // Character repo
    let npc_for_get = npc.clone();
    repos
        .character_repo
        .expect_get()
        .times(0..)
        .returning(move |_| Ok(Some(npc_for_get.clone())));

    // Mock end conversation call - narrative repo ends active conversation
    repos
        .narrative_repo
        .expect_end_active_conversation()
        .times(1)
        .returning(move |p, n| {
            if p == pc_id && n == npc_id {
                Ok(Some(conversation_id))
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

    let mut player_ws = ws_connect(addr).await;
    let mut spectator_ws = ws_connect(addr).await;

    // Player joins
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

    // Spectator joins
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

    // Player sends EndConversation
    ws_send_client(
        &mut player_ws,
        &ClientMessage::EndConversation {
            npc_id: npc_id.to_string(),
            summary: Some("Goodbye".to_string()),
        },
    )
    .await;

    // Player should receive ConversationEnded response
    let player_msg = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::ConversationEnded { .. })
    })
    .await;

    match player_msg {
        ServerMessage::ConversationEnded {
            npc_id: ended_npc_id,
            npc_name,
            summary,
            conversation_id: conv_id_str,
            ..
        } => {
            assert_eq!(ended_npc_id, npc_id.to_string());
            assert_eq!(npc_name, "NPC"); // NPC name
            // The summary returned from use case, not necessarily what was sent
            assert!(summary.is_some());
            assert_eq!(conv_id_str, Some(conversation_id.to_string()));
        }
        other => panic!("expected ConversationEnded, got: {:?}", other),
    }

    // Spectator should also receive broadcasted ConversationEnded
    let spectator_msg = ws_expect_message(&mut spectator_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::ConversationEnded { .. })
    })
    .await;

    match spectator_msg {
        ServerMessage::ConversationEnded {
            npc_id: ended_npc_id,
            npc_name,
            pc_id: ended_pc_id,
            ..
        } => {
            assert_eq!(ended_npc_id, npc_id.to_string());
            assert_eq!(npc_name, "NPC");
            assert_eq!(ended_pc_id, pc_id.to_string());
        }
        other => panic!("expected ConversationEnded, got: {:?}", other),
    }

    server.abort();
}

#[tokio::test]
async fn when_dm_ends_conversation_then_returns_bad_request_error() {
    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let world_name = wrldbldr_domain::WorldName::new("Test World").unwrap();
    let world = wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("desc").unwrap())
        .with_id(world_id);

    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));
    world_repo.expect_save().returning(|_world| Ok(()));

    let mut repos = TestAppRepos::new(world_repo);

    let pc_id = PlayerCharacterId::new();
    let npc_id = CharacterId::new();
    let conversation_id = ConversationId::new();

    let pc = wrldbldr_domain::PlayerCharacter::new(
        wrldbldr_domain::UserId::new("player-1").unwrap(),
        world_id,
        wrldbldr_domain::CharacterName::new("PC").unwrap(),
        LocationId::new(),
        now,
    )
    .with_id(pc_id);

    let mut npc = wrldbldr_domain::Character::new(
        world_id,
        wrldbldr_domain::CharacterName::new("NPC").unwrap(),
        wrldbldr_domain::value_objects::CampbellArchetype::Hero,
    );
    npc = npc.with_id(npc_id);

    // PC repo
    let pc_for_get = pc.clone();
    repos
        .player_character_repo
        .expect_get()
        .times(0..)
        .returning(move |_| Ok(Some(pc_for_get.clone())));

    // Character repo
    let npc_for_get = npc.clone();
    repos
        .character_repo
        .expect_get()
        .times(0..)
        .returning(move |_| Ok(Some(npc_for_get.clone())));

    // Mock end conversation call
    repos
        .narrative_repo
        .expect_end_active_conversation()
        .times(1)
        .returning(move |p, n| {
            if p == pc_id && n == npc_id {
                Ok(Some(conversation_id))
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

    // Player joins
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

    // DM sends EndConversation but DM lacks pc_id
    ws_send_client(
        &mut dm_ws,
        &ClientMessage::EndConversation {
            npc_id: npc_id.to_string(),
            summary: Some("Ending this conversation".to_string()),
        },
    )
    .await;

    // DM should receive Error response (DM doesn't have a PC)
    let dm_msg = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::Error { .. })
    })
    .await;

    match dm_msg {
        ServerMessage::Error { code, message, .. } => {
            assert_eq!(code, "BadRequest");
            assert!(message.contains("Must have a PC to end conversation") || message.contains("Must have a PC"));
        }
        other => panic!("expected Error with BadRequest, got: {:?}", other),
    }

    server.abort();
}

#[tokio::test]
async fn when_npc_not_found_then_returns_error() {
    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let world_name = wrldbldr_domain::WorldName::new("Test World").unwrap();
    let world = wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("desc").unwrap())
        .with_id(world_id);

    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));
    world_repo.expect_save().returning(|_world| Ok(()));

    let mut repos = TestAppRepos::new(world_repo);

    let pc_id = PlayerCharacterId::new();
    let npc_id = CharacterId::new();

    let pc = wrldbldr_domain::PlayerCharacter::new(
        wrldbldr_domain::UserId::new("player-1").unwrap(),
        world_id,
        wrldbldr_domain::CharacterName::new("PC").unwrap(),
        LocationId::new(),
        now,
    )
    .with_id(pc_id);

    // PC repo
    let pc_for_get = pc.clone();
    repos
        .player_character_repo
        .expect_get()
        .times(0..)
        .returning(move |_| Ok(Some(pc_for_get.clone())));

    // Character repo - NPC not found
    repos
        .character_repo
        .expect_get()
        .times(0..)
        .returning(|_| Ok(None));

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

    let mut player_ws = ws_connect(addr).await;

    // Player joins
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

    // Player sends EndConversation for non-existent NPC
    ws_send_client(
        &mut player_ws,
        &ClientMessage::EndConversation {
            npc_id: npc_id.to_string(),
            summary: None,
        },
    )
    .await;

    // Should receive error response
    let error_msg = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::Error { .. })
    })
    .await;

    match error_msg {
        ServerMessage::Error { code, message, .. } => {
            assert_eq!(code, "NotFound");
            assert!(message.contains("NPC not found") || message.contains("not found"));
        }
        other => panic!("expected Error, got: {:?}", other),
    }

    server.abort();
}

#[tokio::test]
async fn when_pc_not_found_then_returns_error() {
    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let world_name = wrldbldr_domain::WorldName::new("Test World").unwrap();
    let world = wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("desc").unwrap())
        .with_id(world_id);

    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));
    world_repo.expect_save().returning(|_world| Ok(()));

    let mut repos = TestAppRepos::new(world_repo);

    let pc_id = PlayerCharacterId::new();

    // PC repo - PC not found
    repos
        .player_character_repo
        .expect_get()
        .times(0..)
        .returning(|_| Ok(None));

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

    let mut player_ws = ws_connect(addr).await;

    // Player joins (this will fail to bind PC, but connection succeeds)
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

    // Player sends EndConversation
    ws_send_client(
        &mut player_ws,
        &ClientMessage::EndConversation {
            npc_id: CharacterId::new().to_string(),
            summary: None,
        },
    )
    .await;

    // Should receive error response
    let error_msg = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::Error { .. })
    })
    .await;

    match error_msg {
        ServerMessage::Error { code, message, .. } => {
            assert_eq!(code, "NotFound");
            assert!(message.contains("Player character not found") || message.contains("not found"));
        }
        other => panic!("expected Error, got: {:?}", other),
    }

    server.abort();
}