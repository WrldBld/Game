//! String conversion utilities.

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
}
