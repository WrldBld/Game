//! Character lifecycle state enum
//!
//! Replaces the previous `is_alive: bool` and `is_active: bool` fields,
//! ensuring mutually exclusive states are properly modeled.

use serde::{Deserialize, Deserializer, Serialize};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Default)]
#[serde(rename_all = "camelCase")]
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

    /// Convert from legacy boolean flags (is_alive, is_active) to CharacterState
    ///
    /// # Mapping
    ///
    /// | is_alive | is_active | Result |
    /// |----------|-----------|--------|
    /// | true     | true      | Active |
    /// | true     | false     | Inactive |
    /// | false    | *         | Dead |
    ///
    /// # Examples
    ///
    /// ```
    /// use wrldbldr_domain::value_objects::CharacterState;
    ///
    /// assert_eq!(CharacterState::from_legacy(true, true), CharacterState::Active);
    /// assert_eq!(CharacterState::from_legacy(true, false), CharacterState::Inactive);
    /// assert_eq!(CharacterState::from_legacy(false, true), CharacterState::Dead);
    /// assert_eq!(CharacterState::from_legacy(false, false), CharacterState::Dead);
    /// ```
    #[inline]
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

// Custom deserializer that handles both the new enum format and legacy boolean format
impl<'de> Deserialize<'de> for CharacterState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct CharacterStateVisitor;

        impl<'de> Visitor<'de> for CharacterStateVisitor {
            type Value = CharacterState;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(
                    "a string (\"active\", \"inactive\", \"dead\") or \
                     an object with is_alive and is_active boolean fields",
                )
            }

            // Handle string format: "active", "inactive", "dead"
            fn visit_str<E>(self, value: &str) -> Result<CharacterState, E>
            where
                E: de::Error,
            {
                match value.to_lowercase().as_str() {
                    "active" => Ok(CharacterState::Active),
                    "inactive" => Ok(CharacterState::Inactive),
                    "dead" => Ok(CharacterState::Dead),
                    _ => Err(de::Error::unknown_variant(
                        value,
                        &["active", "inactive", "dead"],
                    )),
                }
            }

            // Handle legacy object format: { "is_alive": true, "is_active": false }
            fn visit_map<M>(self, mut map: M) -> Result<CharacterState, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut is_alive: Option<bool> = None;
                let mut is_active: Option<bool> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "is_alive" | "isAlive" => {
                            is_alive = Some(map.next_value()?);
                        }
                        "is_active" | "isActive" => {
                            is_active = Some(map.next_value()?);
                        }
                        _ => {
                            // Skip unknown fields
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let is_alive =
                    is_alive.ok_or_else(|| de::Error::missing_field("is_alive or isAlive"))?;
                let is_active =
                    is_active.ok_or_else(|| de::Error::missing_field("is_active or isActive"))?;

                Ok(CharacterState::from_legacy(is_alive, is_active))
            }
        }

        deserializer.deserialize_any(CharacterStateVisitor)
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

    mod serde {
        use super::*;

        #[test]
        fn serialize_to_camel_case() {
            assert_eq!(
                serde_json::to_string(&CharacterState::Active).unwrap(),
                "\"active\""
            );
            assert_eq!(
                serde_json::to_string(&CharacterState::Inactive).unwrap(),
                "\"inactive\""
            );
            assert_eq!(
                serde_json::to_string(&CharacterState::Dead).unwrap(),
                "\"dead\""
            );
        }

        #[test]
        fn deserialize_active_string() {
            let state: CharacterState = serde_json::from_str("\"active\"").unwrap();
            assert_eq!(state, CharacterState::Active);
        }

        #[test]
        fn deserialize_inactive_string() {
            let state: CharacterState = serde_json::from_str("\"inactive\"").unwrap();
            assert_eq!(state, CharacterState::Inactive);
        }

        #[test]
        fn deserialize_dead_string() {
            let state: CharacterState = serde_json::from_str("\"dead\"").unwrap();
            assert_eq!(state, CharacterState::Dead);
        }

        #[test]
        fn deserialize_case_insensitive() {
            let active: CharacterState = serde_json::from_str("\"ACTIVE\"").unwrap();
            assert_eq!(active, CharacterState::Active);

            let inactive: CharacterState = serde_json::from_str("\"Inactive\"").unwrap();
            assert_eq!(inactive, CharacterState::Inactive);

            let dead: CharacterState = serde_json::from_str("\"DEAD\"").unwrap();
            assert_eq!(dead, CharacterState::Dead);
        }

        #[test]
        fn roundtrip_serialization() {
            for state in [
                CharacterState::Active,
                CharacterState::Inactive,
                CharacterState::Dead,
            ] {
                let json = serde_json::to_string(&state).unwrap();
                let deserialized: CharacterState = serde_json::from_str(&json).unwrap();
                assert_eq!(state, deserialized);
            }
        }

        #[test]
        fn deserialize_invalid_string_fails() {
            let result: Result<CharacterState, _> = serde_json::from_str("\"unknown\"");
            assert!(result.is_err());
        }
    }

    mod legacy_format {
        use super::*;

        #[test]
        fn deserialize_legacy_snake_case_active() {
            let json = r#"{"is_alive": true, "is_active": true}"#;
            let state: CharacterState = serde_json::from_str(json).unwrap();
            assert_eq!(state, CharacterState::Active);
        }

        #[test]
        fn deserialize_legacy_snake_case_inactive() {
            let json = r#"{"is_alive": true, "is_active": false}"#;
            let state: CharacterState = serde_json::from_str(json).unwrap();
            assert_eq!(state, CharacterState::Inactive);
        }

        #[test]
        fn deserialize_legacy_snake_case_dead() {
            let json = r#"{"is_alive": false, "is_active": true}"#;
            let state: CharacterState = serde_json::from_str(json).unwrap();
            assert_eq!(state, CharacterState::Dead);

            let json = r#"{"is_alive": false, "is_active": false}"#;
            let state: CharacterState = serde_json::from_str(json).unwrap();
            assert_eq!(state, CharacterState::Dead);
        }

        #[test]
        fn deserialize_legacy_camel_case() {
            let json = r#"{"isAlive": true, "isActive": true}"#;
            let state: CharacterState = serde_json::from_str(json).unwrap();
            assert_eq!(state, CharacterState::Active);

            let json = r#"{"isAlive": true, "isActive": false}"#;
            let state: CharacterState = serde_json::from_str(json).unwrap();
            assert_eq!(state, CharacterState::Inactive);
        }

        #[test]
        fn deserialize_legacy_ignores_extra_fields() {
            let json = r#"{"is_alive": true, "is_active": false, "extra": "ignored"}"#;
            let state: CharacterState = serde_json::from_str(json).unwrap();
            assert_eq!(state, CharacterState::Inactive);
        }

        #[test]
        fn deserialize_legacy_missing_is_alive_fails() {
            let json = r#"{"is_active": true}"#;
            let result: Result<CharacterState, _> = serde_json::from_str(json);
            assert!(result.is_err());
        }

        #[test]
        fn deserialize_legacy_missing_is_active_fails() {
            let json = r#"{"is_alive": true}"#;
            let result: Result<CharacterState, _> = serde_json::from_str(json);
            assert!(result.is_err());
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
