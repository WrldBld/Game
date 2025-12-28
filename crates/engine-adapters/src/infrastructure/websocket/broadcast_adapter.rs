//! WebSocket Broadcast Adapter
//!
//! Implements `BroadcastPort` by converting `GameEvent` to `ServerMessage`
//! and routing to appropriate recipients via `WorldConnectionManager`.
//!
//! # Architecture
//!
//! ```text
//! Use Case Layer                      This Adapter
//! ┌───────────────────┐              ┌─────────────────────────────────────┐
//! │ MovementUseCase   │──GameEvent──>│ WebSocketBroadcastAdapter           │
//! │                   │              │ - Converts GameEvent → ServerMessage│
//! │                   │              │ - Routes via WorldConnectionManager │
//! └───────────────────┘              └─────────────────────────────────────┘
//! ```

use async_trait::async_trait;
use chrono::{Datelike, Timelike};
use wrldbldr_domain::WorldId;
use wrldbldr_engine_ports::outbound::{
    BroadcastPort, GameEvent, NavigationExit, NavigationInfo, NavigationTarget, NpcPresenceData,
    PreviousStagingData, RegionInfo, RegionItemData, SceneChangedEvent, SplitPartyEvent,
    StagedNpcData, StagingPendingEvent, StagingReadyEvent, StagingRequiredEvent, WaitingPcData,
};
use wrldbldr_protocol::{
    GameTime as ProtoGameTime, NavigationData, NavigationExit as ProtoNavigationExit,
    NavigationTarget as ProtoNavigationTarget, NpcPresenceData as ProtoNpcPresenceData,
    NpcPresentInfo, PreviousStagingInfo, ProposedToolInfo, RegionData,
    RegionItemData as ProtoRegionItemData, ServerMessage, SplitPartyLocation, StagedNpcInfo,
    WaitingPcInfo,
};

use crate::infrastructure::world_connection_manager::SharedWorldConnectionManager;

/// Adapter implementing BroadcastPort for WebSocket delivery
///
/// Converts domain GameEvent types to protocol ServerMessage types
/// and routes them to appropriate recipients via WorldConnectionManager.
pub struct WebSocketBroadcastAdapter {
    /// Connection manager for message routing
    connection_manager: SharedWorldConnectionManager,
}

impl WebSocketBroadcastAdapter {
    /// Create a new broadcast adapter
    pub fn new(connection_manager: SharedWorldConnectionManager) -> Self {
        Self { connection_manager }
    }
}

