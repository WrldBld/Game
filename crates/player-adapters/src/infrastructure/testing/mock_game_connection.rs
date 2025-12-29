//! Mock implementation of GameConnectionPort for testing
//!
//! This mock belongs in the adapters layer (not ports) because:
//! 1. It's a concrete implementation of a port trait
//! 2. Mocks are infrastructure concerns, not interface definitions
//! 3. Test utilities should be close to the implementations they mock

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use wrldbldr_player_ports::outbound::{ConnectionState, GameConnectionPort};
use wrldbldr_player_ports::session_types::{
    AdHocOutcomes, ApprovalDecision, ApprovedNpcInfo, ChallengeOutcomeDecision, DiceInput,
    DirectorialContext, ParticipantRole,
};
use wrldbldr_protocol::{RequestError, RequestPayload, ResponseResult};

#[derive(Debug, Clone)]
pub struct SentAction {
    pub action_type: String,
    pub target: Option<String>,
    pub dialogue: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SentJoin {
    pub world_id: String,
    pub user_id: String,
    pub role: ParticipantRole,
}

#[derive(Debug, Clone)]
pub struct SentSceneChange {
    pub scene_id: String,
}

#[derive(Debug, Clone)]
pub struct SentApproval {
    pub request_id: String,
    pub decision: ApprovalDecision,
}

#[derive(Debug, Clone)]
pub struct SentChallengeTrigger {
    pub challenge_id: String,
    pub target_character_id: String,
}

struct State {
    conn_state: ConnectionState,
    sent_joins: Vec<SentJoin>,
    sent_actions: Vec<SentAction>,
    sent_scene_changes: Vec<SentSceneChange>,
    sent_directorial_updates: Vec<DirectorialContext>,
    sent_approvals: Vec<SentApproval>,
    sent_challenge_triggers: Vec<SentChallengeTrigger>,
    sent_rolls: Vec<(String, i32)>,

    on_state_change: Option<Box<dyn FnMut(ConnectionState) + Send + 'static>>,
    on_message: Option<Box<dyn FnMut(serde_json::Value) + Send + 'static>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            conn_state: ConnectionState::Disconnected,
            sent_joins: Vec::new(),
            sent_actions: Vec::new(),
            sent_scene_changes: Vec::new(),
            sent_directorial_updates: Vec::new(),
            sent_approvals: Vec::new(),
            sent_challenge_triggers: Vec::new(),
            sent_rolls: Vec::new(),
            on_state_change: None,
            on_message: None,
        }
    }
}

/// Mock `GameConnectionPort` for tests.
///
/// Lets tests drive connection state + inbound messages and assert outbound calls.
#[derive(Clone, Default)]
pub struct MockGameConnectionPort {
    url: Arc<str>,
    state: Arc<Mutex<State>>,
}

impl MockGameConnectionPort {
    pub fn new(url: impl Into<String>) -> Self {
        let mut s = State::default();
        s.conn_state = ConnectionState::Disconnected;
        Self {
            url: Arc::from(url.into().into_boxed_str()),
            state: Arc::new(Mutex::new(s)),
        }
    }

    pub fn set_state(&self, new_state: ConnectionState) {
        let mut s = self.state.lock().unwrap();
        s.conn_state = new_state;
        if let Some(cb) = s.on_state_change.as_mut() {
            cb(new_state);
        }
    }

    pub fn emit_message(&self, value: serde_json::Value) {
        let mut s = self.state.lock().unwrap();
        if let Some(cb) = s.on_message.as_mut() {
            cb(value);
        }
    }

    pub fn sent_actions(&self) -> Vec<SentAction> {
        self.state.lock().unwrap().sent_actions.clone()
    }

    pub fn sent_joins(&self) -> Vec<SentJoin> {
        self.state.lock().unwrap().sent_joins.clone()
    }
}

impl GameConnectionPort for MockGameConnectionPort {
    fn state(&self) -> ConnectionState {
        self.state.lock().unwrap().conn_state
    }

    fn url(&self) -> &str {
        self.url.as_ref()
    }

    fn connect(&self) -> anyhow::Result<()> {
        // Tests can drive state via `set_state`.
        Ok(())
    }

    fn disconnect(&self) {
        let mut s = self.state.lock().unwrap();
        s.conn_state = ConnectionState::Disconnected;
    }

    fn join_world(
        &self,
        world_id: &str,
        user_id: &str,
        role: ParticipantRole,
    ) -> anyhow::Result<()> {
        let mut s = self.state.lock().unwrap();
        s.sent_joins.push(SentJoin {
            world_id: world_id.to_string(),
            user_id: user_id.to_string(),
            role,
        });
        Ok(())
    }

