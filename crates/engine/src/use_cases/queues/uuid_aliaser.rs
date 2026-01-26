//! UUID aliasing for LLM prompts.
//!
//! UUIDs in LLM prompts consume tokens and can confuse models. This module
//! provides bidirectional mapping between UUIDs and short aliases like
//! `CHAL_0`, `EVT_0`, etc.
//!
//! # Usage
//!
//! ```rust,ignore
//! let mut aliaser = UuidAliaser::new();
//!
//! // Alias UUIDs before sending to LLM
//! let alias = aliaser.alias_challenge("550e8400-e29b-41d4-a716-446655440000");
//! assert_eq!(alias, "CHAL_0");
//!
//! // Same UUID returns same alias
//! let alias2 = aliaser.alias_challenge("550e8400-e29b-41d4-a716-446655440000");
//! assert_eq!(alias2, "CHAL_0");
//!
//! // De-alias LLM response back to UUID
//! let uuid = aliaser.dealias("CHAL_0");
//! assert_eq!(uuid, Some("550e8400-e29b-41d4-a716-446655440000".to_string()));
//! ```

use std::collections::HashMap;

/// Bidirectional UUID <-> alias mapper for LLM prompts.
///
/// Used within a single request scope to replace verbose UUIDs with short
/// aliases before sending to LLM, then de-alias the response.
#[derive(Debug, Clone, Default)]
pub struct UuidAliaser {
    /// UUID -> alias mapping
    uuid_to_alias: HashMap<String, String>,
    /// alias -> UUID mapping (reverse lookup)
    alias_to_uuid: HashMap<String, String>,
    /// Counter for challenge aliases
    challenge_counter: usize,
    /// Counter for event aliases
    event_counter: usize,
}

impl UuidAliaser {
    /// Create a new empty aliaser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Alias a challenge ID. Returns "CHAL_0", "CHAL_1", etc.
    ///
    /// If the UUID has already been aliased, returns the existing alias.
    pub fn alias_challenge(&mut self, id: &str) -> String {
        if let Some(alias) = self.uuid_to_alias.get(id) {
            return alias.clone();
        }

        let alias = format!("CHAL_{}", self.challenge_counter);
        self.challenge_counter += 1;
        self.uuid_to_alias.insert(id.to_string(), alias.clone());
        self.alias_to_uuid.insert(alias.clone(), id.to_string());
        alias
    }

    /// Alias an event ID. Returns "EVT_0", "EVT_1", etc.
    ///
    /// If the UUID has already been aliased, returns the existing alias.
    pub fn alias_event(&mut self, id: &str) -> String {
        if let Some(alias) = self.uuid_to_alias.get(id) {
            return alias.clone();
        }

        let alias = format!("EVT_{}", self.event_counter);
        self.event_counter += 1;
        self.uuid_to_alias.insert(id.to_string(), alias.clone());
        self.alias_to_uuid.insert(alias.clone(), id.to_string());
        alias
    }

    /// De-alias back to original UUID. Returns None if alias not found.
    pub fn dealias(&self, alias: &str) -> Option<String> {
        self.alias_to_uuid.get(alias).cloned()
    }

    /// Get all mappings (for storing in cassette).
    ///
    /// Returns the UUID -> alias mapping. The alias -> UUID mapping can be
    /// reconstructed from this.
    pub fn mappings(&self) -> &HashMap<String, String> {
        &self.uuid_to_alias
    }

