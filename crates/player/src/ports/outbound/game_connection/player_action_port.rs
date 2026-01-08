//! Player Action Port - Handles player-initiated game actions
//!
//! This trait defines operations that players can perform during gameplay,
//! such as sending actions, submitting challenge rolls, and managing inventory.

use crate::outbound::GameConnectionPort;
use crate::session_types::DiceInput;

/// Port for player gameplay actions
///
/// Handles all actions that a player can take during a game session,
/// including combat, challenge responses, and inventory management.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
pub trait PlayerActionPort: Send + Sync {
    /// Send a player action to the server
    ///
    /// # Arguments
    /// * `action_type` - The type of action (e.g., "attack", "talk", "examine")
    /// * `target` - Optional target ID for the action
    /// * `dialogue` - Optional dialogue text for speech actions
    fn send_action(
        &self,
        action_type: &str,
        target: Option<String>,
        dialogue: Option<String>,
    ) -> anyhow::Result<()>;

    /// Start a conversation with an NPC
    fn start_conversation(&self, npc_id: &str, message: &str) -> anyhow::Result<()>;

    /// Continue a conversation with an NPC
    fn continue_conversation(&self, npc_id: &str, message: &str) -> anyhow::Result<()>;

    /// Perform a scene interaction by ID
    fn perform_interaction(&self, interaction_id: &str) -> anyhow::Result<()>;

    /// Submit a challenge roll (Player only) - legacy method using raw i32
    fn submit_challenge_roll(&self, challenge_id: &str, roll: i32) -> anyhow::Result<()>;

    /// Submit a challenge roll with dice input (Player only) - supports formulas and manual input
    fn submit_challenge_roll_input(
        &self,
        challenge_id: &str,
        input: DiceInput,
    ) -> anyhow::Result<()>;

    /// Equip an item (Player only)
    fn equip_item(&self, pc_id: &str, item_id: &str) -> anyhow::Result<()>;

    /// Unequip an item (Player only)
    fn unequip_item(&self, pc_id: &str, item_id: &str) -> anyhow::Result<()>;

    /// Drop an item (Player only) - currently destroys the item
    fn drop_item(&self, pc_id: &str, item_id: &str, quantity: u32) -> anyhow::Result<()>;

    /// Pick up an item from current region (Player only)
    fn pickup_item(&self, pc_id: &str, item_id: &str) -> anyhow::Result<()>;
}

// =============================================================================
// Blanket implementation: GameConnectionPort -> PlayerActionPort
// =============================================================================

/// Blanket implementation allowing any `GameConnectionPort` to be used as `PlayerActionPort`
impl<T: GameConnectionPort + ?Sized> PlayerActionPort for T {
    fn send_action(
        &self,
        action_type: &str,
        target: Option<String>,
        dialogue: Option<String>,
    ) -> anyhow::Result<()> {
        GameConnectionPort::send_action(self, action_type, target.as_deref(), dialogue.as_deref())
    }

    fn start_conversation(&self, npc_id: &str, message: &str) -> anyhow::Result<()> {
        GameConnectionPort::start_conversation(self, npc_id, message)
    }

    fn continue_conversation(&self, npc_id: &str, message: &str) -> anyhow::Result<()> {
        GameConnectionPort::continue_conversation(self, npc_id, message)
    }

    fn perform_interaction(&self, interaction_id: &str) -> anyhow::Result<()> {
        GameConnectionPort::perform_interaction(self, interaction_id)
    }

    fn submit_challenge_roll(&self, challenge_id: &str, roll: i32) -> anyhow::Result<()> {
        GameConnectionPort::submit_challenge_roll(self, challenge_id, roll)
    }

    fn submit_challenge_roll_input(
        &self,
        challenge_id: &str,
        input: DiceInput,
    ) -> anyhow::Result<()> {
        GameConnectionPort::submit_challenge_roll_input(self, challenge_id, input)
    }

    fn equip_item(&self, pc_id: &str, item_id: &str) -> anyhow::Result<()> {
        GameConnectionPort::equip_item(self, pc_id, item_id)
    }

    fn unequip_item(&self, pc_id: &str, item_id: &str) -> anyhow::Result<()> {
        GameConnectionPort::unequip_item(self, pc_id, item_id)
    }

    fn drop_item(&self, pc_id: &str, item_id: &str, quantity: u32) -> anyhow::Result<()> {
        GameConnectionPort::drop_item(self, pc_id, item_id, quantity)
    }

    fn pickup_item(&self, pc_id: &str, item_id: &str) -> anyhow::Result<()> {
        GameConnectionPort::pickup_item(self, pc_id, item_id)
    }
}