#[async_trait]
impl BroadcastPort for WebSocketBroadcastAdapter {
    async fn broadcast(&self, world_id: WorldId, event: GameEvent) {
        let world_uuid = world_id.as_uuid();

        match event {
            // =====================================================================
            // Staging Events
            // =====================================================================
            GameEvent::StagingRequired(evt) => {
                let msg = convert_staging_required(evt);
                self.connection_manager
                    .broadcast_to_dms(*world_uuid, msg)
                    .await;
            }

            GameEvent::StagingReady(evt) => {
                let msg = convert_staging_ready(&evt);
                // Send to each waiting PC's user
                for pc in &evt.waiting_pcs {
                    let _ = self
                        .connection_manager
                        .send_to_user_in_world(&world_uuid, &pc.user_id, msg.clone())
                        .await;
                }
            }

            GameEvent::StagingPending { user_id, event } => {
                let msg = convert_staging_pending(event);
                let _ = self
                    .connection_manager
                    .send_to_user_in_world(&world_uuid, &user_id, msg)
                    .await;
            }

            // =====================================================================
            // Scene Events
            // =====================================================================
            GameEvent::SceneChanged { user_id, event } => {
                let msg = convert_scene_changed(event);
                let _ = self
                    .connection_manager
                    .send_to_user_in_world(&world_uuid, &user_id, msg)
                    .await;
            }

            // =====================================================================
            // Movement Events
            // =====================================================================
            GameEvent::MovementBlocked {
                user_id,
                pc_id,
                reason,
            } => {
                let msg = ServerMessage::MovementBlocked {
                    pc_id: pc_id.as_uuid().to_string(),
                    reason,
                };
                let _ = self
                    .connection_manager
                    .send_to_user_in_world(&world_uuid, &user_id, msg)
                    .await;
            }

            // =====================================================================
            // Party Events
            // =====================================================================
            GameEvent::SplitParty(evt) => {
                let msg = convert_split_party(evt);
                self.connection_manager
                    .broadcast_to_dms(*world_uuid, msg)
                    .await;
            }

            // =====================================================================
            // Time Events
            // =====================================================================
            GameEvent::GameTimeUpdated(game_time) => {
                let msg = ServerMessage::GameTimeUpdated {
                    game_time: convert_game_time(game_time),
                };
                self.connection_manager
                    .broadcast_to_world(*world_uuid, msg)
                    .await;
            }

            // =====================================================================
            // Player Events
            // =====================================================================
            GameEvent::PlayerJoined { user_id, pc_name } => {
                let msg = ServerMessage::UserJoined {
                    user_id: user_id.clone(),
                    username: pc_name,
                    role: wrldbldr_protocol::WorldRole::Player,
                    pc: None,
                };
                // Broadcast to all except the joining user
                let _ = self
                    .connection_manager
                    .broadcast_to_world_except(&world_uuid, &user_id, msg)
                    .await;
            }

            GameEvent::PlayerLeft { user_id } => {
                let msg = ServerMessage::UserLeft {
                    user_id: user_id.clone(),
                };
                // Broadcast to all except the leaving user
                let _ = self
                    .connection_manager
                    .broadcast_to_world_except(&world_uuid, &user_id, msg)
                    .await;
            }

            // =====================================================================
            // Inventory Events
            // =====================================================================
            GameEvent::ItemPickedUp {
                user_id,
                pc_id,
                item,
                quantity: _,
            } => {
                let msg = ServerMessage::ItemPickedUp {
                    pc_id: pc_id.as_uuid().to_string(),
                    item_id: item.item_id.as_uuid().to_string(),
                    item_name: item.name,
                };
                let _ = self
                    .connection_manager
                    .send_to_user_in_world(&world_uuid, &user_id, msg)
                    .await;
            }

            GameEvent::ItemDropped {
                user_id,
                pc_id,
                item,
                quantity,
                region_id: _,
            } => {
                let msg = ServerMessage::ItemDropped {
                    pc_id: pc_id.as_uuid().to_string(),
                    item_id: item.item_id.as_uuid().to_string(),
                    item_name: item.name,
                    quantity,
                };
                let _ = self
                    .connection_manager
                    .send_to_user_in_world(&world_uuid, &user_id, msg)
                    .await;
            }

            GameEvent::ItemEquipChanged {
                user_id,
                pc_id,
                item,
                equipped,
            } => {
                let msg = if equipped {
                    ServerMessage::ItemEquipped {
                        pc_id: pc_id.as_uuid().to_string(),
                        item_id: item.item_id.as_uuid().to_string(),
                        item_name: item.name,
                    }
                } else {
                    ServerMessage::ItemUnequipped {
                        pc_id: pc_id.as_uuid().to_string(),
                        item_id: item.item_id.as_uuid().to_string(),
                        item_name: item.name,
                    }
                };
                let _ = self
                    .connection_manager
                    .send_to_user_in_world(&world_uuid, &user_id, msg)
                    .await;
            }

            // =====================================================================
            // Challenge Events
            // =====================================================================
            GameEvent::ChallengeRollSubmitted {
                world_id: _,
                ref resolution_id,
                ref challenge_id,
                ref challenge_name,
                ref character_id,
                ref character_name,
                roll,
                modifier,
                total,
                ref outcome_type,
                ref outcome_description,
                ref roll_breakdown,
                individual_rolls: _,
                ref outcome_triggers,
            } => {
                // 1. Send full pending data to DM for approval UI
                let dm_message = ServerMessage::ChallengeOutcomePending {
                    resolution_id: resolution_id.clone(),
                    challenge_id: challenge_id.clone(),
                    challenge_name: challenge_name.clone(),
                    character_id: character_id.clone(),
                    character_name: character_name.clone(),
                    roll,
                    modifier,
                    total,
                    outcome_type: outcome_type.clone(),
                    outcome_description: outcome_description.clone(),
                    outcome_triggers: outcome_triggers
                        .iter()
                        .map(|t| ProposedToolInfo {
                            id: uuid::Uuid::new_v4().to_string(),
                            name: t.trigger_type.clone(),
                            description: t.description.clone(),
                            arguments: serde_json::Value::Null,
                        })
                        .collect(),
                    roll_breakdown: roll_breakdown.clone(),
                };
                self.connection_manager
                    .broadcast_to_dms(*world_uuid, dm_message)
                    .await;

                // 2. Send status confirmation to all players
                let player_message = ServerMessage::ChallengeRollSubmitted {
                    challenge_id: challenge_id.clone(),
                    challenge_name: challenge_name.clone(),
                    roll,
                    modifier,
                    total,
                    outcome_type: outcome_type.clone(),
                    status: "pending_approval".to_string(),
                };
                self.connection_manager
                    .broadcast_to_players(*world_uuid, player_message)
                    .await;
            }

            GameEvent::ChallengeResolved {
                world_id: _,
                ref challenge_id,
                ref challenge_name,
                ref character_name,
                roll,
                modifier,
                total,
                ref outcome,
                ref outcome_description,
                ref roll_breakdown,
                ref individual_rolls,
                state_changes: _,
            } => {
                // Broadcast resolution to all players
                let message = ServerMessage::ChallengeResolved {
                    challenge_id: challenge_id.clone(),
                    challenge_name: challenge_name.clone(),
                    character_name: character_name.clone(),
                    roll,
                    modifier,
                    total,
                    outcome: outcome.clone(),
                    outcome_description: outcome_description.clone(),
                    roll_breakdown: roll_breakdown.clone(),
                    individual_rolls: individual_rolls.clone(),
                };
                self.connection_manager
                    .broadcast_to_world(*world_uuid, message)
                    .await;
            }

            GameEvent::ChallengePromptSent {
                world_id: _,
                ref challenge_id,
                ref challenge_name,
                ref skill_name,
                ref difficulty_display,
                ref description,
                character_modifier,
                ref suggested_dice,
                ref rule_system_hint,
            } => {
                // Broadcast challenge prompt to world
                let message = ServerMessage::ChallengePrompt {
                    challenge_id: challenge_id.clone(),
                    challenge_name: challenge_name.clone(),
                    skill_name: skill_name.clone(),
                    difficulty_display: difficulty_display.clone(),
                    description: description.clone(),
                    character_modifier,
                    suggested_dice: Some(suggested_dice.clone()),
                    rule_system_hint: Some(rule_system_hint.clone()),
                };
                self.connection_manager
                    .broadcast_to_world(*world_uuid, message)
                    .await;
            }

            GameEvent::ChallengeSuggestionsReady {
                resolution_id: _,
                suggestions: _,
            } => {
                // Suggestions are sent directly to DM via the approval service
                // This event is for logging/metrics only
                tracing::debug!("ChallengeSuggestionsReady event - already handled by approval service");
            }

            GameEvent::ChallengeBranchesReady {
                resolution_id: _,
                branches: _,
            } => {
                // Branches are sent directly to DM via the approval service
                // This event is for logging/metrics only
                tracing::debug!("ChallengeBranchesReady event - already handled by approval service");
            }
        }
    }
}

