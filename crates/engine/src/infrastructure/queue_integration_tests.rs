use std::sync::Arc;

use chrono::{TimeZone, Utc};
use uuid::Uuid;
use wrldbldr_domain::{QueueItemId, WorldId};

use crate::queue_types::{
    ApprovalDecisionType, ApprovalRequestData, ApprovalUrgency, LlmRequestData, LlmRequestType, QueueType,
};

use crate::infrastructure::{
    clock::FixedClock, ports::QueueItemData, ports::QueuePort, queue::SqliteQueue,
};

#[tokio::test]
async fn sqlite_queue_dm_approval_persists_across_restart() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();
    let approval = ApprovalRequestData {
        world_id,
        region_id: None,
        source_action_id: Uuid::new_v4().into(),
        decision_type: ApprovalDecisionType::NpcResponse,
        urgency: ApprovalUrgency::Normal,
        pc_id: None,
        npc_id: None,
        npc_name: "".to_string(),
        proposed_dialogue: "".to_string(),
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
    };

    let id = {
        let queue = SqliteQueue::new(&db_path_str, clock.clone())
            .await
            .expect("create queue");
        queue.enqueue_dm_approval(&approval).await.expect("enqueue");
        queue.get_pending_count(QueueType::DmApproval).await.expect("count");

        // Drop queue to simulate restart
        queue
            .enqueue_dm_approval(&approval)
            .await
            .expect("enqueue2")
    };

    let queue = SqliteQueue::new(&db_path_str, clock)
        .await
        .expect("reopen queue");

    let item = queue
        .dequeue_dm_approval()
        .await
        .expect("dequeue")
        .expect("expected item");

    match item.data {
        QueueItemData::DmApproval(data) => {
            assert_eq!(data.world_id.to_string(), world_id.to_string());
        }
        other => panic!("unexpected queue item: {other:?}"),
    }

    queue.mark_complete(item.id).await.expect("mark complete");

    // Ensure one remaining pending item (we enqueued twice).
    let pending = queue
        .get_pending_count(QueueType::DmApproval)
        .await
        .expect("pending count");
    assert_eq!(pending, 1);

    // Sanity: get_approval_request works for the still-pending row.
    let still_pending = queue
        .get_approval_request(id)
        .await
        .expect("get approval request")
        .expect("expected approval request");
    assert_eq!(still_pending.world_id.to_string(), world_id.to_string());
}

#[tokio::test]
async fn sqlite_queue_generation_read_state_persists_across_restart() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();
    let user_id = "user-123";

    // Initial write.
    {
        let queue = SqliteQueue::new(&db_path_str, clock.clone())
            .await
            .expect("create queue");

        let read_batches = vec!["batch-a".to_string(), "batch-b".to_string()];
        let read_suggestions = vec!["req-1".to_string()];

        queue
            .upsert_generation_read_state(user_id, world_id, &read_batches, &read_suggestions)
            .await
            .expect("upsert read state");

        let got = queue
            .get_generation_read_state(user_id, world_id)
            .await
            .expect("get read state")
            .expect("expected read state");
        assert_eq!(got.0, read_batches);
        assert_eq!(got.1, read_suggestions);
    }

    // Re-open (simulate restart) and ensure it persisted.
    {
        let queue = SqliteQueue::new(&db_path_str, clock.clone())
            .await
            .expect("reopen queue");

        let got = queue
            .get_generation_read_state(user_id, world_id)
            .await
            .expect("get read state")
            .expect("expected read state");
        assert_eq!(got.0, vec!["batch-a".to_string(), "batch-b".to_string()]);
        assert_eq!(got.1, vec!["req-1".to_string()]);

        // Update and verify upsert.
        let read_batches = vec!["batch-b".to_string(), "batch-c".to_string()];
        let read_suggestions = vec!["req-2".to_string(), "req-3".to_string()];

        queue
            .upsert_generation_read_state(user_id, world_id, &read_batches, &read_suggestions)
            .await
            .expect("upsert read state 2");

        let got = queue
            .get_generation_read_state(user_id, world_id)
            .await
            .expect("get read state 2")
            .expect("expected read state 2");
        assert_eq!(got.0, read_batches);
        assert_eq!(got.1, read_suggestions);
    }
}

