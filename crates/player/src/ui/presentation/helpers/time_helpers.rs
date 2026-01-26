//! Time control helper functions and tests
//!
//! This module provides pure helper functions for time-related UI logic
//! that can be tested independently of Dioxus components.

use wrldbldr_shared::types::TimeSuggestionDecision;

/// Converts hours to seconds for manual time advancement
///
/// # Examples
/// ```
/// use wrldbldr_player::ui::time_helpers::hours_to_seconds;
///
/// assert_eq!(hours_to_seconds(1), 3600);
/// assert_eq!(hours_to_seconds(4), 14400);
/// assert_eq!(hours_to_seconds(0), 0);
/// ```
pub fn hours_to_seconds(hours: u32) -> u32 {
    hours * 3600
}

/// Determines the time suggestion decision type based on modified vs original seconds
///
/// Returns "approve" if the modified seconds match the suggested seconds,
/// otherwise returns "modify" with the modified seconds value.
///
/// # Arguments
/// * `suggested_seconds` - The original suggested seconds
/// * `modified_seconds` - The user's modified seconds
///
/// # Returns
/// * "approve" if seconds match, "modify" if they differ
///
/// # Examples
/// ```
/// use wrldbldr_player::ui::time_helpers::determine_time_decision;
///
/// // Same seconds = approve
/// assert_eq!(determine_time_decision(600, 600), "approve");
///
/// // Different seconds = modify
/// assert_eq!(determine_time_decision(600, 900), "modify");
/// ```
pub fn determine_time_decision(suggested_seconds: u32, modified_seconds: u32) -> &'static str {
    if modified_seconds == suggested_seconds {
        "approve"
    } else {
        "modify"
    }
}

/// Creates a TimeSuggestionDecision for responding to time suggestions
///
/// # Arguments
/// * `decision_type` - Either "approve", "modify", or "skip"
/// * `modified_seconds` - Optional modified seconds (required for "modify")
///
/// # Examples
/// ```
/// use wrldbldr_player::ui::time_helpers::build_time_suggestion_decision;
/// use wrldbldr_shared::types::TimeSuggestionDecision;
///
/// let decision = build_time_suggestion_decision("approve", None);
/// assert!(matches!(decision, TimeSuggestionDecision::Approve));
///
/// let decision = build_time_suggestion_decision("modify", Some(900));
/// assert!(matches!(decision, TimeSuggestionDecision::Modify { seconds: 900 }));
/// ```
pub fn build_time_suggestion_decision(
    decision_type: &str,
    modified_seconds: Option<u32>,
) -> TimeSuggestionDecision {
    match decision_type {
        "approve" => TimeSuggestionDecision::Approve,
        "modify" => TimeSuggestionDecision::Modify {
            seconds: modified_seconds.unwrap_or(0),
        },
        "skip" => TimeSuggestionDecision::Skip,
        _ => TimeSuggestionDecision::Skip, // Default to skip on unknown
    }
}

/// Formats a time advance reason for display based on seconds
///
/// Generates human-readable descriptions like:
/// - "Time advanced by 1 hour"
/// - "Time advanced by 2 hours"
/// - "Time advanced by 30 minutes"
/// - "Time advanced by 1 hour 30 minutes"
///
/// # Arguments
/// * `seconds` - Total seconds to advance
/// * `custom_reason` - Optional custom reason to use instead
///
/// # Examples
/// ```
/// use wrldbldr_player::ui::time_helpers::format_advance_reason;
///
/// assert_eq!(format_advance_reason(3600, None), "Time advanced by 1 hour");
/// assert_eq!(format_advance_reason(7200, None), "Time advanced by 2 hours");
/// assert_eq!(format_advance_reason(1800, None), "Time advanced by 30 minutes");
/// assert_eq!(format_advance_reason(5400, None), "Time advanced by 1 hour 30 minutes");
/// assert_eq!(format_advance_reason(3600, Some("Party rested".to_string())), "Party rested");
/// ```
pub fn format_advance_reason(seconds: u32, custom_reason: Option<String>) -> String {
    if let Some(reason) = custom_reason {
        return reason;
    }

    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;

    if minutes == 0 {
        format!(
            "Time advanced by {} hour{}",
            hours,
            if hours == 1 { "" } else { "s" }
        )
    } else if hours == 0 {
        format!(
            "Time advanced by {} minute{}",
            minutes,
            if minutes == 1 { "" } else { "s" }
        )
    } else {
        format!(
            "Time advanced by {} hour{} {} minute{}",
            hours,
            if hours == 1 { "" } else { "s" },
            minutes,
            if minutes == 1 { "" } else { "s" }
        )
    }
}

