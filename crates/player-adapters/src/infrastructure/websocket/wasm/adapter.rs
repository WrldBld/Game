//! WASM GameConnectionPort adapter with SendWrapper for Send + Sync
//!
//! This adapter uses `send_wrapper::SendWrapper` to satisfy the Send + Sync
//! requirements of the `GameConnectionPort` trait in a WASM single-threaded
//! environment.
//!
//! # Safety
//!
//! `SendWrapper` is safe to use here because:
//! 1. WASM is single-threaded - there IS only one thread
//! 2. SendWrapper will panic if accessed from a different thread, but this
//!    cannot happen in WASM
//! 3. The presentation layer (Dioxus) requires Send + Sync for context, but
//!    all access happens on the main thread

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};

use anyhow::Result;
use send_wrapper::SendWrapper;

use wrldbldr_player_ports::outbound::{ConnectionState as PortConnectionState, GameConnectionPort};
use wrldbldr_player_ports::session_types as app;
use wrldbldr_protocol::{RequestError, RequestPayload, ResponseResult};

use super::client::EngineClient;
use crate::infrastructure::session_type_converters::{
    adhoc_outcomes_to_proto, approval_decision_to_proto, approved_npc_info_to_proto,
    challenge_outcome_decision_to_proto, dice_input_to_proto, directorial_context_to_proto,
    participant_role_to_proto,
};
use crate::infrastructure::websocket::message_builder::ClientMessageBuilder;
use crate::infrastructure::websocket::protocol::{map_state, state_to_u8, u8_to_state};

/// Inner WASM connection data (not Send + Sync)
struct WasmGameConnectionInner {
    client: EngineClient,
    state: Arc<AtomicU8>,
}

impl Clone for WasmGameConnectionInner {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

/// WASM game connection adapter
///
/// Uses SendWrapper to satisfy Send + Sync requirements for Dioxus context.
/// This is safe because WASM is single-threaded - SendWrapper will panic
/// if accessed from a different thread, but there IS only one thread.
#[derive(Clone)]
pub struct WasmGameConnection {
    /// The inner client wrapped in SendWrapper to provide Send + Sync
    inner: SendWrapper<WasmGameConnectionInner>,
}

impl WasmGameConnection {
    pub fn new(client: EngineClient) -> Self {
        let initial = state_to_u8(map_state(client.state()));
        Self {
            inner: SendWrapper::new(WasmGameConnectionInner {
                client,
                state: Arc::new(AtomicU8::new(initial)),
            }),
        }
    }
}

impl GameConnectionPort for WasmGameConnection {
    fn state(&self) -> PortConnectionState {
        u8_to_state(self.inner.state.load(Ordering::SeqCst))
    }

    fn url(&self) -> &str {
        self.inner.client.url()
    }

    fn connect(&self) -> Result<()> {
        self.inner.state.store(state_to_u8(PortConnectionState::Connecting), Ordering::SeqCst);
        self.inner.client.connect()
    }

    fn disconnect(&self) {
        self.inner.client.disconnect();
        self.inner.state.store(state_to_u8(PortConnectionState::Disconnected), Ordering::SeqCst);
    }

    fn join_world(&self, world_id: &str, user_id: &str, role: app::ParticipantRole) -> Result<()> {
        let proto_role = participant_role_to_proto(role);
        self.inner.client.join_world(world_id, user_id, proto_role)
    }

    fn send_action(&self, action_type: &str, target: Option<&str>, dialogue: Option<&str>) -> Result<()> {
        self.inner.client.send_action(action_type, target, dialogue)
    }