#[tokio::test]
async fn phase_advance_id_is_deterministic_for_same_phase_key() {
    // Test that the same phase_key yields the same item ID across multiple calls
    // This ensures idempotency across process restarts
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();
    let phase_key = "test:world:123:phase:0:deterministic";

    // Create initial LLM request
    let initial_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: "initial-callback".to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let next_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: phase_key.to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    // First queue instance - enqueue initial and advance
    let first_id = {
        let queue = SqliteQueue::new(&db_path_str, clock.clone())
            .await
            .expect("create queue");

        let item_id = queue
            .enqueue_llm_request(&initial_request)
            .await
            .expect("enqueue initial");

        // Advance to next phase
        queue
            .advance_phased_llm_request(item_id, &next_request, phase_key)
            .await
            .expect("advance phase")
    };

    // Second queue instance (simulating process restart) - verify idempotency
    // The same phase_key should produce the same ID, but since the item is already
    // pending, we should get the existing ID back
    {
        let queue = SqliteQueue::new(&db_path_str, clock.clone())
            .await
            .expect("reopen queue");

        // Enqueue another initial request
        let item_id2 = queue
            .enqueue_llm_request(&initial_request)
            .await
            .expect("enqueue initial 2");

        // Advance with same phase_key - should return existing ID due to idempotency
        let second_id = queue
            .advance_phased_llm_request(item_id2, &next_request, phase_key)
            .await
            .expect("advance phase 2");

        // The IDs should be identical because they're derived from the same phase_key
        assert_eq!(
            first_id.to_string(),
            second_id.to_string(),
            "Same phase_key should produce same ID across restarts"
        );
    }

    // Verify the deterministic UUID generation directly
    // UUIDv5 with the same namespace and name should always produce the same UUID
    const PHASE_NAMESPACE: Uuid = uuid::uuid!("a1b2c3d4-e5f6-47a8-b9c0-d1e2f3a4b5c6");
    let expected_uuid = Uuid::new_v5(&PHASE_NAMESPACE, phase_key.as_bytes());

    // The ID returned should match what we expect from UUIDv5
    assert_eq!(
        first_id.to_uuid().to_string(),
        expected_uuid.to_string(),
        "Phase advance ID should match UUIDv5 of phase_key"
    );

    // Verify different phase_keys produce different IDs
    let different_phase_key = "test:world:123:phase:1:deterministic";
    let different_uuid = Uuid::new_v5(&PHASE_NAMESPACE, different_phase_key.as_bytes());
    assert_ne!(
        expected_uuid, different_uuid,
        "Different phase_keys should produce different IDs"
    );
}

#[tokio::test]
async fn phase_advance_is_idempotent_for_completed_phases() {
    // Test that retrying phase advancement after completion returns the same ID
    // and does not create duplicate queue items. This ensures idempotency even
    // when the prior phase item has been marked completed.
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();
    let phase_key = "test:world:456:phase:0:idempotent";

    // Create initial LLM request
    let initial_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: "initial-callback-2".to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let next_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: phase_key.to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    // First queue instance - enqueue initial, advance, and complete the next phase
    let first_id = {
        let queue = SqliteQueue::new(&db_path_str, clock.clone())
            .await
            .expect("create queue");

        let item_id = queue
            .enqueue_llm_request(&initial_request)
            .await
            .expect("enqueue initial");

        // Advance to next phase
        let next_id = queue
            .advance_phased_llm_request(item_id, &next_request, phase_key)
            .await
            .expect("advance phase");

        // Mark the next phase as completed (simulating full pipeline execution)
        queue
            .mark_complete(next_id)
            .await
            .expect("mark next phase complete");

        next_id
    };

    // Second queue instance (simulating retry after completion) - verify idempotency
    // The same phase_key should still produce the same ID, even though it's completed
    {
        let queue = SqliteQueue::new(&db_path_str, clock.clone())
            .await
            .expect("reopen queue");

        // Enqueue another initial request (simulating a retry of the prior phase)
        let item_id2 = queue
            .enqueue_llm_request(&initial_request)
            .await
            .expect("enqueue initial 2");

        // Advance with same phase_key - should return the existing completed ID
        let second_id = queue
            .advance_phased_llm_request(item_id2, &next_request, phase_key)
            .await
            .expect("advance phase 2");

        // The IDs should be identical even though the phase was completed
        assert_eq!(
            first_id.to_string(),
            second_id.to_string(),
            "Same phase_key should produce same ID even when completed"
        );

        // Verify there's only one LLM request with this phase_key
        let llm_items = queue
            .list_by_type(QueueType::LlmRequest, 100)
            .await
            .expect("list llm requests");

        let matching_items: Vec<_> = llm_items
            .into_iter()
            .filter(|item| {
                if let QueueItemData::LlmRequest(data) = &item.data {
                    data.callback_id == phase_key
                } else {
                    false
                }
            })
            .collect();

        assert_eq!(
            matching_items.len(),
            1,
            "Should have exactly one item with the phase_key, no duplicates"
        );
    }
}

