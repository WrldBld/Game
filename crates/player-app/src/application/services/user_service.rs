//! User identity management service
//!
//! This service handles user identity management, including creating and
//! persisting anonymous user IDs.

use uuid::Uuid;
use wrldbldr_player_ports::outbound::{storage_keys, StorageProvider};

/// Service for managing user identity
///
/// This service abstracts the creation and retrieval of user IDs,
/// ensuring consistent identity across sessions.
pub struct UserService<S: StorageProvider> {
    storage: S,
}

impl<S: StorageProvider> UserService<S> {
    /// Create a new UserService with the given storage provider
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    /// Get or create a stable anonymous user ID.
    ///
    /// This ID is persisted in storage and reused across sessions until local
    /// storage is cleared, effectively acting as an anonymous user identity.
    ///
    /// # Returns
    /// A stable user ID string in the format "user-{uuid}"
    pub fn get_user_id(&self) -> String {
        if let Some(existing) = self.storage.load(storage_keys::USER_ID) {
            return existing;
        }

        let new_id = format!("user-{}", Uuid::new_v4());
        self.storage.save(storage_keys::USER_ID, &new_id);
        new_id
    }

    /// Clear the stored user ID
    ///
    /// This effectively "logs out" the anonymous user, causing a new ID
    /// to be generated on the next call to `get_user_id()`.
    pub fn clear_user_id(&self) {
        self.storage.remove(storage_keys::USER_ID);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    #[derive(Clone, Default)]
    struct MockStorage {
        data: std::sync::Arc<RwLock<HashMap<String, String>>>,
    }

    impl StorageProvider for MockStorage {
        fn save(&self, key: &str, value: &str) {
            self.data
                .write()
                .unwrap()
                .insert(key.to_string(), value.to_string());
        }

        fn load(&self, key: &str) -> Option<String> {
            self.data.read().unwrap().get(key).cloned()
        }

        fn remove(&self, key: &str) {
            self.data.write().unwrap().remove(key);
        }
    }

    #[test]
    fn test_get_user_id_creates_new_id() {
        let storage = MockStorage::default();
        let service = UserService::new(storage.clone());

        let user_id = service.get_user_id();

        assert!(user_id.starts_with("user-"));
        assert!(storage.load(storage_keys::USER_ID).is_some());
    }

    #[test]
    fn test_get_user_id_returns_existing_id() {
        let storage = MockStorage::default();
        storage.save(storage_keys::USER_ID, "user-existing-id");

        let service = UserService::new(storage);
        let user_id = service.get_user_id();

        assert_eq!(user_id, "user-existing-id");
    }

    #[test]
    fn test_get_user_id_is_stable() {
        let storage = MockStorage::default();
        let service = UserService::new(storage);

        let id1 = service.get_user_id();
        let id2 = service.get_user_id();

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_clear_user_id() {
        let storage = MockStorage::default();
        let service = UserService::new(storage.clone());

        let id1 = service.get_user_id();
        service.clear_user_id();
        let id2 = service.get_user_id();

        assert_ne!(id1, id2);
    }
}
