//! Desktop GameConnectionPort adapter

use std::future::Future;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};

use anyhow::Result;

use crate::ports::outbound::player_events::PlayerEvent;
use crate::ports::outbound::{ConnectionState as PortConnectionState, GameConnectionPort};
use crate::ports::session_types as app;
use wrldbldr_protocol::{ClientMessage, RequestError, RequestPayload, ResponseResult};

use crate::infrastructure::message_translator;

use super::client::EngineClient;
use crate::infrastructure::session_type_converters::{
    adhoc_outcomes_to_proto, approval_decision_to_proto, approved_npc_info_to_proto,
    challenge_outcome_decision_to_proto, dice_input_to_proto, directorial_context_to_proto,
    participant_role_to_proto,
};
use crate::infrastructure::websocket::message_builder::ClientMessageBuilder;
use crate::infrastructure::websocket::protocol::{map_state, state_to_u8, u8_to_state};

/// Desktop game connection adapter
#[derive(Clone)]
pub struct DesktopGameConnection {
    client: EngineClient,
    state: Arc<AtomicU8>,
}

impl DesktopGameConnection {
    pub fn new(client: EngineClient) -> Self {
        Self {
            client,
            state: Arc::new(AtomicU8::new(state_to_u8(
                PortConnectionState::Disconnected,
            ))),
        }
    }

    /// Helper to spawn an async send operation with error logging
    fn spawn_send(&self, msg: ClientMessage, operation: &'static str) {
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to {}: {}", operation, e);
            }
        });
    }
}

impl GameConnectionPort for DesktopGameConnection {
    fn state(&self) -> PortConnectionState {
        u8_to_state(self.state.load(Ordering::SeqCst))
    }

    fn url(&self) -> &str {
        self.client.url()
    }

