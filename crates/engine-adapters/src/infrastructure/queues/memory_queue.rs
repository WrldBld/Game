//! In-memory queue implementation for development and testing
//!
//! This implementation uses a simple Vec-based storage with priority-based dequeue.
//! It does not persist data and is suitable for testing and development only.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use wrldbldr_engine_ports::outbound::{
    ApprovalQueuePort, ProcessingQueuePort, QueueError, QueueItem, QueueItemId, QueueItemStatus,
    QueueNotificationPort, QueuePort,
};
use wrldbldr_domain::WorldId;

/// In-memory queue implementation
pub struct InMemoryQueue<T, N: QueueNotificationPort> {
    items: Arc<RwLock<Vec<QueueItem<T>>>>,
    queue_name: String,
    notifier: N,
}

impl<T, N: QueueNotificationPort> InMemoryQueue<T, N> {
    /// Get the notifier for this queue
    pub fn notifier(&self) -> &N {
        &self.notifier
    }
}

impl<T, N: QueueNotificationPort> InMemoryQueue<T, N>
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    pub fn new(queue_name: impl Into<String>, notifier: N) -> Self {
        Self {
            items: Arc::new(RwLock::new(Vec::new())),
            queue_name: queue_name.into(),
            notifier,
        }
    }
}

#[async_trait]
impl<T, N: QueueNotificationPort + 'static> QueuePort<T> for InMemoryQueue<T, N>
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    async fn enqueue(&self, payload: T, priority: u8) -> Result<QueueItemId, QueueError> {
        let mut items = self.items.write().await;
        let item = QueueItem::new(payload, priority);
        let id = item.id;
        items.push(item);
        drop(items); // Release the lock before notifying
        
        // Notify workers that work is available
        self.notifier.notify_work_available().await;
        
        Ok(id)
    }

    async fn dequeue(&self) -> Result<Option<QueueItem<T>>, QueueError> {
        let mut items = self.items.write().await;
        let now = Utc::now();

        // Find the highest priority pending item (or delayed item that's ready)
        let mut best_idx: Option<usize> = None;
        let mut best_priority = u8::MIN;
        let mut best_created = None;

        for (idx, item) in items.iter().enumerate() {
            let is_ready = match item.status {
                QueueItemStatus::Pending => true,
                QueueItemStatus::Delayed => {
                    item.scheduled_at.map_or(false, |scheduled| scheduled <= now)
                }
                _ => false,
            };

            if is_ready {
                let priority = item.priority;
                let created = item.created_at;

                if best_idx.is_none()
                    || priority > best_priority
                    || (priority == best_priority && created < best_created.unwrap_or(created))
                {
                    best_idx = Some(idx);
                    best_priority = priority;
                    best_created = Some(created);
                }
            }
        }

        if let Some(idx) = best_idx {
            let mut item = items.remove(idx);
            item.status = QueueItemStatus::Processing;
            item.updated_at = Utc::now();
            item.attempts += 1;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    async fn peek(&self) -> Result<Option<QueueItem<T>>, QueueError> {
        let items = self.items.read().await;
        let now = Utc::now();

        let mut best_item: Option<QueueItem<T>> = None;
        let mut best_priority = u8::MIN;
        let mut best_created = None;

        for item in items.iter() {
            let is_ready = match item.status {
                QueueItemStatus::Pending => true,
                QueueItemStatus::Delayed => {
                    item.scheduled_at.map_or(false, |scheduled| scheduled <= now)
                }
                _ => false,
            };

            if is_ready {
                let priority = item.priority;
                let created = item.created_at;

                if best_item.is_none()
                    || priority > best_priority
                    || (priority == best_priority && created < best_created.unwrap_or(created))
                {
                    best_item = Some(item.clone());
                    best_priority = priority;
                    best_created = Some(created);
                }
            }
        }

        Ok(best_item)
    }

    async fn complete(&self, id: QueueItemId) -> Result<(), QueueError> {
        let mut items = self.items.write().await;
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.status = QueueItemStatus::Completed;
            item.updated_at = Utc::now();
            Ok(())
        } else {
            Err(QueueError::NotFound(id.to_string()))
        }
    }

    async fn fail(&self, id: QueueItemId, error: &str) -> Result<(), QueueError> {
        let mut items = self.items.write().await;
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.status = QueueItemStatus::Failed;
            item.error_message = Some(error.to_string());
            item.updated_at = Utc::now();
            Ok(())
        } else {
            Err(QueueError::NotFound(id.to_string()))
        }
    }

    async fn delay(&self, id: QueueItemId, until: DateTime<Utc>) -> Result<(), QueueError> {
        let mut items = self.items.write().await;
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.status = QueueItemStatus::Delayed;
            item.scheduled_at = Some(until);
            item.updated_at = Utc::now();
            Ok(())
        } else {
            Err(QueueError::NotFound(id.to_string()))
        }
    }

    async fn get(&self, id: QueueItemId) -> Result<Option<QueueItem<T>>, QueueError> {
        let items = self.items.read().await;
        Ok(items.iter().find(|i| i.id == id).cloned())
    }

    async fn list_by_status(&self, status: QueueItemStatus) -> Result<Vec<QueueItem<T>>, QueueError> {
        let items = self.items.read().await;
        Ok(items
            .iter()
            .filter(|i| i.status == status)
            .cloned()
            .collect())
    }

    async fn depth(&self) -> Result<usize, QueueError> {
        let items = self.items.read().await;
        Ok(items
            .iter()
            .filter(|i| i.status == QueueItemStatus::Pending)
            .count())
    }

    async fn cleanup(&self, older_than: Duration) -> Result<usize, QueueError> {
        let mut items = self.items.write().await;
        let cutoff = Utc::now() - older_than;
        let initial_len = items.len();

        items.retain(|item| {
            let should_remove = match item.status {
                QueueItemStatus::Completed | QueueItemStatus::Failed => {
                    item.updated_at < cutoff
                }
                _ => false,
            };
            !should_remove
        });

        Ok(initial_len - items.len())
    }
}

