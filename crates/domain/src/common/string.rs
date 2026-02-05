//! String conversion utilities and validation helpers.

use crate::error::DomainError;

/// Validates a string field that must be non-empty after trimming.
///
/// This helper enforces the common validation pattern:
/// - Trim leading/trailing whitespace
/// - Ensure non-empty after trimming
/// - Enforce maximum length
/// - Return descriptive validation errors with field name
///
/// # Arguments
///
/// * `value` - The string value to validate
/// * `field_name` - The name of the field (for error messages)
/// * `max_len` - Maximum allowed length after trimming
///
/// # Errors
///
/// Returns `DomainError::Validation` if:
/// - The value is empty after trimming
/// - The value exceeds `max_len` characters after trimming
///
/// # Examples
///
/// ```ignore
/// use wrldbldr_domain::common::validate_trimmed_non_empty;
/// use wrldbldr_domain::error::DomainError;
///
/// // Valid input
/// let result = validate_trimmed_non_empty("  Gandalf  ", "Character name", 200);
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap(), "Gandalf");
///
/// // Empty after trimming
/// let result = validate_trimmed_non_empty("   ", "Character name", 200);
/// assert!(matches!(result, Err(DomainError::Validation(_))));
/// assert!(result.unwrap_err().to_string().contains("Character name cannot be empty"));
///
/// // Too long
/// let long = "a".repeat(201);
/// let result = validate_trimmed_non_empty(&long, "Character name", 200);
/// assert!(matches!(result, Err(DomainError::Validation(_))));
/// assert!(result.unwrap_err().to_string().contains("200"));
/// ```
pub fn validate_trimmed_non_empty(
    value: impl Into<String>,
    field_name: &str,
    max_len: usize,
) -> Result<String, DomainError> {
    let value = value.into();
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(DomainError::validation(format!(
            "{} cannot be empty",
            field_name
        )));
    }

    if trimmed.chars().count() > max_len {
        return Err(DomainError::validation(format!(
            "{} cannot exceed {} characters",
            field_name, max_len
        )));
    }

    Ok(trimmed.to_string())
}

/// Validates a string field with maximum length (empty is allowed).
///
/// This helper enforces the common validation pattern for fields that can be empty:
/// - Trim leading/trailing whitespace
/// - Enforce maximum length (empty is valid)
/// - Return descriptive validation errors with field name
///
/// # Arguments
///
/// * `value` - The string value to validate
/// * `field_name` - The name of the field (for error messages)
/// * `max_len` - Maximum allowed length after trimming
///
/// # Errors
///
/// Returns `DomainError::Validation` if the value exceeds `max_len` characters.
///
/// # Examples
///
/// ```ignore
/// use wrldbldr_domain::common::validate_trimmed_max;
///
/// // Valid input with content
/// let result = validate_trimmed_max("  Some description  ", "Description", 5000);
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap(), "Some description");
///
/// // Empty is valid
/// let result = validate_trimmed_max("", "Description", 5000);
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap(), "");
///
/// // Too long
/// let long = "a".repeat(5001);
/// let result = validate_trimmed_max(&long, "Description", 5000);
/// assert!(matches!(result, Err(DomainError::Validation(_))));
/// ```
pub fn validate_trimmed_max(
    value: impl Into<String>,
    field_name: &str,
    max_len: usize,
) -> Result<String, DomainError> {
    let value = value.into();
    let trimmed = value.trim();

    if trimmed.chars().count() > max_len {
        return Err(DomainError::validation(format!(
            "{} cannot exceed {} characters",
            field_name, max_len
        )));
    }

    Ok(trimmed.to_string())
}

