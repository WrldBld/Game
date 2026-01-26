//! Character lifecycle state enum
//!
//! Replaces the previous `is_alive: bool` and `is_active: bool` fields,
//! ensuring mutually exclusive states are properly modeled.
//!
//! # Tier Classification
//!
//! **Tier 2: Validated Enum** - Represents mutually exclusive character states.
//! No invalid states can be constructed (e.g., `is_alive=false && is_active=true` is impossible).
//! See [docs/architecture/tier-levels.md](../../../../docs/architecture/tier-levels.md)
//! for complete tier classification system.

use serde::{Deserialize, Serialize};

/// Character lifecycle state
///
/// Replaces the previous `is_alive: bool` and `is_active: bool` fields,
/// ensuring mutually exclusive states are properly modeled.
///
/// # State Transitions
///
/// ```text
/// Active <-> Inactive (can toggle freely while alive)
/// Active -> Dead (death)
/// Inactive -> Dead (death)
/// Dead -> Active (resurrection)
/// ```
///
/// # Examples
///
/// ```
/// use wrldbldr_domain::value_objects::CharacterState;
///
/// let state = CharacterState::Active;
/// assert!(state.is_alive());
/// assert!(state.is_active());
///
/// let dead = CharacterState::Dead;
/// assert!(!dead.is_alive());
/// assert!(dead.is_dead());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CharacterState {
    /// Character is alive and actively participating in the world
    #[default]
    Active,
    /// Character is alive but not currently participating (e.g., traveling, resting)
    Inactive,
    /// Character is dead
    Dead,
}

impl CharacterState {
    /// Returns true if the character is alive (Active or Inactive)
    ///
    /// # Examples
    ///
    /// ```
    /// use wrldbldr_domain::value_objects::CharacterState;
    ///
    /// assert!(CharacterState::Active.is_alive());
    /// assert!(CharacterState::Inactive.is_alive());
    /// assert!(!CharacterState::Dead.is_alive());
    /// ```
    #[inline]
    pub fn is_alive(self) -> bool {
        !matches!(self, Self::Dead)
    }

    /// Returns true if the character is actively participating
    ///
    /// # Examples
    ///
    /// ```
    /// use wrldbldr_domain::value_objects::CharacterState;
    ///
    /// assert!(CharacterState::Active.is_active());
    /// assert!(!CharacterState::Inactive.is_active());
    /// assert!(!CharacterState::Dead.is_active());
    /// ```
    #[inline]
    pub fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns true if the character is dead
    ///
    /// # Examples
    ///
    /// ```
    /// use wrldbldr_domain::value_objects::CharacterState;
    ///
    /// assert!(!CharacterState::Active.is_dead());
    /// assert!(!CharacterState::Inactive.is_dead());
    /// assert!(CharacterState::Dead.is_dead());
    /// ```
    #[inline]
    pub fn is_dead(self) -> bool {
        matches!(self, Self::Dead)
    }

    /// Returns true if the character is inactive (alive but not participating)
    ///
    /// # Examples
    ///
    /// ```
    /// use wrldbldr_domain::value_objects::CharacterState;
    ///
    /// assert!(!CharacterState::Active.is_inactive());
    /// assert!(CharacterState::Inactive.is_inactive());
    /// assert!(!CharacterState::Dead.is_inactive());
    /// ```
    #[inline]
    pub fn is_inactive(self) -> bool {
        matches!(self, Self::Inactive)
    }

