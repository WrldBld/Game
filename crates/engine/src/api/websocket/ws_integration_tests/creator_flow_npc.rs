use super::*;

use wrldbldr_domain::{
    Character, CharacterId, PlayerCharacterId, RegionId, WorldId, DispositionLevel,
    RelationshipLevel, MoodState,
};
use wrldbldr_shared::{
    ClientMessage, ErrorCode, ProtoWorldRole, RequestPayload, ResponseResult, ServerMessage,
};
use wrldbldr_domain::types::{ChangeType, EntityType};

/// Create a test world.
fn create_test_world(
    world_id: WorldId,
    now: chrono::DateTime<chrono::Utc>,
) -> wrldbldr_domain::World {
    let world_name = wrldbldr_domain::WorldName::new("Test World").unwrap();
    wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("A test world").unwrap())
        .with_id(world_id)
}

/// Create a test NPC.
fn create_test_npc(
    npc_id: CharacterId,
    world_id: WorldId,
    now: chrono::DateTime<chrono::Utc>,
) -> Character {
    let name = wrldbldr_domain::CharacterName::new("Test NPC").unwrap();
    Character::new(world_id, name, wrldbldr_domain::CampbellArchetype::Hero)
        .with_id(npc_id)
        .with_created_at(now)
}

/// Create a test PC.
fn create_test_pc(
    pc_id: PlayerCharacterId,
    world_id: WorldId,
    now: chrono::DateTime<chrono::Utc>,
) -> wrldbldr_domain::PlayerCharacter {
    let name = wrldbldr_domain::CharacterName::new("Test PC").unwrap();
    wrldbldr_domain::PlayerCharacter::new(world_id, name)
        .with_id(pc_id)
        .with_created_at(now)
}

#[tokio::test]
async fn npc_set_disposition_emits_entity_changed_broadcast() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let world = create_test_world(world_id, now);
    let npc_id = CharacterId::new();
    let npc = create_test_npc(npc_id, world_id, now);
    let pc_id = PlayerCharacterId::new();
    let pc = create_test_pc(pc_id, world_id, now);

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world.clone())));

    // Setup character repo
    let mut character_repo = MockCharacterRepo::new();
    character_repo
        .expect_get()
        .returning(move |_| Ok(Some(npc.clone())));

    character_repo
        .expect_set_disposition()
        .returning(move |_, _, _, _| {
            Ok(wrldbldr_domain::DispositionUpdate {
                npc_id: npc_id,
                npc_name: "Test NPC".to_string(),
                pc_id: pc_id,
                pc_name: "Test PC".to_string(),
                disposition: DispositionLevel::Friendly,
                relationship: RelationshipLevel::Allied,
                sentiment: 0.8,
                reason: Some("Test reason".to_string()),
            })
        });

    character_repo
        .expect_list_in_world()
        .returning(|_, _, _| Ok(Vec::new()));

    // Setup player character repo
    let mut pc_repo = MockPlayerCharacterRepo::new();
    pc_repo
        .expect_get()
        .returning(move |_| Ok(Some(pc.clone())));

    let repos = TestAppRepos::new(world_repo);
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
            user_id: "test-user".to_string(),
            pc_id: None,
            spectate_pc_id: None,
        },
    )
    .await;

    let _ = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::WorldJoined { .. })
    })
    .await;

    // Set NPC disposition
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::Npc(wrldbldr_shared::NpcRequest::SetNpcDisposition {
                npc_id: npc_id.to_string(),
                pc_id: pc_id.to_string(),
                disposition: DispositionLevel::Friendly,
                reason: Some("Test reason".to_string()),
            }),
        },
    )
    .await;

    // Expect response
    let _response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        if let ServerMessage::Response {
            request_id: rid, ..
        } = m
        {
            rid == &request_id
        } else {
            false
        }
    })
    .await;

    // Expect EntityChanged broadcast
    let entity_changed = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::EntityChanged { .. })
    })
    .await;

    if let ServerMessage::EntityChanged(data) = entity_changed {
        assert_eq!(data.entity_type, EntityType::Relationship);
        assert_eq!(data.change_type, ChangeType::Updated);
        assert!(data.data.is_some());
    } else {
        panic!("Expected EntityChanged message");
    }

    server.abort();
}

