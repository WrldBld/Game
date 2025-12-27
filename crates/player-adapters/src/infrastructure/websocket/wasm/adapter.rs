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
use wrldbldr_protocol::{
    AdHocOutcomes, ApprovalDecision, ApprovedNpcInfo, ChallengeOutcomeDecisionData, ClientMessage,
    DiceInputType, DirectorialContext, ParticipantRole, RequestError, RequestPayload, ResponseResult,
};

use super::client::EngineClient;
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

    fn join_session(
        &self,
        user_id: &str,
        role: ParticipantRole,
        world_id: Option<String>,
    ) -> Result<()> {
        self.inner.client.join_session(user_id, role, world_id)
    }

    fn send_action(&self, action_type: &str, target: Option<&str>, dialogue: Option<&str>) -> Result<()> {
        self.inner.client.send_action(action_type, target, dialogue)
    }

    fn request_scene_change(&self, scene_id: &str) -> Result<()> {
        let msg = ClientMessage::RequestSceneChange { scene_id: scene_id.to_string() };
        self.inner.client.send(msg)
    }

    fn send_directorial_update(&self, context: DirectorialContext) -> Result<()> {
        let msg = ClientMessage::DirectorialUpdate { context };
        self.inner.client.send(msg)
    }

    fn send_approval_decision(&self, request_id: &str, decision: ApprovalDecision) -> Result<()> {
        let msg = ClientMessage::ApprovalDecision {
            request_id: request_id.to_string(),
            decision,
        };
        self.inner.client.send(msg)
    }

    fn send_challenge_outcome_decision(&self, resolution_id: &str, decision: ChallengeOutcomeDecisionData) -> Result<()> {
        let msg = ClientMessage::ChallengeOutcomeDecision {
            resolution_id: resolution_id.to_string(),
            decision,
        };
        self.inner.client.send(msg)
    }

    fn trigger_challenge(&self, challenge_id: &str, target_character_id: &str) -> Result<()> {
        let msg = ClientMessage::TriggerChallenge {
            challenge_id: challenge_id.to_string(),
            target_character_id: target_character_id.to_string(),
        };
        self.inner.client.send(msg)
    }

    fn submit_challenge_roll(&self, challenge_id: &str, roll: i32) -> Result<()> {
        let msg = ClientMessage::ChallengeRoll {
            challenge_id: challenge_id.to_string(),
            roll,
        };
        self.inner.client.send(msg)
    }

    fn submit_challenge_roll_input(&self, challenge_id: &str, input: DiceInputType) -> Result<()> {
        let msg = ClientMessage::ChallengeRollInput {
            challenge_id: challenge_id.to_string(),
            input_type: input,
        };
        self.inner.client.send(msg)
    }

    fn heartbeat(&self) -> Result<()> {
        self.inner.client.heartbeat()
    }

    fn move_to_region(&self, pc_id: &str, region_id: &str) -> Result<()> {
        let msg = ClientMessage::MoveToRegion {
            pc_id: pc_id.to_string(),
            region_id: region_id.to_string(),
        };
        self.inner.client.send(msg)
    }

    fn exit_to_location(&self, pc_id: &str, location_id: &str, arrival_region_id: Option<&str>) -> Result<()> {
        let msg = ClientMessage::ExitToLocation {
            pc_id: pc_id.to_string(),
            location_id: location_id.to_string(),
            arrival_region_id: arrival_region_id.map(|s| s.to_string()),
        };
        self.inner.client.send(msg)
    }

    fn send_staging_approval(
        &self,
        request_id: &str,
        approved_npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
        source: &str,
    ) -> Result<()> {
        let msg = ClientMessage::StagingApprovalResponse {
            request_id: request_id.to_string(),
            approved_npcs,
            ttl_hours,
            source: source.to_string(),
        };
        self.inner.client.send(msg)
    }

    fn request_staging_regenerate(&self, request_id: &str, guidance: &str) -> Result<()> {
        let msg = ClientMessage::StagingRegenerateRequest {
            request_id: request_id.to_string(),
            guidance: guidance.to_string(),
        };
        self.inner.client.send(msg)
    }

    fn pre_stage_region(
        &self,
        region_id: &str,
        npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
    ) -> Result<()> {
        let msg = ClientMessage::PreStageRegion {
            region_id: region_id.to_string(),
            npcs,
            ttl_hours,
        };
        self.inner.client.send(msg)
    }

    fn create_adhoc_challenge(
        &self,
        challenge_name: &str,
        skill_name: &str,
        difficulty: &str,
        target_pc_id: &str,
        outcomes: AdHocOutcomes,
    ) -> Result<()> {
        let msg = ClientMessage::CreateAdHocChallenge {
            challenge_name: challenge_name.to_string(),
            skill_name: skill_name.to_string(),
            difficulty: difficulty.to_string(),
            target_pc_id: target_pc_id.to_string(),
            outcomes,
        };
        self.inner.client.send(msg)
    }

    fn equip_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        let msg = ClientMessage::EquipItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        };
        self.inner.client.send(msg)
    }

    fn unequip_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        let msg = ClientMessage::UnequipItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        };
        self.inner.client.send(msg)
    }

    fn drop_item(&self, pc_id: &str, item_id: &str, quantity: u32) -> Result<()> {
        let msg = ClientMessage::DropItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
            quantity,
        };
        self.inner.client.send(msg)
    }

    fn pickup_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        let msg = ClientMessage::PickupItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        };
        self.inner.client.send(msg)
    }

    fn check_comfyui_health(&self) -> Result<()> {
        let msg = ClientMessage::CheckComfyUIHealth;
        self.inner.client.send(msg)
    }

    fn set_npc_mood(&self, npc_id: &str, pc_id: &str, mood: &str, reason: Option<&str>) -> Result<()> {
        let msg = ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload: RequestPayload::SetNpcMood {
                npc_id: npc_id.to_string(),
                pc_id: pc_id.to_string(),
                mood: mood.to_string(),
                reason: reason.map(|s| s.to_string()),
            },
        };
        self.inner.client.send(msg)
    }

    fn set_npc_relationship(&self, npc_id: &str, pc_id: &str, relationship: &str) -> Result<()> {
        let msg = ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload: RequestPayload::SetNpcRelationship {
                npc_id: npc_id.to_string(),
                pc_id: pc_id.to_string(),
                relationship: relationship.to_string(),
            },
        };
        self.inner.client.send(msg)
    }

    fn get_npc_moods(&self, pc_id: &str) -> Result<()> {
        let msg = ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload: RequestPayload::GetNpcMoods {
                pc_id: pc_id.to_string(),
            },
        };
        self.inner.client.send(msg)
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
