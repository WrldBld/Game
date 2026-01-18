//! Action service for sending player actions to the Engine
//!
//! This service wraps the game connection port and provides convenient
//! methods for sending player actions. It depends on the trait abstraction,
//! not the concrete WebSocket implementation.

use anyhow::Result;
use wrldbldr_shared::ClientMessage;

use crate::application::dto::PlayerAction;
use crate::infrastructure::messaging::CommandBus;

/// Service for sending player actions to the Engine via WebSocket
///
/// This service uses the CommandBus to abstract the actual
/// connection implementation, allowing for different backends or testing.
/// The PlayerActionPort methods are available via blanket implementation.
pub struct ActionService {
    commands: CommandBus,
}

impl ActionService {
    /// Create a new ActionService with the given command bus
    pub fn new(commands: CommandBus) -> Self {
        Self { commands }
    }

    /// Send a player action to the Engine
    pub fn send_action(&self, action: PlayerAction) -> Result<()> {
        self.commands.send(ClientMessage::PlayerAction {
            action_type: action.action_type.as_str().to_string(),
            target: action.target,
            dialogue: action.dialogue,
        })
    }

    /// Send a dialogue choice selection
    pub fn select_choice(&self, choice_id: &str) -> Result<()> {
        let action = PlayerAction::dialogue_choice(choice_id);
        self.send_action(action)
    }

    /// Send custom dialogue input
    pub fn send_custom_dialogue(&self, text: &str) -> Result<()> {
        let action = PlayerAction::custom(text);
        self.send_action(action)
    }

    /// Send a talk action to an NPC
    pub fn talk_to(&self, npc_id: &str, dialogue: Option<&str>) -> Result<()> {
        let action = PlayerAction::talk(npc_id, dialogue);
        self.send_action(action)
    }

    /// Send an examine action
    pub fn examine(&self, target: &str) -> Result<()> {
        let action = PlayerAction::examine(target);
        self.send_action(action)
    }

    /// Send a travel action
    pub fn travel_to(&self, location_id: &str) -> Result<()> {
        let action = PlayerAction::travel(location_id);
        self.send_action(action)
    }

    /// Send a use item action
    pub fn use_item(&self, item_id: &str, target: Option<&str>) -> Result<()> {
        let action = PlayerAction::use_item(item_id, target);
        self.send_action(action)
    }

    /// Get a reference to the underlying command bus
    pub fn commands(&self) -> &CommandBus {
        &self.commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::messaging::{BusMessage, PendingRequests};
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex};

    fn create_test_command_bus() -> (CommandBus, mpsc::Receiver<BusMessage>) {
        let (tx, rx) = mpsc::channel(10);
        let pending = Arc::new(Mutex::new(PendingRequests::default()));
        (CommandBus::new(tx, pending), rx)
    }

    #[tokio::test]
    async fn send_action_records_outbound_call() {
        let (commands, mut rx) = create_test_command_bus();
        let svc = ActionService::new(commands);

        svc.send_action(PlayerAction::custom("hello")).unwrap();

        // Verify that a message was sent
        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg, BusMessage::Send(_)));
    }
}
