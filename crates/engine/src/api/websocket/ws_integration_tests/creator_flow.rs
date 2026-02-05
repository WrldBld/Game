use super::*;

use wrldbldr_domain::{value_objects::RuleSystemConfig, Character, CharacterId, WorldId};
use wrldbldr_shared::{
    ClientMessage, RequestPayload, ServerMessage,
};
use crate::api::websocket::ProtoWorldRole;
use wrldbldr_domain::types::{ChangeType, EntityType};

/// Create a D&D 5e world for testing.
fn create_dnd5e_world(
    world_id: WorldId,
    now: chrono::DateTime<chrono::Utc>,
) -> wrldbldr_domain::World {
    let world_name = wrldbldr_domain::WorldName::new("Test D&D 5e World").expect("test: world name should be valid");
    wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("A test world").expect("test: description should be valid"))
        .with_id(world_id)
        .with_rule_system(RuleSystemConfig::dnd_5e())
}

/// Create a test character.
fn create_test_character(
    character_id: CharacterId,
    world_id: WorldId,
    _now: chrono::DateTime<chrono::Utc>,
) -> Character {
    let name = wrldbldr_domain::CharacterName::new("Test Character").expect("test: character name should be valid");
    Character::new(world_id, name, wrldbldr_domain::CampbellArchetype::Hero)
        .with_id(character_id)
}

#[tokio::test]
async fn character_create_emits_entity_changed_broadcast() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let world = create_dnd5e_world(world_id, now);

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));

    // Setup character repo
    let mut character_repo = MockCharacterRepo::new();
    let character_id = CharacterId::new();
    let _character = create_test_character(character_id, world_id, now);

    character_repo
        .expect_save()
        .returning(move |_| Ok(()));

    character_repo
        .expect_list_in_world()
        .returning(|_, _, _| Ok(Vec::new()));

    let mut repos = TestAppRepos::new(world_repo);
    repos.character_repo = character_repo;
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

    // Create character
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::Character(wrldbldr_shared::CharacterRequest::CreateCharacter {
                world_id: world_id.to_string(),
                data: wrldbldr_shared::CreateCharacterData {
                    name: "New Character".to_string(),
                    description: Some("A test character".to_string()),
                    archetype: Some("Hero".to_string()),
                    sprite_asset: None,
                    portrait_asset: None,
                },
            }),
        },
    )
    .await;

    // Expect response and EntityChanged broadcast (order not guaranteed)
    let (response, entity_changed) =
        ws_expect_response_and_entity_changed(&mut ws, Duration::from_secs(2), &request_id).await;

    if let ServerMessage::Response { result, .. } = response {
        assert!(result.is_success(), "Character creation should succeed");
    } else {
        panic!("Expected Response message");
    }

    if let ServerMessage::EntityChanged(data) = entity_changed {
        assert_eq!(data.entity_type, EntityType::Character);
        assert_eq!(data.change_type, ChangeType::Created);
        assert!(data.data.is_some());
    } else {
        panic!("Expected EntityChanged message");
    }

    server.abort();
}

#[tokio::test]
async fn character_update_emits_entity_changed_broadcast() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let world = create_dnd5e_world(world_id, now);
    let character_id = CharacterId::new();
    let character = create_test_character(character_id, world_id, now);

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world.clone())));

    // Setup character repo
    let mut character_repo = MockCharacterRepo::new();

    character_repo
        .expect_get()
        .returning(move |_| Ok(Some(character.clone())));

    character_repo
        .expect_save()
        .returning(move |_| Ok(()));

    character_repo
        .expect_list_in_world()
        .returning(|_, _, _| Ok(Vec::new()));

    let mut repos = TestAppRepos::new(world_repo);
    repos.character_repo = character_repo;
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

    // Update character
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::Character(wrldbldr_shared::CharacterRequest::UpdateCharacter {
                character_id: character_id.to_string(),
                data: wrldbldr_shared::UpdateCharacterData {
                    name: Some("Updated Name".to_string()),
                    description: Some("Updated description".to_string()),
                    sprite_asset: None,
                    portrait_asset: None,
                    is_alive: Some(true),
                    is_active: Some(true),
                },
            }),
        },
    )
    .await;

    // Expect response and EntityChanged broadcast (order not guaranteed)
    let (_response, entity_changed) =
        ws_expect_response_and_entity_changed(&mut ws, Duration::from_secs(2), &request_id).await;

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
async fn character_delete_emits_entity_changed_broadcast() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let world = create_dnd5e_world(world_id, now);
    let character_id = CharacterId::new();
    let character = create_test_character(character_id, world_id, now);

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world.clone())));

    // Setup character repo
    let mut character_repo = MockCharacterRepo::new();

    character_repo
        .expect_get()
        .returning(move |_| Ok(Some(character.clone())));

    character_repo
        .expect_delete()
        .returning(|_| Ok(()));

    character_repo
        .expect_list_in_world()
        .returning(|_, _, _| Ok(Vec::new()));

    let mut repos = TestAppRepos::new(world_repo);
    repos.character_repo = character_repo;
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

    // Delete character
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::Character(wrldbldr_shared::CharacterRequest::DeleteCharacter {
                character_id: character_id.to_string(),
            }),
        },
    )
    .await;

    // Expect response and EntityChanged broadcast (order not guaranteed)
    let (_response, entity_changed) =
        ws_expect_response_and_entity_changed(&mut ws, Duration::from_secs(2), &request_id).await;

    if let ServerMessage::EntityChanged(data) = entity_changed {
        assert_eq!(data.entity_type, EntityType::Character);
        assert_eq!(data.change_type, ChangeType::Deleted);
        assert_eq!(data.entity_id, character_id.to_string());
    } else {
        panic!("Expected EntityChanged message");
    }

    server.abort();
}

#[tokio::test]
async fn character_change_archetype_emits_entity_changed_broadcast() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let world = create_dnd5e_world(world_id, now);
    let character_id = CharacterId::new();
    let character = create_test_character(character_id, world_id, now);

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world.clone())));

    // Setup character repo
    let mut character_repo = MockCharacterRepo::new();

    character_repo
        .expect_get()
        .returning(move |_| Ok(Some(character.clone())));

    character_repo
        .expect_save()
        .returning(|_| Ok(()));

    character_repo
        .expect_list_in_world()
        .returning(|_, _, _| Ok(Vec::new()));

    let mut repos = TestAppRepos::new(world_repo);
    repos.character_repo = character_repo;
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

    // Change archetype
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::Character(wrldbldr_shared::CharacterRequest::ChangeArchetype {
                character_id: character_id.to_string(),
                data: wrldbldr_shared::ChangeArchetypeData {
                    new_archetype: "Mentor".to_string(),
                    reason: "Character development".to_string(),
                },
            }),
        },
    )
    .await;

    // Expect response and EntityChanged broadcast (order not guaranteed)
    let (_response, entity_changed) =
        ws_expect_response_and_entity_changed(&mut ws, Duration::from_secs(2), &request_id).await;

    if let ServerMessage::EntityChanged(data) = entity_changed {
        assert_eq!(data.entity_type, EntityType::Character);
        assert_eq!(data.change_type, ChangeType::Updated);
        assert_eq!(data.entity_id, character_id.to_string());
        assert!(data.data.is_some());
    } else {
        panic!("Expected EntityChanged message");
    }

    server.abort();
}