    fn connect(&self) -> Result<()> {
        let client = self.client.clone();
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            if let Err(e) = client.connect().await {
                tracing::error!("Failed to connect to Engine: {}", e);
                state.store(state_to_u8(PortConnectionState::Failed), Ordering::SeqCst);
            }
        });
        Ok(())
    }

    fn disconnect(&self) {
        let client = self.client.clone();
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            client.disconnect().await;
            state.store(
                state_to_u8(PortConnectionState::Disconnected),
                Ordering::SeqCst,
            );
        });
    }

    fn join_world(&self, world_id: &str, user_id: &str, role: app::ParticipantRole) -> Result<()> {
        let client = self.client.clone();
        let world_id = world_id.to_string();
        let user_id = user_id.to_string();
        let proto_role = participant_role_to_proto(role);
        tokio::spawn(async move {
            if let Err(e) = client.join_world(&world_id, &user_id, proto_role).await {
                tracing::error!("Failed to join world: {}", e);
            }
        });
        Ok(())
    }

    fn send_action(
        &self,
        action_type: &str,
        target: Option<&str>,
        dialogue: Option<&str>,
    ) -> Result<()> {
        let client = self.client.clone();
        let action_type = action_type.to_string();
        let target = target.map(|s| s.to_string());
        let dialogue = dialogue.map(|s| s.to_string());
        tokio::spawn(async move {
            if let Err(e) = client
                .send_action(&action_type, target.as_deref(), dialogue.as_deref())
                .await
            {
                tracing::error!("Failed to send action: {}", e);
            }
        });
        Ok(())
    }

    fn request_scene_change(&self, scene_id: &str) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::request_scene_change(scene_id),
            "request scene change",
        );
        Ok(())
    }

    fn send_directorial_update(&self, context: app::DirectorialContext) -> Result<()> {
        let proto_context = directorial_context_to_proto(context);
        self.spawn_send(
            ClientMessageBuilder::directorial_update(proto_context),
            "send directorial update",
        );
        Ok(())
    }

    fn send_approval_decision(
        &self,
        request_id: &str,
        decision: app::ApprovalDecision,
    ) -> Result<()> {
        let proto_decision = approval_decision_to_proto(decision);
        self.spawn_send(
            ClientMessageBuilder::approval_decision(request_id, proto_decision),
            "send approval decision",
        );
        Ok(())
    }

    fn send_challenge_outcome_decision(
        &self,
        resolution_id: &str,
        decision: app::ChallengeOutcomeDecision,
    ) -> Result<()> {
        let proto_decision = challenge_outcome_decision_to_proto(decision);
        self.spawn_send(
            ClientMessageBuilder::challenge_outcome_decision(resolution_id, proto_decision),
            "send challenge outcome decision",
        );
        Ok(())
    }

    fn trigger_challenge(&self, challenge_id: &str, target_character_id: &str) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::trigger_challenge(challenge_id, target_character_id),
            "trigger challenge",
        );
        Ok(())
    }

    fn submit_challenge_roll(&self, challenge_id: &str, roll: i32) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::challenge_roll(challenge_id, roll),
            "submit challenge roll",
        );
        Ok(())
    }

    fn submit_challenge_roll_input(&self, challenge_id: &str, input: app::DiceInput) -> Result<()> {
        let proto_input = dice_input_to_proto(input);
        self.spawn_send(
            ClientMessageBuilder::challenge_roll_input(challenge_id, proto_input),
            "submit challenge roll input",
        );
        Ok(())
    }

    fn heartbeat(&self) -> Result<()> {
        self.spawn_send(ClientMessageBuilder::heartbeat(), "send heartbeat");
        Ok(())
    }

    fn move_to_region(&self, pc_id: &str, region_id: &str) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::move_to_region(pc_id, region_id),
            "move to region",
        );
        Ok(())
    }

    fn exit_to_location(
        &self,
        pc_id: &str,
        location_id: &str,
        arrival_region_id: Option<&str>,
    ) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::exit_to_location(pc_id, location_id, arrival_region_id),
            "exit to location",
        );
        Ok(())
    }

    fn send_staging_approval(
        &self,
        request_id: &str,
        approved_npcs: Vec<app::ApprovedNpcInfo>,
        ttl_hours: i32,
        source: &str,
    ) -> Result<()> {
        let proto_npcs = approved_npcs
            .into_iter()
            .map(approved_npc_info_to_proto)
            .collect();
        self.spawn_send(
            ClientMessageBuilder::staging_approval_response(
                request_id, proto_npcs, ttl_hours, source,
            ),
            "send staging approval",
        );
        Ok(())
    }

    fn request_staging_regenerate(&self, request_id: &str, guidance: &str) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::staging_regenerate_request(request_id, guidance),
            "request staging regenerate",
        );
        Ok(())
    }

    fn pre_stage_region(
        &self,
        region_id: &str,
        npcs: Vec<app::ApprovedNpcInfo>,
        ttl_hours: i32,
    ) -> Result<()> {
        let proto_npcs = npcs.into_iter().map(approved_npc_info_to_proto).collect();
        self.spawn_send(
            ClientMessageBuilder::pre_stage_region(region_id, proto_npcs, ttl_hours),
            "pre-stage region",
        );
        Ok(())
    }

    fn create_adhoc_challenge(
        &self,
        challenge_name: &str,
        skill_name: &str,
        difficulty: &str,
        target_pc_id: &str,
        outcomes: app::AdHocOutcomes,
    ) -> Result<()> {
        let proto_outcomes = adhoc_outcomes_to_proto(outcomes);
        self.spawn_send(
            ClientMessageBuilder::create_adhoc_challenge(
                challenge_name,
                skill_name,
                difficulty,
                target_pc_id,
                proto_outcomes,
            ),
            "create ad-hoc challenge",
        );
        Ok(())
    }

    fn equip_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::equip_item(pc_id, item_id),
            "equip item",
        );
        Ok(())
    }

    fn unequip_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::unequip_item(pc_id, item_id),
            "unequip item",
        );
        Ok(())
    }

    fn drop_item(&self, pc_id: &str, item_id: &str, quantity: u32) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::drop_item(pc_id, item_id, quantity),
            "drop item",
        );
        Ok(())
    }

    fn pickup_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::pickup_item(pc_id, item_id),
            "pick up item",
        );
        Ok(())
    }

    fn check_comfyui_health(&self) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::check_comfyui_health(),
            "check ComfyUI health",
        );
        Ok(())
    }

    fn set_npc_disposition(
        &self,
        npc_id: &str,
        pc_id: &str,
        disposition: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::set_npc_disposition(npc_id, pc_id, disposition, reason),
            "set NPC disposition",
        );
        Ok(())
    }

    fn set_npc_relationship(&self, npc_id: &str, pc_id: &str, relationship: &str) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::set_npc_relationship(npc_id, pc_id, relationship),
            "set NPC relationship",
        );
        Ok(())
    }

    fn get_npc_dispositions(&self, pc_id: &str) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::get_npc_dispositions(pc_id),
            "get NPC dispositions",
        );
        Ok(())
    }

    fn advance_time(&self, world_id: &str, minutes: u32, reason: &str) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::advance_time(world_id, minutes, reason),
            "advance time",
        );
        Ok(())
    }

    fn set_game_time(&self, world_id: &str, day: u32, hour: u8) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::set_game_time(world_id, day, hour),
            "set game time",
        );
        Ok(())
    }

    fn skip_to_period(&self, world_id: &str, period: &str) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::skip_to_period(world_id, period),
            "skip to period",
        );
        Ok(())
    }

    fn respond_to_time_suggestion(
        &self,
        suggestion_id: &str,
        decision: &str,
        modified_minutes: Option<u32>,
    ) -> Result<()> {
        self.spawn_send(
            ClientMessageBuilder::respond_to_time_suggestion(suggestion_id, decision, modified_minutes),
            "respond to time suggestion",
        );
        Ok(())
    }

    fn on_state_change(&self, callback: Box<dyn FnMut(PortConnectionState) + Send + 'static>) {
        let state_slot = Arc::clone(&self.state);
        let cb = Arc::new(tokio::sync::Mutex::new(callback));

        let cb_for_engine = Arc::clone(&cb);
        let state_for_engine = Arc::clone(&state_slot);
        let client = self.client.clone();

        tokio::spawn(async move {
            client
                .set_on_state_change(move |infra_state| {
                    let port_state = map_state(infra_state);
                    state_for_engine.store(state_to_u8(port_state), Ordering::SeqCst);

                    let cb_for_call = Arc::clone(&cb_for_engine);
                    tokio::spawn(async move {
                        let mut cb = cb_for_call.lock().await;
                        (cb)(port_state);
                    });
                })
                .await;
        });
    }

    fn on_message(&self, callback: Box<dyn FnMut(PlayerEvent) + Send + 'static>) {
        let cb = Arc::new(tokio::sync::Mutex::new(callback));
        let cb_for_engine = Arc::clone(&cb);
        let client = self.client.clone();

        tokio::spawn(async move {
            client
                .set_on_message(move |msg| {
                    // Translate wire-format ServerMessage to application-layer PlayerEvent
                    let event = message_translator::translate(msg);
                    let cb_for_call = Arc::clone(&cb_for_engine);
                    tokio::spawn(async move {
                        let mut cb = cb_for_call.lock().await;
                        (cb)(event);
                    });
                })
                .await;
        });
    }

    fn request(
        &self,
        payload: RequestPayload,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseResult, RequestError>> + Send + '_>> {
        let client = self.client.clone();

        Box::pin(async move { client.request(payload).await })
    }

    fn request_with_timeout(
        &self,
        payload: RequestPayload,
        timeout_ms: u64,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseResult, RequestError>> + Send + '_>> {
        let client = self.client.clone();

        Box::pin(async move { client.request_with_timeout(payload, timeout_ms).await })
    }
}
