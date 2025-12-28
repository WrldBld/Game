//! Message dispatch for WebSocket handlers
//!
//! Routes incoming ClientMessage variants to their respective handler modules.

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::infrastructure::state::AppState;
use wrldbldr_protocol::{ClientMessage, ServerMessage};

use super::handlers::{
    challenge, connection, inventory, misc, movement, narrative, player_action, request, scene,
    staging,
};

/// Dispatch a parsed client message to the appropriate handler
///
/// This is the main entry point for WebSocket message handling.
/// Each ClientMessage variant is routed to its corresponding handler module.
pub async fn handle_message(
    msg: ClientMessage,
    state: &AppState,
    client_id: Uuid,
    sender: mpsc::UnboundedSender<ServerMessage>,
) -> Option<ServerMessage> {
    match msg {
        // Connection handlers
        ClientMessage::Heartbeat => connection::handle_heartbeat(),

        ClientMessage::JoinWorld {
            world_id,
            role,
            pc_id,
            spectate_pc_id,
        } => connection::handle_join_world(state, client_id, world_id, role, pc_id, spectate_pc_id).await,

        ClientMessage::LeaveWorld => connection::handle_leave_world(state, client_id).await,

        ClientMessage::SetSpectateTarget { pc_id } => {
            connection::handle_set_spectate_target(state, client_id, pc_id).await
        }

        // Player action handler
        ClientMessage::PlayerAction {
            action_type,
            target,
            dialogue,
        } => {
            player_action::handle_player_action(state, client_id, action_type, target, dialogue, sender)
                .await
        }

        // Scene handlers
        ClientMessage::RequestSceneChange { scene_id } => {
            scene::handle_request_scene_change(state, client_id, scene_id).await
        }

        ClientMessage::DirectorialUpdate { context } => {
            scene::handle_directorial_update(state, client_id, context).await
        }

        ClientMessage::ApprovalDecision {
            request_id,
            decision,
        } => scene::handle_approval_decision(state, client_id, request_id, decision).await,

        // Challenge handlers
        ClientMessage::ChallengeRoll { challenge_id, roll } => {
            challenge::handle_challenge_roll(state, client_id, challenge_id, roll).await
        }

        ClientMessage::ChallengeRollInput {
            challenge_id,
            input_type,
        } => challenge::handle_challenge_roll_input(state, client_id, challenge_id, input_type).await,

        ClientMessage::TriggerChallenge {
            challenge_id,
            target_character_id,
        } => {
            challenge::handle_trigger_challenge(state, client_id, challenge_id, target_character_id)
                .await
        }

        ClientMessage::ChallengeSuggestionDecision {
            request_id,
            approved,
            modified_difficulty,
        } => {
            challenge::handle_challenge_suggestion_decision(
                state,
                client_id,
                request_id,
                approved,
                modified_difficulty,
            )
            .await
        }

        ClientMessage::RegenerateOutcome {
            request_id,
            outcome_type,
            guidance,
        } => challenge::handle_regenerate_outcome(state, client_id, request_id, outcome_type, guidance).await,

        ClientMessage::DiscardChallenge {
            request_id,
            feedback,
        } => challenge::handle_discard_challenge(state, client_id, request_id, feedback).await,

        ClientMessage::CreateAdHocChallenge {
            challenge_name,
            skill_name,
            difficulty,
            target_pc_id,
            outcomes,
        } => {
            challenge::handle_create_adhoc_challenge(
                state,
                client_id,
                challenge_name,
                skill_name,
                difficulty,
                target_pc_id,
                outcomes,
            )
            .await
        }

        ClientMessage::ChallengeOutcomeDecision {
            resolution_id,
            decision,
        } => {
            challenge::handle_challenge_outcome_decision(state, client_id, resolution_id, decision)
                .await
        }

        ClientMessage::RequestOutcomeSuggestion {
            resolution_id,
            guidance,
        } => {
            challenge::handle_request_outcome_suggestion(state, client_id, resolution_id, guidance)
                .await
        }

        ClientMessage::RequestOutcomeBranches {
            resolution_id,
            guidance,
        } => {
            challenge::handle_request_outcome_branches(state, client_id, resolution_id, guidance)
                .await
        }

        ClientMessage::SelectOutcomeBranch {
            resolution_id,
            branch_id,
            modified_description,
        } => {
            challenge::handle_select_outcome_branch(
                state,
                client_id,
                resolution_id,
                branch_id,
                modified_description,
            )
            .await
        }

        // Narrative handlers
        ClientMessage::NarrativeEventSuggestionDecision {
            request_id,
            event_id,
            approved,
            selected_outcome,
        } => {
            narrative::handle_narrative_event_suggestion_decision(
                state,
                client_id,
                request_id,
                event_id,
                approved,
                selected_outcome,
            )
            .await
        }

        // Movement handlers
        ClientMessage::SelectPlayerCharacter { pc_id } => {
            movement::handle_select_player_character(state, client_id, pc_id).await
        }

        ClientMessage::MoveToRegion { pc_id, region_id } => {
            movement::handle_move_to_region(state, client_id, pc_id, region_id, sender).await
        }

        ClientMessage::ExitToLocation {
            pc_id,
            location_id,
            arrival_region_id,
        } => {
            movement::handle_exit_to_location(
                state,
                client_id,
                pc_id,
                location_id,
                arrival_region_id,
                sender,
            )
            .await
        }

        // Staging handlers
        ClientMessage::StagingApprovalResponse {
            request_id,
            approved_npcs,
            ttl_hours,
            source,
        } => {
            staging::handle_staging_approval_response(
                state,
                client_id,
                request_id,
                approved_npcs,
                ttl_hours,
                source,
            )
            .await
        }

        ClientMessage::StagingRegenerateRequest {
            request_id,
            guidance,
        } => staging::handle_staging_regenerate_request(state, client_id, request_id, guidance).await,

        ClientMessage::PreStageRegion {
            region_id,
            npcs,
            ttl_hours,
        } => staging::handle_pre_stage_region(state, client_id, region_id, npcs, ttl_hours).await,

        // Inventory handlers
        ClientMessage::EquipItem { pc_id, item_id } => {
            inventory::handle_equip_item(state, client_id, pc_id, item_id).await
        }

        ClientMessage::UnequipItem { pc_id, item_id } => {
            inventory::handle_unequip_item(state, client_id, pc_id, item_id).await
        }

        ClientMessage::DropItem {
            pc_id,
            item_id,
            quantity,
        } => inventory::handle_drop_item(state, client_id, pc_id, item_id, quantity).await,

        ClientMessage::PickupItem { pc_id, item_id } => {
            inventory::handle_pickup_item(state, client_id, pc_id, item_id).await
        }

        // Misc handlers
        ClientMessage::CheckComfyUIHealth => {
            misc::handle_check_comfyui_health(state).await
        }

        ClientMessage::ShareNpcLocation {
            pc_id,
            npc_id,
            location_id,
            region_id,
            notes,
        } => {
            misc::handle_share_npc_location(
                state, client_id, pc_id, npc_id, location_id, region_id, notes,
            )
            .await
        }

        ClientMessage::TriggerApproachEvent {
            npc_id,
            target_pc_id,
            description,
            reveal,
        } => {
            misc::handle_trigger_approach_event(
                state,
                client_id,
                npc_id,
                target_pc_id,
                description,
                reveal,
            )
            .await
        }

        ClientMessage::TriggerLocationEvent {
            region_id,
            description,
        } => misc::handle_trigger_location_event(state, client_id, region_id, description).await,

        // Request/Response pattern handler
        ClientMessage::Request { request_id, payload } => {
            request::handle_request(state, client_id, request_id, payload).await
        }
    }
}