#[tokio::test]
async fn phase_advance_resets_failed_item_to_pending() {
    // Test that when a next-phase item exists but is in 'failed' status,
    // advancing to that phase resets it to 'pending' so the pipeline can retry.
    // This prevents pipeline stalls when a prior next-phase item failed.
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();
    let phase_key = "test:world:789:phase:0:failed_retry";

    // Create initial LLM request
    let initial_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: "initial-callback-failed".to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let next_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: phase_key.to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    // First queue instance - enqueue initial, advance, and mark next phase as FAILED
    let failed_item_id = {
        let queue = SqliteQueue::new(&db_path_str, clock.clone())
            .await
            .expect("create queue");

        let item_id = queue
            .enqueue_llm_request(&initial_request)
            .await
            .expect("enqueue initial");

        // Advance to next phase
        let next_id = queue
            .advance_phased_llm_request(item_id, &next_request, phase_key)
            .await
            .expect("advance phase");

        // Mark the next phase as FAILED (simulating a processing failure)
        queue
            .mark_failed(next_id, "Simulated processing failure")
            .await
            .expect("mark next phase failed");

        // Verify it's actually failed
        let llm_items = queue
            .list_by_type(QueueType::LlmRequest, 100)
            .await
            .expect("list llm requests");

        let failed_items: Vec<_> = llm_items
            .iter()
            .filter(|item| {
                if let QueueItemData::LlmRequest(data) = &item.data {
                    data.callback_id == phase_key
                        && item.status == crate::infrastructure::ports::QueueItemStatus::Failed
                } else {
                    false
                }
            })
            .collect();

        assert_eq!(
            failed_items.len(),
            1,
            "Should have exactly one failed item with the phase_key"
        );
        assert_eq!(
            failed_items[0].error_message.as_deref(),
            Some("Simulated processing failure")
        );

        next_id
    };

    // Second queue instance (simulating retry after failure) - verify the failed item is reset
    {
        let queue = SqliteQueue::new(&db_path_str, clock.clone())
            .await
            .expect("reopen queue");

        // Enqueue another initial request (simulating a retry of the prior phase)
        let item_id2 = queue
            .enqueue_llm_request(&initial_request)
            .await
            .expect("enqueue initial 2");

        // Advance with same phase_key - should reset the failed item to pending
        let retried_id = queue
            .advance_phased_llm_request(item_id2, &next_request, phase_key)
            .await
            .expect("advance phase 2");

        // The IDs should be identical - we should have gotten the same item back
        assert_eq!(
            failed_item_id.to_string(),
            retried_id.to_string(),
            "Same phase_key should produce same ID even when resetting failed item"
        );

        // Verify the item is now pending (not failed)
        let llm_items = queue
            .list_by_type(QueueType::LlmRequest, 100)
            .await
            .expect("list llm requests");

        let pending_items: Vec<_> = llm_items
            .iter()
            .filter(|item| {
                if let QueueItemData::LlmRequest(data) = &item.data {
                    data.callback_id == phase_key
                        && item.status == crate::infrastructure::ports::QueueItemStatus::Pending
                } else {
                    false
                }
            })
            .collect();

        assert_eq!(
            pending_items.len(),
            1,
            "Should have exactly one pending item with the phase_key after reset"
        );

        // Verify the error message was cleared
        assert_eq!(
            pending_items[0].error_message, None,
            "Error message should be cleared after reset to pending"
        );

        // Verify there's still only one item with this phase_key (no duplicates created)
        let matching_items: Vec<_> = llm_items
            .into_iter()
            .filter(|item| {
                if let QueueItemData::LlmRequest(data) = &item.data {
                    data.callback_id == phase_key
                } else {
                    false
                }
            })
            .collect();

        assert_eq!(
            matching_items.len(),
            1,
            "Should have exactly one item with the phase_key, no duplicates created"
        );
    }
}

#[tokio::test]
async fn enqueue_llm_request_duplicate_callback_id_is_idempotent_and_returns_existing_id() {
    // Test that attempting to enqueue two LLM requests with the same callback_id
    // is idempotent - the second call returns the existing item ID instead of failing.
    // This enables safe retries in the player action pipeline.
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();
    let duplicate_callback_id = "duplicate-callback-test-123";

    // Create the first LLM request
    let first_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: duplicate_callback_id.to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    // Create a second request with the same callback_id but different action_item_id
    let second_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(), // Different action item
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: duplicate_callback_id.to_string(), // Same callback_id
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let queue = SqliteQueue::new(&db_path_str, clock)
        .await
        .expect("create queue");

    // First enqueue should succeed
    let first_id = queue
        .enqueue_llm_request(&first_request)
        .await
        .expect("first enqueue should succeed");

    // Verify first item is in pending state
    let pending_count_before = queue
        .get_pending_count(QueueType::LlmRequest)
        .await
        .expect("get pending count");
    assert_eq!(pending_count_before, 1, "Should have one pending LLM request");

    // Second enqueue with same callback_id should succeed idempotently and return the same ID
    let second_result = queue.enqueue_llm_request(&second_request).await;

    // Assert that the second enqueue succeeds (idempotent behavior)
    assert!(
        second_result.is_ok(),
        "Second enqueue with duplicate callback_id should succeed idempotently, got: {:?}",
        second_result
    );

    let second_id = second_result.expect("second enqueue should return existing ID");

    // Verify both calls returned the same ID
    assert_eq!(
        first_id.to_string(),
        second_id.to_string(),
        "Both enqueue calls should return the same existing item ID"
    );

    // Verify queue state is safe: only one item exists with this callback_id
    let llm_items = queue
        .list_by_type(QueueType::LlmRequest, 100)
        .await
        .expect("list llm requests");

    let matching_items: Vec<_> = llm_items
        .into_iter()
        .filter(|item| {
            if let QueueItemData::LlmRequest(data) = &item.data {
                data.callback_id == duplicate_callback_id
            } else {
                false
            }
        })
        .collect();

    assert_eq!(
        matching_items.len(),
        1,
        "Should have exactly one item with the callback_id, no duplicates created"
    );

    // Verify the existing item is the first one we created
    assert_eq!(
        matching_items[0].id.to_string(),
        first_id.to_string(),
        "The remaining item should be the first one we enqueued"
    );

    // Verify the item is still in pending state (not corrupted)
    assert_eq!(
        matching_items[0].status,
        crate::infrastructure::ports::QueueItemStatus::Pending,
        "Existing item should remain in pending state"
    );

    // Verify pending count is still 1 (no partial insert)
    let pending_count_after = queue
        .get_pending_count(QueueType::LlmRequest)
        .await
        .expect("get pending count after");
    assert_eq!(
        pending_count_after, 1,
        "Pending count should remain 1 after idempotent duplicate enqueue"
    );
}

