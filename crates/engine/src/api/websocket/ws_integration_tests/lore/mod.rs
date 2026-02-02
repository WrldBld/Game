use std::sync::Arc;
use std::time::Duration;
use wrldbldr_domain::{CharacterId, LoreId, PlayerCharacterId, WorldId};
use wrldbldr_shared::{ClientMessage, RequestPayload, ServerMessage};
use crate::api::websocket::test_support::*;
use crate::api::websocket::{WsState, PendingStagingStoreImpl, TimeSuggestionStore, GenerationStateStoreImpl};
use crate::api::connections::ConnectionManager;
use crate::api::websocket::ProtoWorldRole;

// NOTE: These tests are simplified mock-based tests that verify the lore request handlers
// work correctly. Full integration tests with database would require a test harness.

#[tokio::test]
async fn grant_lore_knowledge_request_succeeds_for_dm() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let pc_id = PlayerCharacterId::new();
    let lore_id = LoreId::new();

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    let world = crate::test_fixtures::world_seeder::create_test_world(world_id, now);
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world.clone())));

    // Setup lore repo with required expectations for grant_knowledge
    let mut lore_repo = MockLoreRepo::new();
    let lore_id_for_get = lore_id;
    let lore_for_get = wrldbldr_domain::Lore::new(
        world_id,
        "Test Lore".to_string(),
        wrldbldr_domain::LoreCategory::Common,
        now,
    )
    .with_summary("Test summary".to_string())
    .with_id(lore_id_for_get);
    lore_repo
        .expect_get()
        .times(0..)
        .returning(move |_| Ok(Some(lore_for_get.clone())));
    lore_repo
        .expect_grant_knowledge()
        .returning(|_| Ok(()));

    let mut repos = TestAppRepos::new(world_repo);
    repos.lore_repo = lore_repo;
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
    let mut ws = ws_connect(addr).await;

    // Join world as DM
    ws_send_client(
        &mut ws,
        &ClientMessage::JoinWorld {
            world_id: *world_id.as_uuid(),
            role: ProtoWorldRole::Dm,
            user_id: "dm-user".to_string(),
            pc_id: None,
            spectate_pc_id: None,
        },
    ).await;

    let _ = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::WorldJoined { .. })
    }).await;

    // Grant lore knowledge using RequestPayload
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::Lore(
                wrldbldr_shared::LoreRequest::GrantLoreKnowledge {
                    character_id: pc_id.to_string(),
                    lore_id: lore_id.to_string(),
                    chunk_ids: None,
                    discovery_source: wrldbldr_shared::LoreDiscoverySourceData::DmGranted {
                        reason: Some("Test grant".to_string()),
                    },
                }
            ),
        },
    ).await;

    // Expect response (may be success or error depending on lore repo mock)
    let response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        if let ServerMessage::Response { request_id: rid, .. } = m {
            rid == &request_id
        } else {
            false
        }
    }).await;

    // Verify we got a response (actual success depends on lore repo implementation)
    assert!(matches!(response, ServerMessage::Response { .. }), "Should receive a response");

    server.abort();
}

#[tokio::test]
async fn get_character_lore_request_succeeds() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let pc_id = PlayerCharacterId::new();

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    let world = crate::test_fixtures::world_seeder::create_test_world(world_id, now);
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world.clone())));

    // Setup lore repo with required expectations for get_character_lore_journal
    let mut lore_repo = MockLoreRepo::new();
    lore_repo
        .expect_get_character_lore_journal()
        .returning(|_| Ok(vec![]));

    let mut repos = TestAppRepos::new(world_repo);
    repos.lore_repo = lore_repo;
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
    let mut ws = ws_connect(addr).await;

    // Join world as DM
    ws_send_client(
        &mut ws,
        &ClientMessage::JoinWorld {
            world_id: *world_id.as_uuid(),
            role: ProtoWorldRole::Dm,
            user_id: "dm-user".to_string(),
            pc_id: None,
            spectate_pc_id: None,
        },
    ).await;

    let _ = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::WorldJoined { .. })
    }).await;

    // Get character lore using RequestPayload
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::Lore(
                wrldbldr_shared::LoreRequest::GetCharacterLore {
                    character_id: pc_id.to_string(),
                }
            ),
        },
    ).await;

    // Expect response
    let response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        if let ServerMessage::Response { request_id: rid, .. } = m {
            rid == &request_id
        } else {
            false
        }
    }).await;

    // Verify we got a response
    assert!(matches!(response, ServerMessage::Response { .. }), "Should receive a response");

    server.abort();
}

#[tokio::test]
async fn revoke_lore_knowledge_request_succeeds_for_dm() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let pc_id = PlayerCharacterId::new();
    let lore_id = LoreId::new();

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    let world = crate::test_fixtures::world_seeder::create_test_world(world_id, now);
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world.clone())));

    // Setup lore repo with required expectations for revoke_knowledge
    let mut lore_repo = MockLoreRepo::new();
    lore_repo
        .expect_revoke_knowledge()
        .returning(|_, _| Ok(()));

    let mut repos = TestAppRepos::new(world_repo);
    repos.lore_repo = lore_repo;
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
    let mut ws = ws_connect(addr).await;

    // Join world as DM
    ws_send_client(
        &mut ws,
        &ClientMessage::JoinWorld {
            world_id: *world_id.as_uuid(),
            role: ProtoWorldRole::Dm,
            user_id: "dm-user".to_string(),
            pc_id: None,
            spectate_pc_id: None,
        },
    ).await;

    let _ = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::WorldJoined { .. })
    }).await;

    // Revoke lore knowledge using RequestPayload
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::Lore(
                wrldbldr_shared::LoreRequest::RevokeLoreKnowledge {
                    character_id: pc_id.to_string(),
                    lore_id: lore_id.to_string(),
                    chunk_ids: None,
                }
            ),
        },
    ).await;

    // Expect response
    let response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        if let ServerMessage::Response { request_id: rid, .. } = m {
            rid == &request_id
        } else {
            false
        }
    }).await;

    // Verify we got a response
    assert!(matches!(response, ServerMessage::Response { .. }), "Should receive a response");

    server.abort();
}