/// Validates time suggestion seconds are within acceptable range
///
/// Returns true if seconds are valid (0 to 86400), false otherwise.
///
/// # Examples
/// ```
/// use wrldbldr_player::ui::time_helpers::validate_seconds_range;
///
/// assert!(validate_seconds_range(0));
/// assert!(validate_seconds_range(3600));
/// assert!(validate_seconds_range(86400));
/// assert!(!validate_seconds_range(86401));
/// assert!(!validate_seconds_range(u32::MAX));
/// ```
pub fn validate_seconds_range(seconds: u32) -> bool {
    seconds <= 86400 // Max 24 hours
}

#[cfg(test)]
mod tests {
    use super::*;

    mod hours_to_seconds_tests {
        use super::*;

        #[test]
        fn zero_hours_returns_zero_seconds() {
            assert_eq!(hours_to_seconds(0), 0);
        }

        #[test]
        fn one_hour_returns_3600_seconds() {
            assert_eq!(hours_to_seconds(1), 3600);
        }

        #[test]
        fn four_hours_returns_14400_seconds() {
            assert_eq!(hours_to_seconds(4), 14400);
        }

        #[test]
        fn eight_hours_returns_28800_seconds() {
            assert_eq!(hours_to_seconds(8), 28800);
        }

        #[test]
        fn twelve_hours_returns_43200_seconds() {
            assert_eq!(hours_to_seconds(12), 43200);
        }

        #[test]
        fn twenty_four_hours_returns_86400_seconds() {
            assert_eq!(hours_to_seconds(24), 86400);
        }
    }

    mod determine_time_decision_tests {
        use super::*;

        #[test]
        fn matching_seconds_returns_approve() {
            assert_eq!(determine_time_decision(600, 600), "approve");
            assert_eq!(determine_time_decision(0, 0), "approve");
            assert_eq!(determine_time_decision(86400, 86400), "approve");
        }

        #[test]
        fn different_seconds_returns_modify() {
            assert_eq!(determine_time_decision(600, 900), "modify");
            assert_eq!(determine_time_decision(600, 300), "modify");
            assert_eq!(determine_time_decision(3600, 1800), "modify");
        }

        #[test]
        fn modify_upwards_returns_modify() {
            assert_eq!(determine_time_decision(600, 601), "modify");
        }

        #[test]
        fn modify_downwards_returns_modify() {
            assert_eq!(determine_time_decision(600, 599), "modify");
        }
    }

    mod build_time_suggestion_decision_tests {
        use super::*;

        #[test]
        fn approve_creates_approve_decision() {
            let decision = build_time_suggestion_decision("approve", None);
            assert!(matches!(decision, TimeSuggestionDecision::Approve));
        }

        #[test]
        fn modify_creates_modify_decision_with_seconds() {
            let decision = build_time_suggestion_decision("modify", Some(900));
            match decision {
                TimeSuggestionDecision::Modify { seconds } => {
                    assert_eq!(seconds, 900);
                }
                _ => panic!("Expected Modify decision"),
            }
        }

