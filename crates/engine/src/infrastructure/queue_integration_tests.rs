use std::sync::Arc;

use chrono::{TimeZone, Utc};
use uuid::Uuid;
use wrldbldr_domain::{ApprovalDecisionType, ApprovalRequestData, ApprovalUrgency, WorldId};

use crate::infrastructure::{clock::FixedClock, ports::QueueItemData, ports::QueuePort, queue::SqliteQueue};

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
        source_action_id: Uuid::new_v4(),
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
    };

    let id = {
        let queue = SqliteQueue::new(&db_path_str, clock.clone())
            .await
            .expect("create queue");
        queue
            .enqueue_dm_approval(&approval)
            .await
            .expect("enqueue");
        queue
            .get_pending_count("dm_approval")
            .await
            .expect("count");

        // Drop queue to simulate restart
        queue.enqueue_dm_approval(&approval).await.expect("enqueue2")
    };

    let queue = SqliteQueue::new(&db_path_str, clock).await.expect("reopen queue");

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
        .get_pending_count("dm_approval")
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
