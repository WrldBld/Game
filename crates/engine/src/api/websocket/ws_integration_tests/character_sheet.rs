use super::*;

use wrldbldr_domain::{value_objects::RuleSystemConfig, WorldId};
use wrldbldr_protocol::{ClientMessage, ErrorCode, RequestPayload, ResponseResult, ServerMessage, WorldRequest};

/// Create a D&D 5e world for testing.
fn create_dnd5e_world(world_id: WorldId, now: chrono::DateTime<chrono::Utc>) -> wrldbldr_domain::World {
    let mut world = wrldbldr_domain::World::new("Test D&D 5e World", "A test world", now);
    world.id = world_id;
    world.rule_system = RuleSystemConfig::dnd_5e();
    world
}

#[tokio::test]
async fn get_sheet_template_returns_dnd5e_schema_for_dnd5e_world() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();

    let world = create_dnd5e_world(world_id, now);

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));

    let repos = TestAppRepos::new(world_repo);
    let app = build_test_app(repos, now);
    let connections = Arc::new(ConnectionManager::new());

    let ws_state = Arc::new(WsState {
        app,
        connections,
        pending_time_suggestions: TimeSuggestionStoreImpl::new(),
        pending_staging_requests: PendingStagingStoreImpl::new(),
        generation_read_state: tokio::sync::RwLock::new(std::collections::HashMap::new()),
    });

    let (addr, server) = spawn_ws_server(ws_state.clone()).await;
    let mut ws = ws_connect(addr).await;

    // Join world first
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

    // Request sheet template
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::World(WorldRequest::GetSheetTemplate {
                world_id: world_id.as_uuid().to_string(),
            }),
        },
    )
    .await;

    // Expect response with schema
    let response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        if let ServerMessage::Response { request_id: rid, .. } = m {
            rid == &request_id
        } else {
            false
        }
    })
    .await;

    if let ServerMessage::Response { result, .. } = response {
        match result {
            ResponseResult::Success { data } => {
                let data = data.expect("Should have data");

                let system_id = data.get("systemId").and_then(|v: &serde_json::Value| v.as_str());
                assert_eq!(system_id, Some("dnd5e"), "Should return D&D 5e schema");

                // Verify sections exist
                let sections = data.get("sections").and_then(|v: &serde_json::Value| v.as_array());
                assert!(sections.is_some(), "Schema should have sections");
                let sections = sections.unwrap();

                // Check for expected sections
                let section_ids: Vec<&str> = sections
                    .iter()
                    .filter_map(|s| s.get("id").and_then(|v: &serde_json::Value| v.as_str()))
                    .collect();
                assert!(section_ids.contains(&"identity"), "Should have identity section");
                assert!(section_ids.contains(&"ability_scores"), "Should have ability_scores section");
                assert!(section_ids.contains(&"skills"), "Should have skills section");

                // Verify creation steps exist
                let creation_steps = data.get("creationSteps").and_then(|v: &serde_json::Value| v.as_array());
                assert!(creation_steps.is_some(), "Schema should have creation steps");
            }
            ResponseResult::Error { code, message, .. } => {
                panic!("Expected success, got error: {:?} - {}", code, message);
            }
            _ => panic!("Unexpected response result"),
        }
    } else {
        panic!("Expected Response message");
    }

    server.abort();
}

#[tokio::test]
async fn get_sheet_template_returns_error_for_nonexistent_world() {
    let now = chrono::Utc::now();

    // Setup world repo to return None
    let mut world_repo = MockWorldRepo::new();
    world_repo.expect_get().returning(move |_| Ok(None));

    let repos = TestAppRepos::new(world_repo);
    let app = build_test_app(repos, now);
    let connections = Arc::new(ConnectionManager::new());

    let ws_state = Arc::new(WsState {
        app,
        connections,
        pending_time_suggestions: TimeSuggestionStoreImpl::new(),
        pending_staging_requests: PendingStagingStoreImpl::new(),
        generation_read_state: tokio::sync::RwLock::new(std::collections::HashMap::new()),
    });

    let (addr, server) = spawn_ws_server(ws_state.clone()).await;
    let mut ws = ws_connect(addr).await;

    // Request sheet template for non-existent world
    let nonexistent_world_id = WorldId::new();
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::World(WorldRequest::GetSheetTemplate {
                world_id: nonexistent_world_id.as_uuid().to_string(),
            }),
        },
    )
    .await;

    // Expect error response
    let response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        if let ServerMessage::Response { request_id: rid, .. } = m {
            rid == &request_id
        } else {
            false
        }
    })
    .await;

    if let ServerMessage::Response { result, .. } = response {
        match result {
            ResponseResult::Error { code, .. } => {
                assert_eq!(code, ErrorCode::NotFound, "Should return NotFound error");
            }
            ResponseResult::Success { .. } => {
                panic!("Expected error, got success");
            }
            _ => panic!("Unexpected response result"),
        }
    } else {
        panic!("Expected Response message");
    }

    server.abort();
}

