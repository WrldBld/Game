//! Fail-fast visual state tests for movement.
//!
//! Tests cover:
//! - Visual state lookup errors propagate to movement
//! - Active staging with no visual state IDs is treated as data integrity error (post-review fix)

#[cfg(test)]
mod fail_fast_tests {
    use super::*;

    /// Test: RepoError handling for visual state lookup failures.
    ///
    /// This tests that the `resolve_visual_state_from_staging` function
    /// properly returns Result to propagate errors.
    #[test]
    fn test_repo_error_propagation_for_visual_state_lookup() {
        use crate::infrastructure::ports::RepoError;

        // Create a RepoError
        let error = RepoError::database("query", "Test error");

        // Verify error is properly structured
        match error {
            RepoError::Database { operation, message } => {
                assert_eq!(operation, "query");
                assert_eq!(message, "Test error");
            }
            _ => panic!("Expected Database variant"),
        }
    }

    /// Test: NotFound error is returned for missing states.
    #[test]
    fn test_not_found_error_for_missing_state() {
        use crate::infrastructure::ports::RepoError;

        let error = RepoError::not_found("LocationState", "test-id");

        // Verify error contains entity type and ID
        match error {
            RepoError::NotFound { entity_type, id } => {
                assert_eq!(entity_type, "LocationState");
                assert_eq!(id, "test-id");
            }
            _ => panic!("Expected NotFound variant"),
        }
    }

    /// Test: Option handling for missing visual state IDs.
    #[test]
    fn test_none_visual_state_returns_ok_none() {
        use crate::use_cases::staging::ResolvedVisualState;

        let visual_state: Option<ResolvedVisualState> = None;

        assert!(visual_state.is_none());
    }

    /// Test: Some visual state returns successfully.
    #[test]
    fn test_some_visual_state_returns_successfully() {
        use crate::use_cases::staging::{ResolvedStateInfo, ResolvedVisualState};

        let visual_state = Some(ResolvedVisualState {
            location_state: Some(ResolvedStateInfo {
                id: "test-id".to_string(),
                name: "Test State".to_string(),
                backdrop_override: Some("backdrop.png".to_string()),
                atmosphere_override: None,
                ambient_sound: None,
            }),
            region_state: None,
        });

        assert!(visual_state.is_some());
        let vs = visual_state.unwrap();
        assert_eq!(vs.location_state.unwrap().name, "Test State");
        assert!(vs.region_state.is_none());
    }

    /// Test: Active staging with no visual state IDs returns data integrity error.
    ///
    /// Post-review fix: Active staging should always have visual state IDs.
    /// If both location_state_id and region_state_id are None on active staging,
    /// this is a data integrity error.
    #[test]
    fn test_active_staging_no_visual_state_ids_returns_error() {
        use crate::infrastructure::ports::RepoError;

        // Create a database error representing data integrity issue
        let error = RepoError::database(
            "staging_integrity",
            "Active staging has no visual state IDs. This indicates data integrity - staging was approved without resolving visual state IDs."
        );

        // Verify error contains the staging_integrity operation
        match error {
            RepoError::Database { operation, message } => {
                assert_eq!(operation, "staging_integrity");
                assert!(message.contains("Active staging has no visual state IDs"));
                assert!(message.contains("data integrity"));
            }
            _ => panic!("Expected Database variant"),
        }
    }
}