// =============================================================================
// Conversion Functions
// =============================================================================

fn convert_staging_required(evt: StagingRequiredEvent) -> ServerMessage {
    ServerMessage::StagingApprovalRequired {
        request_id: evt.request_id,
        region_id: evt.region_id.as_uuid().to_string(),
        region_name: evt.region_name,
        location_id: evt.location_id.as_uuid().to_string(),
        location_name: evt.location_name,
        game_time: convert_game_time(evt.game_time),
        previous_staging: evt.previous_staging.map(convert_previous_staging),
        rule_based_npcs: evt.rule_based_npcs.into_iter().map(convert_staged_npc).collect(),
        llm_based_npcs: evt.llm_based_npcs.into_iter().map(convert_staged_npc).collect(),
        default_ttl_hours: evt.default_ttl_hours,
        waiting_pcs: evt.waiting_pcs.into_iter().map(convert_waiting_pc).collect(),
    }
}

fn convert_staging_ready(evt: &StagingReadyEvent) -> ServerMessage {
    ServerMessage::StagingReady {
        region_id: evt.region_id.as_uuid().to_string(),
        npcs_present: evt
            .npcs_present
            .iter()
            .map(|npc| NpcPresentInfo {
                character_id: npc.character_id.as_uuid().to_string(),
                name: npc.name.clone(),
                sprite_asset: npc.sprite_asset.clone(),
                portrait_asset: npc.portrait_asset.clone(),
                is_hidden_from_players: false,
            })
            .collect(),
    }
}

fn convert_staging_pending(evt: StagingPendingEvent) -> ServerMessage {
    ServerMessage::StagingPending {
        region_id: evt.region_id.as_uuid().to_string(),
        region_name: evt.region_name,
    }
}

fn convert_scene_changed(evt: SceneChangedEvent) -> ServerMessage {
    ServerMessage::SceneChanged {
        pc_id: evt.pc_id.as_uuid().to_string(),
        region: convert_region_info(evt.region),
        npcs_present: evt
            .npcs_present
            .into_iter()
            .map(convert_npc_presence)
            .collect(),
        navigation: convert_navigation_info(evt.navigation),
        region_items: evt
            .region_items
            .into_iter()
            .map(convert_region_item)
            .collect(),
    }
}

fn convert_split_party(evt: SplitPartyEvent) -> ServerMessage {
    let locations: Vec<SplitPartyLocation> = evt
        .location_groups
        .into_iter()
        .map(|group| SplitPartyLocation {
            location_id: group.location_id.as_uuid().to_string(),
            location_name: group.location_name,
            pc_count: group.pcs.len(),
            pc_names: group.pcs.iter().map(|pc| pc.pc_name.clone()).collect(),
        })
        .collect();

    ServerMessage::SplitPartyNotification {
        location_count: locations.len(),
        locations,
    }
}

