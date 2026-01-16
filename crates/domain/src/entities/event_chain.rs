//! EventChain entity - Connected sequences of narrative events
//!
//! EventChains link multiple NarrativeEvents together to form story arcs
//! with branching paths and progression tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use wrldbldr_domain::{ActId, EventChainId, NarrativeEventId, WorldId};

/// A chain of connected narrative events forming a story arc
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventChain {
    id: EventChainId,
    world_id: WorldId,

    /// Name of this story arc/chain
    name: String,
    /// Description of what this chain represents narratively
    description: String,

    /// IDs of events in this chain (ordered by chain_position)
    events: Vec<NarrativeEventId>,

    /// Whether this chain is currently active
    is_active: bool,
    /// Current position in the chain (index of next event)
    current_position: u32,
    /// Events that have been completed in this chain
    completed_events: Vec<NarrativeEventId>,

    /// Optional: Act this chain belongs to
    act_id: Option<ActId>,

    /// Tags for organization
    tags: Vec<String>,

    /// Color for visualization (hex)
    color: Option<String>,

    /// Is this a favorite for quick access
    is_favorite: bool,

    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl EventChain {
    pub fn new(world_id: WorldId, name: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            id: EventChainId::new(),
            world_id,
            name: name.into(),
            description: String::new(),
            events: Vec::new(),
            is_active: true,
            current_position: 0,
            completed_events: Vec::new(),
            act_id: None,
            tags: Vec::new(),
            color: None,
            is_favorite: false,
            created_at: now,
            updated_at: now,
        }
    }

    // Read accessors
    pub fn id(&self) -> EventChainId {
        self.id
    }

    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn events(&self) -> &[NarrativeEventId] {
        &self.events
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn current_position(&self) -> u32 {
        self.current_position
    }

    pub fn completed_events(&self) -> &[NarrativeEventId] {
        &self.completed_events
    }

    pub fn act_id(&self) -> Option<ActId> {
        self.act_id
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn color(&self) -> Option<&str> {
        self.color.as_deref()
    }

    pub fn is_favorite(&self) -> bool {
        self.is_favorite
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // Builder methods
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_act_id(mut self, act_id: ActId) -> Self {
        self.act_id = Some(act_id);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn with_favorite(mut self, is_favorite: bool) -> Self {
        self.is_favorite = is_favorite;
        self
    }

    pub fn with_active(mut self, is_active: bool) -> Self {
        self.is_active = is_active;
        self
    }

    // Setter methods for updating existing chains
    pub fn set_name(&mut self, name: impl Into<String>, now: DateTime<Utc>) {
        self.name = name.into();
        self.updated_at = now;
    }

    pub fn set_description(&mut self, description: impl Into<String>, now: DateTime<Utc>) {
        self.description = description.into();
        self.updated_at = now;
    }

    pub fn set_tags(&mut self, tags: Vec<String>, now: DateTime<Utc>) {
        self.tags = tags;
        self.updated_at = now;
    }

    pub fn set_color(&mut self, color: Option<String>, now: DateTime<Utc>) {
        self.color = color;
        self.updated_at = now;
    }

    pub fn set_favorite(&mut self, is_favorite: bool, now: DateTime<Utc>) {
        self.is_favorite = is_favorite;
        self.updated_at = now;
    }

    pub fn set_act_id(&mut self, act_id: Option<ActId>, now: DateTime<Utc>) {
        self.act_id = act_id;
        self.updated_at = now;
    }

    /// Reconstruct an EventChain from stored parts (for repository deserialization).
    ///
    /// This bypasses normal validation since we trust the stored data.
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: EventChainId,
        world_id: WorldId,
        name: String,
        description: String,
        events: Vec<NarrativeEventId>,
        is_active: bool,
        current_position: u32,
        completed_events: Vec<NarrativeEventId>,
        act_id: Option<ActId>,
        tags: Vec<String>,
        color: Option<String>,
        is_favorite: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            world_id,
            name,
            description,
            events,
            is_active,
            current_position,
            completed_events,
            act_id,
            tags,
            color,
            is_favorite,
            created_at,
            updated_at,
        }
    }

    /// Add an event to the chain
    pub fn add_event(&mut self, event_id: NarrativeEventId, now: DateTime<Utc>) {
        self.events.push(event_id);
        self.updated_at = now;
    }

    /// Add an event at a specific position
    pub fn insert_event(
        &mut self,
        position: usize,
        event_id: NarrativeEventId,
        now: DateTime<Utc>,
    ) {
        if position <= self.events.len() {
            self.events.insert(position, event_id);
            // Adjust current_position if inserting at or before it
            // to maintain the same semantic event reference
            if position <= self.current_position as usize {
                self.current_position += 1;
            }
            self.updated_at = now;
        }
    }

    /// Remove an event from the chain
    pub fn remove_event(&mut self, event_id: &NarrativeEventId, now: DateTime<Utc>) -> bool {
        // Find the position of the event before removing it
        let position = self.events.iter().position(|e| e == event_id);

        let original_len = self.events.len();
        self.events.retain(|e| e != event_id);
        // Also remove from completed_events to prevent stale entries
        self.completed_events.retain(|e| e != event_id);

        if self.events.len() != original_len {
            // Adjust current_position if we removed an event at or before it
            if let Some(pos) = position {
                if pos < self.current_position as usize {
                    // Event was before current_position, so decrement to maintain reference
                    self.current_position -= 1;
                } else if pos == self.current_position as usize {
                    // Event was at current_position, next event moves into that position
                    // current_position stays the same (points to what was next)
                }
                // If pos > current_position, no adjustment needed
            }
            self.updated_at = now;
            true
        } else {
            false
        }
    }

    /// Reorder events in the chain
    ///
    /// Note: This replaces the entire events vector. If the current event
    /// is not in the new order, current_position will be clamped to valid range.
    pub fn reorder_events(&mut self, event_ids: Vec<NarrativeEventId>, now: DateTime<Utc>) {
        // Clamp current_position to valid range after reordering
        let max_position = event_ids.len().saturating_sub(1) as u32;
        self.current_position = self.current_position.min(max_position);

        self.events = event_ids;
        self.updated_at = now;
    }

    /// Mark an event as completed
    pub fn complete_event(&mut self, event_id: NarrativeEventId, now: DateTime<Utc>) {
        if !self.completed_events.contains(&event_id) {
            self.completed_events.push(event_id);
        }

        // Advance current position if this was the current event
        if let Some(pos) = self.events.iter().position(|e| *e == event_id) {
            if pos as u32 == self.current_position {
                self.current_position = (pos as u32) + 1;
            }
        }

        self.updated_at = now;
    }

    /// Get the current event in the chain (if any)
    pub fn current_event(&self) -> Option<&NarrativeEventId> {
        self.events.get(self.current_position as usize)
    }

    /// Get the next event after current (if any)
    pub fn next_event(&self) -> Option<&NarrativeEventId> {
        self.events.get((self.current_position + 1) as usize)
    }

    /// Check if the chain is complete
    pub fn is_complete(&self) -> bool {
        self.current_position as usize >= self.events.len()
    }

    /// Get progress as a fraction (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.events.is_empty() {
            0.0
        } else {
            self.completed_events.len() as f32 / self.events.len() as f32
        }
    }

    /// Get progress as percentage string
    pub fn progress_string(&self) -> String {
        format!("{}%", (self.progress() * 100.0) as u32)
    }

    /// Get remaining events count
    pub fn remaining_events(&self) -> usize {
        self.events.len() - self.completed_events.len()
    }

    /// Reset the chain to the beginning
    pub fn reset(&mut self, now: DateTime<Utc>) {
        self.current_position = 0;
        self.completed_events.clear();
        self.updated_at = now;
    }

    /// Deactivate the chain
    pub fn deactivate(&mut self, now: DateTime<Utc>) {
        self.is_active = false;
        self.updated_at = now;
    }

    /// Activate the chain
    pub fn activate(&mut self, now: DateTime<Utc>) {
        self.is_active = true;
        self.updated_at = now;
    }

    /// Check if an event is in this chain
    pub fn contains_event(&self, event_id: &NarrativeEventId) -> bool {
        self.events.contains(event_id)
    }

    /// Get the position of an event in this chain
    pub fn event_position(&self, event_id: &NarrativeEventId) -> Option<usize> {
        self.events.iter().position(|e| e == event_id)
    }

    /// Check if an event has been completed in this chain
    pub fn is_event_completed(&self, event_id: &NarrativeEventId) -> bool {
        self.completed_events.contains(event_id)
    }
}