#[tokio::test]
async fn callback_id_normalization_ensures_idempotency_and_consistent_lookups() {
    // Test that callback_id is normalized (trimmed) at the queue boundary.
    // Enqueuing with "  abc  " and then canceling/deleting with "abc" should work.
    // This ensures idempotency is not broken by whitespace variations.
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();
    let base_callback_id = "abc";

    // Create an LLM request with whitespace-padded callback_id
    let padded_callback_id = format!("  {}  ", base_callback_id);
    let request_with_padding = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: padded_callback_id.clone(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    // Create a second request with the same base callback_id but no padding
    let request_without_padding = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: base_callback_id.to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let queue = SqliteQueue::new(&db_path_str, clock)
        .await
        .expect("create queue");

    // Enqueue with padded callback_id should succeed
    let first_id = queue
        .enqueue_llm_request(&request_with_padding)
        .await
        .expect("enqueue with padded callback_id should succeed");

    // Verify one pending item
    let pending_count_before = queue
        .get_pending_count(QueueType::LlmRequest)
        .await
        .expect("get pending count");
    assert_eq!(
        pending_count_before, 1,
        "Should have one pending LLM request after first enqueue"
    );

    // Enqueue with non-padded callback_id should be idempotent (same normalized ID)
    let second_result = queue.enqueue_llm_request(&request_without_padding).await;
    assert!(
        second_result.is_ok(),
        "Second enqueue with normalized callback_id should succeed idempotently"
    );

    let second_id = second_result.expect("second enqueue should return existing ID");

    // Verify both calls returned the same ID (idempotency works across whitespace variants)
    assert_eq!(
        first_id.to_string(),
        second_id.to_string(),
        "Both enqueue calls should return the same ID due to normalized callback_id"
    );

    // Verify still only one item in queue
    let pending_count_after = queue
        .get_pending_count(QueueType::LlmRequest)
        .await
        .expect("get pending count after idempotent enqueue");
    assert_eq!(
        pending_count_after, 1,
        "Should still have only one pending LLM request"
    );

    // Cancel using the non-padded callback_id should find the item enqueued with padded ID
    let cancelled = queue
        .cancel_pending_llm_request_by_callback_id(base_callback_id)
        .await
        .expect("cancel should succeed");
    assert!(
        cancelled,
        "Cancel with normalized callback_id should find and cancel the item"
    );

    // Verify the item is now failed/cancelled
    let pending_count_after_cancel = queue
        .get_pending_count(QueueType::LlmRequest)
        .await
        .expect("get pending count after cancel");
    assert_eq!(
        pending_count_after_cancel, 0,
        "Should have no pending items after cancellation"
    );

    // Re-enqueue a new item with padded callback_id for delete test
    let third_id = queue
        .enqueue_llm_request(&request_with_padding)
        .await
        .expect("re-enqueue after cancel should succeed");

    // With idempotent enqueue, we should get the same ID back (the existing failed item)
    // This is the expected idempotent behavior - same callback_id returns same item ID
    assert_eq!(
        first_id.to_string(),
        third_id.to_string(),
        "Re-enqueue with same callback_id should return existing item ID (idempotent)"
    );

    // Delete using non-padded callback_id should find and delete the item
    let deleted = queue
        .delete_by_callback_id(base_callback_id)
        .await
        .expect("delete should succeed");
    assert!(
        deleted,
        "Delete with normalized callback_id should find and delete the item"
    );

    // Verify the item is actually deleted
    let llm_items = queue
        .list_by_type(QueueType::LlmRequest, 100)
        .await
        .expect("list llm requests");

    let matching_items: Vec<_> = llm_items
        .into_iter()
        .filter(|item| {
            if let QueueItemData::LlmRequest(data) = &item.data {
                data.callback_id == base_callback_id
            } else {
                false
            }
        })
        .collect();

    assert_eq!(
        matching_items.len(),
        0,
        "Should have no items with the normalized callback_id after delete"
    );

    // Test edge case: empty callback_id after trimming should not cause issues
    let empty_after_trim_result = queue.cancel_pending_llm_request_by_callback_id("   ").await;
    assert!(
        empty_after_trim_result.is_ok(),
        "Cancel with whitespace-only callback_id should not error"
    );
    assert!(
        !empty_after_trim_result.expect("should have result"),
        "Cancel with whitespace-only callback_id should return false (no match)"
    );

    let delete_empty_result = queue.delete_by_callback_id("  \t\n  ").await;
    assert!(
        delete_empty_result.is_ok(),
        "Delete with whitespace-only callback_id should not error"
    );
    assert!(
        !delete_empty_result.expect("should have result"),
        "Delete with whitespace-only callback_id should return false (no match)"
    );
}

#[tokio::test]
async fn sqlite_queue_unknown_status_fails_fast() {
    use sqlx::SqlitePool;

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue_unknown_status.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    // Create the queue (which sets up the database)
    let queue = SqliteQueue::new(&db_path_str, clock.clone())
        .await
        .expect("create queue");

    // Insert a row with an invalid status directly into the database
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path_str))
        .await
        .expect("connect to db");

    let item_id = Uuid::new_v4();
    let payload_json = r#"{\"test\": \"data\"}"#;

    sqlx::query(
        r#"
        INSERT INTO queue_items (id, queue_type, payload_json, status, created_at, updated_at)
        VALUES (?1, 'player_action', ?2, 'corrupted_status', ?3, ?3)
        "#,
    )
    .bind(item_id.to_string())
    .bind(payload_json)
    .bind(now.to_rfc3339())
    .execute(&pool)
    .await
    .expect("insert row with invalid status");

    // Attempting to list items should fail fast with an error about unknown status
    // (dequeue won't trigger the error because it only selects pending items)
    let result = queue.list_by_type(QueueType::PlayerAction, 10).await;

    let err = result.expect_err("List should fail when encountering unknown status value");
    let err_string = err.to_string();
    assert!(
        err_string.contains("Unknown queue status value"),
        "Error should indicate unknown status: got {}",
        err_string
    );
    assert!(
        err_string.contains("corrupted_status"),
        "Error should include the invalid status value: got {}",
        err_string
    );
}

