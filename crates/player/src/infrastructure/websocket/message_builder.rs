//! ClientMessage builder for player adapters
//!
//! Centralizes construction of ClientMessage variants to eliminate duplication
//! between WASM and Desktop adapters. Both adapters share identical message
//! construction logic - only the send mechanism differs.

use wrldbldr_protocol::{
    AdHocOutcomes, ApprovalDecision, ApprovedNpcInfo, ChallengeOutcomeDecisionData, ClientMessage,
    DiceInputType, DirectorialContext, RequestPayload,
};

/// Builder for ClientMessage variants
///
/// Provides a single location for all message construction logic, ensuring
/// consistency between WASM and Desktop adapters.
///
/// # Usage
///
/// ```rust,ignore
/// use crate::infrastructure::infrastructure::websocket::ClientMessageBuilder;
///
/// let msg = ClientMessageBuilder::move_to_region("pc_123", "region_456");
/// self.client.send(msg)?;
/// ```
pub struct ClientMessageBuilder;

impl ClientMessageBuilder {
    // =========================================================================
    // Scene / Directorial Messages
    // =========================================================================

    /// Create a RequestSceneChange message
    pub fn request_scene_change(scene_id: &str) -> ClientMessage {
        ClientMessage::RequestSceneChange {
            scene_id: scene_id.to_string(),
        }
    }

    /// Create a DirectorialUpdate message
    pub fn directorial_update(context: DirectorialContext) -> ClientMessage {
        ClientMessage::DirectorialUpdate { context }
    }

    /// Create an ApprovalDecision message
    pub fn approval_decision(request_id: &str, decision: ApprovalDecision) -> ClientMessage {
        ClientMessage::ApprovalDecision {
            request_id: request_id.to_string(),
            decision,
        }
    }

    // =========================================================================
    // Challenge Messages
    // =========================================================================

    /// Create a ChallengeOutcomeDecision message
    pub fn challenge_outcome_decision(
        resolution_id: &str,
        decision: ChallengeOutcomeDecisionData,
    ) -> ClientMessage {
        ClientMessage::ChallengeOutcomeDecision {
            resolution_id: resolution_id.to_string(),
            decision,
        }
    }

    /// Create a TriggerChallenge message
    pub fn trigger_challenge(challenge_id: &str, target_character_id: &str) -> ClientMessage {
        ClientMessage::TriggerChallenge {
            challenge_id: challenge_id.to_string(),
            target_character_id: target_character_id.to_string(),
        }
    }

    /// Create a ChallengeRoll message (legacy)
    pub fn challenge_roll(challenge_id: &str, roll: i32) -> ClientMessage {
        ClientMessage::ChallengeRoll {
            challenge_id: challenge_id.to_string(),
            roll,
        }
    }

    /// Create a ChallengeRollInput message
    pub fn challenge_roll_input(challenge_id: &str, input: DiceInputType) -> ClientMessage {
        ClientMessage::ChallengeRollInput {
            challenge_id: challenge_id.to_string(),
            input_type: input,
        }
    }

    /// Create a CreateAdHocChallenge message
    pub fn create_adhoc_challenge(
        challenge_name: &str,
        skill_name: &str,
        difficulty: &str,
        target_pc_id: &str,
        outcomes: AdHocOutcomes,
    ) -> ClientMessage {
        ClientMessage::CreateAdHocChallenge {
            challenge_name: challenge_name.to_string(),
            skill_name: skill_name.to_string(),
            difficulty: difficulty.to_string(),
            target_pc_id: target_pc_id.to_string(),
            outcomes,
        }
    }

    // =========================================================================
    // Navigation Messages
    // =========================================================================

    /// Create a MoveToRegion message
    pub fn move_to_region(pc_id: &str, region_id: &str) -> ClientMessage {
        ClientMessage::MoveToRegion {
            pc_id: pc_id.to_string(),
            region_id: region_id.to_string(),
        }
    }

    /// Create an ExitToLocation message
    pub fn exit_to_location(
        pc_id: &str,
        location_id: &str,
        arrival_region_id: Option<&str>,
    ) -> ClientMessage {
        ClientMessage::ExitToLocation {
            pc_id: pc_id.to_string(),
            location_id: location_id.to_string(),
            arrival_region_id: arrival_region_id.map(|s| s.to_string()),
        }
    }