    fn send_action(
        &self,
        action_type: &str,
        target: Option<&str>,
        dialogue: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut s = self.state.lock().unwrap();
        s.sent_actions.push(SentAction {
            action_type: action_type.to_string(),
            target: target.map(|s| s.to_string()),
            dialogue: dialogue.map(|s| s.to_string()),
        });
        Ok(())
    }

    fn request_scene_change(&self, scene_id: &str) -> anyhow::Result<()> {
        let mut s = self.state.lock().unwrap();
        s.sent_scene_changes.push(SentSceneChange {
            scene_id: scene_id.to_string(),
        });
        Ok(())
    }

    fn send_directorial_update(&self, context: DirectorialContext) -> anyhow::Result<()> {
        let mut s = self.state.lock().unwrap();
        s.sent_directorial_updates.push(context);
        Ok(())
    }

    fn send_approval_decision(
        &self,
        request_id: &str,
        decision: ApprovalDecision,
    ) -> anyhow::Result<()> {
        let mut s = self.state.lock().unwrap();
        s.sent_approvals.push(SentApproval {
            request_id: request_id.to_string(),
            decision,
        });
        Ok(())
    }

    fn send_challenge_outcome_decision(
        &self,
        _resolution_id: &str,
        _decision: ChallengeOutcomeDecision,
    ) -> anyhow::Result<()> {
        // Mock implementation - does nothing for now
        Ok(())
    }

    fn trigger_challenge(
        &self,
        challenge_id: &str,
        target_character_id: &str,
    ) -> anyhow::Result<()> {
        let mut s = self.state.lock().unwrap();
        s.sent_challenge_triggers.push(SentChallengeTrigger {
            challenge_id: challenge_id.to_string(),
            target_character_id: target_character_id.to_string(),
        });
        Ok(())
    }

    fn submit_challenge_roll(&self, challenge_id: &str, roll: i32) -> anyhow::Result<()> {
        let mut s = self.state.lock().unwrap();
        s.sent_rolls.push((challenge_id.to_string(), roll));
        Ok(())
    }

    fn submit_challenge_roll_input(
        &self,
        challenge_id: &str,
        input: DiceInput,
    ) -> anyhow::Result<()> {
        // For mock purposes, extract the value and use the existing roll tracking
        let roll_value = match &input {
            DiceInput::Manual(v) => *v,
            DiceInput::Formula(_) => 0, // Formula parsing not implemented in mock
        };
        let mut s = self.state.lock().unwrap();
        s.sent_rolls.push((challenge_id.to_string(), roll_value));
        Ok(())
    }

    fn heartbeat(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn move_to_region(&self, _pc_id: &str, _region_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn exit_to_location(
        &self,
        _pc_id: &str,
        _location_id: &str,
        _arrival_region_id: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn send_staging_approval(
        &self,
        _request_id: &str,
        _approved_npcs: Vec<ApprovedNpcInfo>,
        _ttl_hours: i32,
        _source: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn request_staging_regenerate(&self, _request_id: &str, _guidance: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn pre_stage_region(
        &self,
        _region_id: &str,
        _npcs: Vec<ApprovedNpcInfo>,
        _ttl_hours: i32,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_adhoc_challenge(
        &self,
        _challenge_name: &str,
        _skill_name: &str,
        _difficulty: &str,
        _target_pc_id: &str,
        _outcomes: AdHocOutcomes,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn equip_item(&self, _pc_id: &str, _item_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn unequip_item(&self, _pc_id: &str, _item_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn drop_item(&self, _pc_id: &str, _item_id: &str, _quantity: u32) -> anyhow::Result<()> {
        Ok(())
    }

    fn pickup_item(&self, _pc_id: &str, _item_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn check_comfyui_health(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn set_npc_disposition(
        &self,
        _npc_id: &str,
        _pc_id: &str,
        _disposition: &str,
        _reason: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn set_npc_relationship(
        &self,
        _npc_id: &str,
        _pc_id: &str,
        _relationship: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_npc_dispositions(&self, _pc_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_state_change(&self, callback: Box<dyn FnMut(ConnectionState) + Send + 'static>) {
        let mut s = self.state.lock().unwrap();
        s.on_state_change = Some(callback);
    }

    fn on_message(&self, callback: Box<dyn FnMut(serde_json::Value) + Send + 'static>) {
        let mut s = self.state.lock().unwrap();
        s.on_message = Some(callback);
    }

    fn request(
        &self,
        _payload: RequestPayload,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseResult, RequestError>> + Send + '_>> {
        // Mock always returns success with empty data
        Box::pin(async move { Ok(ResponseResult::Success { data: None }) })
    }

    fn request_with_timeout(
        &self,
        payload: RequestPayload,
        _timeout_ms: u64,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseResult, RequestError>> + Send + '_>> {
        // Just delegate to request for mock
        self.request(payload)
    }
}
