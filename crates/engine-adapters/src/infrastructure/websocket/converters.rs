//! Type conversion helpers for WebSocket message handling
//!
//! Contains functions to convert between protocol types, domain types, and application DTOs.

use wrldbldr_engine_app::application::dto::{AdHocOutcomesDto, ChallengeOutcomeDecision};
use wrldbldr_engine_app::application::services::challenge_resolution_service as crs;
use wrldbldr_protocol::{ActantialRoleData, ServerMessage, WantVisibilityData};

/// Convert wrldbldr_protocol::DiceInputType to challenge_resolution_service::DiceInputType
pub fn to_service_dice_input(input: wrldbldr_protocol::DiceInputType) -> crs::DiceInputType {
    match input {
        wrldbldr_protocol::DiceInputType::Formula(f) => crs::DiceInputType::Formula(f),
        wrldbldr_protocol::DiceInputType::Manual(v) => crs::DiceInputType::Manual(v),
    }
}

/// Convert wrldbldr_protocol::AdHocOutcomes to application dto AdHocOutcomesDto
pub fn to_adhoc_outcomes_dto(outcomes: wrldbldr_protocol::AdHocOutcomes) -> AdHocOutcomesDto {
    AdHocOutcomesDto {
        success: outcomes.success,
        failure: outcomes.failure,
        critical_success: outcomes.critical_success,
        critical_failure: outcomes.critical_failure,
    }
}

/// Try to deserialize a serde_json::Value into a ServerMessage
pub fn value_to_server_message(value: serde_json::Value) -> Option<ServerMessage> {
    serde_json::from_value(value).ok()
}

/// Convert wire format ChallengeOutcomeDecisionData to application DTO ChallengeOutcomeDecision
pub fn to_challenge_outcome_decision(
    decision: wrldbldr_protocol::ChallengeOutcomeDecisionData,
) -> ChallengeOutcomeDecision {
    match decision {
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Accept => ChallengeOutcomeDecision::Accept,
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Edit {
            modified_description,
        } => ChallengeOutcomeDecision::Edit {
            modified_description,
        },
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Suggest { guidance } => {
            ChallengeOutcomeDecision::Suggest { guidance }
        }
    }
}

// =============================================================================
// Actantial Model Conversion Helpers (P1.5)
// =============================================================================

/// Convert WantVisibilityData to domain WantVisibility
#[allow(dead_code)]
pub fn to_domain_visibility(v: WantVisibilityData) -> wrldbldr_domain::entities::WantVisibility {
    match v {
        WantVisibilityData::Known => wrldbldr_domain::entities::WantVisibility::Known,
        WantVisibilityData::Suspected => wrldbldr_domain::entities::WantVisibility::Suspected,
        WantVisibilityData::Hidden => wrldbldr_domain::entities::WantVisibility::Hidden,
    }
}

/// Convert domain WantVisibility to WantVisibilityData
#[allow(dead_code)]
pub fn from_domain_visibility(v: wrldbldr_domain::entities::WantVisibility) -> WantVisibilityData {
    match v {
        wrldbldr_domain::entities::WantVisibility::Known => WantVisibilityData::Known,
        wrldbldr_domain::entities::WantVisibility::Suspected => WantVisibilityData::Suspected,
        wrldbldr_domain::entities::WantVisibility::Hidden => WantVisibilityData::Hidden,
    }
}

/// Convert ActantialRoleData to domain ActantialRole
#[allow(dead_code)]
pub fn to_domain_role(r: ActantialRoleData) -> wrldbldr_domain::entities::ActantialRole {
    match r {
        ActantialRoleData::Helper => wrldbldr_domain::entities::ActantialRole::Helper,
        ActantialRoleData::Opponent => wrldbldr_domain::entities::ActantialRole::Opponent,
        ActantialRoleData::Sender => wrldbldr_domain::entities::ActantialRole::Sender,
        ActantialRoleData::Receiver => wrldbldr_domain::entities::ActantialRole::Receiver,
    }
}