#[tokio::test]
async fn get_sheet_template_schema_has_ability_score_validation() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();

    let world = create_dnd5e_world(world_id, now);

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));

    let repos = TestAppRepos::new(world_repo);
    let app = build_test_app(repos, now);
    let connections = Arc::new(ConnectionManager::new());

    let ws_state = Arc::new(WsState {
        app,
        connections,
        pending_time_suggestions: TimeSuggestionStoreImpl::new(),
        pending_staging_requests: PendingStagingStoreImpl::new(),
        generation_read_state: tokio::sync::RwLock::new(std::collections::HashMap::new()),
    });

    let (addr, server) = spawn_ws_server(ws_state.clone()).await;
    let mut ws = ws_connect(addr).await;

    // Join world first
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

    // Request sheet template
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::World(WorldRequest::GetSheetTemplate {
                world_id: world_id.as_uuid().to_string(),
            }),
        },
    )
    .await;

    let response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        if let ServerMessage::Response { request_id: rid, .. } = m {
            rid == &request_id
        } else {
            false
        }
    })
    .await;

    if let ServerMessage::Response { result, .. } = response {
        match result {
            ResponseResult::Success { data } => {
                let data = data.expect("Should have data");

                // Find ability_scores section
                let sections = data.get("sections").and_then(|v: &serde_json::Value| v.as_array()).unwrap();
                let ability_section = sections
                    .iter()
                    .find(|s| s.get("id").and_then(|v: &serde_json::Value| v.as_str()) == Some("ability_scores"))
                    .expect("Should have ability_scores section");

                // Check fields have validation
                let fields = ability_section.get("fields").and_then(|v: &serde_json::Value| v.as_array()).unwrap();
                let str_field = fields
                    .iter()
                    .find(|f| f.get("id").and_then(|v: &serde_json::Value| v.as_str()) == Some("STR"))
                    .expect("Should have STR field");

                // Verify STR has validation rules
                let validation = str_field.get("validation");
                assert!(validation.is_some(), "STR should have validation rules");

                let validation = validation.unwrap();
                let min = validation.get("min").and_then(|v: &serde_json::Value| v.as_i64());
                let max = validation.get("max").and_then(|v: &serde_json::Value| v.as_i64());

                assert_eq!(min, Some(1), "Min should be 1");
                assert_eq!(max, Some(30), "Max should be 30");
            }
            ResponseResult::Error { code, message, .. } => {
                panic!("Expected success, got error: {:?} - {}", code, message);
            }
            _ => panic!("Unexpected response result"),
        }
    } else {
        panic!("Expected Response message");
    }

    server.abort();
}

#[tokio::test]
async fn get_sheet_template_schema_includes_creation_steps() {
    let now = chrono::Utc::now();
    let world_id = WorldId::new();

    let world = create_dnd5e_world(world_id, now);

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));

    let repos = TestAppRepos::new(world_repo);
    let app = build_test_app(repos, now);
    let connections = Arc::new(ConnectionManager::new());

    let ws_state = Arc::new(WsState {
        app,
        connections,
        pending_time_suggestions: TimeSuggestionStoreImpl::new(),
        pending_staging_requests: PendingStagingStoreImpl::new(),
        generation_read_state: tokio::sync::RwLock::new(std::collections::HashMap::new()),
    });

    let (addr, server) = spawn_ws_server(ws_state.clone()).await;
    let mut ws = ws_connect(addr).await;

    // Join world first
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

    // Request sheet template
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::World(WorldRequest::GetSheetTemplate {
                world_id: world_id.as_uuid().to_string(),
            }),
        },
    )
    .await;

    let response = ws_expect_message(&mut ws, Duration::from_secs(2), |m| {
        if let ServerMessage::Response { request_id: rid, .. } = m {
            rid == &request_id
        } else {
            false
        }
    })
    .await;

    if let ServerMessage::Response { result, .. } = response {
        match result {
            ResponseResult::Success { data } => {
                let data = data.expect("Should have data");

                // Verify creation steps
                let creation_steps = data
                    .get("creationSteps")
                    .and_then(|v: &serde_json::Value| v.as_array())
                    .expect("Should have creation steps");

                assert!(!creation_steps.is_empty(), "Should have at least one creation step");

                // Check first step is identity
                let first_step = &creation_steps[0];
                assert_eq!(
                    first_step.get("id").and_then(|v: &serde_json::Value| v.as_str()),
                    Some("identity"),
                    "First step should be identity"
                );
                assert_eq!(
                    first_step.get("order").and_then(|v: &serde_json::Value| v.as_i64()),
                    Some(1),
                    "Identity should be order 1"
                );

                // Check second step is abilities
                let second_step = &creation_steps[1];
                assert_eq!(
                    second_step.get("id").and_then(|v: &serde_json::Value| v.as_str()),
                    Some("abilities"),
                    "Second step should be abilities"
                );
                assert_eq!(
                    second_step.get("order").and_then(|v: &serde_json::Value| v.as_i64()),
                    Some(2),
                    "Abilities should be order 2"
                );

                // Verify steps are in order
                let mut prev_order = 0i64;
                for step in creation_steps {
                    let order = step.get("order").and_then(|v: &serde_json::Value| v.as_i64()).unwrap_or(0);
                    assert!(order > prev_order, "Steps should be in order");
                    prev_order = order;
                }
            }
            ResponseResult::Error { code, message, .. } => {
                panic!("Expected success, got error: {:?} - {}", code, message);
            }
            _ => panic!("Unexpected response result"),
        }
    } else {
        panic!("Expected Response message");
    }

    server.abort();
}