    /// Convert legacy boolean fields to CharacterState.
    ///
    /// This is used for backward compatibility with existing data that uses
    /// `is_alive` and `is_active` boolean fields.
    ///
    /// # Arguments
    ///
    /// * `is_alive` - Legacy "is alive" flag
    /// * `is_active` - Legacy "is active" flag
    ///
    /// # Returns
    ///
    /// * `Active` if `is_alive` is true and `is_active` is true
    /// * `Inactive` if `is_alive` is true and `is_active` is false
    /// * `Dead` if `is_alive` is false (regardless of `is_active`)
    ///
    /// # Examples
    ///
    /// ```
    /// use wrldbldr_domain::value_objects::CharacterState;
    ///
    /// // Alive and active
    /// assert_eq!(CharacterState::from_legacy(true, true), CharacterState::Active);
    ///
    /// // Alive but not active
    /// assert_eq!(CharacterState::from_legacy(true, false), CharacterState::Inactive);
    ///
    /// // Dead (regardless of active state)
    /// assert_eq!(CharacterState::from_legacy(false, true), CharacterState::Dead);
    /// assert_eq!(CharacterState::from_legacy(false, false), CharacterState::Dead);
    /// ```
    pub fn from_legacy(is_alive: bool, is_active: bool) -> Self {
        if !is_alive {
            Self::Dead
        } else if is_active {
            Self::Active
        } else {
            Self::Inactive
        }
    }
}

impl std::fmt::Display for CharacterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Inactive => write!(f, "inactive"),
            Self::Dead => write!(f, "dead"),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod state_methods {
        use super::*;

        #[test]
        fn is_alive_returns_correct_values() {
            assert!(CharacterState::Active.is_alive());
            assert!(CharacterState::Inactive.is_alive());
            assert!(!CharacterState::Dead.is_alive());
        }

        #[test]
        fn is_active_returns_correct_values() {
            assert!(CharacterState::Active.is_active());
            assert!(!CharacterState::Inactive.is_active());
            assert!(!CharacterState::Dead.is_active());
        }

        #[test]
        fn is_dead_returns_correct_values() {
            assert!(!CharacterState::Active.is_dead());
            assert!(!CharacterState::Inactive.is_dead());
            assert!(CharacterState::Dead.is_dead());
        }

        #[test]
        fn is_inactive_returns_correct_values() {
            assert!(!CharacterState::Active.is_inactive());
            assert!(CharacterState::Inactive.is_inactive());
            assert!(!CharacterState::Dead.is_inactive());
        }

        #[test]
        fn default_returns_active() {
            assert_eq!(CharacterState::default(), CharacterState::Active);
        }

        #[test]
        fn display_formats_correctly() {
            assert_eq!(CharacterState::Active.to_string(), "active");
            assert_eq!(CharacterState::Inactive.to_string(), "inactive");
            assert_eq!(CharacterState::Dead.to_string(), "dead");
        }
    }

    mod from_legacy {
        use super::*;

        #[test]
        fn alive_and_active_gives_active() {
            assert_eq!(
                CharacterState::from_legacy(true, true),
                CharacterState::Active
            );
        }

        #[test]
        fn alive_and_not_active_gives_inactive() {
            assert_eq!(
                CharacterState::from_legacy(true, false),
                CharacterState::Inactive
            );
        }

        #[test]
        fn not_alive_gives_dead_regardless_of_active() {
            assert_eq!(
                CharacterState::from_legacy(false, true),
                CharacterState::Dead
            );
            assert_eq!(
                CharacterState::from_legacy(false, false),
                CharacterState::Dead
            );
        }
    }

    mod traits {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn copy_trait() {
            let state = CharacterState::Active;
            let copied = state;
            assert_eq!(state, copied); // Both are usable after copy
        }

        #[test]
        fn clone_trait() {
            let state = CharacterState::Inactive;
            let cloned = state.clone();
            assert_eq!(state, cloned);
        }

        #[test]
        fn hash_trait() {
            let mut set = HashSet::new();
            set.insert(CharacterState::Active);
            set.insert(CharacterState::Inactive);
            set.insert(CharacterState::Dead);

            assert!(set.contains(&CharacterState::Active));
            assert!(set.contains(&CharacterState::Inactive));
            assert!(set.contains(&CharacterState::Dead));
            assert_eq!(set.len(), 3);
        }

        #[test]
        fn eq_trait() {
            assert_eq!(CharacterState::Active, CharacterState::Active);
            assert_ne!(CharacterState::Active, CharacterState::Inactive);
            assert_ne!(CharacterState::Active, CharacterState::Dead);
        }

        #[test]
        fn debug_trait() {
            let debug_str = format!("{:?}", CharacterState::Active);
            assert_eq!(debug_str, "Active");
        }
    }
}
