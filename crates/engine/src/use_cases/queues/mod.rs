//! Queue processing use cases.

use std::sync::Arc;

use crate::infrastructure::queue::SqliteQueue;

/// Container for queue use cases.
pub struct QueueUseCases {
    pub process_player_action: Arc<ProcessPlayerAction>,
    pub process_llm_request: Arc<ProcessLlmRequest>,
}

impl QueueUseCases {
    pub fn new(
        process_player_action: Arc<ProcessPlayerAction>,
        process_llm_request: Arc<ProcessLlmRequest>,
    ) -> Self {
        Self {
            process_player_action,
            process_llm_request,
        }
    }
}

/// Process player action from queue.
pub struct ProcessPlayerAction {
    #[allow(dead_code)]
    queue: Arc<SqliteQueue>,
}

impl ProcessPlayerAction {
    pub fn new(queue: Arc<SqliteQueue>) -> Self {
        Self { queue }
    }

    pub async fn execute(&self) -> Result<Option<()>, QueueError> {
        todo!("Process player action use case")
    }
}

/// Process LLM request from queue.
pub struct ProcessLlmRequest {
    #[allow(dead_code)]
    queue: Arc<SqliteQueue>,
}

impl ProcessLlmRequest {
    pub fn new(queue: Arc<SqliteQueue>) -> Self {
        Self { queue }
    }

    pub async fn execute(&self) -> Result<Option<()>, QueueError> {
        todo!("Process LLM request use case")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Queue error: {0}")]
    Queue(#[from] crate::infrastructure::ports::QueueError),
}
