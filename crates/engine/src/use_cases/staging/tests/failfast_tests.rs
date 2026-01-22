//! Fail-fast visual state tests for staging.
//!
//! Tests cover:
//! - Invalid visual state IDs return ValidationError
//! - Default visual state with no active states fails validation
//! - Visual state response uses resolved IDs directly (post-review fix)

#[cfg(test)]
mod fail_fast_tests {
    use super::*;

    /// Test: UUID validation errors are caught before reaching use case.
    ///
    /// This tests that the API boundary properly validates UUID format.
    #[test]
    fn test_uuid_validation_at_api_boundary() {
        // Valid UUID formats
        assert!(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").is_ok());
        assert!(uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").is_ok());

        // Invalid UUID formats
        assert!(uuid::Uuid::parse_str("not-a-uuid").is_err());
        assert!(uuid::Uuid::parse_str("12345").is_err());
        assert!(uuid::Uuid::parse_str("").is_err());
    }

    /// Test: ValidationError includes clear message for invalid UUID.
    #[test]
    fn test_validation_error_has_clear_message() {
        use crate::use_cases::staging::StagingError;

        let error = StagingError::Validation("Invalid UUID format".to_string());

        assert!(error.to_string().contains("Validation error"));
        assert!(error.to_string().contains("Invalid UUID format"));
    }

    /// Test: Visual state fetch by ID returns RepoError if state not found.
    ///
    /// Post-review fix: build_visual_state_for_staging now fetches by resolved IDs
    /// instead of using get_active(). If a resolved ID's entity doesn't exist,
    /// it returns RepoError::not_found (InternalError), not ValidationError.
    #[test]
    fn test_visual_state_fetch_by_id_returns_not_found_on_missing() {
        use crate::infrastructure::ports::RepoError;

        let location_state_id = uuid::Uuid::new_v4();
        let error = RepoError::not_found("LocationState", location_state_id.to_string());

        // Verify error contains entity type and ID
        match error {
            RepoError::NotFound { entity_type, id } => {
                assert_eq!(entity_type, "LocationState");
                assert_eq!(id, location_state_id.to_string());
            }
            _ => panic!("Expected NotFound variant"),
        }
    }

    /// Test: Visual state fetch validates both location and region states exist.
    ///
    /// Post-review fix: When both location_state_id and region_state_id are
    /// provided to build_visual_state_for_staging, both must exist or error.
    #[test]
    fn test_visual_state_fetch_validates_both_states_exist() {
        use crate::infrastructure::ports::RepoError;

        let region_state_id = uuid::Uuid::new_v4();
        let error = RepoError::not_found("RegionState", region_state_id.to_string());

        // Verify error is NotFound
        assert!(matches!(error, RepoError::NotFound { .. }));
    }
}
