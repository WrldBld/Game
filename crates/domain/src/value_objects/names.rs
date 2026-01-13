//! Validated name newtypes for domain entities
//!
//! These newtypes ensure that names are valid by construction:
//! - Non-empty (except Description)
//! - Within length limits
//! - Trimmed of leading/trailing whitespace

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::DomainError;

/// Maximum length for name fields (CharacterName, LocationName, WorldName)
const MAX_NAME_LENGTH: usize = 200;

/// Maximum length for description fields
const MAX_DESCRIPTION_LENGTH: usize = 5000;

// ============================================================================
// CharacterName
// ============================================================================

/// A validated character name (non-empty, <=200 chars, trimmed)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CharacterName(String);

impl CharacterName {
    /// Create a new validated character name.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::Validation` if:
    /// - The name is empty after trimming
    /// - The name exceeds 200 characters after trimming
    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into();
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(DomainError::validation("Character name cannot be empty"));
        }
        if trimmed.len() > MAX_NAME_LENGTH {
            return Err(DomainError::validation(format!(
                "Character name cannot exceed {} characters",
                MAX_NAME_LENGTH
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Returns the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CharacterName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for CharacterName {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<CharacterName> for String {
    fn from(name: CharacterName) -> String {
        name.0
    }
}

// ============================================================================
// LocationName
// ============================================================================

/// A validated location name (non-empty, <=200 chars, trimmed)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct LocationName(String);

impl LocationName {
    /// Create a new validated location name.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::Validation` if:
    /// - The name is empty after trimming
    /// - The name exceeds 200 characters after trimming
    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into();
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(DomainError::validation("Location name cannot be empty"));
        }
        if trimmed.len() > MAX_NAME_LENGTH {
            return Err(DomainError::validation(format!(
                "Location name cannot exceed {} characters",
                MAX_NAME_LENGTH
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Returns the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LocationName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for LocationName {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<LocationName> for String {
    fn from(name: LocationName) -> String {
        name.0
    }
}

// ============================================================================
// WorldName
// ============================================================================

/// A validated world name (non-empty, <=200 chars, trimmed)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WorldName(String);

impl WorldName {
    /// Create a new validated world name.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::Validation` if:
    /// - The name is empty after trimming
    /// - The name exceeds 200 characters after trimming
    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into();
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(DomainError::validation("World name cannot be empty"));
        }
        if trimmed.len() > MAX_NAME_LENGTH {
            return Err(DomainError::validation(format!(
                "World name cannot exceed {} characters",
                MAX_NAME_LENGTH
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Returns the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorldName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for WorldName {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<WorldName> for String {
    fn from(name: WorldName) -> String {
        name.0
    }
}

// ============================================================================
// Description
// ============================================================================

/// A validated description (<=5000 chars, empty is valid)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Description(String);

impl Description {
    /// Create a new validated description.
    ///
    /// Empty strings are valid for descriptions.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::Validation` if the description exceeds 5000 characters.
    pub fn new(text: impl Into<String>) -> Result<Self, DomainError> {
        let text = text.into();
        if text.len() > MAX_DESCRIPTION_LENGTH {
            return Err(DomainError::validation(format!(
                "Description cannot exceed {} characters",
                MAX_DESCRIPTION_LENGTH
            )));
        }
        Ok(Self(text))
    }

    /// Create an empty description.
    pub fn empty() -> Self {
        Self(String::new())
    }

    /// Returns the description as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns true if the description is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for Description {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Description {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<Description> for String {
    fn from(desc: Description) -> String {
        desc.0
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod character_name {
        use super::*;

        #[test]
        fn valid_name() {
            let name = CharacterName::new("Gandalf").unwrap();
            assert_eq!(name.as_str(), "Gandalf");
            assert_eq!(name.to_string(), "Gandalf");
        }

        #[test]
        fn empty_name_rejected() {
            let result = CharacterName::new("");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, DomainError::Validation(_)));
            assert!(err.to_string().contains("cannot be empty"));
        }

        #[test]
        fn whitespace_only_rejected() {
            let result = CharacterName::new("   ");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, DomainError::Validation(_)));
        }

        #[test]
        fn name_is_trimmed() {
            let name = CharacterName::new("  Frodo Baggins  ").unwrap();
            assert_eq!(name.as_str(), "Frodo Baggins");
        }

        #[test]
        fn too_long_rejected() {
            let long_name = "a".repeat(201);
            let result = CharacterName::new(long_name);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, DomainError::Validation(_)));
            assert!(err.to_string().contains("200"));
        }

        #[test]
        fn max_length_accepted() {
            let max_name = "a".repeat(200);
            let name = CharacterName::new(max_name).unwrap();
            assert_eq!(name.as_str().len(), 200);
        }

        #[test]
        fn try_from_string() {
            let name: CharacterName = "Aragorn".to_string().try_into().unwrap();
            assert_eq!(name.as_str(), "Aragorn");
        }

        #[test]
        fn into_string() {
            let name = CharacterName::new("Legolas").unwrap();
            let s: String = name.into();
            assert_eq!(s, "Legolas");
        }

        #[test]
        fn serde_roundtrip() {
            let name = CharacterName::new("Gimli").unwrap();
            let json = serde_json::to_string(&name).unwrap();
            assert_eq!(json, "\"Gimli\"");

            let deserialized: CharacterName = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.as_str(), "Gimli");
        }

        #[test]
        fn serde_invalid_rejected() {
            let result: Result<CharacterName, _> = serde_json::from_str("\"\"");
            assert!(result.is_err());
        }
    }

    mod location_name {
        use super::*;

        #[test]
        fn valid_name() {
            let name = LocationName::new("Rivendell").unwrap();
            assert_eq!(name.as_str(), "Rivendell");
        }

        #[test]
        fn empty_name_rejected() {
            let result = LocationName::new("");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("cannot be empty"));
        }

        #[test]
        fn name_is_trimmed() {
            let name = LocationName::new("  Moria  ").unwrap();
            assert_eq!(name.as_str(), "Moria");
        }

        #[test]
        fn too_long_rejected() {
            let long_name = "a".repeat(201);
            let result = LocationName::new(long_name);
            assert!(result.is_err());
        }

        #[test]
        fn serde_roundtrip() {
            let name = LocationName::new("Gondor").unwrap();
            let json = serde_json::to_string(&name).unwrap();
            let deserialized: LocationName = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.as_str(), "Gondor");
        }
    }

    mod world_name {
        use super::*;

        #[test]
        fn valid_name() {
            let name = WorldName::new("Middle-earth").unwrap();
            assert_eq!(name.as_str(), "Middle-earth");
        }

        #[test]
        fn empty_name_rejected() {
            let result = WorldName::new("");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("cannot be empty"));
        }

        #[test]
        fn name_is_trimmed() {
            let name = WorldName::new("  Narnia  ").unwrap();
            assert_eq!(name.as_str(), "Narnia");
        }

        #[test]
        fn too_long_rejected() {
            let long_name = "a".repeat(201);
            let result = WorldName::new(long_name);
            assert!(result.is_err());
        }

        #[test]
        fn serde_roundtrip() {
            let name = WorldName::new("Westeros").unwrap();
            let json = serde_json::to_string(&name).unwrap();
            let deserialized: WorldName = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.as_str(), "Westeros");
        }
    }

    mod description {
        use super::*;

        #[test]
        fn valid_description() {
            let desc = Description::new("A powerful wizard").unwrap();
            assert_eq!(desc.as_str(), "A powerful wizard");
        }

        #[test]
        fn empty_is_valid() {
            let desc = Description::new("").unwrap();
            assert!(desc.is_empty());
            assert_eq!(desc.as_str(), "");
        }

        #[test]
        fn empty_constructor() {
            let desc = Description::empty();
            assert!(desc.is_empty());
        }

        #[test]
        fn default_is_empty() {
            let desc = Description::default();
            assert!(desc.is_empty());
        }

        #[test]
        fn too_long_rejected() {
            let long_desc = "a".repeat(5001);
            let result = Description::new(long_desc);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("5000"));
        }

        #[test]
        fn max_length_accepted() {
            let max_desc = "a".repeat(5000);
            let desc = Description::new(max_desc).unwrap();
            assert_eq!(desc.as_str().len(), 5000);
            assert!(!desc.is_empty());
        }

        #[test]
        fn serde_roundtrip() {
            let desc = Description::new("An ancient elf lord").unwrap();
            let json = serde_json::to_string(&desc).unwrap();
            let deserialized: Description = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.as_str(), "An ancient elf lord");
        }

        #[test]
        fn serde_empty_roundtrip() {
            let desc = Description::empty();
            let json = serde_json::to_string(&desc).unwrap();
            assert_eq!(json, "\"\"");
            let deserialized: Description = serde_json::from_str(&json).unwrap();
            assert!(deserialized.is_empty());
        }
    }
}