    // =========================================================================
    // Staging Messages
    // =========================================================================

    /// Create a StagingApprovalResponse message
    pub fn staging_approval_response(
        request_id: &str,
        approved_npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
        source: &str,
    ) -> ClientMessage {
        ClientMessage::StagingApprovalResponse {
            request_id: request_id.to_string(),
            approved_npcs,
            ttl_hours,
            source: source.to_string(),
            location_state_id: None, // TODO: Visual state selection not yet implemented in UI
            region_state_id: None,
        }
    }

    /// Create a StagingRegenerateRequest message
    pub fn staging_regenerate_request(request_id: &str, guidance: &str) -> ClientMessage {
        ClientMessage::StagingRegenerateRequest {
            request_id: request_id.to_string(),
            guidance: guidance.to_string(),
        }
    }

    /// Create a PreStageRegion message
    pub fn pre_stage_region(
        region_id: &str,
        npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
    ) -> ClientMessage {
        ClientMessage::PreStageRegion {
            region_id: region_id.to_string(),
            npcs,
            ttl_hours,
            location_state_id: None, // TODO: Visual state selection not yet implemented in UI
            region_state_id: None,
        }
    }

    // =========================================================================
    // Inventory Messages
    // =========================================================================

    /// Create an EquipItem message
    pub fn equip_item(pc_id: &str, item_id: &str) -> ClientMessage {
        ClientMessage::EquipItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        }
    }

    /// Create an UnequipItem message
    pub fn unequip_item(pc_id: &str, item_id: &str) -> ClientMessage {
        ClientMessage::UnequipItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        }
    }

    /// Create a DropItem message
    pub fn drop_item(pc_id: &str, item_id: &str, quantity: u32) -> ClientMessage {
        ClientMessage::DropItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
            quantity,
        }
    }

    /// Create a PickupItem message
    pub fn pickup_item(pc_id: &str, item_id: &str) -> ClientMessage {
        ClientMessage::PickupItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        }
    }

    // =========================================================================
    // Utility Messages
    // =========================================================================

    /// Create a Heartbeat message
    pub fn heartbeat() -> ClientMessage {
        ClientMessage::Heartbeat
    }

    /// Create a CheckComfyUIHealth message
    pub fn check_comfyui_health() -> ClientMessage {
        ClientMessage::CheckComfyUIHealth
    }

    // =========================================================================
    // Request-Wrapped Messages (with auto-generated request_id)
    // =========================================================================

    /// Create a SetNpcDisposition request message
    ///
    /// Automatically generates a UUID for the request_id.
    pub fn set_npc_disposition(
        npc_id: &str,
        pc_id: &str,
        disposition: &str,
        reason: Option<&str>,
    ) -> ClientMessage {
        ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload: RequestPayload::SetNpcDisposition {
                npc_id: npc_id.to_string(),
                pc_id: pc_id.to_string(),
                disposition: disposition.to_string(),
                reason: reason.map(|s| s.to_string()),
            },
        }
    }

    /// Create a SetNpcRelationship request message
    ///
    /// Automatically generates a UUID for the request_id.
    pub fn set_npc_relationship(npc_id: &str, pc_id: &str, relationship: &str) -> ClientMessage {
        ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload: RequestPayload::SetNpcRelationship {
                npc_id: npc_id.to_string(),
                pc_id: pc_id.to_string(),
                relationship: relationship.to_string(),
            },
        }
    }

    /// Create a GetNpcDispositions request message
    ///
    /// Automatically generates a UUID for the request_id.
    pub fn get_npc_dispositions(pc_id: &str) -> ClientMessage {
        ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload: RequestPayload::GetNpcDispositions {
                pc_id: pc_id.to_string(),
            },
        }
    }

    // =========================================================================
    // Time Control Messages (DM only)
    // =========================================================================

    /// Create a SetGameTime message
    ///
    /// Note: world_id should be provided by the caller from session state.
    /// Pass empty string if not available - handler should fill from session.
    pub fn set_game_time(world_id: &str, day: u32, hour: u8) -> ClientMessage {
        ClientMessage::SetGameTime {
            world_id: world_id.to_string(),
            day,
            hour,
            notify_players: true,
        }
    }

    /// Create a SkipToPeriod message
    ///
    /// Note: world_id should be provided by the caller from session state.
    pub fn skip_to_period(world_id: &str, period: &str) -> ClientMessage {
        ClientMessage::SkipToPeriod {
            world_id: world_id.to_string(),
            period: period.to_string(),
        }
    }

    /// Create a RespondToTimeSuggestion message
    pub fn respond_to_time_suggestion(
        suggestion_id: &str,
        decision: &str,
        modified_minutes: Option<u32>,
    ) -> ClientMessage {
        use wrldbldr_protocol::types::TimeSuggestionDecision;

        let decision = match decision {
            "approve" => TimeSuggestionDecision::Approve,
            "modify" => TimeSuggestionDecision::Modify {
                minutes: modified_minutes.unwrap_or(0),
            },
            "skip" => TimeSuggestionDecision::Skip,
            _ => TimeSuggestionDecision::Skip,
        };

        ClientMessage::RespondToTimeSuggestion {
            suggestion_id: suggestion_id.to_string(),
            decision,
        }
    }

    /// Create an AdvanceGameTimeMinutes request message
    ///
    /// Note: world_id should be provided by the caller from session state.
    /// Pass empty string if not available - handler should fill from session.
    pub fn advance_time(world_id: &str, minutes: u32, reason: &str) -> ClientMessage {
        ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload: RequestPayload::AdvanceGameTimeMinutes {
                world_id: world_id.to_string(),
                minutes,
                reason: Some(reason.to_string()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_protocol::ApprovalDecision;

    #[test]
    fn test_move_to_region() {
        let msg = ClientMessageBuilder::move_to_region("pc_123", "region_456");
        match msg {
            ClientMessage::MoveToRegion { pc_id, region_id } => {
                assert_eq!(pc_id, "pc_123");
                assert_eq!(region_id, "region_456");
            }
            _ => panic!("Expected MoveToRegion message"),
        }
    }

    #[test]
    fn test_exit_to_location_with_arrival() {
        let msg = ClientMessageBuilder::exit_to_location("pc_123", "loc_456", Some("region_789"));
        match msg {
            ClientMessage::ExitToLocation {
                pc_id,
                location_id,
                arrival_region_id,
            } => {
                assert_eq!(pc_id, "pc_123");
                assert_eq!(location_id, "loc_456");
                assert_eq!(arrival_region_id, Some("region_789".to_string()));
            }
            _ => panic!("Expected ExitToLocation message"),
        }
    }

    #[test]
    fn test_exit_to_location_without_arrival() {
        let msg = ClientMessageBuilder::exit_to_location("pc_123", "loc_456", None);
        match msg {
            ClientMessage::ExitToLocation {
                arrival_region_id, ..
            } => {
                assert_eq!(arrival_region_id, None);
            }
            _ => panic!("Expected ExitToLocation message"),
        }
    }

    #[test]
    fn test_approval_decision() {
        let msg = ClientMessageBuilder::approval_decision("req_123", ApprovalDecision::Accept);
        match msg {
            ClientMessage::ApprovalDecision {
                request_id,
                decision,
            } => {
                assert_eq!(request_id, "req_123");
                assert!(matches!(decision, ApprovalDecision::Accept));
            }
            _ => panic!("Expected ApprovalDecision message"),
        }
    }

    #[test]
    fn test_heartbeat() {
        let msg = ClientMessageBuilder::heartbeat();
        assert!(matches!(msg, ClientMessage::Heartbeat));
    }

    #[test]
    fn test_set_npc_disposition_generates_request_id() {
        let msg = ClientMessageBuilder::set_npc_disposition("npc_1", "pc_1", "friendly", None);
        match msg {
            ClientMessage::Request {
                request_id,
                payload,
            } => {
                // Request ID should be a valid UUID
                assert!(uuid::Uuid::parse_str(&request_id).is_ok());
                assert!(matches!(payload, RequestPayload::SetNpcDisposition { .. }));
            }
            _ => panic!("Expected Request message"),
        }
    }
}
