//! Common validation helpers for use cases.

/// Validation error type.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("{field_name} cannot be empty")]
    Empty { field_name: &'static str },

    #[error("{field_name} exceeds maximum length of {max}")]
    TooLong { field_name: &'static str, max: usize },

    #[error("{field_name} is invalid: {reason}")]
    Invalid { field_name: &'static str, reason: String },
}

/// Validate a string is non-empty after trimming.
pub fn require_non_empty(value: &str, field_name: &'static str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::Empty { field_name });
    }
    Ok(())
}

/// Validate a string doesn't exceed max length.
pub fn require_max_length(value: &str, max: usize, field_name: &'static str) -> Result<(), ValidationError> {
    if value.len() > max {
        return Err(ValidationError::TooLong { field_name, max });
    }
    Ok(())
}

/// Validate an optional string is non-empty if present.
pub fn require_non_empty_if_present(
    value: &Option<String>,
    field_name: &'static str,
) -> Result<(), ValidationError> {
    if let Some(v) = value {
        require_non_empty(v, field_name)?;
    }
    Ok(())
}

/// Validate a string length is within range.
pub fn require_length_range(
    value: &str,
    min: usize,
    max: usize,
    field_name: &'static str,
) -> Result<(), ValidationError> {
    if value.len() < min {
        return Err(ValidationError::Invalid {
            field_name,
            reason: format!("must be at least {} characters", min),
        });
    }
    if value.len() > max {
        return Err(ValidationError::TooLong { field_name, max });
    }
    Ok(())
}