/// Summary information about a chain's state
///
/// This is a status/display DTO with no invariants - uses public fields per ADR-008.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainStatus {
    pub chain_id: EventChainId,
    pub chain_name: String,
    pub is_active: bool,
    pub is_complete: bool,
    pub total_events: usize,
    pub completed_events: usize,
    pub progress_percent: u32,
    pub current_event_id: Option<NarrativeEventId>,
}

impl From<&EventChain> for ChainStatus {
    fn from(chain: &EventChain) -> Self {
        Self {
            chain_id: chain.id(),
            chain_name: chain.name().to_string(),
            is_active: chain.is_active(),
            is_complete: chain.is_complete(),
            total_events: chain.events().len(),
            completed_events: chain.completed_events().len(),
            progress_percent: (chain.progress() * 100.0) as u32,
            current_event_id: chain.current_event().copied(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn test_world_id() -> WorldId {
        WorldId::from_uuid(Uuid::nil())
    }

    fn test_event_id(n: u8) -> NarrativeEventId {
        NarrativeEventId::from_uuid(Uuid::from_bytes([
            n, n, n, n, n, n, n, n, n, n, n, n, n, n, n, n,
        ]))
    }

    fn test_now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn test_new() {
        let world_id = test_world_id();
        let now = test_now();
        let chain = EventChain::new(world_id, "Test Chain", now);

        assert_eq!(chain.name(), "Test Chain");
        assert_eq!(chain.world_id(), world_id);
        assert!(chain.is_active());
        assert_eq!(chain.current_position(), 0);
        assert!(chain.events().is_empty());
        assert!(chain.completed_events().is_empty());
        assert_eq!(chain.created_at(), now);
        assert_eq!(chain.updated_at(), now);
    }

    #[test]
    fn test_add_event() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event1 = test_event_id(1);
        let event2 = test_event_id(2);

        let later = now + chrono::Duration::seconds(1);
        chain.add_event(event1, later);
        assert_eq!(chain.events().len(), 1);
        assert_eq!(chain.events()[0], event1);
        assert_eq!(chain.updated_at(), later);

        chain.add_event(event2, later);
        assert_eq!(chain.events().len(), 2);
        assert_eq!(chain.events()[1], event2);
    }

    #[test]
    fn test_insert_event_at_beginning() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);
        let event_x = test_event_id(10);

        // Setup: [A, B, C] with current_position = 1 (pointing to B)
        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);
        chain.current_position = 1;