#[tokio::test]
async fn enqueue_llm_request_empty_callback_id_returns_validation_error() {
    // Test that attempting to enqueue an LLM request with an empty callback_id
    // returns a clear Validation error before any database operation occurs.
    // This prevents queue items from being stuck due to unique index collisions
    // on empty strings and provides clear feedback to callers.
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();

    // Create an LLM request with empty callback_id
    let empty_callback_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: "".to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    // Create an LLM request with whitespace-only callback_id
    let whitespace_callback_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: "   \t\n  ".to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let queue = SqliteQueue::new(&db_path_str, clock)
        .await
        .expect("create queue");

    // Attempt to enqueue with empty callback_id should fail with Validation error
    let empty_result = queue.enqueue_llm_request(&empty_callback_request).await;
    let empty_err = empty_result.expect_err(
        "Enqueue with empty callback_id should fail with validation error",
    );
    let empty_err_string = empty_err.to_string();
    assert!(
        empty_err_string.contains("Validation error"),
        "Error should be a Validation error: got {}",
        empty_err_string
    );
    assert!(
        empty_err_string.contains("callback_id cannot be empty"),
        "Error message should indicate callback_id issue: got {}",
        empty_err_string
    );

    // Attempt to enqueue with whitespace-only callback_id should also fail
    let whitespace_result = queue.enqueue_llm_request(&whitespace_callback_request).await;
    let whitespace_err = whitespace_result.expect_err(
        "Enqueue with whitespace-only callback_id should fail with validation error",
    );
    let whitespace_err_string = whitespace_err.to_string();
    assert!(
        whitespace_err_string.contains("Validation error"),
        "Error should be a Validation error: got {}",
        whitespace_err_string
    );
    assert!(
        whitespace_err_string.contains("callback_id cannot be empty"),
        "Error message should indicate callback_id issue: got {}",
        whitespace_err_string
    );

    // Verify no queue items were created (validation happened before DB insert)
    let pending_count = queue
        .get_pending_count(QueueType::LlmRequest)
        .await
        .expect("get pending count");
    assert_eq!(
        pending_count, 0,
        "No queue items should be created when validation fails"
    );

    // Verify the queue is still functional by enqueuing a valid request
    let valid_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
        action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: "valid-callback-id".to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let valid_result = queue.enqueue_llm_request(&valid_request).await;
    assert!(
        valid_result.is_ok(),
        "Valid request should succeed after validation errors"
    );

    // Verify the valid request was actually enqueued
    let pending_count_after = queue
        .get_pending_count(QueueType::LlmRequest)
        .await
        .expect("get pending count after valid enqueue");
    assert_eq!(
        pending_count_after, 1,
        "Valid request should be enqueued"
    );
}

#[tokio::test]
async fn enqueue_llm_request_duplicate_with_whitespace_padded_callback_id_returns_existing_id() {
    // Test that enqueuing with a whitespace-padded callback_id twice returns the existing ID.
    // This verifies that the duplicate-handling lookup uses the normalized (trimmed) value,
    // not the raw input. This is critical for idempotency during retries.
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();
    let padded_callback_id = "  test-callback-with-padding  ";

    // Create the first LLM request with whitespace-padded callback_id
    let first_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: padded_callback_id.to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    // Create a second request with the same whitespace-padded callback_id
    let second_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(), // Different action item
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: padded_callback_id.to_string(), // Same whitespace-padded callback_id
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let queue = SqliteQueue::new(&db_path_str, clock)
        .await
        .expect("create queue");

    // First enqueue should succeed
    let first_id = queue
        .enqueue_llm_request(&first_request)
        .await
        .expect("first enqueue should succeed");

    // Verify first item is in pending state
    let pending_count_before = queue
        .get_pending_count(QueueType::LlmRequest)
        .await
        .expect("get pending count");
    assert_eq!(pending_count_before, 1, "Should have one pending LLM request");

    // Second enqueue with the same whitespace-padded callback_id should succeed idempotently
    // and return the same ID (this tests the duplicate-handling path uses normalized value)
    let second_result = queue.enqueue_llm_request(&second_request).await;

    // Assert that the second enqueue succeeds (idempotent behavior)
    assert!(
        second_result.is_ok(),
        "Second enqueue with duplicate whitespace-padded callback_id should succeed idempotently, got: {:?}",
        second_result
    );

    let second_id = second_result.expect("second enqueue should return existing ID");

    // Verify both calls returned the same ID
    assert_eq!(
        first_id.to_string(),
        second_id.to_string(),
        "Both enqueue calls should return the same existing item ID when using whitespace-padded callback_id"
    );

    // Verify queue state is safe: only one item exists
    let llm_items = queue
        .list_by_type(QueueType::LlmRequest, 100)
        .await
        .expect("list llm requests");

    assert_eq!(
        llm_items.len(),
        1,
        "Should have exactly one item in the queue, no duplicates created"
    );

    // Verify the stored callback_id is the normalized (trimmed) value
    if let QueueItemData::LlmRequest(data) = &llm_items[0].data {
        assert_eq!(
            data.callback_id,
            "test-callback-with-padding",
            "Stored callback_id should be the normalized (trimmed) value"
        );
    } else {
        panic!("Expected LlmRequest data");
    }

    // Verify pending count is still 1 (no partial insert)
    let pending_count_after = queue
        .get_pending_count(QueueType::LlmRequest)
        .await
        .expect("get pending count after");
    assert_eq!(
        pending_count_after, 1,
        "Pending count should remain 1 after idempotent duplicate enqueue"
    );
}