    /// Restore from stored mappings (for cassette playback).
    ///
    /// Reconstructs both forward and reverse mappings, and sets counters
    /// appropriately to continue aliasing without collisions.
    pub fn from_mappings(mappings: HashMap<String, String>) -> Self {
        let mut aliaser = Self::new();

        // Find max counters from existing aliases
        let mut max_challenge = 0usize;
        let mut max_event = 0usize;

        for (uuid, alias) in &mappings {
            aliaser.uuid_to_alias.insert(uuid.clone(), alias.clone());
            aliaser.alias_to_uuid.insert(alias.clone(), uuid.clone());

            // Parse alias to update counters
            if let Some(num_str) = alias.strip_prefix("CHAL_") {
                if let Ok(num) = num_str.parse::<usize>() {
                    max_challenge = max_challenge.max(num + 1);
                }
            } else if let Some(num_str) = alias.strip_prefix("EVT_") {
                if let Ok(num) = num_str.parse::<usize>() {
                    max_event = max_event.max(num + 1);
                }
            }
        }

        aliaser.challenge_counter = max_challenge;
        aliaser.event_counter = max_event;

        aliaser
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alias_challenge() {
        let mut aliaser = UuidAliaser::new();

        let alias1 = aliaser.alias_challenge("550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(alias1, "CHAL_0");

        let alias2 = aliaser.alias_challenge("550e8400-e29b-41d4-a716-446655440001");
        assert_eq!(alias2, "CHAL_1");

        let alias3 = aliaser.alias_challenge("550e8400-e29b-41d4-a716-446655440002");
        assert_eq!(alias3, "CHAL_2");
    }

    #[test]
    fn test_alias_event() {
        let mut aliaser = UuidAliaser::new();

        let alias1 = aliaser.alias_event("660e8400-e29b-41d4-a716-446655440000");
        assert_eq!(alias1, "EVT_0");

        let alias2 = aliaser.alias_event("660e8400-e29b-41d4-a716-446655440001");
        assert_eq!(alias2, "EVT_1");
    }

    #[test]
    fn test_same_uuid_returns_same_alias() {
        let mut aliaser = UuidAliaser::new();

        let alias1 = aliaser.alias_challenge("550e8400-e29b-41d4-a716-446655440000");
        let alias2 = aliaser.alias_challenge("550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(alias1, alias2);
        assert_eq!(alias1, "CHAL_0");

        // Counter should not have advanced
        let alias3 = aliaser.alias_challenge("550e8400-e29b-41d4-a716-446655440001");
        assert_eq!(alias3, "CHAL_1");
    }

    #[test]
    fn test_dealias() {
        let mut aliaser = UuidAliaser::new();

        aliaser.alias_challenge("550e8400-e29b-41d4-a716-446655440000");
        aliaser.alias_event("660e8400-e29b-41d4-a716-446655440000");

        assert_eq!(
            aliaser.dealias("CHAL_0"),
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
        assert_eq!(
            aliaser.dealias("EVT_0"),
            Some("660e8400-e29b-41d4-a716-446655440000".to_string())
        );
        assert_eq!(aliaser.dealias("CHAL_99"), None);
        assert_eq!(aliaser.dealias("UNKNOWN"), None);
    }

    #[test]
    fn test_round_trip() {
        let mut aliaser = UuidAliaser::new();

        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let alias = aliaser.alias_challenge(uuid);
        let recovered = aliaser.dealias(&alias);

        assert_eq!(recovered, Some(uuid.to_string()));
    }

    #[test]
    fn test_mixed_challenges_and_events() {
        let mut aliaser = UuidAliaser::new();

        // Interleave challenge and event aliasing
        let c1 = aliaser.alias_challenge("challenge-uuid-1");
        let e1 = aliaser.alias_event("event-uuid-1");
        let c2 = aliaser.alias_challenge("challenge-uuid-2");
        let e2 = aliaser.alias_event("event-uuid-2");

        assert_eq!(c1, "CHAL_0");
        assert_eq!(e1, "EVT_0");
        assert_eq!(c2, "CHAL_1");
        assert_eq!(e2, "EVT_1");

        // All should dealias correctly
        assert_eq!(
            aliaser.dealias("CHAL_0"),
            Some("challenge-uuid-1".to_string())
        );
        assert_eq!(
            aliaser.dealias("CHAL_1"),
            Some("challenge-uuid-2".to_string())
        );
        assert_eq!(aliaser.dealias("EVT_0"), Some("event-uuid-1".to_string()));
        assert_eq!(aliaser.dealias("EVT_1"), Some("event-uuid-2".to_string()));
    }

    #[test]
    fn test_mappings() {
        let mut aliaser = UuidAliaser::new();

        aliaser.alias_challenge("uuid-1");
        aliaser.alias_event("uuid-2");

        let mappings = aliaser.mappings();
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings.get("uuid-1"), Some(&"CHAL_0".to_string()));
        assert_eq!(mappings.get("uuid-2"), Some(&"EVT_0".to_string()));
    }

    #[test]
    fn test_from_mappings() {
        let mut original = UuidAliaser::new();
        original.alias_challenge("uuid-1");
        original.alias_challenge("uuid-2");
        original.alias_event("uuid-3");

        // Reconstruct from mappings
        let restored = UuidAliaser::from_mappings(original.mappings().clone());

        // Should have same dealias behavior
        assert_eq!(restored.dealias("CHAL_0"), Some("uuid-1".to_string()));
        assert_eq!(restored.dealias("CHAL_1"), Some("uuid-2".to_string()));
        assert_eq!(restored.dealias("EVT_0"), Some("uuid-3".to_string()));
    }

    #[test]
    fn test_from_mappings_continues_counters() {
        let mut original = UuidAliaser::new();
        original.alias_challenge("uuid-1");
        original.alias_challenge("uuid-2");
        original.alias_event("uuid-3");

        // Reconstruct from mappings
        let mut restored = UuidAliaser::from_mappings(original.mappings().clone());

        // New aliases should continue from where we left off
        let new_challenge = restored.alias_challenge("uuid-4");
        let new_event = restored.alias_event("uuid-5");

        assert_eq!(new_challenge, "CHAL_2"); // Continues after CHAL_1
        assert_eq!(new_event, "EVT_1"); // Continues after EVT_0
    }

    #[test]
    fn test_empty_aliaser() {
        let aliaser = UuidAliaser::new();

        assert!(aliaser.mappings().is_empty());
        assert_eq!(aliaser.dealias("CHAL_0"), None);
    }

    #[test]
    fn test_from_empty_mappings() {
        let restored = UuidAliaser::from_mappings(HashMap::new());

        assert!(restored.mappings().is_empty());

        // Should work normally for new aliases
        let mut restored = restored;
        assert_eq!(restored.alias_challenge("uuid"), "CHAL_0");
    }
}