        // Insert X at position 0: [X, A, B, C]
        chain.insert_event(0, event_x, now);

        assert_eq!(chain.events().len(), 4);
        assert_eq!(chain.events()[0], event_x);
        assert_eq!(chain.events()[1], event_a);
        assert_eq!(chain.events()[2], event_b);
        assert_eq!(chain.events()[3], event_c);
        // current_position should be adjusted to 2 (still pointing to B)
        assert_eq!(chain.current_position(), 2);
    }

    #[test]
    fn test_insert_event_at_current_position() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);
        let event_x = test_event_id(10);

        // Setup: [A, B, C] with current_position = 1 (pointing to B)
        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);
        chain.current_position = 1;

        // Insert X at position 1: [A, X, B, C]
        chain.insert_event(1, event_x, now);

        assert_eq!(chain.events().len(), 4);
        assert_eq!(chain.events()[0], event_a);
        assert_eq!(chain.events()[1], event_x);
        assert_eq!(chain.events()[2], event_b);
        assert_eq!(chain.events()[3], event_c);
        // current_position should be adjusted to 2 (still pointing to B)
        assert_eq!(chain.current_position(), 2);
    }

    #[test]
    fn test_insert_event_after_current_position() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);
        let event_x = test_event_id(10);

        // Setup: [A, B, C] with current_position = 1 (pointing to B)
        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);
        chain.current_position = 1;

        // Insert X at position 2: [A, B, X, C]
        chain.insert_event(2, event_x, now);

        assert_eq!(chain.events().len(), 4);
        assert_eq!(chain.events()[0], event_a);
        assert_eq!(chain.events()[1], event_b);
        assert_eq!(chain.events()[2], event_x);
        assert_eq!(chain.events()[3], event_c);
        // current_position should remain 1 (still pointing to B)
        assert_eq!(chain.current_position(), 1);
    }

    #[test]
    fn test_insert_event_at_end() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_x = test_event_id(10);

        // Setup: [A, B] with current_position = 0 (pointing to A)
        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.current_position = 0;

        // Insert X at end: [A, B, X]
        chain.insert_event(2, event_x, now);

        assert_eq!(chain.events().len(), 3);
        assert_eq!(chain.events()[2], event_x);
        // current_position should remain 0 (still pointing to A)
        assert_eq!(chain.current_position(), 0);
    }

    #[test]
    fn test_insert_event_out_of_bounds() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_x = test_event_id(10);

        chain.add_event(event_a, now);
        let original_len = chain.events().len();

        // Try to insert beyond bounds
        chain.insert_event(10, event_x, now);

        // Should not insert
        assert_eq!(chain.events().len(), original_len);
    }

    #[test]
    fn test_remove_event() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);

        let later = now + chrono::Duration::seconds(1);
        let removed = chain.remove_event(&event_b, later);

        assert!(removed);
        assert_eq!(chain.events().len(), 2);
        assert_eq!(chain.events()[0], event_a);
        assert_eq!(chain.events()[1], event_c);
        assert!(!chain.contains_event(&event_b));
        assert_eq!(chain.updated_at(), later);
    }

    #[test]
    fn test_remove_event_not_found() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);

        chain.add_event(event_a, now);
        let original_updated = chain.updated_at();

        let removed = chain.remove_event(&event_b, now);

        assert!(!removed);
        assert_eq!(chain.events().len(), 1);
        assert_eq!(chain.updated_at(), original_updated);
    }

    #[test]
    fn test_remove_event_also_removes_from_completed() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.complete_event(event_b, now);

        chain.remove_event(&event_b, now);

        assert!(!chain.completed_events().contains(&event_b));
    }

    #[test]
    fn test_remove_event_before_current_position() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);

        // Setup: [A, B, C] with current_position = 2 (pointing to C)
        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);
        chain.current_position = 2;

        // Remove A at position 0: [B, C]
        chain.remove_event(&event_a, now);

        assert_eq!(chain.events().len(), 2);
        assert_eq!(chain.events()[0], event_b);
        assert_eq!(chain.events()[1], event_c);
        // current_position should be adjusted to 1 (still pointing to C)
        assert_eq!(chain.current_position(), 1);
    }

    #[test]
    fn test_remove_event_at_current_position() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);

        // Setup: [A, B, C] with current_position = 1 (pointing to B)
        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);
        chain.current_position = 1;

        // Remove B at position 1: [A, C]
        chain.remove_event(&event_b, now);

        assert_eq!(chain.events().len(), 2);
        assert_eq!(chain.events()[0], event_a);
        assert_eq!(chain.events()[1], event_c);
        // current_position should stay at 1 (now pointing to C, which moved into that position)
        assert_eq!(chain.current_position(), 1);
    }

    #[test]
    fn test_remove_event_after_current_position() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);

        // Setup: [A, B, C] with current_position = 0 (pointing to A)
        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);
        chain.current_position = 0;

        // Remove C at position 2: [A, B]
        chain.remove_event(&event_c, now);

        assert_eq!(chain.events().len(), 2);
        assert_eq!(chain.events()[0], event_a);
        assert_eq!(chain.events()[1], event_b);
        // current_position should remain 0 (still pointing to A)
        assert_eq!(chain.current_position(), 0);
    }

    #[test]
    fn test_reorder_events() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);

        let later = now + chrono::Duration::seconds(1);
        chain.reorder_events(vec![event_c, event_a, event_b], later);

        assert_eq!(chain.events().len(), 3);
        assert_eq!(chain.events()[0], event_c);
        assert_eq!(chain.events()[1], event_a);
        assert_eq!(chain.events()[2], event_b);
        assert_eq!(chain.updated_at(), later);
    }

    #[test]
    fn test_reorder_events_clamps_current_position() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.current_position = 5; // Out of bounds

        // Reorder to smaller list
        chain.reorder_events(vec![event_b], now);

        // current_position should be clamped to valid range (0 for 1-element list)
        assert_eq!(chain.current_position(), 0);
    }

    #[test]
    fn test_reorder_events_empty_list() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);

        chain.add_event(event_a, now);
        chain.current_position = 0;

        // Reorder to empty list
        chain.reorder_events(vec![], now);

        // current_position should be clamped to 0
        assert_eq!(chain.current_position(), 0);
        assert!(chain.events().is_empty());
    }

    #[test]
    fn test_complete_event() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);
        chain.current_position = 1; // Pointing to B

        let later = now + chrono::Duration::seconds(1);
        chain.complete_event(event_b, later);

        assert!(chain.completed_events().contains(&event_b));
        assert_eq!(chain.current_position(), 2); // Should advance to C
        assert_eq!(chain.updated_at(), later);
    }

    #[test]
    fn test_complete_event_not_current() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);
        chain.current_position = 1; // Pointing to B

        chain.complete_event(event_a, now);

        assert!(chain.completed_events().contains(&event_a));
        assert_eq!(chain.current_position(), 1); // Should not change
    }

    #[test]
    fn test_complete_event_duplicate() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);

        chain.add_event(event_a, now);
        chain.complete_event(event_a, now);

        let completed_count = chain.completed_events().len();
        chain.complete_event(event_a, now);

        // Should not add duplicate
        assert_eq!(chain.completed_events().len(), completed_count);
    }

    #[test]
    fn test_current_event() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.current_position = 0;

        assert_eq!(chain.current_event(), Some(&event_a));

        chain.current_position = 1;
        assert_eq!(chain.current_event(), Some(&event_b));

        chain.current_position = 2;
        assert_eq!(chain.current_event(), None);
    }

    #[test]
    fn test_next_event() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);
        chain.current_position = 0;

        assert_eq!(chain.next_event(), Some(&event_b));

        chain.current_position = 1;
        assert_eq!(chain.next_event(), Some(&event_c));

        chain.current_position = 2;
        assert_eq!(chain.next_event(), None);
    }

    #[test]
    fn test_is_complete() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);

        chain.current_position = 0;
        assert!(!chain.is_complete());

        chain.current_position = 1;
        assert!(!chain.is_complete());

        chain.current_position = 2;
        assert!(chain.is_complete());

        chain.current_position = 5; // Beyond bounds
        assert!(chain.is_complete());
    }

    #[test]
    fn test_is_complete_empty() {
        let world_id = test_world_id();
        let now = test_now();
        let chain = EventChain::new(world_id, "Test", now);

        assert!(chain.is_complete());
    }

    #[test]
    fn test_progress() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);

        assert_eq!(chain.progress(), 0.0);

        chain.complete_event(event_a, now);
        assert_eq!(chain.progress(), 1.0 / 3.0);

        chain.complete_event(event_b, now);
        assert_eq!(chain.progress(), 2.0 / 3.0);

        chain.complete_event(event_c, now);
        assert_eq!(chain.progress(), 1.0);
    }

    #[test]
    fn test_progress_empty() {
        let world_id = test_world_id();
        let now = test_now();
        let chain = EventChain::new(world_id, "Test", now);

        assert_eq!(chain.progress(), 0.0);
    }

    #[test]
    fn test_progress_string() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.complete_event(event_a, now);

        assert_eq!(chain.progress_string(), "50%");
    }

    #[test]
    fn test_remaining_events() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);

        assert_eq!(chain.remaining_events(), 3);

        chain.complete_event(event_a, now);
        assert_eq!(chain.remaining_events(), 2);
    }

    #[test]
    fn test_reset() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.current_position = 2;
        chain.complete_event(event_a, now);
        chain.complete_event(event_b, now);

        let later = now + chrono::Duration::seconds(1);
        chain.reset(later);

        assert_eq!(chain.current_position(), 0);
        assert!(chain.completed_events().is_empty());
        assert_eq!(chain.updated_at(), later);
    }

    #[test]
    fn test_activate_deactivate() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);

        assert!(chain.is_active());

        let later = now + chrono::Duration::seconds(1);
        chain.deactivate(later);
        assert!(!chain.is_active());
        assert_eq!(chain.updated_at(), later);

        let later2 = later + chrono::Duration::seconds(1);
        chain.activate(later2);
        assert!(chain.is_active());
        assert_eq!(chain.updated_at(), later2);
    }

    #[test]
    fn test_contains_event() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);

        chain.add_event(event_a, now);

        assert!(chain.contains_event(&event_a));
        assert!(!chain.contains_event(&event_b));
    }

    #[test]
    fn test_event_position() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);

        assert_eq!(chain.event_position(&event_a), Some(0));
        assert_eq!(chain.event_position(&event_b), Some(1));
        assert_eq!(chain.event_position(&event_c), Some(2));

        let event_d = test_event_id(4);
        assert_eq!(chain.event_position(&event_d), None);
    }

    #[test]
    fn test_is_event_completed() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.complete_event(event_a, now);

        assert!(chain.is_event_completed(&event_a));
        assert!(!chain.is_event_completed(&event_b));
    }

    #[test]
    fn test_chain_status_from() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test Chain", now);
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.current_position = 1;
        chain.complete_event(event_a, now);

        let status = ChainStatus::from(&chain);

        assert_eq!(status.chain_id, chain.id());
        assert_eq!(status.chain_name, "Test Chain");
        assert!(status.is_active);
        assert!(!status.is_complete);
        assert_eq!(status.total_events, 2);
        assert_eq!(status.completed_events, 1);
        assert_eq!(status.progress_percent, 50);
        assert_eq!(status.current_event_id, Some(event_b));
    }

    #[test]
    fn test_chain_status_complete() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(world_id, "Test", now);
        let event_a = test_event_id(1);

        chain.add_event(event_a, now);
        chain.current_position = 1; // Beyond last event

        let status = ChainStatus::from(&chain);

        assert!(status.is_complete);
        assert_eq!(status.current_event_id, None);
    }
}