#[tokio::test]
async fn npc_set_home_region_emits_entity_changed_broadcast() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let world = create_test_world(world_id, now);
    let npc_id = CharacterId::new();
    let npc = create_test_npc(npc_id, world_id, now);
    let region_id = RegionId::new();

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world.clone())));

    // Setup character repo
    let mut character_repo = MockCharacterRepo::new();
    character_repo
        .expect_get()
        .returning(move |_| Ok(Some(npc.clone())));

    character_repo
        .expect_set_home_region()
        .returning(|_, _| Ok(()));

    character_repo
        .expect_list_in_world()
        .returning(|_, _, _| Ok(Vec::new()));

    let repos = TestAppRepos::new(world_repo);
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
            user_id: "test-user".to_string(),
            pc_id: None,
            spectate_pc_id: None,
        },
    )
    .await;

    let _ = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::WorldJoined { .. })
    })
    .await;

    // Set NPC home region
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::Npc(wrldbldr_shared::NpcRequest::SetCharacterHomeRegion {
                character_id: npc_id.to_string(),
                region_id: region_id.to_string(),
            }),
        },
    )
    .await;

    // Expect response
    let _response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        if let ServerMessage::Response {
            request_id: rid, ..
        } = m
        {
            rid == &request_id
        } else {
            false
        }
    })
    .await;

    // Expect EntityChanged broadcast
    let entity_changed = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::EntityChanged { .. })
    })
    .await;

    if let ServerMessage::EntityChanged(data) = entity_changed {
        assert_eq!(data.entity_type, EntityType::Relationship);
        assert_eq!(data.change_type, ChangeType::Updated);
        assert!(data.data.is_some());
    } else {
        panic!("Expected EntityChanged message");
    }

    server.abort();
}

#[tokio::test]
async fn npc_set_work_region_emits_entity_changed_broadcast() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let world = create_test_world(world_id, now);
    let npc_id = CharacterId::new();
    let npc = create_test_npc(npc_id, world_id, now);
    let region_id = RegionId::new();

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world.clone())));

    // Setup character repo
    let mut character_repo = MockCharacterRepo::new();
    character_repo
        .expect_get()
        .returning(move |_| Ok(Some(npc.clone())));

    character_repo
        .expect_set_work_region()
        .returning(|_, _| Ok(()));

    character_repo
        .expect_list_in_world()
        .returning(|_, _, _| Ok(Vec::new()));

    let repos = TestAppRepos::new(world_repo);
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
            user_id: "test-user".to_string(),
            pc_id: None,
            spectate_pc_id: None,
        },
    )
    .await;

    let _ = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::WorldJoined { .. })
    })
    .await;

    // Set NPC work region
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::Npc(wrldbldr_shared::NpcRequest::SetCharacterWorkRegion {
                character_id: npc_id.to_string(),
                region_id: region_id.to_string(),
            }),
        },
    )
    .await;

    // Expect response
    let _response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        if let ServerMessage::Response {
            request_id: rid, ..
        } = m
        {
            rid == &request_id
        } else {
            false
        }
    })
    .await;

    // Expect EntityChanged broadcast
    let entity_changed = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::EntityChanged { .. })
    })
    .await;

    if let ServerMessage::EntityChanged(data) = entity_changed {
        assert_eq!(data.entity_type, EntityType::Relationship);
        assert_eq!(data.change_type, ChangeType::Updated);
        assert!(data.data.is_some());
    } else {
        panic!("Expected EntityChanged message");
    }

    server.abort();
}

