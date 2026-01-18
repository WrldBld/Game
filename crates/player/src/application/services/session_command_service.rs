//! Session command service for sending real-time (WebSocket) commands.
//!
//! This service is intentionally small: it exists to keep presentation code
//! free of transport concerns and to centralize command semantics.

use anyhow::Result;
use wrldbldr_shared::ClientMessage;

use crate::application::dto::{ApprovalDecision, DiceInput, DirectorialContext};
use crate::infrastructure::messaging::CommandBus;

/// Application service for sending session commands via the game connection.
///
/// Uses `CommandBus` to send commands to the engine.
#[derive(Clone)]
pub struct SessionCommandService {
    commands: CommandBus,
}

impl SessionCommandService {
    pub fn new(commands: CommandBus) -> Self {
        Self { commands }
    }

    pub fn send_directorial_update(&self, context: DirectorialContext) -> Result<()> {
        self.commands.send(ClientMessage::DirectorialUpdate {
            context: context.into(),
        })
    }

    pub fn send_approval_decision(
        &self,
        request_id: &str,
        decision: ApprovalDecision,
    ) -> Result<()> {
        self.commands.send(ClientMessage::ApprovalDecision {
            request_id: request_id.to_string(),
            decision,
        })
    }

    pub fn trigger_challenge(&self, challenge_id: &str, target_character_id: &str) -> Result<()> {
        self.commands.send(ClientMessage::TriggerChallenge {
            challenge_id: challenge_id.to_string(),
            target_character_id: target_character_id.to_string(),
        })
    }

    pub fn submit_challenge_roll(&self, challenge_id: &str, roll: i32) -> Result<()> {
        self.commands.send(ClientMessage::ChallengeRoll {
            challenge_id: challenge_id.to_string(),
            roll,
        })
    }

    pub fn submit_challenge_roll_input(&self, challenge_id: &str, input: DiceInput) -> Result<()> {
        self.commands.send(ClientMessage::ChallengeRollInput {
            challenge_id: challenge_id.to_string(),
            input_type: input.into(),
        })
    }
}
