//! Generation State - Track asset generation queue and status
//!
//! Manages the state of ComfyUI asset generation batches and LLM suggestions,
//! including queue tracking, progress updates, and ready results.

use dioxus::prelude::*;

/// Status of a generation batch
#[derive(Debug, Clone, PartialEq)]
pub enum BatchStatus {
    /// Batch is waiting in queue
    Queued { position: u32 },
    /// Batch is currently generating
    Generating { progress: u8 },
    /// Batch is ready for selection
    Ready { asset_count: u32 },
    /// Batch generation failed
    Failed { error: String },
}

/// Status of a suggestion request
#[derive(Debug, Clone, PartialEq)]
pub enum SuggestionStatus {
    /// Suggestion is queued
    Queued,
    /// Suggestion is being processed
    Processing,
    /// Suggestion is ready with results
    Ready { suggestions: Vec<String> },
    /// Suggestion failed
    Failed { error: String },
}

/// A generation batch in the queue (for images)
#[derive(Debug, Clone, PartialEq)]
pub struct GenerationBatch {
    pub batch_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub asset_type: String,
    pub status: BatchStatus,
    pub is_read: bool,
}

/// A suggestion task in the queue (for text suggestions)
#[derive(Debug, Clone, PartialEq)]
pub struct SuggestionTask {
    pub request_id: String,
    pub field_type: String,
    pub entity_id: Option<String>,
    pub status: SuggestionStatus,
    pub is_read: bool,
    /// Original context for retry (stored when task is created)
    pub context: Option<wrldbldr_player_app::application::services::suggestion_service::SuggestionContext>,
    /// World ID for routing (needed for retries)
    pub world_id: Option<String>,
}

/// State for managing asset generation and suggestions
#[derive(Clone, Copy)]
pub struct GenerationState {
    /// All tracked batches (for images)
    batches: Signal<Vec<GenerationBatch>>,
    /// All tracked suggestion tasks (for text)
    suggestions: Signal<Vec<SuggestionTask>>,
    /// Whether there are batches ready for selection
    has_ready_batches: Signal<bool>,
    /// Whether there are suggestions ready for selection
    has_ready_suggestions: Signal<bool>,
}

impl GenerationState {
    /// Create a new generation state
    pub fn new() -> Self {
        Self {
            batches: Signal::new(Vec::new()),
            suggestions: Signal::new(Vec::new()),
            has_ready_batches: Signal::new(false),
            has_ready_suggestions: Signal::new(false),
        }
    }

    /// Add a new batch to the queue
    pub fn add_batch(&mut self, batch: GenerationBatch) {
        self.batches.write().push(batch);
        self.update_ready_flag();
    }

    /// Update batch status when queued
    pub fn batch_queued(
        &mut self,
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        position: u32,
    ) {
        let batch = GenerationBatch {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            status: BatchStatus::Queued { position },
            is_read: false,
        };
        self.add_batch(batch);
    }

    /// Update batch progress
    pub fn batch_progress(&mut self, batch_id: &str, progress: u8) {
        let mut batches = self.batches.write();
        if let Some(batch) = batches.iter_mut().find(|b| b.batch_id == batch_id) {
            batch.status = BatchStatus::Generating { progress };
        }
    }

    /// Mark batch as complete
    pub fn batch_complete(&mut self, batch_id: &str, asset_count: u32) {
        {
            let mut batches = self.batches.write();
            if let Some(batch) = batches.iter_mut().find(|b| b.batch_id == batch_id) {
                batch.status = BatchStatus::Ready { asset_count };
            }
        }
        self.update_ready_flag();
    }

    /// Mark batch as failed
    pub fn batch_failed(&mut self, batch_id: &str, error: String) {
        let mut batches = self.batches.write();
        if let Some(batch) = batches.iter_mut().find(|b| b.batch_id == batch_id) {
            batch.status = BatchStatus::Failed { error };
        }
    }

    /// Remove a batch (after selection or dismissal)
    pub fn remove_batch(&mut self, batch_id: &str) {
        self.batches.write().retain(|b| b.batch_id != batch_id);
        self.update_ready_flag();
    }

    /// Get all batches
    pub fn get_batches(&self) -> Vec<GenerationBatch> {
        self.batches.read().clone()
    }

    /// Get batches for a specific entity
    pub fn get_batches_for_entity(&self, entity_type: &str, entity_id: &str) -> Vec<GenerationBatch> {
        self.batches
            .read()
            .iter()
            .filter(|b| b.entity_type == entity_type && b.entity_id == entity_id)
            .cloned()
            .collect()
    }

    /// Get ready batches
    pub fn get_ready_batches(&self) -> Vec<GenerationBatch> {
        self.batches
            .read()
            .iter()
            .filter(|b| matches!(b.status, BatchStatus::Ready { .. }))
            .cloned()
            .collect()
    }

    /// Check if there are ready batches
    pub fn has_ready(&self) -> bool {
        *self.has_ready_batches.read()
    }

    /// Get count of active (queued or generating) batches
    pub fn active_count(&self) -> usize {
        self.batches
            .read()
            .iter()
            .filter(|b| {
                matches!(
                    b.status,
                    BatchStatus::Queued { .. } | BatchStatus::Generating { .. }
                )
            })
            .count()
    }

