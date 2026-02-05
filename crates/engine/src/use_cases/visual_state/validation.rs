//! Shared validation helpers for visual state operations.
//!
//! This module consolidates validation logic between the API and use case layers
//! to prevent drift in limits and error messaging. All validations use character
//! count (Unicode scalar values) rather than byte length for consistent UX across
//! all languages.

/// Maximum length for state names (characters, not bytes).
pub const MAX_STATE_NAME: usize = 100;

/// Maximum length for state descriptions (characters, not bytes).
pub const MAX_STATE_DESCRIPTION: usize = 5000;

/// Maximum length for generation prompts (characters, not bytes).
pub const MAX_PROMPT_LENGTH: usize = 5000;

/// Maximum length for workflow IDs (characters, not bytes).
pub const MAX_WORKFLOW_LENGTH: usize = 100;

/// Maximum length for negative prompts (characters, not bytes).
pub const MAX_NEGATIVE_PROMPT_LENGTH: usize = 2000;

/// Maximum number of tags allowed.
pub const MAX_TAGS_COUNT: usize = 20;

/// Maximum length for individual tags (characters, not bytes).
pub const MAX_TAG_LENGTH: usize = 50;

/// Maximum length for asset paths (characters, not bytes).
pub const MAX_ASSET_PATH_LENGTH: usize = 500;

/// Result type for validation operations.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub error_message: Option<String>,
}

impl ValidationResult {
    /// Create a successful validation result.
    pub fn success() -> Self {
        Self {
            is_valid: true,
            error_message: None,
        }
    }

    /// Create a failed validation result with the given message.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            error_message: Some(message.into()),
        }
    }

    /// Returns true if validation passed.
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    /// Returns the error message if validation failed.
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

/// Validates a state name for length (character count, not bytes).
///
/// # Errors
/// Returns an error if the name exceeds `MAX_STATE_NAME` characters.
pub fn validate_state_name(name: &str) -> ValidationResult {
    if name.chars().count() > MAX_STATE_NAME {
        return ValidationResult::error(format!("Name too long (max {} chars)", MAX_STATE_NAME));
    }
    ValidationResult::success()
}

/// Validates a state name for non-empty and length constraints.
///
/// # Errors
/// Returns an error if the name is empty (after trimming) or exceeds `MAX_STATE_NAME` characters.
pub fn validate_state_name_required(name: &str) -> ValidationResult {
    if name.trim().is_empty() {
        return ValidationResult::error("Name cannot be empty".to_string());
    }
    validate_state_name(name)
}

/// Validates an optional state description for length (character count, not bytes).
///
/// # Errors
/// Returns an error if the description exceeds `MAX_STATE_DESCRIPTION` characters.
pub fn validate_state_description(description: Option<&str>) -> ValidationResult {
    if let Some(desc) = description {
        if desc.chars().count() > MAX_STATE_DESCRIPTION {
            return ValidationResult::error(format!(
                "Description too long (max {} chars)",
                MAX_STATE_DESCRIPTION
            ));
        }
    }
    ValidationResult::success()
}

/// Validates an asset path for length (character count, not bytes).
///
/// # Errors
/// Returns an error if the path exceeds `MAX_ASSET_PATH_LENGTH` characters.
pub fn validate_asset_path(path: Option<&str>, field_name: &str) -> ValidationResult {
    if let Some(p) = path {
        if p.chars().count() > MAX_ASSET_PATH_LENGTH {
            return ValidationResult::error(format!(
                "{} path too long (max {} chars)",
                field_name, MAX_ASSET_PATH_LENGTH
            ));
        }
    }
    ValidationResult::success()
}

/// Validates a generation prompt for length (character count, not bytes).
///
/// # Errors
/// Returns an error if the prompt exceeds `MAX_PROMPT_LENGTH` characters.
pub fn validate_generation_prompt(prompt: Option<&str>) -> ValidationResult {
    if let Some(p) = prompt {
        if p.chars().count() > MAX_PROMPT_LENGTH {
            return ValidationResult::error(format!(
                "Generation prompt too long (max {} chars)",
                MAX_PROMPT_LENGTH
            ));
        }
    }
    ValidationResult::success()
}

