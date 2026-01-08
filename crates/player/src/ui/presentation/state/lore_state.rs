//! Lore state management
//!
//! Tracks lore entries known to the current player character.

use dioxus::prelude::*;
use std::collections::HashMap;
use wrldbldr_protocol::types::{LoreData, LoreKnowledgeData, LoreSummaryData};

/// Lore state for tracking player knowledge
#[derive(Clone)]
pub struct LoreState {
    /// Lore entries known to the current PC (full data)
    pub known_lore: Signal<Vec<KnownLoreEntry>>,
    /// Lore summaries by character ID (for list views)
    /// Key is character_id, value is list of summaries
    pub lore_summaries: Signal<HashMap<String, Vec<LoreSummaryData>>>,
}

/// A lore entry with knowledge metadata
#[derive(Clone, Debug)]
pub struct KnownLoreEntry {
    /// The full lore data
    pub lore: LoreData,
    /// How and when this was discovered
    pub knowledge: LoreKnowledgeData,
}

impl LoreState {
    /// Create a new LoreState with empty values
    pub fn new() -> Self {
        Self {
            known_lore: Signal::new(Vec::new()),
            lore_summaries: Signal::new(HashMap::new()),
        }
    }

    /// Set lore summaries for a character (from CharacterLoreResponse)
    pub fn set_character_lore_summaries(
        &mut self,
        character_id: String,
        summaries: Vec<LoreSummaryData>,
    ) {
        self.lore_summaries.write().insert(character_id, summaries);
    }

    /// Get lore summaries for a character
    pub fn get_character_lore_summaries(&self, character_id: &str) -> Vec<LoreSummaryData> {
        self.lore_summaries
            .read()
            .get(character_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Add a newly discovered lore entry
    pub fn add_lore(&mut self, lore: LoreData, knowledge: LoreKnowledgeData) {
        let mut entries = self.known_lore.write();

        // Check if we already have this lore (update if so)
        if let Some(existing) = entries.iter_mut().find(|e| e.lore.id == lore.id) {
            existing.lore = lore;
            existing.knowledge = knowledge;
        } else {
            entries.push(KnownLoreEntry { lore, knowledge });
        }
    }

    /// Remove a lore entry (when knowledge is revoked)
    pub fn remove_lore(&mut self, lore_id: &str) {
        let mut entries = self.known_lore.write();
        entries.retain(|e| e.lore.id != lore_id);
    }

    /// Update an existing lore entry
    pub fn update_lore(&mut self, lore: LoreData) {
        let mut entries = self.known_lore.write();
        if let Some(existing) = entries.iter_mut().find(|e| e.lore.id == lore.id) {
            existing.lore = lore;
        }
    }

    /// Get a lore entry by ID
    pub fn get_lore(&self, lore_id: &str) -> Option<KnownLoreEntry> {
        self.known_lore
            .read()
            .iter()
            .find(|e| e.lore.id == lore_id)
            .cloned()
    }

    /// Clear all lore (e.g., on disconnect)
    pub fn clear(&mut self) {
        self.known_lore.write().clear();
        self.lore_summaries.write().clear();
    }

    /// Get lore entries by category
    pub fn get_by_category(&self, category: &str) -> Vec<KnownLoreEntry> {
        self.known_lore
            .read()
            .iter()
            .filter(|e| format!("{:?}", e.lore.category) == category)
            .cloned()
            .collect()
    }
}

impl Default for LoreState {
    fn default() -> Self {
        Self::new()
    }
}