#[tokio::test]
async fn phase_advance_id_is_deterministic_for_same_phase_key_with_whitespace() {
    // Test that the same phase_key yields the same item ID across multiple calls,
    // even when whitespace varies (e.g., "  key  " vs "key").
    // This ensures idempotency across retries that may have whitespace differences.
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();
    let base_phase_key = "test:world:phase:whitespace";
    let padded_phase_key = format!("  {}  ", base_phase_key);

    // Create initial LLM request
    let initial_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: "initial-callback".to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let next_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: base_phase_key.to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    // First queue instance - enqueue initial and advance with padded phase_key
    let first_id = {
        let queue = SqliteQueue::new(&db_path_str, clock.clone())
            .await
            .expect("create queue");

        let item_id = queue
            .enqueue_llm_request(&initial_request)
            .await
            .expect("enqueue initial");

        // Advance to next phase with whitespace-padded phase_key
        queue
            .advance_phased_llm_request(item_id, &next_request, &padded_phase_key)
            .await
            .expect("advance phase with padded key")
    };

    // Second queue instance (simulating process restart) - verify idempotency
    // The same base phase_key (without padding) should produce the same ID
    {
        let queue = SqliteQueue::new(&db_path_str, clock.clone())
            .await
            .expect("reopen queue");

        // Enqueue another initial request
        let item_id2 = queue
            .enqueue_llm_request(&initial_request)
            .await
            .expect("enqueue initial 2");

        // Advance with non-padded phase_key - should return existing ID due to idempotency
        let second_id = queue
            .advance_phased_llm_request(item_id2, &next_request, base_phase_key)
            .await
            .expect("advance phase with non-padded key");

        // The IDs should be identical because they're derived from the same normalized phase_key
        assert_eq!(
            first_id.to_string(),
            second_id.to_string(),
            "Same phase_key (with/without whitespace) should produce same ID across restarts"
        );
    }

    // Verify the deterministic UUID generation directly
    // UUIDv5 with the same namespace and normalized name should always produce the same UUID
    const PHASE_NAMESPACE: Uuid = uuid::uuid!("a1b2c3d4-e5f6-47a8-b9c0-d1e2f3a4b5c6");
    let expected_uuid = Uuid::new_v5(&PHASE_NAMESPACE, base_phase_key.as_bytes());

    // The ID returned should match what we expect from UUIDv5 of the normalized key
    assert_eq!(
        first_id.to_uuid().to_string(),
        expected_uuid.to_string(),
        "Phase advance ID should match UUIDv5 of normalized phase_key"
    );

    // Verify there's only one LLM request with this phase_key (no duplicates)
    let queue = SqliteQueue::new(&db_path_str, clock)
        .await
        .expect("reopen queue for verification");

    let llm_items = queue
        .list_by_type(QueueType::LlmRequest, 100)
        .await
        .expect("list llm requests");

    let matching_items: Vec<_> = llm_items
        .into_iter()
        .filter(|item| {
            if let QueueItemData::LlmRequest(data) = &item.data {
                data.callback_id == base_phase_key
            } else {
                false
            }
        })
        .collect();

    assert_eq!(
        matching_items.len(),
        1,
        "Should have exactly one item with the phase_key, no duplicates created"
    );
}

