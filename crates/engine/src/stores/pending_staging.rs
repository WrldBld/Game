// Pending staging store - methods for future staging approval
#![allow(dead_code)]

//! Pending staging request storage wrapper.

use std::sync::Arc;

use crate::api::websocket::PendingStagingStoreImpl;
use crate::infrastructure::ports::PendingStagingRequest;

/// Pending staging store wrapper for use cases.
pub struct PendingStagingStore {
    store: Arc<PendingStagingStoreImpl>,
}

impl PendingStagingStore {
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

// Keep the old name as a type alias for backwards compatibility
pub type PendingStaging = PendingStagingStore;
