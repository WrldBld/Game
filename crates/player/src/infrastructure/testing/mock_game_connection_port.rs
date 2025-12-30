use std::sync::{Arc, Mutex};

use crate::application::ports::outbound::{
    ApprovalDecision, ChallengeOutcomeDecisionData, ConnectionState, DirectorialContext, GameConnectionPort, ParticipantRole,
};

#[derive(Debug, Clone)]
pub struct SentAction {
    pub action_type: String,
    pub target: Option<String>,
    pub dialogue: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SentJoin {
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

    fn join_session(&self, user_id: &str, role: ParticipantRole) -> anyhow::Result<()> {
        let mut s = self.state.lock().unwrap();
        s.sent_joins.push(SentJoin {
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

    fn send_approval_decision(&self, request_id: &str, decision: ApprovalDecision) -> anyhow::Result<()> {
        let mut s = self.state.lock().unwrap();
        s.sent_approvals.push(SentApproval {
            request_id: request_id.to_string(),
            decision,
        });
        Ok(())
    }

    fn send_challenge_outcome_decision(&self, _resolution_id: &str, _decision: ChallengeOutcomeDecisionData) -> anyhow::Result<()> {
        // Mock implementation - does nothing for now
        Ok(())
    }

    fn trigger_challenge(&self, challenge_id: &str, target_character_id: &str) -> anyhow::Result<()> {
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

    fn submit_challenge_roll_input(&self, challenge_id: &str, input: crate::application::dto::websocket_messages::DiceInputType) -> anyhow::Result<()> {
        // For mock purposes, extract the value and use the existing roll tracking
        let roll_value = match &input {
            crate::application::dto::websocket_messages::DiceInputType::Manual(v) => *v,
            crate::application::dto::websocket_messages::DiceInputType::Formula(_) => 0, // Formula parsing not implemented in mock
        };
        let mut s = self.state.lock().unwrap();
        s.sent_rolls.push((challenge_id.to_string(), roll_value));
        Ok(())
    }

    fn heartbeat(&self) -> anyhow::Result<()> {
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
}