    fn update_ready_flag(&mut self) {
        let has_ready = self
            .batches
            .read()
            .iter()
            .any(|b| matches!(b.status, BatchStatus::Ready { .. }));
        self.has_ready_batches.set(has_ready);
        
        let has_ready_suggestions = self
            .suggestions
            .read()
            .iter()
            .any(|s| matches!(s.status, SuggestionStatus::Ready { .. }));
        self.has_ready_suggestions.set(has_ready_suggestions);
    }

    // ========== Suggestion methods ==========

    /// Add a new suggestion task to the queue
    pub fn add_suggestion_task(
        &mut self,
        request_id: String,
        field_type: String,
        entity_id: Option<String>,
        context: Option<wrldbldr_player_app::application::services::suggestion_service::SuggestionContext>,
        world_id: Option<String>,
    ) {
        let task = SuggestionTask {
            request_id,
            field_type,
            entity_id,
            status: SuggestionStatus::Queued,
            is_read: false,
            context,
            world_id,
        };
        self.suggestions.write().push(task);
        self.update_ready_flag();
    }

    /// Update suggestion status when queued
    pub fn suggestion_queued(
        &mut self,
        request_id: String,
        field_type: String,
        entity_id: Option<String>,
    ) {
        let needs_update = {
            let mut suggestions = self.suggestions.write();
            if let Some(task) = suggestions.iter_mut().find(|s| s.request_id == request_id) {
                task.status = SuggestionStatus::Queued;
                false
            } else {
                // Add if not found (context will be None if not provided)
                suggestions.push(SuggestionTask {
                    request_id,
                    field_type,
                    entity_id,
                    status: SuggestionStatus::Queued,
                    is_read: false,
                    context: None,
                    world_id: None, // Not available when receiving queued event from server
                });
                true
            }
        };
        if needs_update {
            self.update_ready_flag();
        }
    }

    /// Update suggestion progress
    pub fn suggestion_progress(&mut self, request_id: &str, _status: &str) {
        let mut suggestions = self.suggestions.write();
        if let Some(task) = suggestions.iter_mut().find(|s| s.request_id == request_id) {
            task.status = SuggestionStatus::Processing;
        }
    }

    /// Mark suggestion as complete
    pub fn suggestion_complete(&mut self, request_id: &str, suggestions: Vec<String>) {
        let needs_update = {
            let mut tasks = self.suggestions.write();
            if let Some(task) = tasks.iter_mut().find(|s| s.request_id == request_id) {
                task.status = SuggestionStatus::Ready { suggestions };
                true
            } else {
                false
            }
        };
        if needs_update {
            self.update_ready_flag();
        }
    }

    /// Mark suggestion as failed
    pub fn suggestion_failed(&mut self, request_id: &str, error: String) {
        let mut suggestions = self.suggestions.write();
        if let Some(task) = suggestions.iter_mut().find(|s| s.request_id == request_id) {
            task.status = SuggestionStatus::Failed { error };
        }
    }

    /// Remove a suggestion task (after selection or dismissal)
    pub fn remove_suggestion(&mut self, request_id: &str) {
        self.suggestions.write().retain(|s| s.request_id != request_id);
        self.update_ready_flag();
    }

    /// Get all suggestion tasks
    pub fn get_suggestions(&self) -> Vec<SuggestionTask> {
        self.suggestions.read().clone()
    }

    /// Get ready suggestions
    pub fn get_ready_suggestions(&self) -> Vec<SuggestionTask> {
        self.suggestions
            .read()
            .iter()
            .filter(|s| matches!(s.status, SuggestionStatus::Ready { .. }))
            .cloned()
            .collect()
    }

    /// Check if there are ready suggestions
    pub fn has_ready_suggestions(&self) -> bool {
        *self.has_ready_suggestions.read()
    }

    /// Get count of active (queued or processing) suggestions
    pub fn active_suggestion_count(&self) -> usize {
        self.suggestions
            .read()
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    SuggestionStatus::Queued | SuggestionStatus::Processing
                )
            })
            .count()
    }

    /// Mark a batch as read
    pub fn mark_batch_read(&mut self, batch_id: &str) {
        let mut batches = self.batches.write();
        if let Some(batch) = batches.iter_mut().find(|b| b.batch_id == batch_id) {
            batch.is_read = true;
        }
    }

    /// Mark a suggestion as read
    pub fn mark_suggestion_read(&mut self, request_id: &str) {
        let mut suggestions = self.suggestions.write();
        if let Some(task) = suggestions.iter_mut().find(|s| s.request_id == request_id) {
            task.is_read = true;
        }
    }

    /// Clear all batches and suggestions (used when hydrating from snapshot)
    pub fn clear(&mut self) {
        self.batches.set(Vec::new());
        self.suggestions.set(Vec::new());
        self.has_ready_batches.set(false);
        self.has_ready_suggestions.set(false);
    }
}

impl Default for GenerationState {
    fn default() -> Self {
        Self::new()
    }
}
