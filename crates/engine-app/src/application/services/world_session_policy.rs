//! World Session Policy Service
//!
//! Encapsulates business rules for world session management:
//! - Role validation (Player requires PC, Spectator requires target)
//! - DM uniqueness enforcement (one DM per world from different users)
//!
//! These are domain rules that belong in the application layer, not
//! in the connection management infrastructure.

use uuid::Uuid;
use wrldbldr_engine_ports::outbound::{ConnectionError, WorldRole};

/// Errors that can occur when validating a join request
#[derive(Debug, Clone, PartialEq)]
pub enum JoinPolicyError {
    /// Player role requires a PC ID
    PlayerRequiresPc,
    /// Spectator role requires a spectate target PC ID
    SpectatorRequiresTarget,
    /// Another user is already DM in this world
    DmAlreadyConnected { existing_user_id: String },
}

impl From<JoinPolicyError> for ConnectionError {
    fn from(err: JoinPolicyError) -> Self {
        match err {
            JoinPolicyError::PlayerRequiresPc => ConnectionError::PlayerRequiresPc,
            JoinPolicyError::SpectatorRequiresTarget => ConnectionError::SpectatorRequiresTarget,
            JoinPolicyError::DmAlreadyConnected { existing_user_id } => {
                ConnectionError::DmAlreadyConnected { existing_user_id }
            }
        }
    }
}

/// Result of validating a join request
#[derive(Debug, Clone)]
pub enum JoinValidation {
    /// Join is allowed
    Allowed,
    /// Join is denied with a specific error
    Denied(JoinPolicyError),
}

/// World session policy service
///
/// Validates join requests according to business rules.
/// This service is stateless - it takes current state as input and returns validation results.
pub struct WorldSessionPolicy;

impl WorldSessionPolicy {
    pub fn new() -> Self {
        Self
    }

    /// Validate role-specific requirements
    ///
    /// Business rules:
    /// - Player role requires a PC ID
    /// - Spectator role requires a spectate target PC ID
    pub fn validate_role_requirements(
        &self,
        role: WorldRole,
        pc_id: Option<Uuid>,
        spectate_pc_id: Option<Uuid>,
    ) -> JoinValidation {
        match role {
            WorldRole::Player if pc_id.is_none() => {
                JoinValidation::Denied(JoinPolicyError::PlayerRequiresPc)
            }
            WorldRole::Spectator if spectate_pc_id.is_none() => {
                JoinValidation::Denied(JoinPolicyError::SpectatorRequiresTarget)
            }
            _ => JoinValidation::Allowed,
        }
    }

    /// Validate DM uniqueness for a world
    ///
    /// Business rules:
    /// - Only one DM allowed per world
    /// - Same user can have multiple DM connections (multi-screen)
    /// - Different user cannot join as DM if DM exists
    pub fn validate_dm_availability(
        &self,
        role: WorldRole,
        user_id: &str,
        current_dm_user_id: Option<&str>,
    ) -> JoinValidation {
        if role != WorldRole::DM {
            return JoinValidation::Allowed;
        }

        match current_dm_user_id {
            None => JoinValidation::Allowed,
            Some(existing) if existing == user_id => {
                // Same user, allow multi-screen
                JoinValidation::Allowed
            }
            Some(existing) => {
                // Different user, deny
                JoinValidation::Denied(JoinPolicyError::DmAlreadyConnected {
                    existing_user_id: existing.to_string(),
                })
            }
        }
    }

    /// Full join validation combining all rules
    pub fn validate_join(
        &self,
        role: WorldRole,
        user_id: &str,
        pc_id: Option<Uuid>,
        spectate_pc_id: Option<Uuid>,
        current_dm_user_id: Option<&str>,
    ) -> JoinValidation {
        // Check role requirements first
        if let JoinValidation::Denied(err) =
            self.validate_role_requirements(role, pc_id, spectate_pc_id)
        {
            return JoinValidation::Denied(err);
        }

        // Check DM availability
        self.validate_dm_availability(role, user_id, current_dm_user_id)
    }
}

impl Default for WorldSessionPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_requires_pc() {
        let policy = WorldSessionPolicy::new();

        // Without PC - denied
        let result = policy.validate_role_requirements(WorldRole::Player, None, None);
        assert!(matches!(
            result,
            JoinValidation::Denied(JoinPolicyError::PlayerRequiresPc)
        ));

        // With PC - allowed
        let result =
            policy.validate_role_requirements(WorldRole::Player, Some(Uuid::new_v4()), None);
        assert!(matches!(result, JoinValidation::Allowed));
    }

    #[test]
    fn test_spectator_requires_target() {
        let policy = WorldSessionPolicy::new();

        // Without target - denied
        let result = policy.validate_role_requirements(WorldRole::Spectator, None, None);
        assert!(matches!(
            result,
            JoinValidation::Denied(JoinPolicyError::SpectatorRequiresTarget)
        ));

        // With target - allowed
        let result =
            policy.validate_role_requirements(WorldRole::Spectator, None, Some(Uuid::new_v4()));
        assert!(matches!(result, JoinValidation::Allowed));
    }

    #[test]
    fn test_dm_uniqueness() {
        let policy = WorldSessionPolicy::new();

        // No existing DM - allowed
        let result = policy.validate_dm_availability(WorldRole::DM, "user1", None);
        assert!(matches!(result, JoinValidation::Allowed));

        // Same user - allowed (multi-screen)
        let result = policy.validate_dm_availability(WorldRole::DM, "user1", Some("user1"));
        assert!(matches!(result, JoinValidation::Allowed));

        // Different user - denied
        let result = policy.validate_dm_availability(WorldRole::DM, "user2", Some("user1"));
        assert!(matches!(
            result,
            JoinValidation::Denied(JoinPolicyError::DmAlreadyConnected { .. })
        ));
    }
}
