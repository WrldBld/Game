//! Suggestion Service - Application service for AI-powered content suggestions
//!
//! This service provides use case implementations for fetching content suggestions
//! from the Engine API. It abstracts away the HTTP client details from the
//! presentation layer.

use serde::{Deserialize, Serialize};

use wrldbldr_player_ports::outbound::{ApiError, ApiPort};

/// Context for generating suggestions
#[derive(Clone, Default, Debug, PartialEq, Serialize)]
pub struct SuggestionContext {
    /// Type of entity (e.g., "character", "location", "tavern", "forest")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    /// Name of the entity (if already set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_name: Option<String>,
    /// World/setting name or type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_setting: Option<String>,
    /// Hints or keywords to guide generation (e.g., archetype)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<String>,
    /// Additional context from other fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Response from suggestion API (synchronous)
#[derive(Clone, Debug, Deserialize)]
pub struct SuggestionResponse {
    pub suggestions: Vec<String>,
}

/// Response from queued suggestion API
#[derive(Clone, Debug, Deserialize)]
pub struct SuggestionQueuedResponse {
    pub request_id: String,
    pub status: String,
}

/// Suggestion service for fetching AI-powered content suggestions
///
/// This service provides methods for suggestion-related operations
/// while depending only on the `ApiPort` trait, not concrete
/// infrastructure implementations.
pub struct SuggestionService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> SuggestionService<A> {
    /// Create a new SuggestionService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    /// Get character name suggestions
    pub async fn suggest_character_name(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>, ApiError> {
        let response: SuggestionResponse = self
            .api
            .post("/api/suggest/character/name", context)
            .await?;
        Ok(response.suggestions)
    }

    /// Get character description suggestions
    pub async fn suggest_character_description(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>, ApiError> {
        let response: SuggestionResponse = self
            .api
            .post("/api/suggest/character/description", context)
            .await?;
        Ok(response.suggestions)
    }

    /// Get character wants suggestions
    pub async fn suggest_character_wants(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>, ApiError> {
        let response: SuggestionResponse = self
            .api
            .post("/api/suggest/character/wants", context)
            .await?;
        Ok(response.suggestions)
    }

    /// Get character fears suggestions
    pub async fn suggest_character_fears(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>, ApiError> {
        let response: SuggestionResponse = self
            .api
            .post("/api/suggest/character/fears", context)
            .await?;
        Ok(response.suggestions)
    }

    /// Get character backstory suggestions
    pub async fn suggest_character_backstory(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>, ApiError> {
        let response: SuggestionResponse = self
            .api
            .post("/api/suggest/character/backstory", context)
            .await?;
        Ok(response.suggestions)
    }

    /// Get location name suggestions
    pub async fn suggest_location_name(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>, ApiError> {
        let response: SuggestionResponse = self
            .api
            .post("/api/suggest/location/name", context)
            .await?;
        Ok(response.suggestions)
    }

    /// Get location description suggestions
    pub async fn suggest_location_description(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>, ApiError> {
        let response: SuggestionResponse = self
            .api
            .post("/api/suggest/location/description", context)
            .await?;
        Ok(response.suggestions)
    }

    /// Get location atmosphere suggestions
    pub async fn suggest_location_atmosphere(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>, ApiError> {
        let response: SuggestionResponse = self
            .api
            .post("/api/suggest/location/atmosphere", context)
            .await?;
        Ok(response.suggestions)
    }

    /// Get location features suggestions
    pub async fn suggest_location_features(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>, ApiError> {
        let response: SuggestionResponse = self
            .api
            .post("/api/suggest/location/features", context)
            .await?;
        Ok(response.suggestions)
    }

    /// Get location secrets suggestions
    pub async fn suggest_location_secrets(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>, ApiError> {
        let response: SuggestionResponse = self
            .api
            .post("/api/suggest/location/secrets", context)
            .await?;
        Ok(response.suggestions)
    }

    /// Enqueue a suggestion request (async, returns request_id)
    /// 
    /// This method queues the suggestion request instead of waiting for results.
    /// Results will be delivered via WebSocket events.
    /// 
    /// The `world_id` is required for routing the response back to the correct
    /// clients (all sessions viewing this world).
    pub async fn enqueue_suggestion(
        &self,
        field_type: &str,
        world_id: &str,
        context: &SuggestionContext,
    ) -> Result<String, ApiError> {
        #[derive(Serialize)]
        struct UnifiedRequest {
            #[serde(rename = "suggestion_type")]
            suggestion_type: String,
            world_id: String,
            #[serde(flatten)]
            context: SuggestionContext,
        }
        
        let request = UnifiedRequest {
            suggestion_type: field_type.to_string(),
            world_id: world_id.to_string(),
            context: context.clone(),
        };
        
        let response: SuggestionQueuedResponse = self
            .api
            .post("/api/suggest", &request)
            .await?;
        Ok(response.request_id)
    }

    /// Cancel a pending suggestion request
    pub async fn cancel_suggestion(&self, request_id: &str) -> Result<(), ApiError> {
        self.api.delete(&format!("/api/suggest/{}/cancel", request_id)).await
    }
}

impl<A: ApiPort + Clone> Clone for SuggestionService<A> {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}
