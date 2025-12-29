//! Movement handlers
//!
//! Thin handlers for PC movement between regions and locations.
//! All business logic is delegated to MovementUseCase.

use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::infrastructure::websocket::converters::{
    movement_result_to_message, select_character_result_to_message,
};
use crate::infrastructure::websocket::IntoServerError;
use wrldbldr_domain::{LocationId, PlayerCharacterId, RegionId, WorldId};
use wrldbldr_engine_app::application::use_cases::{
    ExitToLocationInput, MoveToRegionInput, SelectCharacterInput, UseCaseContext,
};
use wrldbldr_protocol::ServerMessage;

// =============================================================================
// SelectPlayerCharacter Handler
// =============================================================================

/// Handles a request to select a player character for play.
///
/// Delegates to `MovementUseCase::select_character` which:
/// 1. Validates the PC exists
/// 2. Returns the PC's current position information
pub async fn handle_select_player_character(
    state: &AppState,
    client_id: Uuid,
    pc_id: String,
) -> Option<ServerMessage> {
    tracing::debug!(pc_id = %pc_id, "SelectPlayerCharacter request received");

    // Extract context (we only need the world_id for context, but select_character doesn't use it)
    let ctx = extract_context(state, client_id).await?;

    // Parse pc_id
    let pc_uuid = parse_pc_id(&pc_id)?;

    let input = SelectCharacterInput { pc_id: pc_uuid };

    match state.use_cases.movement.select_character(ctx, input).await {
        Ok(result) => {
            tracing::info!(
                client_id = %client_id,
                pc_id = %pc_id,
                pc_name = %result.pc_name,
                "Player selected character"
            );
            Some(select_character_result_to_message(result))
        }
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// MoveToRegion Handler
// =============================================================================

/// Handles a request to move a player character to a different region.
///
/// Delegates to `MovementUseCase::move_to_region` which:
/// 1. Validates the connection and PC
/// 2. Checks for locked region connections
/// 3. Updates PC position in the database
/// 4. Handles the staging system workflow
pub async fn handle_move_to_region(
    state: &AppState,
    client_id: Uuid,
    pc_id: String,
    region_id: String,
    _sender: tokio::sync::mpsc::Sender<ServerMessage>,
) -> Option<ServerMessage> {
    tracing::debug!(
        pc_id = %pc_id,
        region_id = %region_id,
        "MoveToRegion request received"
    );

    // Extract context
    let ctx = extract_context(state, client_id).await?;

    // Parse IDs
    let pc_uuid = parse_pc_id(&pc_id)?;
    let region_uuid = parse_region_id(&region_id)?;

    let input = MoveToRegionInput {
        pc_id: pc_uuid,
        target_region_id: region_uuid,
    };

    match state.use_cases.movement.move_to_region(ctx, input).await {
        Ok(result) => Some(movement_result_to_message(result, &pc_id)),
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// ExitToLocation Handler
// =============================================================================

/// Handles a request to exit to a different location.
///
/// Delegates to `MovementUseCase::exit_to_location` which:
/// 1. Validates the connection and PC
/// 2. Determines the arrival region
/// 3. Updates PC position (location and region)
/// 4. Handles the staging system workflow
pub async fn handle_exit_to_location(
    state: &AppState,
    client_id: Uuid,
    pc_id: String,
    location_id: String,
    arrival_region_id: Option<String>,
    _sender: tokio::sync::mpsc::Sender<ServerMessage>,
) -> Option<ServerMessage> {
    tracing::debug!(
        pc_id = %pc_id,
        location_id = %location_id,
        arrival_region_id = ?arrival_region_id,
        "ExitToLocation request received"
    );

    // Extract context
    let ctx = extract_context(state, client_id).await?;

    // Parse IDs
    let pc_uuid = parse_pc_id(&pc_id)?;
    let location_uuid = parse_location_id(&location_id)?;
    let arrival_uuid = arrival_region_id
        .as_ref()
        .and_then(|id| parse_region_id(id));

    let input = ExitToLocationInput {
        pc_id: pc_uuid,
        target_location_id: location_uuid,
        arrival_region_id: arrival_uuid,
    };

    match state.use_cases.movement.exit_to_location(ctx, input).await {
        Ok(result) => Some(movement_result_to_message(result, &pc_id)),
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Extract UseCaseContext from connection state
async fn extract_context(state: &AppState, client_id: Uuid) -> Option<UseCaseContext> {
    let conn = state
        .world_connection_manager
        .get_connection_by_client_id(&client_id.to_string())
        .await?;

    let world_id = conn.world_id?;

    Some(UseCaseContext {
        world_id: WorldId::from_uuid(world_id),
        user_id: conn.user_id.clone(),
        is_dm: conn.is_dm(),
        pc_id: conn.pc_id.map(PlayerCharacterId::from_uuid),
    })
}

fn parse_pc_id(id: &str) -> Option<PlayerCharacterId> {
    Uuid::parse_str(id).ok().map(PlayerCharacterId::from_uuid)
}

fn parse_region_id(id: &str) -> Option<RegionId> {
    Uuid::parse_str(id).ok().map(RegionId::from_uuid)
}

fn parse_location_id(id: &str) -> Option<LocationId> {
    Uuid::parse_str(id).ok().map(LocationId::from_uuid)
}