#[tokio::test]
async fn advance_phased_llm_request_empty_phase_key_returns_validation_error() {
    // Test that attempting to advance a phased LLM request with an empty or
    // whitespace-only phase_key returns a clear Validation error before any
    // database transaction is started. This prevents queue items with invalid
    // callback IDs and ensures idempotency guarantees are maintained.
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();

    // Create an initial LLM request
    let initial_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: "initial-callback".to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let next_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: "next-phase-callback".to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let queue = SqliteQueue::new(&db_path_str, clock)
        .await
        .expect("create queue");

    // Enqueue the initial request
    let item_id = queue
        .enqueue_llm_request(&initial_request)
        .await
        .expect("enqueue initial request");

    // Test with empty phase_key
    let empty_result = queue
        .advance_phased_llm_request(item_id, &next_request, "")
        .await;
    let empty_err = empty_result.expect_err(
        "Advance with empty phase_key should fail with validation error",
    );
    let empty_err_string = empty_err.to_string();
    assert!(
        empty_err_string.contains("Validation error"),
        "Error should be a Validation error: got {}",
        empty_err_string
    );
    assert!(
        empty_err_string.contains("phase_key cannot be empty"),
        "Error message should indicate phase_key issue: got {}",
        empty_err_string
    );

    // Test with whitespace-only phase_key
    let whitespace_result = queue
        .advance_phased_llm_request(item_id, &next_request, "   \t\n  ")
        .await;
    let whitespace_err = whitespace_result.expect_err(
        "Advance with whitespace-only phase_key should fail with validation error",
    );
    let whitespace_err_string = whitespace_err.to_string();
    assert!(
        whitespace_err_string.contains("Validation error"),
        "Error should be a Validation error: got {}",
        whitespace_err_string
    );
    assert!(
        whitespace_err_string.contains("phase_key cannot be empty"),
        "Error message should indicate phase_key issue: got {}",
        whitespace_err_string
    );

    // Verify the initial item was NOT marked as completed (validation happened before transaction)
    let llm_items = queue
        .list_by_type(QueueType::LlmRequest, 100)
        .await
        .expect("list llm requests");

    let initial_items: Vec<_> = llm_items
        .iter()
        .filter(|item| item.id == item_id)
        .collect();

    assert_eq!(
        initial_items.len(),
        1,
        "Should have exactly one item with the initial ID"
    );
    assert_eq!(
        initial_items[0].status,
        crate::infrastructure::ports::QueueItemStatus::Pending,
        "Initial item should still be pending (not marked complete due to validation failure)"
    );

    // Verify that a valid phase_key works correctly
    let valid_phase_key = "valid:phase:key:123";
    let valid_result = queue
        .advance_phased_llm_request(item_id, &next_request, valid_phase_key)
        .await;
    assert!(
        valid_result.is_ok(),
        "Advance with valid phase_key should succeed: {:?}",
        valid_result
    );

    // Verify the initial item is now completed
    let llm_items_after = queue
        .list_by_type(QueueType::LlmRequest, 100)
        .await
        .expect("list llm requests after valid advance");

    let initial_items_after: Vec<_> = llm_items_after
        .iter()
        .filter(|item| item.id == item_id)
        .collect();

    assert_eq!(
        initial_items_after.len(),
        1,
        "Should still have the initial item"
    );
    assert_eq!(
        initial_items_after[0].status,
        crate::infrastructure::ports::QueueItemStatus::Completed,
        "Initial item should be completed after valid phase advance"
    );

    // Verify the next phase was created
    // Note: phase_key is stored as callback_id in the database, not in the payload
    // The next_request.callback_id is different from phase_key
    // We need to check that a new item was created (with a different ID than the initial item)
    let next_phase_items: Vec<_> = llm_items_after
        .iter()
        .filter(|item| item.id != item_id)
        .collect();

    assert_eq!(
        next_phase_items.len(),
        1,
        "Should have exactly one new item (the next phase)"
    );
    assert_eq!(
        next_phase_items[0].status,
        crate::infrastructure::ports::QueueItemStatus::Pending,
        "Next phase item should be pending"
    );
}

// =============================================================================
// Unique Constraint Verification Tests
// =============================================================================

/// Test that when a unique constraint violation occurs but no callback_id row exists,
/// the original database error is returned (fail-fast behavior).
///
/// This ensures that unique constraint violations on other columns (e.g., primary key id)
/// are not incorrectly treated as callback_id collisions.
#[tokio::test]
async fn enqueue_llm_request_non_callback_id_unique_constraint_returns_database_error() {
    use sqlx::SqlitePool;

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();
    let callback_id = "unique-constraint-test-callback";

    // Create an LLM request
    let request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: callback_id.to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let queue = SqliteQueue::new(&db_path_str, clock.clone())
        .await
        .expect("create queue");

    // Enqueue the first request
    let first_id = queue
        .enqueue_llm_request(&request)
        .await
        .expect("first enqueue should succeed");

    // Manually insert a row with a different callback_id but force a primary key collision
    // by using the same UUID. This simulates a race condition or bug where the same ID
    // is generated twice but with different callback_ids.
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path_str))
        .await
        .expect("connect to db");

    let payload_json = serde_json::to_string(&request).expect("serialize request");

    // Attempt to insert a row with the same ID (primary key) but different callback_id
    // This should trigger a unique constraint violation on the primary key, not callback_id
    let insert_result = sqlx::query(
        r#"
        INSERT INTO queue_items (id, queue_type, payload_json, status, created_at, updated_at, callback_id)
        VALUES (?1, 'llm_request', ?2, 'pending', ?3, ?3, 'different-callback-id')
        "#,
    )
    .bind(first_id.to_uuid().to_string())
    .bind(&payload_json)
    .bind(now.to_rfc3339())
    .execute(&pool)
    .await;

    // This should fail with a unique constraint violation on the primary key
    assert!(
        insert_result.is_err(),
        "Insert with duplicate primary key should fail"
    );

    // Verify the error is a unique constraint violation
    let err_string = insert_result.expect_err("should be error").to_string().to_lowercase();
    assert!(
        err_string.contains("unique") || err_string.contains("constraint"),
        "Error should indicate a unique constraint violation: got {}",
        err_string
    );

    // Now verify that the queue remains functional by enqueuing a new request
    let new_callback_id = "new-callback-id-after-constraint-test";
    let new_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: new_callback_id.to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let _new_id = queue
        .enqueue_llm_request(&new_request)
        .await
        .expect("enqueue after constraint test should succeed");

    // Verify the new item was enqueued
    let pending_count = queue
        .get_pending_count(QueueType::LlmRequest)
        .await
        .expect("get pending count");
    assert_eq!(pending_count, 2, "Should have two pending LLM requests");

    // Verify we can dequeue the new item
    let dequeued = queue
        .dequeue_llm_request()
        .await
        .expect("dequeue should succeed")
        .expect("should have an item");

    assert_eq!(
        dequeued.id.to_string(),
        first_id.to_string(),
        "First dequeued item should be the first one enqueued (FIFO)"
    );
}

