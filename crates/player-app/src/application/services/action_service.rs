//! Action service for sending player actions to the Engine
//!
//! This service wraps the game connection port and provides convenient
//! methods for sending player actions. It depends on the trait abstraction,
//! not the concrete WebSocket implementation.

use anyhow::Result;

use std::sync::Arc;

use wrldbldr_player_ports::outbound::GameConnectionPort;
use crate::application::dto::PlayerAction;

/// Service for sending player actions to the Engine via WebSocket
///
/// This service uses the GameConnectionPort trait to abstract the actual
/// connection implementation, allowing for different backends or testing.
pub struct ActionService {
    connection: Arc<dyn GameConnectionPort>,
}

impl ActionService {
    /// Create a new ActionService with the given connection
    pub fn new(connection: Arc<dyn GameConnectionPort>) -> Self {
        Self { connection }
    }

    /// Send a player action to the Engine
    pub fn send_action(&self, action: PlayerAction) -> Result<()> {
        let action_type = action.action_type.as_str();

        self.connection.send_action(
            action_type,
            action.target.as_deref(),
            action.dialogue.as_deref(),
        )
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

    /// Get a reference to the underlying connection
    pub fn connection(&self) -> &dyn GameConnectionPort {
        self.connection.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_player_ports::outbound::MockGameConnectionPort;

    #[test]
    fn send_action_records_outbound_call() {
        let conn = Arc::new(MockGameConnectionPort::new("ws://test/ws"));
        let conn_dyn: Arc<dyn GameConnectionPort> = conn.clone();
        let svc = ActionService::new(conn_dyn);

        svc.send_action(PlayerAction::custom("hello")).unwrap();

        let sent = conn.sent_actions();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].action_type, "custom");
        assert_eq!(sent[0].dialogue.as_deref(), Some("hello"));
    }
}