fn convert_game_time(gt: wrldbldr_domain::GameTime) -> ProtoGameTime {
    let current = gt.current();
    ProtoGameTime {
        day: current.ordinal(),
        hour: current.hour() as u8,
        minute: current.minute() as u8,
        is_paused: gt.is_paused(),
    }
}

fn convert_staged_npc(npc: StagedNpcData) -> StagedNpcInfo {
    StagedNpcInfo {
        character_id: npc.character_id.as_uuid().to_string(),
        name: npc.name,
        sprite_asset: npc.sprite_asset,
        portrait_asset: npc.portrait_asset,
        is_present: npc.is_present,
        reasoning: npc.reasoning,
        is_hidden_from_players: npc.is_hidden_from_players,
    }
}

fn convert_waiting_pc(pc: WaitingPcData) -> WaitingPcInfo {
    WaitingPcInfo {
        pc_id: pc.pc_id.as_uuid().to_string(),
        pc_name: pc.pc_name,
        player_id: pc.user_id,
    }
}

fn convert_previous_staging(ps: PreviousStagingData) -> PreviousStagingInfo {
    PreviousStagingInfo {
        staging_id: ps.staging_id.as_uuid().to_string(),
        approved_at: ps.approved_at.to_rfc3339(),
        npcs: ps.npcs.into_iter().map(convert_staged_npc).collect(),
    }
}

fn convert_region_info(ri: RegionInfo) -> RegionData {
    RegionData {
        id: ri.id.as_uuid().to_string(),
        name: ri.name,
        location_id: ri.location_id.as_uuid().to_string(),
        location_name: ri.location_name,
        backdrop_asset: ri.backdrop_asset,
        atmosphere: ri.atmosphere,
        map_asset: ri.map_asset,
    }
}

fn convert_npc_presence(npc: NpcPresenceData) -> ProtoNpcPresenceData {
    ProtoNpcPresenceData {
        character_id: npc.character_id.as_uuid().to_string(),
        name: npc.name,
        sprite_asset: npc.sprite_asset,
        portrait_asset: npc.portrait_asset,
    }
}

fn convert_navigation_info(nav: NavigationInfo) -> NavigationData {
    NavigationData {
        connected_regions: nav
            .connected_regions
            .into_iter()
            .map(convert_navigation_target)
            .collect(),
        exits: nav.exits.into_iter().map(convert_navigation_exit).collect(),
    }
}

fn convert_navigation_target(target: NavigationTarget) -> ProtoNavigationTarget {
    ProtoNavigationTarget {
        region_id: target.region_id.as_uuid().to_string(),
        name: target.name,
        is_locked: target.is_locked,
        lock_description: target.lock_description,
    }
}

fn convert_navigation_exit(exit: NavigationExit) -> ProtoNavigationExit {
    ProtoNavigationExit {
        location_id: exit.location_id.as_uuid().to_string(),
        location_name: exit.location_name,
        arrival_region_id: exit.arrival_region_id.as_uuid().to_string(),
        description: exit.description,
    }
}

fn convert_region_item(item: RegionItemData) -> ProtoRegionItemData {
    ProtoRegionItemData {
        id: item.item_id.as_uuid().to_string(),
        name: item.name,
        description: item.description,
        item_type: None, // Not available in domain type
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, RegionId};

    #[test]
    fn test_convert_region_info() {
        let region = RegionInfo {
            id: RegionId::from_uuid(uuid::Uuid::new_v4()),
            name: "Test Region".to_string(),
            location_id: LocationId::from_uuid(uuid::Uuid::new_v4()),
            location_name: "Test Location".to_string(),
            backdrop_asset: Some("backdrop.png".to_string()),
            atmosphere: Some("Mysterious".to_string()),
            map_asset: None,
        };

        let converted = convert_region_info(region.clone());
        assert_eq!(converted.name, "Test Region");
        assert_eq!(converted.location_name, "Test Location");
        assert_eq!(converted.backdrop_asset, Some("backdrop.png".to_string()));
    }

    #[test]
    fn test_convert_npc_presence() {
        let npc = NpcPresenceData {
            character_id: CharacterId::from_uuid(uuid::Uuid::new_v4()),
            name: "Merchant".to_string(),
            sprite_asset: Some("merchant.png".to_string()),
            portrait_asset: None,
        };

        let converted = convert_npc_presence(npc);
        assert_eq!(converted.name, "Merchant");
        assert_eq!(converted.sprite_asset, Some("merchant.png".to_string()));
    }
}