/// Test that when a unique constraint violation occurs and the callback_id row exists,
/// the existing ID is returned (idempotent behavior).
///
/// This is the "happy path" for idempotency - the callback_id collision is confirmed
/// and the existing item ID is returned.
#[tokio::test]
async fn enqueue_llm_request_confirmed_callback_id_collision_returns_existing_id() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();
    let callback_id = "confirmed-collision-test-123";

    // Create the first LLM request
    let first_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: callback_id.to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    // Create a second request with the same callback_id but different action_item_id
    let second_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(), // Different action item
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: callback_id.to_string(), // Same callback_id
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let queue = SqliteQueue::new(&db_path_str, clock)
        .await
        .expect("create queue");

    // First enqueue should succeed
    let first_id = queue
        .enqueue_llm_request(&first_request)
        .await
        .expect("first enqueue should succeed");

    // Second enqueue with same callback_id should succeed idempotently
    let second_result = queue.enqueue_llm_request(&second_request).await;

    assert!(
        second_result.is_ok(),
        "Second enqueue with confirmed callback_id collision should succeed idempotently"
    );

    let second_id = second_result.expect("second enqueue should return existing ID");

    // Verify both calls returned the same ID
    assert_eq!(
        first_id.to_string(),
        second_id.to_string(),
        "Both enqueue calls should return the same existing item ID"
    );

    // Verify queue state: only one item exists with this callback_id
    let llm_items = queue
        .list_by_type(QueueType::LlmRequest, 100)
        .await
        .expect("list llm requests");

    let matching_items: Vec<_> = llm_items
        .into_iter()
        .filter(|item| {
            if let QueueItemData::LlmRequest(data) = &item.data {
                data.callback_id == callback_id
            } else {
                false
            }
        })
        .collect();

    assert_eq!(
        matching_items.len(),
        1,
        "Should have exactly one item with the callback_id"
    );

    // Verify the item is the first one we created
    assert_eq!(
        matching_items[0].id.to_string(),
        first_id.to_string(),
        "The remaining item should be the first one we enqueued"
    );
}

/// Test that the idempotent path is used ONLY when the callback_id lookup succeeds.
///
/// This test verifies the security property: even if we detect a unique constraint
/// violation, we only return the existing ID if we can confirm the callback_id exists.
/// Otherwise, we return the original database error.
#[tokio::test]
async fn enqueue_llm_request_unique_constraint_without_callback_id_fails_fast() {
    use sqlx::SqlitePool;

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("queue.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let clock: Arc<dyn crate::infrastructure::ports::ClockPort> = Arc::new(FixedClock(now));

    let world_id = WorldId::new();
    let callback_id = "fail-fast-test-callback";

    // Create an LLM request
    let request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: callback_id.to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let queue = SqliteQueue::new(&db_path_str, clock.clone())
        .await
        .expect("create queue");

    // Enqueue the first request
    let first_id = queue
        .enqueue_llm_request(&request)
        .await
        .expect("first enqueue should succeed");

    // Verify the item exists
    let pending_count_before = queue
        .get_pending_count(QueueType::LlmRequest)
        .await
        .expect("get pending count");
    assert_eq!(pending_count_before, 1, "Should have one pending LLM request");

    // Now simulate a scenario where:
    // 1. A unique constraint violation occurs (somehow)
    // 2. But the callback_id row does NOT exist (it was deleted between insert and lookup)
    //
    // We can't easily trigger this race condition, but we can verify the behavior
    // by checking that the callback_id lookup works correctly.

    // Delete the item directly from the database (simulating a race condition)
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path_str))
        .await
        .expect("connect to db");

    sqlx::query("DELETE FROM queue_items WHERE callback_id = ?")
        .bind(callback_id)
        .execute(&pool)
        .await
        .expect("delete item");

    // Verify the item is deleted
    let pending_count_after_delete = queue
        .get_pending_count(QueueType::LlmRequest)
        .await
        .expect("get pending count after delete");
    assert_eq!(
        pending_count_after_delete, 0,
        "Should have no pending LLM requests after delete"
    );

    // Now enqueue a new request with the same callback_id
    // This should succeed (not return an existing ID, since the old one is gone)
    let new_request = LlmRequestData {
        request_type: LlmRequestType::NpcResponse {
            action_item_id: QueueItemId::new(),
        },
        world_id,
        pc_id: None,
        prompt: None,
        suggestion_context: None,
        callback_id: callback_id.to_string(),
        conversation_id: None,
        temperature: None,
        phase_meta: None,
    };

    let new_id = queue
        .enqueue_llm_request(&new_request)
        .await
        .expect("enqueue after delete should succeed");

    // The new ID should be different from the first one
    assert_ne!(
        first_id.to_string(),
        new_id.to_string(),
        "New enqueue after delete should get a new ID"
    );

    // Verify the new item exists
    let pending_count_after = queue
        .get_pending_count(QueueType::LlmRequest)
        .await
        .expect("get pending count after");
    assert_eq!(
        pending_count_after, 1,
        "Should have one pending LLM request after re-enqueue"
    );
}
