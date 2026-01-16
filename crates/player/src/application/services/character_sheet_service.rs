//! Character Sheet Service - Application service for character sheet operations
//!
//! This service provides use case implementations for fetching character sheet
//! schemas and managing character creation using the CharacterSheetRequest protocol.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::application::dto::CharacterSheetSchema;
use crate::application::{get_request_timeout_ms, ParseResponse, ServiceError};
use crate::infrastructure::messaging::CommandBus;
use wrldbldr_protocol::character_sheet::SheetValue;
use wrldbldr_protocol::{CharacterSheetRequest, RequestPayload};

/// Info about a game system
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSystemInfo {
    pub id: String,
    pub name: String,
    pub has_spellcasting: bool,
    pub has_sheet_schema: bool,
}

/// Response from listing systems
#[derive(Clone, Debug, Deserialize)]
pub struct ListSystemsResponse {
    pub systems: Vec<GameSystemInfo>,
}

/// Response from starting character creation
#[derive(Clone, Debug, Deserialize)]
pub struct StartCreationResponse {
    pub character_id: String,
    pub schema: Option<CharacterSheetSchema>,
    pub defaults: HashMap<String, SheetValue>,
}

/// Response from updating a creation field
#[derive(Clone, Debug, Deserialize)]
pub struct UpdateFieldResponse {
    pub field_id: String,
    pub value: SheetValue,
    pub calculated: HashMap<String, SheetValue>,
}

/// Response from completing creation
#[derive(Clone, Debug, Deserialize)]
pub struct CompleteCreationResponse {
    pub character_id: String,
    pub name: String,
    pub status: String,
}

/// Response from getting a character sheet
#[derive(Clone, Debug, Deserialize)]
pub struct GetSheetResponse {
    pub character_id: String,
    pub name: String,
    pub schema: Option<CharacterSheetSchema>,
    pub values: HashMap<String, SheetValue>,
    pub calculated: HashMap<String, SheetValue>,
}

/// Character sheet service for schema and creation operations
///
/// This service provides methods for character sheet operations using the
/// CharacterSheetRequest protocol via the `CommandBus`.
#[derive(Clone)]
pub struct CharacterSheetService {
    commands: CommandBus,
}

impl CharacterSheetService {
    /// Create a new CharacterSheetService with the given command bus
    pub fn new(commands: CommandBus) -> Self {
        Self { commands }
    }

    /// List all available game systems
    pub async fn list_systems(&self) -> Result<Vec<GameSystemInfo>, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::CharacterSheet(CharacterSheetRequest::ListSystems),
                get_request_timeout_ms(),
            )
            .await?;

        let response: ListSystemsResponse = result.parse()?;
        Ok(response.systems)
    }

    /// Get the character sheet schema for a game system
    pub async fn get_schema(&self, system_id: &str) -> Result<CharacterSheetSchema, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::CharacterSheet(CharacterSheetRequest::GetSchema {
                    system_id: system_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Start character creation for a world with a specific game system
    pub async fn start_creation(
        &self,
        world_id: &str,
        system_id: &str,
        name: Option<String>,
    ) -> Result<StartCreationResponse, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::CharacterSheet(CharacterSheetRequest::StartCreation {
                    world_id: world_id.to_string(),
                    system_id: system_id.to_string(),
                    name,
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Update a field during character creation
    pub async fn update_creation_field(
        &self,
        character_id: &str,
        field_id: &str,
        value: SheetValue,
    ) -> Result<UpdateFieldResponse, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::CharacterSheet(CharacterSheetRequest::UpdateCreationField {
                    character_id: character_id.to_string(),
                    field_id: field_id.to_string(),
                    value,
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Complete character creation
    pub async fn complete_creation(
        &self,
        character_id: &str,
    ) -> Result<CompleteCreationResponse, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::CharacterSheet(CharacterSheetRequest::CompleteCreation {
                    character_id: character_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Cancel character creation
    pub async fn cancel_creation(&self, character_id: &str) -> Result<(), ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::CharacterSheet(CharacterSheetRequest::CancelCreation {
                    character_id: character_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse_empty()
    }

    /// Get a character's full sheet with schema and values
    pub async fn get_sheet(&self, character_id: &str) -> Result<GetSheetResponse, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::CharacterSheet(CharacterSheetRequest::GetSheet {
                    character_id: character_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Update a field on an existing character
    pub async fn update_field(
        &self,
        character_id: &str,
        field_id: &str,
        value: SheetValue,
    ) -> Result<UpdateFieldResponse, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::CharacterSheet(CharacterSheetRequest::UpdateField {
                    character_id: character_id.to_string(),
                    field_id: field_id.to_string(),
                    value,
                }),
                get_request_timeout_ms(),
            )
            .await?;

        result.parse()
    }

    /// Recalculate all derived values for a character
    pub async fn recalculate_all(
        &self,
        character_id: &str,
    ) -> Result<HashMap<String, SheetValue>, ServiceError> {
        let result = self
            .commands
            .request_with_timeout(
                RequestPayload::CharacterSheet(CharacterSheetRequest::RecalculateAll {
                    character_id: character_id.to_string(),
                }),
                get_request_timeout_ms(),
            )
            .await?;

        #[derive(Deserialize)]
        struct Response {
            calculated: HashMap<String, SheetValue>,
        }

        let response: Response = result.parse()?;
        Ok(response.calculated)
    }
}