/// Converts an empty string to `None`, otherwise returns `Some(value)`.
///
/// This is useful when deserializing optional fields that may be stored
/// as empty strings in databases.
///
/// # Examples
///
/// ```
/// use wrldbldr_domain::common::none_if_empty;
///
/// assert_eq!(none_if_empty("hello"), Some("hello"));
/// assert_eq!(none_if_empty(""), None);
/// assert_eq!(none_if_empty(" "), Some(" ")); // Whitespace is not empty
/// ```
pub fn none_if_empty(value: &str) -> Option<&str> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Converts an empty `String` to `None`, otherwise returns `Some(value)`.
///
/// Owned version of [`none_if_empty`] for when you have a `String`.
///
/// # Examples
///
/// ```
/// use wrldbldr_domain::common::some_if_not_empty;
///
/// assert_eq!(some_if_not_empty("hello".to_string()), Some("hello".to_string()));
/// assert_eq!(some_if_not_empty(String::new()), None);
/// ```
pub fn some_if_not_empty(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Extension trait for strings providing `into_option` as a method.
///
/// This allows for more fluent code when converting strings to options.
///
/// # Examples
///
/// ```
/// use wrldbldr_domain::common::StringExt;
///
/// let s = "hello".to_string();
/// assert_eq!(s.into_option(), Some("hello".to_string()));
///
/// let empty = String::new();
/// assert_eq!(empty.into_option(), None);
/// ```
pub trait StringExt {
    /// Converts this string to `None` if empty, otherwise `Some(self)`.
    fn into_option(self) -> Option<String>;
}

impl StringExt for String {
    fn into_option(self) -> Option<String> {
        some_if_not_empty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_none_if_empty_with_content() {
        assert_eq!(none_if_empty("hello"), Some("hello"));
        assert_eq!(none_if_empty("a"), Some("a"));
        assert_eq!(none_if_empty("hello world"), Some("hello world"));
    }

    #[test]
    fn test_none_if_empty_empty_string() {
        assert_eq!(none_if_empty(""), None);
    }

    #[test]
    fn test_none_if_empty_whitespace_is_not_empty() {
        assert_eq!(none_if_empty(" "), Some(" "));
        assert_eq!(none_if_empty("\t"), Some("\t"));
        assert_eq!(none_if_empty("\n"), Some("\n"));
        assert_eq!(none_if_empty("   "), Some("   "));
    }

    #[test]
    fn test_some_if_not_empty_with_content() {
        assert_eq!(
            some_if_not_empty("hello".to_string()),
            Some("hello".to_string())
        );
    }

    #[test]
    fn test_some_if_not_empty_empty_string() {
        assert_eq!(some_if_not_empty(String::new()), None);
        assert_eq!(some_if_not_empty("".to_string()), None);
    }

    #[test]
    fn test_string_ext_into_option() {
        assert_eq!("hello".to_string().into_option(), Some("hello".to_string()));
        assert_eq!(String::new().into_option(), None);
    }

    #[test]
    fn test_string_ext_whitespace() {
        assert_eq!(" ".to_string().into_option(), Some(" ".to_string()));
    }

    mod validate_trimmed_non_empty {
        use super::*;

        #[test]
        fn valid_string() {
            let result = validate_trimmed_non_empty("hello", "Test field", 100);
            assert!(result.is_ok());
            assert_eq!(result.expect("valid string"), "hello");
        }

        #[test]
        fn string_is_trimmed() {
            let result = validate_trimmed_non_empty("  hello  ", "Test field", 100);
            assert!(result.is_ok());
            assert_eq!(result.expect("valid trimmed string"), "hello");
        }

        #[test]
        fn empty_string_rejected() {
            let result = validate_trimmed_non_empty("", "Test field", 100);
            assert!(result.is_err());
            assert!(result
                .expect_err("empty string should fail")
                .to_string()
                .contains("Test field cannot be empty"));
        }

        #[test]
        fn whitespace_only_rejected() {
            let result = validate_trimmed_non_empty("   ", "Test field", 100);
            assert!(result.is_err());
            assert!(result
                .expect_err("whitespace only should fail")
                .to_string()
                .contains("cannot be empty"));
        }

        #[test]
        fn too_long_rejected() {
            let long = "a".repeat(101);
            let result = validate_trimmed_non_empty(&long, "Test field", 100);
            assert!(result.is_err());
            assert!(result
                .expect_err("too long should fail")
                .to_string()
                .contains("100"));
        }

        #[test]
        fn max_length_accepted() {
            let max = "a".repeat(100);
            let result = validate_trimmed_non_empty(&max, "Test field", 100);
            assert!(result.is_ok());
            assert_eq!(result.expect("max length should be valid").len(), 100);
        }

        #[test]
        fn error_message_contains_field_name() {
            let result = validate_trimmed_non_empty("", "Character name", 100);
            assert!(result.is_err());
            assert!(result
                .expect_err("empty should fail")
                .to_string()
                .contains("Character name"));
        }

        #[test]
        fn emoji_counted_as_single_character() {
            // Emoji are multi-byte (4 bytes each), but should count as 1 character
            let emoji_string = "😀".repeat(100);
            let result = validate_trimmed_non_empty(&emoji_string, "Test field", 100);
            assert!(result.is_ok(), "100 emoji should be valid for max_len=100");

            // 101 emoji should fail
            let emoji_string_101 = "😀".repeat(101);
            let result = validate_trimmed_non_empty(&emoji_string_101, "Test field", 100);
            assert!(result.is_err(), "101 emoji should fail for max_len=100");
        }

        #[test]
        fn mixed_ascii_and_multibyte_counted_correctly() {
            // "Hello 🌍" = 6 ASCII + 1 emoji (4 bytes) = 7 characters
            let mixed = "Hello 🌍".to_string();
            let result = validate_trimmed_non_empty(&mixed, "Test field", 7);
            assert!(
                result.is_ok(),
                "7 characters (including emoji) should be valid"
            );

            let result = validate_trimmed_non_empty(&mixed, "Test field", 6);
            assert!(result.is_err(), "7 characters should fail for max_len=6");
        }

        #[test]
        fn cjk_characters_counted_correctly() {
            // CJK characters are 3 bytes each in UTF-8
            let cjk = "日本語".to_string(); // 3 characters, 9 bytes
            let result = validate_trimmed_non_empty(&cjk, "Test field", 3);
            assert!(
                result.is_ok(),
                "3 CJK characters should be valid for max_len=3"
            );

            let result = validate_trimmed_non_empty(&cjk, "Test field", 2);
            assert!(
                result.is_err(),
                "3 CJK characters should fail for max_len=2"
            );
        }
    }

    mod validate_trimmed_max {
        use super::*;

        #[test]
        fn valid_string() {
            let result = validate_trimmed_max("hello", "Test field", 100);
            assert!(result.is_ok());
            assert_eq!(result.expect("valid string"), "hello");
        }

        #[test]
        fn string_is_trimmed() {
            let result = validate_trimmed_max("  hello  ", "Test field", 100);
            assert!(result.is_ok());
            assert_eq!(result.expect("valid trimmed string"), "hello");
        }

        #[test]
        fn empty_string_is_valid() {
            let result = validate_trimmed_max("", "Test field", 100);
            assert!(result.is_ok());
            assert_eq!(result.expect("empty should be valid"), "");
        }

        #[test]
        fn whitespace_only_becomes_empty() {
            let result = validate_trimmed_max("   ", "Test field", 100);
            assert!(result.is_ok());
            assert_eq!(result.expect("whitespace becomes empty"), "");
        }

        #[test]
        fn too_long_rejected() {
            let long = "a".repeat(101);
            let result = validate_trimmed_max(&long, "Test field", 100);
            assert!(result.is_err());
            assert!(result
                .expect_err("too long should fail")
                .to_string()
                .contains("100"));
        }

        #[test]
        fn max_length_accepted() {
            let max = "a".repeat(100);
            let result = validate_trimmed_max(&max, "Test field", 100);
            assert!(result.is_ok());
            assert_eq!(result.expect("max length valid").len(), 100);
        }

        #[test]
        fn error_message_contains_field_name() {
            let long = "a".repeat(101);
            let result = validate_trimmed_max(&long, "Description", 100);
            assert!(result.is_err());
            assert!(result
                .expect_err("too long should fail")
                .to_string()
                .contains("Description"));
        }

        #[test]
        fn emoji_counted_as_single_character() {
            // Emoji are multi-byte (4 bytes each), but should count as 1 character
            let emoji_string = "😀".repeat(100);
            let result = validate_trimmed_max(&emoji_string, "Test field", 100);
            assert!(result.is_ok(), "100 emoji should be valid for max_len=100");

            // 101 emoji should fail
            let emoji_string_101 = "😀".repeat(101);
            let result = validate_trimmed_max(&emoji_string_101, "Test field", 100);
            assert!(result.is_err(), "101 emoji should fail for max_len=100");
        }

        #[test]
        fn mixed_ascii_and_multibyte_counted_correctly() {
            // "Hello 🌍" = 6 ASCII + 1 emoji (4 bytes) = 7 characters
            let mixed = "Hello 🌍".to_string();
            let result = validate_trimmed_max(&mixed, "Test field", 7);
            assert!(
                result.is_ok(),
                "7 characters (including emoji) should be valid"
            );

            let result = validate_trimmed_max(&mixed, "Test field", 6);
            assert!(result.is_err(), "7 characters should fail for max_len=6");
        }

        #[test]
        fn cjk_characters_counted_correctly() {
            // CJK characters are 3 bytes each in UTF-8
            let cjk = "日本語".to_string(); // 3 characters, 9 bytes
            let result = validate_trimmed_max(&cjk, "Test field", 3);
            assert!(
                result.is_ok(),
                "3 CJK characters should be valid for max_len=3"
            );

            let result = validate_trimmed_max(&cjk, "Test field", 2);
            assert!(
                result.is_err(),
                "3 CJK characters should fail for max_len=2"
            );
        }

        #[test]
        fn empty_string_with_multibyte_limit_is_valid() {
            // Empty string should be valid even with small max_len
            let result = validate_trimmed_max("", "Test field", 1);
            assert!(
                result.is_ok(),
                "Empty string should be valid regardless of max_len"
            );
        }
    }
}