#[async_trait]
impl<T, N: QueueNotificationPort + 'static> ApprovalQueuePort<T> for InMemoryQueue<T, N>
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<QueueItem<T>>, QueueError> {
        let world_id_str = world_id.to_string();
        let items = self.items.read().await;
        
        Ok(items
            .iter()
            .filter(|i| {
                // Must be pending or processing
                if !matches!(i.status, QueueItemStatus::Pending | QueueItemStatus::Processing) {
                    return false;
                }
                
                // Extract world_id from payload by serializing and checking JSON
                if let Ok(json) = serde_json::to_value(&i.payload) {
                    if let Some(payload_world_id) = json.get("world_id").and_then(|v| v.as_str()) {
                        return payload_world_id == world_id_str;
                    }
                }
                
                // If we can't extract world_id, don't include this item
                false
            })
            .cloned()
            .collect())
    }

    async fn get_history_by_world(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> Result<Vec<QueueItem<T>>, QueueError> {
        let world_id_str = world_id.to_string();
        let items = self.items.read().await;
        
        let mut history: Vec<_> = items
            .iter()
            .filter(|i| {
                // Must be completed, failed, or expired
                if !matches!(
                    i.status,
                    QueueItemStatus::Completed | QueueItemStatus::Failed | QueueItemStatus::Expired
                ) {
                    return false;
                }
                
                // Extract world_id from payload by serializing and checking JSON
                if let Ok(json) = serde_json::to_value(&i.payload) {
                    if let Some(payload_world_id) = json.get("world_id").and_then(|v| v.as_str()) {
                        return payload_world_id == world_id_str;
                    }
                }
                
                // If we can't extract world_id, don't include this item
                false
            })
            .cloned()
            .collect();

        // Sort by updated_at descending (most recent first)
        history.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        history.truncate(limit);
        Ok(history)
    }

    async fn expire_old(&self, older_than: Duration) -> Result<usize, QueueError> {
        let mut items = self.items.write().await;
        let cutoff = Utc::now() - older_than;
        let mut expired_count = 0;

        for item in items.iter_mut() {
            if matches!(
                item.status,
                QueueItemStatus::Pending | QueueItemStatus::Delayed
            ) && item.created_at < cutoff
            {
                item.status = QueueItemStatus::Expired;
                item.updated_at = Utc::now();
                expired_count += 1;
            }
        }

        Ok(expired_count)
    }
}

#[async_trait]
impl<T, N: QueueNotificationPort + 'static> ProcessingQueuePort<T> for InMemoryQueue<T, N>
where
    T: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    fn batch_size(&self) -> usize {
        1 // Default to sequential processing
    }

    async fn processing_count(&self) -> Result<usize, QueueError> {
        let items = self.items.read().await;
        Ok(items
            .iter()
            .filter(|i| i.status == QueueItemStatus::Processing)
            .count())
    }

    async fn has_capacity(&self) -> Result<bool, QueueError> {
        let processing = self.processing_count().await?;
        Ok(processing < self.batch_size())
    }
}
