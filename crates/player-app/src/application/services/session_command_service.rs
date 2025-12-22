//! Session command service for sending real-time (WebSocket) commands.
//!
//! This service is intentionally small: it exists to keep presentation code
//! free of transport concerns and to centralize command semantics.

use std::sync::Arc;

use anyhow::Result;

use wrldbldr_protocol::messages::{DiceInputType, DirectorialContext};
use wrldbldr_protocol::types::ApprovalDecision;
use wrldbldr_player_ports::outbound::GameConnectionPort;

/// Application service for sending session commands via the game connection.
#[derive(Clone)]
pub struct SessionCommandService {
    connection: Arc<dyn GameConnectionPort>,
}

impl SessionCommandService {
    pub fn new(connection: Arc<dyn GameConnectionPort>) -> Self {
        Self { connection }
    }

    pub fn send_directorial_update(&self, context: DirectorialContext) -> Result<()> {
        self.connection.send_directorial_update(context)
    }

    pub fn send_approval_decision(&self, request_id: &str, decision: ApprovalDecision) -> Result<()> {
        self.connection.send_approval_decision(request_id, decision)
    }

    pub fn trigger_challenge(&self, challenge_id: &str, target_character_id: &str) -> Result<()> {
        self.connection.trigger_challenge(challenge_id, target_character_id)
    }

    pub fn submit_challenge_roll(&self, challenge_id: &str, roll: i32) -> Result<()> {
        self.connection.submit_challenge_roll(challenge_id, roll)
    }

    pub fn submit_challenge_roll_input(&self, challenge_id: &str, input: DiceInputType) -> Result<()> {
        self.connection.submit_challenge_roll_input(challenge_id, input)
    }
}

