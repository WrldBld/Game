use super::*;

use wrldbldr_domain::{value_objects::RuleSystemConfig, Character, CharacterId, WorldId};
use wrldbldr_shared::{
    ClientMessage, ErrorCode, ProtoWorldRole, RequestPayload, ResponseResult, ServerMessage,
};
use wrldbldr_domain::types::{ChangeType, EntityType};

/// Create a D&D 5e world for testing.
fn create_dnd5e_world(
    world_id: WorldId,
    now: chrono::DateTime<chrono::Utc>,
) -> wrldbldr_domain::World {
    let world_name = wrldbldr_domain::WorldName::new("Test D&D 5e World").unwrap();
    wrldbldr_domain::World::new(world_name, now)
        .with_description(wrldbldr_domain::Description::new("A test world").unwrap())
        .with_id(world_id)
        .with_rule_system(RuleSystemConfig::dnd_5e())
}

/// Create a test character.
fn create_test_character(
    character_id: CharacterId,
    world_id: WorldId,
    now: chrono::DateTime<chrono::Utc>,
) -> Character {
    let name = wrldbldr_domain::CharacterName::new("Test Character").unwrap();
    Character::new(world_id, name, wrldbldr_domain::CampbellArchetype::Hero)
        .with_id(character_id)
        .with_created_at(now)
}

#[tokio::test]
async fn character_sheet_update_field_emits_entity_changed() {
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
    // update_field internally calls get() once, and we also call it for world_id resolution
    let character_clone = character.clone();
    character_repo
        .expect_get()
        .times(2)
        .returning(move |_| Ok(Some(character_clone.clone())));

    character_repo
        .expect_update_sheet_value()
        .returning(|_, _, _| Ok(Some(10)));

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

    // Update character sheet field
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::CharacterSheet(
                wrldbldr_shared::CharacterSheetRequest::UpdateField {
                    character_id: character_id.to_string(),
                    field_id: "strength".to_string(),
                    value: 18,
                },
            ),
        },
    )
    .await;

    // Expect response
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
        assert!(result.is_success(), "Field update should succeed");
    } else {
        panic!("Expected Response message");
    }

    // Expect EntityChanged broadcast
    let entity_changed = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::EntityChanged { .. })
    })
    .await;

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

#[tokio::test]
async fn character_sheet_update_fields_emits_entity_changed() {
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
    // update_fields internally calls get() once, and we also call it for world_id resolution
    let character_clone = character.clone();
    character_repo
        .expect_get()
        .times(2)
        .returning(move |_| Ok(Some(character_clone.clone())));

    character_repo
        .expect_update_sheet_values()
        .returning(|_, _| Ok(vec![]));

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

    // Update multiple character sheet fields
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::CharacterSheet(
                wrldbldr_shared::CharacterSheetRequest::UpdateFields {
                    character_id: character_id.to_string(),
                    updates: vec![
                        wrldbldr_shared::UpdateFieldData {
                            field_id: "strength".to_string(),
                            value: 18,
                        },
                        wrldbldr_shared::UpdateFieldData {
                            field_id: "dexterity".to_string(),
                            value: 16,
                        },
                    ],
                },
            ),
        },
    )
    .await;

    // Expect response
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
        assert!(result.is_success(), "Fields update should succeed");
    } else {
        panic!("Expected Response message");
    }

    // Expect EntityChanged broadcast
    let entity_changed = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::EntityChanged { .. })
    })
    .await;

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
