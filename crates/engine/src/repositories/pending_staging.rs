//! Pending staging request storage wrapper.

use std::sync::Arc;

use crate::api::websocket::PendingStagingStoreImpl;
use crate::infrastructure::ports::PendingStagingRequest;

/// Pending staging store wrapper for use cases.
pub struct PendingStaging {
    store: Arc<PendingStagingStoreImpl>,
}

impl PendingStaging {
    pub fn new(store: Arc<PendingStagingStoreImpl>) -> Self {
        Self { store }
    }

    pub async fn insert(&self, key: String, request: PendingStagingRequest) {
        self.store.insert(key, request).await;
    }

    pub async fn get(&self, key: &str) -> Option<PendingStagingRequest> {
        self.store.get(key).await
    }

    pub async fn remove(&self, key: &str) -> Option<PendingStagingRequest> {
        self.store.remove(key).await
    }
}
