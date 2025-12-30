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

use crate::routes::Route;

/// Parse a wrldbldr:// URL into a Route
///
/// Extracts the path from a wrldbldr:// scheme URL and maps it to the
/// corresponding application route. Returns None if the URL is invalid.
///
/// # Arguments
/// * `url` - Full URL string (e.g., "wrldbldr://worlds/abc-123/dm")
///
/// # Returns
/// Some(Route) if the URL is valid, None if it cannot be parsed
///
/// # Examples
/// ```ignore
/// assert_eq!(
///     parse_url_scheme("wrldbldr://"),
///     Some(Route::MainMenuRoute {})
/// );
///
/// assert_eq!(
///     parse_url_scheme("wrldbldr://worlds/abc-123/dm"),
///     Some(Route::DMViewRoute { world_id: "abc-123".to_string() })
/// );
/// ```
pub fn parse_url_scheme(url: &str) -> Option<Route> {
    let url = url.strip_prefix("wrldbldr://")?;

    // Parse path segments, filtering out empty strings
    let segments: Vec<&str> = url.split('/').filter(|s| !s.is_empty()).collect();

    // Match on path segments to determine the route
    match segments.as_slice() {
        // wrldbldr:// → MainMenu
        [] => Some(Route::MainMenuRoute {}),

        // wrldbldr://roles → RoleSelect
        ["roles"] => Some(Route::RoleSelectRoute {}),

        // wrldbldr://worlds → WorldSelect
        ["worlds"] => Some(Route::WorldSelectRoute {}),

        // wrldbldr://worlds/{world_id}/dm → DMView
        ["worlds", world_id, "dm"] => Some(Route::DMViewRoute {
            world_id: world_id.to_string(),
        }),

        // wrldbldr://worlds/{world_id}/play → PCView
        ["worlds", world_id, "play"] => Some(Route::PCViewRoute {
            world_id: world_id.to_string(),
        }),

        // wrldbldr://worlds/{world_id}/watch → SpectatorView
        ["worlds", world_id, "watch"] => Some(Route::SpectatorViewRoute {
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
        assert!(matches!(
            parse_url_scheme("wrldbldr://"),
            Some(Route::MainMenuRoute {})
        ));
    }

    #[test]
    fn test_parse_role_select() {
        assert!(matches!(
            parse_url_scheme("wrldbldr://roles"),
            Some(Route::RoleSelectRoute {})
        ));
    }

    #[test]
    fn test_parse_world_select() {
        assert!(matches!(
            parse_url_scheme("wrldbldr://worlds"),
            Some(Route::WorldSelectRoute {})
        ));
    }

    #[test]
    fn test_parse_dm_view() {
        let route = parse_url_scheme("wrldbldr://worlds/abc-123/dm");
        assert!(matches!(
            route,
            Some(Route::DMViewRoute { world_id }) if world_id == "abc-123"
        ));
    }

    #[test]
    fn test_parse_pc_view() {
        let route = parse_url_scheme("wrldbldr://worlds/test-world/play");
        assert!(matches!(
            route,
            Some(Route::PCViewRoute { world_id }) if world_id == "test-world"
        ));
    }

    #[test]
    fn test_parse_spectator_view() {
        let route = parse_url_scheme("wrldbldr://worlds/world-001/watch");
        assert!(matches!(
            route,
            Some(Route::SpectatorViewRoute { world_id }) if world_id == "world-001"
        ));
    }

    #[test]
    fn test_parse_invalid_path() {
        assert_eq!(parse_url_scheme("wrldbldr://invalid/path"), None);
        assert_eq!(parse_url_scheme("wrldbldr://worlds/id"), None);
        assert_eq!(parse_url_scheme("http://example.com"), None);
    }

    #[test]
    fn test_parse_with_trailing_slash() {
        assert!(matches!(
            parse_url_scheme("wrldbldr://roles/"),
            Some(Route::RoleSelectRoute {})
        ));
    }
}
