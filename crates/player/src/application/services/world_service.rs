//! World Service - Application service for world management
//!
//! This service provides use case implementations for listing, loading,
//! and creating worlds. It uses WebSocket for real-time operations and
//! REST for specific endpoints that remain HTTP-only.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::application::dto::CharacterSheetSchema;
use crate::infrastructure::messaging::CommandBus;
use crate::ports::outbound::{ApiError, RawApiPort};
use wrldbldr_protocol::ErrorCode;
use wrldbldr_protocol::{RequestPayload, WorldRequest};

use crate::application::dto::requests::CreateWorldRequest;
use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};

/// Summary of a world for list views
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// World service for managing worlds
///
/// This service provides methods for world-related operations.
/// Most operations use WebSocket via `CommandBus`, while
/// a few REST-only endpoints use `RawApiPort`.
pub struct WorldService {
    commands: CommandBus,
    api: Arc<dyn RawApiPort>,
}

impl WorldService {
    /// Create a new WorldService with the given command bus and API port
    pub fn new(commands: CommandBus, api: Arc<dyn RawApiPort>) -> Self {
        Self { commands, api }
    }

    /// List all available worlds
    pub async fn list_worlds(&self) -> Result<Vec<WorldSummary>, ServiceError> {
        let value = self
            .api
            .get_json("/api/worlds")
            .await
            .map_err(|e: ApiError| ServiceError::ServerError {
                code: ErrorCode::InternalError,
                message: e.to_string(),
            })?;
        serde_json::from_value(value).map_err(|e| ServiceError::ParseError(e.to_string()))
    }

    /// Get a world by ID (returns basic info)
    pub async fn get_world(&self, id: &str) -> Result<Option<WorldSummary>, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::World(WorldRequest::GetWorld {
                    world_id: id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse_optional()
    }

    /// Create a new world
    ///
    /// # Arguments
    /// * `name` - The name of the world
    /// * `description` - Optional description
    /// * `_rule_system` - Optional rule system configuration (not yet supported via WebSocket)
    ///
    /// # Returns
    /// The ID of the created world
    pub async fn create_world(
        &self,
        name: &str,
        description: Option<&str>,
        _rule_system: Option<serde_json::Value>,
    ) -> Result<String, ServiceError> {
        // Note: rule_system is not yet part of the protocol
        let request = CreateWorldRequest {
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            setting: None,
        };

        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::World(WorldRequest::CreateWorld {
                    data: request.into(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        #[derive(Deserialize)]
        struct CreateResponse {
            id: String,
        }

        let response: CreateResponse = result.parse()?;
        Ok(response.id)
    }

    /// Delete a world by ID
    pub async fn delete_world(&self, id: &str) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::World(WorldRequest::DeleteWorld {
                    world_id: id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse_empty()
    }

    /// Fetch a rule system preset configuration
    ///
    /// This remains a REST call as rule system presets are static configuration.
    ///
    /// # Arguments
    /// * `system_type` - The type (D20, D100, Narrative, Custom)
    /// * `variant` - The specific variant name
    pub async fn get_rule_system_preset(
        &self,
        system_type: &str,
        variant: &str,
    ) -> Result<serde_json::Value, ApiError> {
        let path = format!("/api/rule-systems/{}/presets/{}", system_type, variant);
        self.api.get_json(&path).await
    }

    /// Fetch the character sheet template for a world
    pub async fn get_sheet_template(
        &self,
        world_id: &str,
    ) -> Result<CharacterSheetSchema, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::World(WorldRequest::GetSheetTemplate {
                    world_id: world_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;
        result.parse()
    }
}

impl Clone for WorldService {
    fn clone(&self) -> Self {
        Self {
            commands: self.commands.clone(),
            api: Arc::clone(&self.api),
        }
    }
}
