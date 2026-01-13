use super::*;

#[tokio::test]
async fn when_dm_accepts_approval_suggestion_then_marks_complete_and_broadcasts_dialogue() {
    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let mut world = wrldbldr_domain::World::new("Test World", "desc", now);
    world.id = world_id;

    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));
    world_repo.expect_save().returning(|_world| Ok(()));

    let repos = TestAppRepos::new(world_repo);

    let queue = RecordingApprovalQueue::default();
    let queue_port: Arc<dyn QueuePort> = Arc::new(queue.clone());

    let app = build_test_app_with_ports(repos, now, queue_port, Arc::new(NoopLlm));
    let connections = Arc::new(ConnectionManager::new());

    let ws_state = Arc::new(WsState {
        app,
        connections,
        pending_time_suggestions: TimeSuggestionStoreImpl::new(),
        pending_staging_requests: PendingStagingStoreImpl::new(),
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

    // Spectator joins (receives world broadcasts).
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

    // DM receives UserJoined broadcast.
    let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::UserJoined { .. })
    })
    .await;

    // Seed an approval request.
    let approval_id = Uuid::new_v4();
    let npc_id = CharacterId::new();
    let proposed_dialogue = "Hello there".to_string();

    queue.insert_approval(
        approval_id,
        wrldbldr_domain::ApprovalRequestData {
            world_id,
            source_action_id: Uuid::new_v4(),
            decision_type: wrldbldr_domain::ApprovalDecisionType::NpcResponse,
            urgency: wrldbldr_domain::ApprovalUrgency::Normal,
            pc_id: None,
            npc_id: Some(npc_id),
            npc_name: "NPC".to_string(),
            proposed_dialogue: proposed_dialogue.clone(),
            internal_reasoning: "".to_string(),
            proposed_tools: vec![],
            retry_count: 0,
            challenge_suggestion: None,
            narrative_event_suggestion: None,
            challenge_outcome: None,
            player_dialogue: None,
            scene_id: None,
            location_id: None,
            game_time: None,
            topics: vec![],
            conversation_id: None,
        },
    );

    // DM accepts.
    ws_send_client(
        &mut dm_ws,
        &ClientMessage::ApprovalDecision {
            request_id: approval_id.to_string(),
            decision: wrldbldr_protocol::ApprovalDecision::Accept,
        },
    )
    .await;

    // DM sees ResponseApproved.
    let dm_msg = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::ResponseApproved { .. })
    })
    .await;
    match dm_msg {
        ServerMessage::ResponseApproved {
            npc_dialogue,
            executed_tools,
        } => {
            assert_eq!(npc_dialogue, proposed_dialogue);
            assert!(executed_tools.is_empty());
        }
        other => panic!("expected ResponseApproved, got: {:?}", other),
    }

    // World sees DialogueResponse.
    let world_msg = ws_expect_message(&mut spectator_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::DialogueResponse { .. })
    })
    .await;
    match world_msg {
        ServerMessage::DialogueResponse {
            speaker_id, text, ..
        } => {
            assert_eq!(speaker_id, npc_id.to_string());
            assert_eq!(text, proposed_dialogue);
        }
        other => panic!("expected DialogueResponse, got: {:?}", other),
    }

    assert!(queue.completed_contains(approval_id));
    assert!(!queue.failed_contains(approval_id));

    server.abort();
}

#[tokio::test]
async fn when_dm_rejects_approval_suggestion_then_marks_failed_and_does_not_broadcast_dialogue() {
    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let mut world = wrldbldr_domain::World::new("Test World", "desc", now);
    world.id = world_id;

    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));
    world_repo.expect_save().returning(|_world| Ok(()));

    let repos = TestAppRepos::new(world_repo);

    let queue = RecordingApprovalQueue::default();
    let queue_port: Arc<dyn QueuePort> = Arc::new(queue.clone());
    let app = build_test_app_with_ports(repos, now, queue_port, Arc::new(NoopLlm));
    let connections = Arc::new(ConnectionManager::new());

    let ws_state = Arc::new(WsState {
        app,
        connections,
        pending_time_suggestions: TimeSuggestionStoreImpl::new(),
        pending_staging_requests: PendingStagingStoreImpl::new(),
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

    // Spectator joins.
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

    // DM receives UserJoined broadcast.
    let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::UserJoined { .. })
    })
    .await;

    // Seed an approval request.
    let approval_id = Uuid::new_v4();
    let npc_id = CharacterId::new();
    queue.insert_approval(
        approval_id,
        wrldbldr_domain::ApprovalRequestData {
            world_id,
            source_action_id: Uuid::new_v4(),
            decision_type: wrldbldr_domain::ApprovalDecisionType::NpcResponse,
            urgency: wrldbldr_domain::ApprovalUrgency::Normal,
            pc_id: None,
            npc_id: Some(npc_id),
            npc_name: "NPC".to_string(),
            proposed_dialogue: "Hello".to_string(),
            internal_reasoning: "".to_string(),
            proposed_tools: vec![],
            retry_count: 0,
            challenge_suggestion: None,
            narrative_event_suggestion: None,
            challenge_outcome: None,
            player_dialogue: None,
            scene_id: None,
            location_id: None,
            game_time: None,
            topics: vec![],
            conversation_id: None,
        },
    );

    // DM rejects.
    ws_send_client(
        &mut dm_ws,
        &ClientMessage::ApprovalDecision {
            request_id: approval_id.to_string(),
            decision: wrldbldr_protocol::ApprovalDecision::Reject {
                feedback: "no".to_string(),
            },
        },
    )
    .await;

    // Ensure no DialogueResponse is broadcast.
    ws_expect_no_message_matching(&mut spectator_ws, Duration::from_millis(250), |m| {
        matches!(m, ServerMessage::DialogueResponse { .. })
    })
    .await;

    assert!(!queue.completed_contains(approval_id));
    assert!(queue.failed_contains(approval_id));

    server.abort();
}

