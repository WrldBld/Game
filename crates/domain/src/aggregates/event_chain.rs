//! EventChain aggregate - Connected sequences of narrative events
//!
//! EventChains link multiple NarrativeEvents together to form story arcs
//! with branching paths and progression tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{ActId, EventChainId, NarrativeEventId, WorldId};
use crate::value_objects::{Description, EventChainName, Tag};
use crate::DomainError;

/// Whether an event chain is currently available for play
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventChainActivation {
    /// Chain is available for triggering events
    #[default]
    Active,
    /// Chain is temporarily disabled
    Inactive,
    /// Chain has been completed and archived (inactive)
    Archived,
}

impl EventChainActivation {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }
}

/// Whether this chain is marked as a favorite for quick access
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventChainFavorite {
    #[default]
    Normal,
    Favorite,
}

impl EventChainFavorite {
    pub fn is_favorite(&self) -> bool {
        matches!(self, Self::Favorite)
    }
}

/// A chain of connected narrative events forming a story arc
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventChain {
    id: EventChainId,
    world_id: WorldId,

    /// Name of this story arc/chain
    name: EventChainName,
    /// Description of what this chain represents narratively
    description: Description,

    /// IDs of events in this chain (ordered by chain_position)
    events: Vec<NarrativeEventId>,

    /// Whether this chain is currently active
    activation: EventChainActivation,
    /// Current position in the chain (index of next event)
    current_position: u32,
    /// Events that have been completed in this chain
    completed_events: Vec<NarrativeEventId>,

    /// Optional: Act this chain belongs to
    act_id: Option<ActId>,

    /// Tags for organization
    tags: Vec<Tag>,

    /// Is this a favorite for quick access
    favorite: EventChainFavorite,

    /// Display color (hex or named), UI-only
    color: Option<String>,

    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl EventChain {
    pub fn new(
        world_id: WorldId,
        name: EventChainName,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id: EventChainId::new(),
            world_id,
            name,
            description: Description::empty(),
            events: Vec::new(),
            activation: EventChainActivation::Active,
            current_position: 0,
            completed_events: Vec::new(),
            act_id: None,
            tags: Vec::new(),
            favorite: EventChainFavorite::Normal,
            color: None,
            created_at: now,
            updated_at: now,
        })
    }

    // Read accessors
    pub fn id(&self) -> EventChainId {
        self.id
    }

    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn name(&self) -> &EventChainName {
        &self.name
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn events(&self) -> &[NarrativeEventId] {
        &self.events
    }

    pub fn activation(&self) -> EventChainActivation {
        self.activation
    }

    pub fn is_active(&self) -> bool {
        self.activation.is_active()
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

    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    pub fn favorite(&self) -> EventChainFavorite {
        self.favorite
    }

    pub fn is_favorite(&self) -> bool {
        self.favorite.is_favorite()
    }

    pub fn color(&self) -> Option<&str> {
        self.color.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // Builder methods
    pub fn with_description(mut self, description: Description) -> Self {
        self.description = description;
        self
    }

    pub fn with_act_id(mut self, act_id: ActId) -> Self {
        self.act_id = Some(act_id);
        self
    }

    pub fn with_tags(mut self, tags: Vec<Tag>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_tag(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self
    }

    pub fn with_favorite(mut self, favorite: EventChainFavorite) -> Self {
        self.favorite = favorite;
        self
    }

    pub fn with_color(mut self, color: Option<String>) -> Self {
        self.color = color;
        self
    }

    pub fn with_active(mut self, activation: EventChainActivation) -> Self {
        self.activation = activation;
        self
    }

    // Setter methods for updating existing chains
    pub fn set_name(
        &mut self,
        name: EventChainName,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.name = name;
        self.updated_at = now;
        Ok(())
    }

    pub fn set_description(
        &mut self,
        description: Description,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.description = description;
        self.updated_at = now;
        Ok(())
    }

    pub fn set_tags(&mut self, tags: Vec<Tag>, now: DateTime<Utc>) {
        self.tags = tags;
        self.updated_at = now;
    }

    pub fn set_favorite(&mut self, favorite: EventChainFavorite, now: DateTime<Utc>) {
        self.favorite = favorite;
        self.updated_at = now;
    }

    pub fn set_color(&mut self, color: Option<String>, now: DateTime<Utc>) {
        self.color = color;
        self.updated_at = now;
    }

    pub fn set_act_id(&mut self, act_id: Option<ActId>, now: DateTime<Utc>) {
        self.act_id = act_id;
        self.updated_at = now;
    }

    /// Reconstruct an EventChain from storage
    ///
    /// This bypasses normal validation since we trust stored data.
    #[allow(clippy::too_many_arguments)]
    pub fn from_storage(
        id: EventChainId,
        world_id: WorldId,
        name: EventChainName,
        description: Description,
        events: Vec<NarrativeEventId>,
        activation: EventChainActivation,
        current_position: u32,
        completed_events: Vec<NarrativeEventId>,
        act_id: Option<ActId>,
        tags: Vec<Tag>,
        favorite: EventChainFavorite,
        color: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            world_id,
            name,
            description,
            events,
            activation,
            current_position,
            completed_events,
            act_id,
            tags,
            favorite,
            color,
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
        self.activation = EventChainActivation::Inactive;
        self.updated_at = now;
    }

    /// Activate the chain
    pub fn activate(&mut self, now: DateTime<Utc>) {
        self.activation = EventChainActivation::Active;
        self.updated_at = now;
    }

    /// Archive the chain (completed and inactive)
    pub fn archive(&mut self, now: DateTime<Utc>) {
        self.activation = EventChainActivation::Archived;
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
            .expect("valid RFC3339 timestamp")
            .with_timezone(&Utc)
    }

    #[test]
    fn test_new() {
        let world_id = test_world_id();
        let now = test_now();
        let chain = EventChain::new(
            world_id,
            EventChainName::new("Test Chain").expect("valid name"),
            now,
        )
        .expect("valid chain");

        assert_eq!(chain.name().to_string(), "Test Chain");
        assert_eq!(chain.world_id(), world_id);
        assert!(chain.is_active());
        assert_eq!(chain.activation(), EventChainActivation::Active);
        assert_eq!(chain.favorite(), EventChainFavorite::Normal);
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
        let mut chain = EventChain::new(
            world_id,
            EventChainName::new("Test").expect("valid name"),
            now,
        )
        .expect("valid chain");
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
    fn test_complete_event() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(
            world_id,
            EventChainName::new("Test").expect("valid name"),
            now,
        )
        .expect("valid chain");
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);
        let event_c = test_event_id(3);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.add_event(event_c, now);

        let later = now + chrono::Duration::seconds(1);
        chain.complete_event(event_a, later);

        assert!(chain.completed_events().contains(&event_a));
        assert_eq!(chain.current_position(), 1); // Should advance to B
        assert_eq!(chain.updated_at(), later);
    }

    #[test]
    fn test_progress() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(
            world_id,
            EventChainName::new("Test").expect("valid name"),
            now,
        )
        .expect("valid chain");
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
    fn test_is_complete() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(
            world_id,
            EventChainName::new("Test").expect("valid name"),
            now,
        )
        .expect("valid chain");
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);

        assert!(!chain.is_complete());

        chain.complete_event(event_a, now);
        assert!(!chain.is_complete());

        chain.complete_event(event_b, now);
        assert!(chain.is_complete());
    }

    #[test]
    fn test_reset() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(
            world_id,
            EventChainName::new("Test").expect("valid name"),
            now,
        )
        .expect("valid chain");
        let event_a = test_event_id(1);

        chain.add_event(event_a, now);
        chain.complete_event(event_a, now);

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
        let mut chain = EventChain::new(
            world_id,
            EventChainName::new("Test").expect("valid name"),
            now,
        )
        .expect("valid chain");

        assert!(chain.is_active());

        let later = now + chrono::Duration::seconds(1);
        chain.deactivate(later);
        assert!(!chain.is_active());
        assert_eq!(chain.activation(), EventChainActivation::Inactive);

        let later2 = later + chrono::Duration::seconds(1);
        chain.activate(later2);
        assert!(chain.is_active());
        assert_eq!(chain.activation(), EventChainActivation::Active);
    }

    #[test]
    fn test_archive() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(
            world_id,
            EventChainName::new("Test").expect("valid name"),
            now,
        )
        .expect("valid chain");

        assert_eq!(chain.activation(), EventChainActivation::Active);

        let later = now + chrono::Duration::seconds(1);
        chain.archive(later);
        assert_eq!(chain.activation(), EventChainActivation::Archived);
        assert!(!chain.is_active());
    }

    #[test]
    fn test_contains_event() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(
            world_id,
            EventChainName::new("Test").expect("valid name"),
            now,
        )
        .expect("valid chain");
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);

        chain.add_event(event_a, now);

        assert!(chain.contains_event(&event_a));
        assert!(!chain.contains_event(&event_b));
    }

    #[test]
    fn test_is_event_completed() {
        let world_id = test_world_id();
        let now = test_now();
        let mut chain = EventChain::new(
            world_id,
            EventChainName::new("Test").expect("valid name"),
            now,
        )
        .expect("valid chain");
        let event_a = test_event_id(1);
        let event_b = test_event_id(2);

        chain.add_event(event_a, now);
        chain.add_event(event_b, now);
        chain.complete_event(event_a, now);

        assert!(chain.is_event_completed(&event_a));
        assert!(!chain.is_event_completed(&event_b));
    }

    #[test]
    fn test_favorite_enum() {
        let world_id = test_world_id();
        let now = test_now();
        let chain = EventChain::new(
            world_id,
            EventChainName::new("Test").expect("valid name"),
            now,
        )
        .expect("valid chain");

        assert_eq!(chain.favorite(), EventChainFavorite::Normal);
        assert!(!chain.is_favorite());

        let chain = chain.with_favorite(EventChainFavorite::Favorite);
        assert_eq!(chain.favorite(), EventChainFavorite::Favorite);
        assert!(chain.is_favorite());

        let mut chain = EventChain::new(
            world_id,
            EventChainName::new("Test").expect("valid name"),
            now,
        )
        .expect("valid chain");
        chain.set_favorite(EventChainFavorite::Favorite, now);
        assert_eq!(chain.favorite(), EventChainFavorite::Favorite);
        assert!(chain.is_favorite());
    }
}