/// Fetch region items and convert to protocol format
pub async fn fetch_region_items(
    state: &crate::infrastructure::state::AppState,
    region_id: wrldbldr_domain::RegionId,
) -> Vec<wrldbldr_protocol::RegionItemData> {
    use wrldbldr_engine_ports::outbound::RegionRepositoryPort;

    match state.repository.regions().get_region_items(region_id).await {
        Ok(items) => items
            .into_iter()
            .map(|item| wrldbldr_protocol::RegionItemData {
                id: item.id.to_string(),
                name: item.name,
                description: item.description,
                item_type: item.item_type,
            })
            .collect(),
        Err(e) => {
            tracing::warn!(
                region_id = %region_id,
                error = %e,
                "Failed to fetch region items for SceneChanged"
            );
            vec![]
        }
    }
}

// =============================================================================
// GameTime Conversion (Domain -> Protocol)
// =============================================================================

/// Convert domain GameTime to protocol GameTime for wire transfer.
///
/// This is a convenience re-export of `protocol::GameTime::from_domain()`.
/// Prefer using the protocol method directly where possible.
pub fn to_protocol_game_time(game_time: &wrldbldr_domain::GameTime) -> wrldbldr_protocol::GameTime {
    wrldbldr_protocol::GameTime::from_domain(game_time)
}

// =============================================================================
// Movement Result Conversion (Use Case -> Protocol)
// =============================================================================

use wrldbldr_engine_app::application::use_cases::{MovementResult, SelectCharacterResult};
use wrldbldr_engine_ports::outbound::SceneChangedEvent;

/// Convert a MovementResult to a ServerMessage
pub fn movement_result_to_message(result: MovementResult, pc_id: &str) -> ServerMessage {
    match result {
        MovementResult::SceneChanged(event) => scene_changed_event_to_message(event),
        MovementResult::StagingPending { region_id, region_name } => {
            ServerMessage::StagingPending {
                region_id: region_id.to_string(),
                region_name,
            }
        }
        MovementResult::Blocked { reason } => {
            ServerMessage::MovementBlocked {
                pc_id: pc_id.to_string(),
                reason,
            }
        }
    }
}

/// Convert a SelectCharacterResult to a ServerMessage
pub fn select_character_result_to_message(result: SelectCharacterResult) -> ServerMessage {
    ServerMessage::PcSelected {
        pc_id: result.pc_id.to_string(),
        pc_name: result.pc_name,
        location_id: result.location_id.to_string(),
        region_id: result.region_id.map(|r| r.to_string()),
    }
}

/// Convert a SceneChangedEvent to a ServerMessage::SceneChanged
pub fn scene_changed_event_to_message(event: SceneChangedEvent) -> ServerMessage {
    ServerMessage::SceneChanged {
        pc_id: event.pc_id.to_string(),
        region: wrldbldr_protocol::RegionData {
            id: event.region.id.to_string(),
            name: event.region.name,
            location_id: event.region.location_id.to_string(),
            location_name: event.region.location_name,
            backdrop_asset: event.region.backdrop_asset,
            atmosphere: event.region.atmosphere,
            map_asset: event.region.map_asset,
        },
        npcs_present: event.npcs_present.into_iter().map(|n| {
            wrldbldr_protocol::NpcPresenceData {
                character_id: n.character_id.to_string(),
                name: n.name,
                sprite_asset: n.sprite_asset,
                portrait_asset: n.portrait_asset,
            }
        }).collect(),
        navigation: wrldbldr_protocol::NavigationData {
            connected_regions: event.navigation.connected_regions.into_iter().map(|r| {
                wrldbldr_protocol::NavigationTarget {
                    region_id: r.region_id.to_string(),
                    name: r.name,
                    is_locked: r.is_locked,
                    lock_description: r.lock_description,
                }
            }).collect(),
            exits: event.navigation.exits.into_iter().map(|e| {
                wrldbldr_protocol::NavigationExit {
                    location_id: e.location_id.to_string(),
                    location_name: e.location_name,
                    arrival_region_id: e.arrival_region_id.to_string(),
                    description: e.description,
                }
            }).collect(),
        },
        region_items: event.region_items.into_iter().map(|i| {
            wrldbldr_protocol::RegionItemData {
                id: i.item_id.to_string(),
                name: i.name,
                description: i.description,
                item_type: None, // Port type doesn't have item_type
            }
        }).collect(),
    }
}