#[tokio::test]
async fn npc_remove_region_relationship_emits_entity_changed_broadcast() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let world = create_test_world(world_id, now);
    let npc_id = CharacterId::new();
    let npc = create_test_npc(npc_id, world_id, now);
    let region_id = RegionId::new();

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world.clone())));

    // Setup character repo
    let mut character_repo = MockCharacterRepo::new();
    character_repo
        .expect_get()
        .returning(move |_| Ok(Some(npc.clone())));

    character_repo
        .expect_remove_region_relationship()
        .returning(|_, _, _| Ok(()));

    character_repo
        .expect_list_in_world()
        .returning(|_, _, _| Ok(Vec::new()));

    let repos = TestAppRepos::new(world_repo);
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
            user_id: "test-user".to_string(),
            pc_id: None,
            spectate_pc_id: None,
        },
    )
    .await;

    let _ = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::WorldJoined { .. })
    })
    .await;

    // Remove NPC region relationship
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::Npc(wrldbldr_shared::NpcRequest::RemoveCharacterRegionRelationship {
                character_id: npc_id.to_string(),
                region_id: region_id.to_string(),
                relationship_type: "HOME_REGION".to_string(),
            }),
        },
    )
    .await;

    // Expect response
    let _response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        if let ServerMessage::Response {
            request_id: rid, ..
        } = m
        {
            rid == &request_id
        } else {
            false
        }
    })
    .await;

    // Expect EntityChanged broadcast
    let entity_changed = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::EntityChanged { .. })
    })
    .await;

    if let ServerMessage::EntityChanged(data) = entity_changed {
        assert_eq!(data.entity_type, EntityType::Relationship);
        assert_eq!(data.change_type, ChangeType::Deleted);
    } else {
        panic!("Expected EntityChanged message");
    }

    server.abort();
}

    })
    .await;

    // Expect EntityChanged broadcast
    let entity_changed = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::EntityChanged { .. })
    })
    .await;

    if let ServerMessage::EntityChanged(data) = entity_changed {
        assert_eq!(data.entity_type, EntityType::Character);
        assert_eq!(data.change_type, ChangeType::Updated);
        assert!(data.data.is_some());
    } else {
        panic!("Expected EntityChanged message");
    }

    server.abort();
}

#[tokio::test]
async fn npc_set_home_region_requires_dm() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let world = create_test_world(world_id, now);
    let npc_id = CharacterId::new();
    let npc = create_test_npc(npc_id, world_id, now);
    let region_id = RegionId::new();

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world.clone())));

    // Setup character repo
    let mut character_repo = MockCharacterRepo::new();
    character_repo
        .expect_get()
        .returning(move |_| Ok(Some(npc.clone())));

    character_repo
        .expect_set_home_region()
        .returning(|_, _| Ok(()));

    character_repo
        .expect_list_in_world()
        .returning(|_, _, _| Ok(Vec::new()));

    let repos = TestAppRepos::new(world_repo);
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

    // Join world as Player (not DM)
    ws_send_client(
        &mut ws,
        &ClientMessage::JoinWorld {
            world_id: *world_id.as_uuid(),
            role: ProtoWorldRole::Player,
            user_id: "test-user".to_string(),
            pc_id: None,
            spectate_pc_id: None,
        },
    )
    .await;

    let _ = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::WorldJoined { .. })
    })
    .await;

    // Try to set NPC home region as Player
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::Npc(wrldbldr_shared::NpcRequest::SetCharacterHomeRegion {
                character_id: npc_id.to_string(),
                region_id: region_id.to_string(),
            }),
        },
    )
    .await;

    // Expect unauthorized error
    let response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        if let ServerMessage::Response {
            request_id: rid, ..
        } = m
        {
            rid == &request_id
        } else {
            false
        }
    })
    .await;

    if let ServerMessage::Response { result, .. } = response {
        match result {
            ResponseResult::Error { code, message, .. } => {
                assert_eq!(code, ErrorCode::Unauthorized);
                assert!(message.contains("Only DMs"));
            }
            _ => panic!("Expected error response"),
        }
    } else {
        panic!("Expected Response message");
    }

    server.abort();
}