/// Validates a workflow ID for length (character count, not bytes).
///
/// # Errors
/// Returns an error if the workflow ID exceeds `MAX_WORKFLOW_LENGTH` characters.
pub fn validate_workflow_id(workflow_id: Option<&str>) -> ValidationResult {
    if let Some(w) = workflow_id {
        if w.chars().count() > MAX_WORKFLOW_LENGTH {
            return ValidationResult::error(format!(
                "Workflow ID too long (max {} chars)",
                MAX_WORKFLOW_LENGTH
            ));
        }
    }
    ValidationResult::success()
}

/// Validates a negative prompt for length (character count, not bytes).
///
/// # Errors
/// Returns an error if the negative prompt exceeds `MAX_NEGATIVE_PROMPT_LENGTH` characters.
pub fn validate_negative_prompt(neg_prompt: Option<&str>) -> ValidationResult {
    if let Some(np) = neg_prompt {
        if np.chars().count() > MAX_NEGATIVE_PROMPT_LENGTH {
            return ValidationResult::error(format!(
                "Negative prompt too long (max {} chars)",
                MAX_NEGATIVE_PROMPT_LENGTH
            ));
        }
    }
    ValidationResult::success()
}

/// Validates tags for count and individual tag length.
///
/// # Errors
/// Returns an error if there are more than `MAX_TAGS_COUNT` tags,
/// or if any individual tag exceeds `MAX_TAG_LENGTH` characters.
pub fn validate_tags(tags: &[String]) -> ValidationResult {
    if tags.len() > MAX_TAGS_COUNT {
        return ValidationResult::error(format!("Too many tags (max {})", MAX_TAGS_COUNT));
    }
    for tag in tags {
        if tag.chars().count() > MAX_TAG_LENGTH {
            return ValidationResult::error(format!("Tag too long (max {} chars)", MAX_TAG_LENGTH));
        }
    }
    ValidationResult::success()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_state_name_within_limit() {
        let name = "A".repeat(MAX_STATE_NAME);
        assert!(validate_state_name(&name).is_valid());
    }

    #[test]
    fn test_validate_state_name_exceeds_limit() {
        let name = "A".repeat(MAX_STATE_NAME + 1);
        let result = validate_state_name(&name);
        assert!(!result.is_valid());
        assert!(result.error_message().expect("result should be Some").contains("Name too long"));
    }

    #[test]
    fn test_validate_state_name_uses_char_count_not_bytes() {
        // Emoji is 4 bytes but 1 character
        let name = "🎮".repeat(MAX_STATE_NAME);
        assert_eq!(name.chars().count(), MAX_STATE_NAME);
        assert!(name.len() > MAX_STATE_NAME); // Byte length exceeds
        assert!(validate_state_name(&name).is_valid());
    }

    #[test]
    fn test_validate_state_name_required_exceeds_limit() {
        let name = "A".repeat(MAX_STATE_NAME + 1);
        let result = validate_state_name_required(&name);
        assert!(!result.is_valid());
        assert!(result.error_message().expect("result should be Some").contains("Name too long"));
    }

    #[test]
    fn test_validate_state_name_required_empty() {
        let result = validate_state_name_required("");
        assert!(!result.is_valid());
        assert!(result.error_message().expect("result should be Some").contains("cannot be empty"));
    }

    #[test]
    fn test_validate_state_name_required_whitespace_only() {
        let result = validate_state_name_required("   ");
        assert!(!result.is_valid());
        assert!(result.error_message().expect("result should be Some").contains("cannot be empty"));
    }

    #[test]
    fn test_validate_state_description_within_limit() {
        let desc = Some("A".repeat(MAX_STATE_DESCRIPTION));
        assert!(validate_state_description(desc.as_deref()).is_valid());
    }

    #[test]
    fn test_validate_state_description_exceeds_limit() {
        let desc = Some("A".repeat(MAX_STATE_DESCRIPTION + 1));
        let result = validate_state_description(desc.as_deref());
        assert!(!result.is_valid());
        assert!(result
            .error_message()
            .expect("result should be Some")
            .contains("Description too long"));
    }

    #[test]
    fn test_validate_state_description_none_is_valid() {
        assert!(validate_state_description(None).is_valid());
    }

    #[test]
    fn test_validate_asset_path_within_limit() {
        let path = Some("/assets/test.png".to_string());
        assert!(validate_asset_path(path.as_deref(), "Backdrop").is_valid());
    }

    #[test]
    fn test_validate_asset_path_exceeds_limit() {
        let path = Some("/".repeat(MAX_ASSET_PATH_LENGTH + 1));
        let result = validate_asset_path(path.as_deref(), "Backdrop");
        assert!(!result.is_valid());
        assert!(result
            .error_message()
            .expect("result should be Some")
            .contains("Backdrop path too long"));
    }

    #[test]
    fn test_validate_asset_path_none_is_valid() {
        assert!(validate_asset_path(None, "Backdrop").is_valid());
    }

    #[test]
    fn test_validate_generation_prompt_within_limit() {
        let prompt = Some("A".repeat(MAX_PROMPT_LENGTH));
        assert!(validate_generation_prompt(prompt.as_deref()).is_valid());
    }

    #[test]
    fn test_validate_generation_prompt_exceeds_limit() {
        let prompt = Some("A".repeat(MAX_PROMPT_LENGTH + 1));
        let result = validate_generation_prompt(prompt.as_deref());
        assert!(!result.is_valid());
        assert!(result
            .error_message()
            .expect("result should be Some")
            .contains("Generation prompt too long"));
    }

    #[test]
    fn test_validate_generation_prompt_none_is_valid() {
        assert!(validate_generation_prompt(None).is_valid());
    }

    #[test]
    fn test_validate_workflow_id_within_limit() {
        let workflow = Some("workflow_v1".to_string());
        assert!(validate_workflow_id(workflow.as_deref()).is_valid());
    }

    #[test]
    fn test_validate_workflow_id_exceeds_limit() {
        let workflow = Some("A".repeat(MAX_WORKFLOW_LENGTH + 1));
        let result = validate_workflow_id(workflow.as_deref());
        assert!(!result.is_valid());
        assert!(result
            .error_message()
            .expect("result should be Some")
            .contains("Workflow ID too long"));
    }

    #[test]
    fn test_validate_workflow_id_none_is_valid() {
        assert!(validate_workflow_id(None).is_valid());
    }

    #[test]
    fn test_validate_negative_prompt_within_limit() {
        let prompt = Some("A".repeat(MAX_NEGATIVE_PROMPT_LENGTH));
        assert!(validate_negative_prompt(prompt.as_deref()).is_valid());
    }

    #[test]
    fn test_validate_negative_prompt_exceeds_limit() {
        let prompt = Some("A".repeat(MAX_NEGATIVE_PROMPT_LENGTH + 1));
        let result = validate_negative_prompt(prompt.as_deref());
        assert!(!result.is_valid());
        assert!(result
            .error_message()
            .expect("result should be Some")
            .contains("Negative prompt too long"));
    }

    #[test]
    fn test_validate_negative_prompt_none_is_valid() {
        assert!(validate_negative_prompt(None).is_valid());
    }

    #[test]
    fn test_validate_tags_within_limits() {
        let tags: Vec<String> = (0..MAX_TAGS_COUNT).map(|i| format!("tag{}", i)).collect();
        assert!(validate_tags(&tags).is_valid());
    }

    #[test]
    fn test_validate_tags_exceeds_count() {
        let tags: Vec<String> = (0..MAX_TAGS_COUNT + 1)
            .map(|i| format!("tag{}", i))
            .collect();
        let result = validate_tags(&tags);
        assert!(!result.is_valid());
        assert!(result.error_message().expect("result should be Some").contains("Too many tags"));
    }

    #[test]
    fn test_validate_tags_exceeds_length() {
        let tags: Vec<String> = vec!["A".repeat(MAX_TAG_LENGTH + 1)];
        let result = validate_tags(&tags);
        assert!(!result.is_valid());
        assert!(result.error_message().expect("result should be Some").contains("Tag too long"));
    }

    #[test]
    fn test_validate_tags_empty_is_valid() {
        assert!(validate_tags(&[]).is_valid());
    }
}