#[tokio::test]
async fn when_dm_modifies_approval_suggestion_then_marks_complete_and_broadcasts_modified_dialogue()
{
    let now = chrono::Utc::now();

    let world_id = WorldId::new();
    let mut world = wrldbldr_domain::World::new("Test World", "desc", now);
    world.id = world_id;

    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));
    world_repo.expect_save().returning(|_world| Ok(()));

    let repos = TestAppRepos::new(world_repo);

    let queue = RecordingApprovalQueue::default();
    let queue_port: Arc<dyn QueuePort> = Arc::new(queue.clone());
    let app = build_test_app_with_ports(repos, now, queue_port, Arc::new(NoopLlm));
    let connections = Arc::new(ConnectionManager::new());

    let ws_state = Arc::new(WsState {
        app,
        connections,
        pending_time_suggestions: TimeSuggestionStoreImpl::new(),
        pending_staging_requests: PendingStagingStoreImpl::new(),
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

    // Spectator joins.
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

    // DM receives UserJoined broadcast.
    let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::UserJoined { .. })
    })
    .await;

    // Seed an approval request.
    let approval_id = Uuid::new_v4();
    let npc_id = CharacterId::new();
    queue.insert_approval(
        approval_id,
        wrldbldr_domain::ApprovalRequestData {
            world_id,
            source_action_id: Uuid::new_v4(),
            decision_type: wrldbldr_domain::ApprovalDecisionType::NpcResponse,
            urgency: wrldbldr_domain::ApprovalUrgency::Normal,
            pc_id: None,
            npc_id: Some(npc_id),
            npc_name: "NPC".to_string(),
            proposed_dialogue: "Original".to_string(),
            internal_reasoning: "".to_string(),
            proposed_tools: vec![],
            retry_count: 0,
            challenge_suggestion: None,
            narrative_event_suggestion: None,
            challenge_outcome: None,
            player_dialogue: None,
            scene_id: None,
            location_id: None,
            game_time: None,
            topics: vec![],
            conversation_id: None,
        },
    );

    let modified_dialogue = "Modified dialogue".to_string();
    let approved_tools = vec!["tool_a".to_string(), "tool_b".to_string()];

    // DM modifies.
    ws_send_client(
        &mut dm_ws,
        &ClientMessage::ApprovalDecision {
            request_id: approval_id.to_string(),
            decision: wrldbldr_protocol::ApprovalDecision::AcceptWithModification {
                modified_dialogue: modified_dialogue.clone(),
                approved_tools: approved_tools.clone(),
                rejected_tools: vec![],
                item_recipients: std::collections::HashMap::new(),
            },
        },
    )
    .await;

    let dm_msg = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::ResponseApproved { .. })
    })
    .await;
    match dm_msg {
        ServerMessage::ResponseApproved {
            npc_dialogue,
            executed_tools,
        } => {
            assert_eq!(npc_dialogue, modified_dialogue);
            assert_eq!(executed_tools, approved_tools);
        }
        other => panic!("expected ResponseApproved, got: {:?}", other),
    }

    let world_msg = ws_expect_message(&mut spectator_ws, Duration::from_secs(2), |m| {
        matches!(m, ServerMessage::DialogueResponse { .. })
    })
    .await;
    match world_msg {
        ServerMessage::DialogueResponse { text, .. } => {
            assert_eq!(text, modified_dialogue);
        }
        other => panic!("expected DialogueResponse, got: {:?}", other),
    }

    assert!(queue.completed_contains(approval_id));
    assert!(!queue.failed_contains(approval_id));

    server.abort();
}