    fn request_scene_change(&self, scene_id: &str) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::request_scene_change(scene_id))
    }

    fn send_directorial_update(&self, context: app::DirectorialContext) -> Result<()> {
        let proto_context = directorial_context_to_proto(context);
        self.inner.client.send(ClientMessageBuilder::directorial_update(proto_context))
    }

    fn send_approval_decision(&self, request_id: &str, decision: app::ApprovalDecision) -> Result<()> {
        let proto_decision = approval_decision_to_proto(decision);
        self.inner.client.send(ClientMessageBuilder::approval_decision(request_id, proto_decision))
    }

    fn send_challenge_outcome_decision(&self, resolution_id: &str, decision: app::ChallengeOutcomeDecision) -> Result<()> {
        let proto_decision = challenge_outcome_decision_to_proto(decision);
        self.inner.client.send(ClientMessageBuilder::challenge_outcome_decision(resolution_id, proto_decision))
    }

    fn trigger_challenge(&self, challenge_id: &str, target_character_id: &str) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::trigger_challenge(challenge_id, target_character_id))
    }

    fn submit_challenge_roll(&self, challenge_id: &str, roll: i32) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::challenge_roll(challenge_id, roll))
    }

    fn submit_challenge_roll_input(&self, challenge_id: &str, input: app::DiceInput) -> Result<()> {
        let proto_input = dice_input_to_proto(input);
        self.inner.client.send(ClientMessageBuilder::challenge_roll_input(challenge_id, proto_input))
    }

    fn heartbeat(&self) -> Result<()> {
        self.inner.client.heartbeat()
    }

    fn move_to_region(&self, pc_id: &str, region_id: &str) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::move_to_region(pc_id, region_id))
    }

    fn exit_to_location(&self, pc_id: &str, location_id: &str, arrival_region_id: Option<&str>) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::exit_to_location(pc_id, location_id, arrival_region_id))
    }

    fn send_staging_approval(
        &self,
        request_id: &str,
        approved_npcs: Vec<app::ApprovedNpcInfo>,
        ttl_hours: i32,
        source: &str,
    ) -> Result<()> {
        let proto_npcs = approved_npcs.into_iter().map(approved_npc_info_to_proto).collect();
        self.inner.client.send(ClientMessageBuilder::staging_approval_response(request_id, proto_npcs, ttl_hours, source))
    }

    fn request_staging_regenerate(&self, request_id: &str, guidance: &str) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::staging_regenerate_request(request_id, guidance))
    }

    fn pre_stage_region(&self, region_id: &str, npcs: Vec<app::ApprovedNpcInfo>, ttl_hours: i32) -> Result<()> {
        let proto_npcs = npcs.into_iter().map(approved_npc_info_to_proto).collect();
        self.inner.client.send(ClientMessageBuilder::pre_stage_region(region_id, proto_npcs, ttl_hours))
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
        self.inner.client.send(ClientMessageBuilder::create_adhoc_challenge(
            challenge_name, skill_name, difficulty, target_pc_id, proto_outcomes,
        ))
    }

    fn equip_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::equip_item(pc_id, item_id))
    }

    fn unequip_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::unequip_item(pc_id, item_id))
    }

    fn drop_item(&self, pc_id: &str, item_id: &str, quantity: u32) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::drop_item(pc_id, item_id, quantity))
    }

    fn pickup_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::pickup_item(pc_id, item_id))
    }

    fn check_comfyui_health(&self) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::check_comfyui_health())
    }

    fn set_npc_disposition(&self, npc_id: &str, pc_id: &str, disposition: &str, reason: Option<&str>) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::set_npc_disposition(npc_id, pc_id, disposition, reason))
    }

    fn set_npc_relationship(&self, npc_id: &str, pc_id: &str, relationship: &str) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::set_npc_relationship(npc_id, pc_id, relationship))
    }

    fn get_npc_dispositions(&self, pc_id: &str) -> Result<()> {
        self.inner.client.send(ClientMessageBuilder::get_npc_dispositions(pc_id))
    }

    /// Register callback for state changes
    ///
    /// The Send callback is wrapped in Rc<RefCell> for internal WASM use.
    fn on_state_change(&self, callback: Box<dyn FnMut(PortConnectionState) + Send + 'static>) {
        let state_slot = Arc::clone(&self.inner.state);
        let cb = Rc::new(RefCell::new(callback));
        let cb_for_engine = Rc::clone(&cb);

        self.inner.client.set_on_state_change(move |infra_state| {
            let port_state = map_state(infra_state);
            state_slot.store(state_to_u8(port_state), Ordering::SeqCst);
            (cb_for_engine.borrow_mut())(port_state);
        });
    }

    /// Register callback for server messages
    fn on_message(&self, callback: Box<dyn FnMut(serde_json::Value) + Send + 'static>) {
        let cb = Rc::new(RefCell::new(callback));
        let cb_for_engine = Rc::clone(&cb);

        self.inner.client.set_on_message(move |msg| {
            let value = serde_json::to_value(msg).unwrap_or(serde_json::Value::Null);
            (cb_for_engine.borrow_mut())(value);
        });
    }

    /// Send a request and await the response
    ///
    /// The non-Send future from the client is wrapped in SendWrapper.
    fn request(
        &self,
        payload: RequestPayload,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseResult, RequestError>> + Send + '_>> {
        let future = self.inner.client.request(payload);
        Box::pin(SendWrapper::new(future))
    }

    /// Send a request with a custom timeout
    fn request_with_timeout(
        &self,
        payload: RequestPayload,
        timeout_ms: u64,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseResult, RequestError>> + Send + '_>> {
        let future = self.inner.client.request_with_timeout(payload, timeout_ms);
        Box::pin(SendWrapper::new(future))
    }
}
