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
// SceneName
// ============================================================================

/// A validated scene name (non-empty, <=200 chars, trimmed)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SceneName(String);

impl SceneName {
    /// Create a new validated scene name.
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
            return Err(DomainError::validation("Scene name cannot be empty"));
        }
        if trimmed.len() > MAX_NAME_LENGTH {
            return Err(DomainError::validation(format!(
                "Scene name cannot exceed {} characters",
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

impl fmt::Display for SceneName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for SceneName {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<SceneName> for String {
    fn from(name: SceneName) -> String {
        name.0
    }
}

// ============================================================================
// NarrativeEventName
// ============================================================================

/// A validated narrative event name (non-empty, <=200 chars, trimmed)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NarrativeEventName(String);

impl NarrativeEventName {
    /// Create a new validated narrative event name.
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
            return Err(DomainError::validation(
                "Narrative event name cannot be empty",
            ));
        }
        if trimmed.len() > MAX_NAME_LENGTH {
            return Err(DomainError::validation(format!(
                "Narrative event name cannot exceed {} characters",
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

impl fmt::Display for NarrativeEventName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for NarrativeEventName {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<NarrativeEventName> for String {
    fn from(name: NarrativeEventName) -> String {
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
// Tag
// ============================================================================

/// Maximum length for tag values
const MAX_TAG_LENGTH: usize = 50;

/// A validated tag (non-empty, <=50 chars, trimmed, lowercase)
///
/// Tags are used for categorization and filtering. They are:
/// - Non-empty
/// - Maximum 50 characters
/// - Trimmed of whitespace
/// - Converted to lowercase for consistent comparison
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Tag(String);

impl Tag {
    /// Create a new validated tag.
    ///
    /// The tag is trimmed and converted to lowercase.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::Validation` if:
    /// - The tag is empty after trimming
    /// - The tag exceeds 50 characters after trimming
    pub fn new(tag: impl Into<String>) -> Result<Self, DomainError> {
        let tag = tag.into();
        let trimmed = tag.trim().to_lowercase();
        if trimmed.is_empty() {
            return Err(DomainError::validation("Tag cannot be empty"));
        }
        if trimmed.len() > MAX_TAG_LENGTH {
            return Err(DomainError::validation(format!(
                "Tag cannot exceed {} characters",
                MAX_TAG_LENGTH
            )));
        }
        Ok(Self(trimmed))
    }

    /// Returns the tag as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Tag {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<Tag> for String {
    fn from(tag: Tag) -> String {
        tag.0
    }
}

// ============================================================================
// AssetPath
// ============================================================================

/// Maximum length for asset paths
const MAX_ASSET_PATH_LENGTH: usize = 500;

/// A validated asset path (non-empty, <=500 chars, trimmed, no invalid chars)
///
/// Asset paths are used for file references to images, audio, and other media.
/// They are:
/// - Non-empty after trimming
/// - Maximum 500 characters
/// - Cannot contain null bytes or control characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssetPath(String);

impl AssetPath {
    /// Create a new validated asset path.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::Validation` if:
    /// - The path is empty after trimming
    /// - The path exceeds 500 characters
    /// - The path contains null bytes or control characters
    pub fn new(path: impl Into<String>) -> Result<Self, DomainError> {
        let path = path.into();
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(DomainError::validation("Asset path cannot be empty"));
        }
        if trimmed.len() > MAX_ASSET_PATH_LENGTH {
            return Err(DomainError::validation(format!(
                "Asset path cannot exceed {} characters",
                MAX_ASSET_PATH_LENGTH
            )));
        }
        // Check for invalid characters (null bytes, control chars except newline/tab)
        if trimmed
            .chars()
            .any(|c| c == '\0' || (c.is_control() && c != '\n' && c != '\t'))
        {
            return Err(DomainError::validation(
                "Asset path cannot contain null bytes or control characters",
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Returns the path as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AssetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for AssetPath {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<AssetPath> for String {
    fn from(path: AssetPath) -> String {
        path.0
    }
}

impl AsRef<str> for AssetPath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// RegionName
// ============================================================================

/// A validated region name (non-empty, <=200 chars, trimmed)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RegionName(String);

impl RegionName {
    /// Create a new validated region name.
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
            return Err(DomainError::validation("Region name cannot be empty"));
        }
        if trimmed.len() > MAX_NAME_LENGTH {
            return Err(DomainError::validation(format!(
                "Region name cannot exceed {} characters",
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

impl fmt::Display for RegionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for RegionName {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<RegionName> for String {
    fn from(name: RegionName) -> String {
        name.0
    }
}

impl AsRef<str> for RegionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// ItemName
// ============================================================================

/// A validated item name (non-empty, <=200 chars, trimmed)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ItemName(String);

impl ItemName {
    /// Create a new validated item name.
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
            return Err(DomainError::validation("Item name cannot be empty"));
        }
        if trimmed.len() > MAX_NAME_LENGTH {
            return Err(DomainError::validation(format!(
                "Item name cannot exceed {} characters",
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

impl fmt::Display for ItemName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for ItemName {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<ItemName> for String {
    fn from(name: ItemName) -> String {
        name.0
    }
}

impl AsRef<str> for ItemName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// ChallengeName
// ============================================================================

/// A validated challenge name (non-empty, <=200 chars, trimmed)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ChallengeName(String);

impl ChallengeName {
    /// Create a new validated challenge name.
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
            return Err(DomainError::validation("Challenge name cannot be empty"));
        }
        if trimmed.len() > MAX_NAME_LENGTH {
            return Err(DomainError::validation(format!(
                "Challenge name cannot exceed {} characters",
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

impl fmt::Display for ChallengeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for ChallengeName {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<ChallengeName> for String {
    fn from(name: ChallengeName) -> String {
        name.0
    }
}

impl AsRef<str> for ChallengeName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// GoalName
// ============================================================================

/// A validated goal name (non-empty, <=200 chars, trimmed)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct GoalName(String);

impl GoalName {
    /// Create a new validated goal name.
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
            return Err(DomainError::validation("Goal name cannot be empty"));
        }
        if trimmed.len() > MAX_NAME_LENGTH {
            return Err(DomainError::validation(format!(
                "Goal name cannot exceed {} characters",
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

impl fmt::Display for GoalName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for GoalName {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<GoalName> for String {
    fn from(name: GoalName) -> String {
        name.0
    }
}

impl AsRef<str> for GoalName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// Atmosphere
// ============================================================================

/// Maximum length for atmosphere fields
const MAX_ATMOSPHERE_LENGTH: usize = 2000;

/// A validated atmosphere description (<=2000 chars, empty is valid)
///
/// Atmosphere describes the sensory/emotional feel of a location or region.
/// Examples: "Warm candlelight flickers across polished brass...",
/// "The air is thick with the smell of salt and rotting fish..."
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Atmosphere(String);

impl Atmosphere {
    /// Create a new validated atmosphere description.
    ///
    /// Empty strings are valid for atmosphere (represents no special atmosphere).
    /// The input is trimmed of leading/trailing whitespace.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::Validation` if the atmosphere exceeds 2000 characters.
    pub fn new(text: impl Into<String>) -> Result<Self, DomainError> {
        let text = text.into();
        let trimmed = text.trim().to_string();
        if trimmed.len() > MAX_ATMOSPHERE_LENGTH {
            return Err(DomainError::validation(format!(
                "Atmosphere cannot exceed {} characters",
                MAX_ATMOSPHERE_LENGTH
            )));
        }
        Ok(Self(trimmed))
    }

    /// Create an empty atmosphere.
    pub fn empty() -> Self {
        Self(String::new())
    }

    /// Returns the atmosphere as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns true if the atmosphere is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Display for Atmosphere {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Atmosphere {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<Atmosphere> for String {
    fn from(atm: Atmosphere) -> String {
        atm.0
    }
}

// ============================================================================
// StateName
// ============================================================================

/// Maximum length for state name fields
const MAX_STATE_NAME_LENGTH: usize = 100;

/// A validated state name (non-empty, <=100 chars, trimmed)
///
/// Used for LocationState and RegionState names.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct StateName(String);

impl StateName {
    /// Create a new validated state name.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::Validation` if:
    /// - The name is empty after trimming
    /// - The name exceeds 100 characters after trimming
    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into();
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(DomainError::validation("State name cannot be empty"));
        }
        if trimmed.len() > MAX_STATE_NAME_LENGTH {
            return Err(DomainError::validation(format!(
                "State name cannot exceed {} characters",
                MAX_STATE_NAME_LENGTH
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Returns the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for StateName {
    fn default() -> Self {
        Self("Default".to_string())
    }
}

impl fmt::Display for StateName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for StateName {
    type Error = DomainError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<StateName> for String {
    fn from(name: StateName) -> String {
        name.0
    }
}

impl AsRef<str> for StateName {
    fn as_ref(&self) -> &str {
        &self.0
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
        fn clone_preserves_name() {
            let name = CharacterName::new("Gimli").unwrap();
            let cloned = name.clone();
            assert_eq!(cloned.as_str(), "Gimli");
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
        fn clone_preserves_name() {
            let name = LocationName::new("Gondor").unwrap();
            let cloned = name.clone();
            assert_eq!(cloned.as_str(), "Gondor");
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
        fn clone_preserves_name() {
            let name = WorldName::new("Westeros").unwrap();
            let cloned = name.clone();
            assert_eq!(cloned.as_str(), "Westeros");
        }
    }

    mod scene_name {
        use super::*;

        #[test]
        fn valid_name() {
            let name = SceneName::new("The Hidden Grove").unwrap();
            assert_eq!(name.as_str(), "The Hidden Grove");
        }

        #[test]
        fn empty_name_rejected() {
            let result = SceneName::new("");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("cannot be empty"));
        }

        #[test]
        fn name_is_trimmed() {
            let name = SceneName::new("  The Threshold  ").unwrap();
            assert_eq!(name.as_str(), "The Threshold");
        }

        #[test]
        fn too_long_rejected() {
            let long_name = "a".repeat(201);
            let result = SceneName::new(long_name);
            assert!(result.is_err());
        }

        #[test]
        fn clone_preserves_name() {
            let name = SceneName::new("Crossing the Bridge").unwrap();
            let cloned = name.clone();
            assert_eq!(cloned.as_str(), "Crossing the Bridge");
        }
    }

    mod narrative_event_name {
        use super::*;

        #[test]
        fn valid_name() {
            let name = NarrativeEventName::new("The Door Unlocks").unwrap();
            assert_eq!(name.as_str(), "The Door Unlocks");
        }

        #[test]
        fn empty_name_rejected() {
            let result = NarrativeEventName::new("");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("cannot be empty"));
        }

        #[test]
        fn name_is_trimmed() {
            let name = NarrativeEventName::new("  A Sudden Storm  ").unwrap();
            assert_eq!(name.as_str(), "A Sudden Storm");
        }

        #[test]
        fn too_long_rejected() {
            let long_name = "a".repeat(201);
            let result = NarrativeEventName::new(long_name);
            assert!(result.is_err());
        }

        #[test]
        fn clone_preserves_name() {
            let name = NarrativeEventName::new("The Oath Broken").unwrap();
            let cloned = name.clone();
            assert_eq!(cloned.as_str(), "The Oath Broken");
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
        fn clone_preserves_description() {
            let desc = Description::new("An ancient elf lord").unwrap();
            let cloned = desc.clone();
            assert_eq!(cloned.as_str(), "An ancient elf lord");
        }

        #[test]
        fn empty_description_is_empty() {
            let desc = Description::empty();
            assert!(desc.is_empty());
        }
    }

    mod tag {
        use super::*;

        #[test]
        fn valid_tag() {
            let tag = Tag::new("combat").unwrap();
            assert_eq!(tag.as_str(), "combat");
            assert_eq!(tag.to_string(), "combat");
        }

        #[test]
        fn empty_tag_rejected() {
            let result = Tag::new("");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, DomainError::Validation(_)));
            assert!(err.to_string().contains("cannot be empty"));
        }

        #[test]
        fn whitespace_only_rejected() {
            let result = Tag::new("   ");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, DomainError::Validation(_)));
        }

        #[test]
        fn tag_is_trimmed() {
            let tag = Tag::new("  combat  ").unwrap();
            assert_eq!(tag.as_str(), "combat");
        }

        #[test]
        fn tag_is_lowercased() {
            let tag = Tag::new("COMBAT").unwrap();
            assert_eq!(tag.as_str(), "combat");
        }

        #[test]
        fn mixed_case_tag_is_lowercased() {
            let tag = Tag::new("High-Priority").unwrap();
            assert_eq!(tag.as_str(), "high-priority");
        }

        #[test]
        fn too_long_rejected() {
            let long_tag = "a".repeat(51);
            let result = Tag::new(long_tag);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, DomainError::Validation(_)));
            assert!(err.to_string().contains("50"));
        }

        #[test]
        fn max_length_accepted() {
            let max_tag = "a".repeat(50);
            let tag = Tag::new(max_tag).unwrap();
            assert_eq!(tag.as_str().len(), 50);
        }

        #[test]
        fn try_from_string() {
            let tag: Tag = "Stealth".to_string().try_into().unwrap();
            assert_eq!(tag.as_str(), "stealth"); // Lowercased
        }

        #[test]
        fn into_string() {
            let tag = Tag::new("puzzle").unwrap();
            let s: String = tag.into();
            assert_eq!(s, "puzzle");
        }

        #[test]
        fn clone_preserves_tag() {
            let tag = Tag::new("exploration").unwrap();
            let cloned = tag.clone();
            assert_eq!(cloned.as_str(), "exploration");
        }
    }

    mod asset_path {
        use super::*;

        #[test]
        fn valid_path() {
            let path = AssetPath::new("assets/images/hero.png").unwrap();
            assert_eq!(path.as_str(), "assets/images/hero.png");
            assert_eq!(path.to_string(), "assets/images/hero.png");
        }

        #[test]
        fn empty_path_rejected() {
            let result = AssetPath::new("");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, DomainError::Validation(_)));
            assert!(err.to_string().contains("cannot be empty"));
        }

        #[test]
        fn whitespace_only_rejected() {
            let result = AssetPath::new("   ");
            assert!(result.is_err());
        }

        #[test]
        fn path_is_trimmed() {
            let path = AssetPath::new("  assets/hero.png  ").unwrap();
            assert_eq!(path.as_str(), "assets/hero.png");
        }

        #[test]
        fn too_long_rejected() {
            let long_path = "a".repeat(501);
            let result = AssetPath::new(long_path);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("500"));
        }

        #[test]
        fn max_length_accepted() {
            let max_path = "a".repeat(500);
            let path = AssetPath::new(max_path).unwrap();
            assert_eq!(path.as_str().len(), 500);
        }

        #[test]
        fn null_byte_rejected() {
            let result = AssetPath::new("assets/\0/image.png");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("control characters"));
        }

        #[test]
        fn control_char_rejected() {
            let result = AssetPath::new("assets/\x01/image.png");
            assert!(result.is_err());
        }

        #[test]
        fn newline_and_tab_allowed() {
            // These are valid in file content paths (though unusual)
            let path = AssetPath::new("assets/path\twith\ttabs").unwrap();
            assert!(path.as_str().contains('\t'));
        }

        #[test]
        fn try_from_string() {
            let path: AssetPath = "sprites/npc.png".to_string().try_into().unwrap();
            assert_eq!(path.as_str(), "sprites/npc.png");
        }

        #[test]
        fn into_string() {
            let path = AssetPath::new("map.png").unwrap();
            let s: String = path.into();
            assert_eq!(s, "map.png");
        }
    }

    mod region_name {
        use super::*;

        #[test]
        fn valid_name() {
            let name = RegionName::new("The Great Hall").unwrap();
            assert_eq!(name.as_str(), "The Great Hall");
        }

        #[test]
        fn empty_name_rejected() {
            let result = RegionName::new("");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("cannot be empty"));
        }

        #[test]
        fn name_is_trimmed() {
            let name = RegionName::new("  The Courtyard  ").unwrap();
            assert_eq!(name.as_str(), "The Courtyard");
        }

        #[test]
        fn too_long_rejected() {
            let long_name = "a".repeat(201);
            let result = RegionName::new(long_name);
            assert!(result.is_err());
        }
    }

    mod item_name {
        use super::*;

        #[test]
        fn valid_name() {
            let name = ItemName::new("Sword of Flames").unwrap();
            assert_eq!(name.as_str(), "Sword of Flames");
        }

        #[test]
        fn empty_name_rejected() {
            let result = ItemName::new("");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("cannot be empty"));
        }

        #[test]
        fn name_is_trimmed() {
            let name = ItemName::new("  Healing Potion  ").unwrap();
            assert_eq!(name.as_str(), "Healing Potion");
        }

        #[test]
        fn too_long_rejected() {
            let long_name = "a".repeat(201);
            let result = ItemName::new(long_name);
            assert!(result.is_err());
        }
    }

    mod challenge_name {
        use super::*;

        #[test]
        fn valid_name() {
            let name = ChallengeName::new("Climb the Wall").unwrap();
            assert_eq!(name.as_str(), "Climb the Wall");
        }

        #[test]
        fn empty_name_rejected() {
            let result = ChallengeName::new("");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("cannot be empty"));
        }

        #[test]
        fn name_is_trimmed() {
            let name = ChallengeName::new("  Pick the Lock  ").unwrap();
            assert_eq!(name.as_str(), "Pick the Lock");
        }

        #[test]
        fn too_long_rejected() {
            let long_name = "a".repeat(201);
            let result = ChallengeName::new(long_name);
            assert!(result.is_err());
        }
    }

    mod goal_name {
        use super::*;

        #[test]
        fn valid_name() {
            let name = GoalName::new("Power").unwrap();
            assert_eq!(name.as_str(), "Power");
        }

        #[test]
        fn empty_name_rejected() {
            let result = GoalName::new("");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("cannot be empty"));
        }

        #[test]
        fn name_is_trimmed() {
            let name = GoalName::new("  Revenge  ").unwrap();
            assert_eq!(name.as_str(), "Revenge");
        }

        #[test]
        fn too_long_rejected() {
            let long_name = "a".repeat(201);
            let result = GoalName::new(long_name);
            assert!(result.is_err());
        }
    }

    mod atmosphere {
        use super::*;

        #[test]
        fn valid_atmosphere() {
            let atm =
                Atmosphere::new("Warm candlelight flickers across polished brass...").unwrap();
            assert_eq!(
                atm.as_str(),
                "Warm candlelight flickers across polished brass..."
            );
        }

        #[test]
        fn empty_is_valid() {
            let atm = Atmosphere::new("").unwrap();
            assert!(atm.is_empty());
            assert_eq!(atm.as_str(), "");
        }

        #[test]
        fn empty_constructor() {
            let atm = Atmosphere::empty();
            assert!(atm.is_empty());
        }

        #[test]
        fn default_is_empty() {
            let atm = Atmosphere::default();
            assert!(atm.is_empty());
        }

        #[test]
        fn whitespace_only_becomes_empty() {
            let atm = Atmosphere::new("   ").unwrap();
            assert!(atm.is_empty());
        }

        #[test]
        fn is_trimmed() {
            let atm = Atmosphere::new("  The air is thick with smoke.  ").unwrap();
            assert_eq!(atm.as_str(), "The air is thick with smoke.");
        }

        #[test]
        fn too_long_rejected() {
            let long_atm = "a".repeat(2001);
            let result = Atmosphere::new(long_atm);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("2000"));
        }

        #[test]
        fn max_length_accepted() {
            let max_atm = "a".repeat(2000);
            let atm = Atmosphere::new(max_atm).unwrap();
            assert_eq!(atm.as_str().len(), 2000);
            assert!(!atm.is_empty());
        }

        #[test]
        fn try_from_string() {
            let atm: Atmosphere = "A cold wind blows".to_string().try_into().unwrap();
            assert_eq!(atm.as_str(), "A cold wind blows");
        }

        #[test]
        fn into_string() {
            let atm = Atmosphere::new("Musty").unwrap();
            let s: String = atm.into();
            assert_eq!(s, "Musty");
        }

        #[test]
        fn clone_preserves_atmosphere() {
            let atm = Atmosphere::new("Damp and cold").unwrap();
            let cloned = atm.clone();
            assert_eq!(cloned.as_str(), "Damp and cold");
        }
    }

    mod state_name {
        use super::*;

        #[test]
        fn valid_name() {
            let name = StateName::new("Evening State").unwrap();
            assert_eq!(name.as_str(), "Evening State");
            assert_eq!(name.to_string(), "Evening State");
        }

        #[test]
        fn empty_name_rejected() {
            let result = StateName::new("");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, DomainError::Validation(_)));
            assert!(err.to_string().contains("cannot be empty"));
        }

        #[test]
        fn whitespace_only_rejected() {
            let result = StateName::new("   ");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, DomainError::Validation(_)));
        }

        #[test]
        fn name_is_trimmed() {
            let name = StateName::new("  Festival Day  ").unwrap();
            assert_eq!(name.as_str(), "Festival Day");
        }

        #[test]
        fn too_long_rejected() {
            let long_name = "a".repeat(101);
            let result = StateName::new(long_name);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, DomainError::Validation(_)));
            assert!(err.to_string().contains("100"));
        }

        #[test]
        fn max_length_accepted() {
            let max_name = "a".repeat(100);
            let name = StateName::new(max_name).unwrap();
            assert_eq!(name.as_str().len(), 100);
        }

        #[test]
        fn try_from_string() {
            let name: StateName = "Morning State".to_string().try_into().unwrap();
            assert_eq!(name.as_str(), "Morning State");
        }

        #[test]
        fn into_string() {
            let name = StateName::new("Under Siege").unwrap();
            let s: String = name.into();
            assert_eq!(s, "Under Siege");
        }

        #[test]
        fn clone_preserves_name() {
            let name = StateName::new("Holiday").unwrap();
            let cloned = name.clone();
            assert_eq!(cloned.as_str(), "Holiday");
        }

        #[test]
        fn default_returns_valid_name() {
            let name = StateName::default();
            assert_eq!(name.as_str(), "Default");
        }
    }
}