        #[test]
        fn modify_without_seconds_defaults_to_zero() {
            let decision = build_time_suggestion_decision("modify", None);
            match decision {
                TimeSuggestionDecision::Modify { seconds } => {
                    assert_eq!(seconds, 0);
                }
                _ => panic!("Expected Modify decision"),
            }
        }

        #[test]
        fn skip_creates_skip_decision() {
            let decision = build_time_suggestion_decision("skip", None);
            assert!(matches!(decision, TimeSuggestionDecision::Skip));
        }

        #[test]
        fn unknown_decision_defaults_to_skip() {
            let decision = build_time_suggestion_decision("invalid", None);
            assert!(matches!(decision, TimeSuggestionDecision::Skip));
        }

        #[test]
        fn modify_ignores_seconds_for_other_types() {
            let decision = build_time_suggestion_decision("approve", Some(999));
            assert!(matches!(decision, TimeSuggestionDecision::Approve));
        }
    }

    mod format_advance_reason_tests {
        use super::*;

        #[test]
        fn one_hour_displays_correctly() {
            let reason = format_advance_reason(3600, None);
            assert_eq!(reason, "Time advanced by 1 hour");
        }

        #[test]
        fn multiple_hours_display_correctly() {
            let reason = format_advance_reason(7200, None);
            assert_eq!(reason, "Time advanced by 2 hours");

            let reason = format_advance_reason(14400, None);
            assert_eq!(reason, "Time advanced by 4 hours");
        }

        #[test]
        fn zero_minutes_not_shown() {
            let reason = format_advance_reason(10800, None); // 3 hours exactly
            assert_eq!(reason, "Time advanced by 3 hours");
        }

        #[test]
        fn only_minutes_shown_when_hours_zero() {
            let reason = format_advance_reason(1800, None); // 30 minutes
            assert_eq!(reason, "Time advanced by 30 minutes");

            let reason = format_advance_reason(60, None); // 1 minute
            assert_eq!(reason, "Time advanced by 1 minute");
        }

        #[test]
        fn hours_and_minutes_combined() {
            let reason = format_advance_reason(5400, None); // 1.5 hours
            assert_eq!(reason, "Time advanced by 1 hour 30 minutes");

            let reason = format_advance_reason(9000, None); // 2.5 hours
            assert_eq!(reason, "Time advanced by 2 hours 30 minutes");
        }

        #[test]
        fn one_hour_one_minute() {
            let reason = format_advance_reason(3660, None); // 1 hour 1 minute
            assert_eq!(reason, "Time advanced by 1 hour 1 minute");
        }

        #[test]
        fn custom_reason_overrides_default() {
            let reason = format_advance_reason(3600, Some("Party rested".to_string()));
            assert_eq!(reason, "Party rested");

            let reason = format_advance_reason(0, Some("Time skip".to_string()));
            assert_eq!(reason, "Time skip");
        }

        #[test]
        fn empty_custom_reason_still_used() {
            let reason = format_advance_reason(3600, Some("".to_string()));
            assert_eq!(reason, "");
        }

        #[test]
        fn zero_seconds_shows_zero_hours() {
            let reason = format_advance_reason(0, None);
            // Note: 0 is not 1, so we get "hours" plural
            assert_eq!(reason, "Time advanced by 0 hours");
        }
    }

    mod validate_seconds_range_tests {
        use super::*;

        #[test]
        fn zero_is_valid() {
            assert!(validate_seconds_range(0));
        }

        #[test]
        fn typical_values_are_valid() {
            assert!(validate_seconds_range(600)); // 10 minutes
            assert!(validate_seconds_range(3600)); // 1 hour
            assert!(validate_seconds_range(28800)); // 8 hours
        }

        #[test]
        fn max_24_hours_is_valid() {
            assert!(validate_seconds_range(86400));
        }

        #[test]
        fn beyond_24_hours_is_invalid() {
            assert!(!validate_seconds_range(86401));
            assert!(!validate_seconds_range(90000));
            assert!(!validate_seconds_range(u32::MAX));
        }
    }
}
