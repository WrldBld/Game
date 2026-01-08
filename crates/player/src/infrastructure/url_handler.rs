//! URL scheme handler for desktop deep linking
//!
//! Handles `wrldbldr://` URLs for deep linking on desktop platforms:
//! - wrldbldr:// - Main menu
//! - wrldbldr://roles - Role selection
//! - wrldbldr://worlds - World selection
//! - wrldbldr://worlds/{world_id}/dm - DM view
//! - wrldbldr://worlds/{world_id}/play - Player view
//! - wrldbldr://worlds/{world_id}/watch - Spectator view
//!
//! On web platforms, URL navigation is handled by the Dioxus Router.
//! On desktop platforms, the OS will pass `wrldbldr://` URLs to the application.

/// Deep link extracted from a `wrldbldr://` url.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeepLink {
    MainMenu,
    RoleSelect,
    WorldSelect,
    DmView { world_id: String },
    PcView { world_id: String },
    SpectatorView { world_id: String },
}

/// Parse a `wrldbldr://` URL into a deep link.
///
/// Extracts the path from a `wrldbldr://` scheme URL and maps it to a `DeepLink`.
/// Returns `None` if the URL is invalid.
///
/// # Arguments
/// * `url` - Full URL string (e.g., "wrldbldr://worlds/abc-123/dm")
///
/// # Returns
/// `Some(DeepLink)` if the URL is valid, `None` if it cannot be parsed
///
/// # Examples
/// ```ignore
/// assert_eq!(parse_url_scheme("wrldbldr://"), Some(DeepLink::MainMenu));
/// assert_eq!(
///     parse_url_scheme("wrldbldr://worlds/abc-123/dm"),
///     Some(DeepLink::DmView {
///         world_id: "abc-123".to_string(),
///     })
/// );
/// ```
pub fn parse_url_scheme(url: &str) -> Option<DeepLink> {
    let url = url.strip_prefix("wrldbldr://")?;

    // Parse path segments, filtering out empty strings
    let segments: Vec<&str> = url.split('/').filter(|s| !s.is_empty()).collect();

    // Match on path segments to determine the deep link
    match segments.as_slice() {
        // wrldbldr:// → MainMenu
        [] => Some(DeepLink::MainMenu),

        // wrldbldr://roles → RoleSelect
        ["roles"] => Some(DeepLink::RoleSelect),

        // wrldbldr://worlds → WorldSelect
        ["worlds"] => Some(DeepLink::WorldSelect),

        // wrldbldr://worlds/{world_id}/dm → DMView
        ["worlds", world_id, "dm"] => Some(DeepLink::DmView {
            world_id: world_id.to_string(),
        }),

        // wrldbldr://worlds/{world_id}/play → PCView
        ["worlds", world_id, "play"] => Some(DeepLink::PcView {
            world_id: world_id.to_string(),
        }),

        // wrldbldr://worlds/{world_id}/watch → SpectatorView
        ["worlds", world_id, "watch"] => Some(DeepLink::SpectatorView {
            world_id: world_id.to_string(),
        }),

        // Invalid paths
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_main_menu() {
        assert_eq!(parse_url_scheme("wrldbldr://"), Some(DeepLink::MainMenu));
    }

    #[test]
    fn test_parse_role_select() {
        assert_eq!(
            parse_url_scheme("wrldbldr://roles"),
            Some(DeepLink::RoleSelect)
        );
    }

    #[test]
    fn test_parse_world_select() {
        assert_eq!(
            parse_url_scheme("wrldbldr://worlds"),
            Some(DeepLink::WorldSelect)
        );
    }

    #[test]
    fn test_parse_dm_view() {
        assert_eq!(
            parse_url_scheme("wrldbldr://worlds/abc-123/dm"),
            Some(DeepLink::DmView {
                world_id: "abc-123".to_string(),
            })
        );
    }

    #[test]
    fn test_parse_pc_view() {
        assert_eq!(
            parse_url_scheme("wrldbldr://worlds/test-world/play"),
            Some(DeepLink::PcView {
                world_id: "test-world".to_string(),
            })
        );
    }

    #[test]
    fn test_parse_spectator_view() {
        assert_eq!(
            parse_url_scheme("wrldbldr://worlds/world-001/watch"),
            Some(DeepLink::SpectatorView {
                world_id: "world-001".to_string(),
            })
        );
    }

    #[test]
    fn test_parse_invalid_path() {
        assert_eq!(parse_url_scheme("wrldbldr://invalid/path"), None);
        assert_eq!(parse_url_scheme("wrldbldr://worlds/id"), None);
        assert_eq!(parse_url_scheme("http://example.com"), None);
    }

    #[test]
    fn test_parse_with_trailing_slash() {
        assert_eq!(
            parse_url_scheme("wrldbldr://roles/"),
            Some(DeepLink::RoleSelect)
        );
    }
}
