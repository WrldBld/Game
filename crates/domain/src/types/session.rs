//! Session-related domain types
//!
//! Types related to user sessions and roles in a world.

use serde::{Deserialize, Serialize};

/// Role of a user in a world.
///
/// This is a core domain concept representing the different types of
/// participants in a game session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WorldRole {
    /// Dungeon Master - can approve suggestions, control NPCs, full control
    Dm,
    /// Player - controls a player character
    Player,
    /// Spectator - can view but not interact
    #[default]
    Spectator,
}

impl WorldRole {
    /// Check if this role can modify data (DM or Player)
    pub fn can_modify(&self) -> bool {
        matches!(self, WorldRole::Dm | WorldRole::Player)
    }

    /// Check if this role is DM
    pub fn is_dm(&self) -> bool {
        matches!(self, WorldRole::Dm)
    }

    /// Check if this role is Player
    pub fn is_player(&self) -> bool {
        matches!(self, WorldRole::Player)
    }

    /// Check if this role is Spectator
    pub fn is_spectator(&self) -> bool {
        matches!(self, WorldRole::Spectator)
    }
}

impl std::fmt::Display for WorldRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorldRole::Dm => write!(f, "DM"),
            WorldRole::Player => write!(f, "Player"),
            WorldRole::Spectator => write!(f, "Spectator"),
        }
    }
}
