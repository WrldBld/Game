//! Suggestion Service - Application service for AI-powered content suggestions
//!
//! This service provides use case implementations for enqueuing content suggestions
//! via WebSocket. All suggestions are processed asynchronously through the LLM queue.
//! Results are delivered via WebSocket events (SuggestionCompleted, SuggestionFailed).
//!
//! ## Architecture
//!
//! All suggestion requests are async/queued:
//! 1. Client calls a suggestion method (e.g., `suggest_character_name`)
//! 2. Service sends `EnqueueContentSuggestion` request via WebSocket
//! 3. Engine returns `request_id` immediately
//! 4. Engine processes suggestion via LLM queue
//! 5. Results delivered via WebSocket events (SuggestionCompleted/SuggestionFailed)
//!
//! ## Auto-Enrichment
//!
//! The engine automatically enriches the suggestion context with world data
//! when `world_id` is provided but `world_setting` is not. This provides
//! better suggestion quality without requiring the UI to fetch world data.

use serde::Deserialize;

use crate::application::dto::requests::SuggestionContext;
use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::infrastructure::messaging::CommandBus;
use wrldbldr_protocol::{AiRequest, RequestPayload};

/// Response from queued suggestion (immediate response, results via events)
#[derive(Clone, Debug, Deserialize)]
pub struct SuggestionQueuedResponse {
    pub request_id: String,
    pub status: String,
}

/// Response from cancel suggestion
#[derive(Clone, Debug, Deserialize)]
struct CancelResponse {
    cancelled: bool,
}

/// Suggestion service for enqueuing AI-powered content suggestions
///
/// All suggestions are processed asynchronously through the LLM queue.
/// This service returns a request_id immediately; results are delivered
/// via WebSocket events (SuggestionQueued, SuggestionProgress,
/// SuggestionCompleted, SuggestionFailed).
#[derive(Clone)]
pub struct SuggestionService {
    commands: CommandBus,
}

impl SuggestionService {
    /// Create a new SuggestionService with the given command bus
    pub fn new(commands: CommandBus) -> Self {
        Self { commands }
    }

    // =========================================================================
    // Character Suggestions
    // =========================================================================

    /// Enqueue a character name suggestion request
    ///
    /// Returns a request_id for tracking. Results delivered via WebSocket events.
    pub async fn suggest_character_name(
        &self,
        world_id: &str,
        context: &SuggestionContext,
    ) -> Result<String, ServiceError> {
        self.enqueue_suggestion("character_name", world_id, context)
            .await
    }

    /// Enqueue a character description suggestion request
    ///
    /// Returns a request_id for tracking. Results delivered via WebSocket events.
    pub async fn suggest_character_description(
        &self,
        world_id: &str,
        context: &SuggestionContext,
    ) -> Result<String, ServiceError> {
        self.enqueue_suggestion("character_description", world_id, context)
            .await
    }

    /// Enqueue a character wants suggestion request
    ///
    /// Returns a request_id for tracking. Results delivered via WebSocket events.
    pub async fn suggest_character_wants(
        &self,
        world_id: &str,
        context: &SuggestionContext,
    ) -> Result<String, ServiceError> {
        self.enqueue_suggestion("character_wants", world_id, context)
            .await
    }

    /// Enqueue a character fears suggestion request
    ///
    /// Returns a request_id for tracking. Results delivered via WebSocket events.
    pub async fn suggest_character_fears(
        &self,
        world_id: &str,
        context: &SuggestionContext,
    ) -> Result<String, ServiceError> {
        self.enqueue_suggestion("character_fears", world_id, context)
            .await
    }

    /// Enqueue a character backstory suggestion request
    ///
    /// Returns a request_id for tracking. Results delivered via WebSocket events.
    pub async fn suggest_character_backstory(
        &self,
        world_id: &str,
        context: &SuggestionContext,
    ) -> Result<String, ServiceError> {
        self.enqueue_suggestion("character_backstory", world_id, context)
            .await
    }

    // =========================================================================
    // Location Suggestions
    // =========================================================================

    /// Enqueue a location name suggestion request
    ///
    /// Returns a request_id for tracking. Results delivered via WebSocket events.
    pub async fn suggest_location_name(
        &self,
        world_id: &str,
        context: &SuggestionContext,
    ) -> Result<String, ServiceError> {
        self.enqueue_suggestion("location_name", world_id, context)
            .await
    }

    /// Enqueue a location description suggestion request
    ///
    /// Returns a request_id for tracking. Results delivered via WebSocket events.
    pub async fn suggest_location_description(
        &self,
        world_id: &str,
        context: &SuggestionContext,
    ) -> Result<String, ServiceError> {
        self.enqueue_suggestion("location_description", world_id, context)
            .await
    }

    /// Enqueue a location atmosphere suggestion request
    ///
    /// Returns a request_id for tracking. Results delivered via WebSocket events.
    pub async fn suggest_location_atmosphere(
        &self,
        world_id: &str,
        context: &SuggestionContext,
    ) -> Result<String, ServiceError> {
        self.enqueue_suggestion("location_atmosphere", world_id, context)
            .await
    }

    /// Enqueue a location features suggestion request
    ///
    /// Returns a request_id for tracking. Results delivered via WebSocket events.
    pub async fn suggest_location_features(
        &self,
        world_id: &str,
        context: &SuggestionContext,
    ) -> Result<String, ServiceError> {
        self.enqueue_suggestion("location_features", world_id, context)
            .await
    }

    /// Enqueue a location secrets suggestion request
    ///
    /// Returns a request_id for tracking. Results delivered via WebSocket events.
    pub async fn suggest_location_secrets(
        &self,
        world_id: &str,
        context: &SuggestionContext,
    ) -> Result<String, ServiceError> {
        self.enqueue_suggestion("location_secrets", world_id, context)
            .await
    }

    // =========================================================================
    // Generic Suggestion Methods
    // =========================================================================

    /// Enqueue a suggestion request (generic method)
    ///
    /// This is the core method that all specific suggestion methods delegate to.
    /// Returns the request_id for tracking. Results will be delivered via
    /// WebSocket events (SuggestionCompleted, SuggestionFailed).
    ///
    /// # Arguments
    /// * `suggestion_type` - Type of suggestion (e.g., "character_name", "location_description")
    /// * `world_id` - World ID for routing and context enrichment
    /// * `context` - Context information to help the LLM generate suggestions
    pub async fn enqueue_suggestion(
        &self,
        suggestion_type: &str,
        world_id: &str,
        context: &SuggestionContext,
    ) -> Result<String, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Ai(AiRequest::EnqueueContentSuggestion {
                    world_id: world_id.to_string(),
                    suggestion_type: suggestion_type.to_string(),
                    context: context.clone().into(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        let response: SuggestionQueuedResponse = result.parse()?;
        Ok(response.request_id)
    }

    /// Cancel a pending suggestion request
    ///
    /// Returns true if the request was found and cancelled.
    pub async fn cancel_suggestion(&self, request_id: &str) -> Result<bool, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::Ai(AiRequest::CancelContentSuggestion {
                    request_id: request_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        let response: CancelResponse = result.parse()?;
        Ok(response.cancelled)
    }
}
