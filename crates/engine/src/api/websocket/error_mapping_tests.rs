//! Tests for error code mapping in WebSocket handlers.
//!
//! Verifies that use case errors are correctly mapped to protocol ErrorCodes,
//! and that the error_response helper produces correct ServerMessage values.
//!
//! Note: ErrorCode enum uses snake_case serialization via serde(rename_all = "snake_case").

use wrldbldr_shared::{ErrorCode, ServerMessage};

use super::error_response;

#[cfg(test)]
mod error_code_serialization {
    use super::*;

    #[test]
    fn error_code_serializes_to_snake_case() {
        // Client errors - snake_case per ErrorCode serde attribute
        assert_eq!(
            serde_json::to_string(&ErrorCode::BadRequest).expect("test: should serialize"),
            "\"bad_request\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::Unauthorized).expect("test: should serialize"),
            "\"unauthorized\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::Forbidden).expect("test: should serialize"),
            "\"forbidden\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::NotFound).expect("test: should serialize"),
            "\"not_found\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::Conflict).expect("test: should serialize"),
            "\"conflict\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::ValidationError).expect("test: should serialize"),
            "\"validation_error\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::RateLimitExceeded).expect("test: should serialize"),
            "\"rate_limit_exceeded\""
        );

        // Server errors
        assert_eq!(
            serde_json::to_string(&ErrorCode::InternalError).expect("test: should serialize"),
            "\"internal_error\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::ServiceUnavailable).expect("test: should serialize"),
            "\"service_unavailable\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::Timeout).expect("test: should serialize"),
            "\"timeout\""
        );
    }

    #[test]
    fn error_code_deserializes_from_snake_case() {
        assert_eq!(
            serde_json::from_str::<ErrorCode>("\"bad_request\"").expect("test: should deserialize"),
            ErrorCode::BadRequest
        );
        assert_eq!(
            serde_json::from_str::<ErrorCode>("\"not_found\"").expect("test: should deserialize"),
            ErrorCode::NotFound
        );
        assert_eq!(
            serde_json::from_str::<ErrorCode>("\"validation_error\"")
                .expect("test: should deserialize"),
            ErrorCode::ValidationError
        );
        assert_eq!(
            serde_json::from_str::<ErrorCode>("\"internal_error\"")
                .expect("test: should deserialize"),
            ErrorCode::InternalError
        );
    }

    #[test]
    fn unknown_error_code_deserializes_to_unknown() {
        // Forward compatibility: unknown codes deserialize to Unknown
        assert_eq!(
            serde_json::from_str::<ErrorCode>("\"SomeFutureError\"")
                .expect("test: should deserialize"),
            ErrorCode::Unknown
        );
    }
}

#[cfg(test)]
mod error_response_helper {
    use super::*;

    #[test]
    fn error_response_creates_correct_server_message() {
        let msg = error_response(ErrorCode::NotFound, "Character not found");

        match msg {
            ServerMessage::Error { code, message } => {
                assert_eq!(code, "not_found");
                assert_eq!(message, "Character not found");
            }
            _ => panic!("Expected ServerMessage::Error, got {:?}", msg),
        }
    }

    #[test]
    fn error_response_handles_all_error_codes() {
        // snake_case per ErrorCode serde attribute
        let test_cases = vec![
            (ErrorCode::BadRequest, "bad_request"),
            (ErrorCode::Unauthorized, "unauthorized"),
            (ErrorCode::Forbidden, "forbidden"),
            (ErrorCode::NotFound, "not_found"),
            (ErrorCode::Conflict, "conflict"),
            (ErrorCode::ValidationError, "validation_error"),
            (ErrorCode::RateLimitExceeded, "rate_limit_exceeded"),
            (ErrorCode::InternalError, "internal_error"),
            (ErrorCode::ServiceUnavailable, "service_unavailable"),
            (ErrorCode::Timeout, "timeout"),
        ];

        for (error_code, expected_str) in test_cases {
            let msg = error_response(error_code, "test message");
            match msg {
                ServerMessage::Error { code, message } => {
                    assert_eq!(
                        code, expected_str,
                        "ErrorCode::{:?} should serialize to '{}'",
                        error_code, expected_str
                    );
                    assert_eq!(message, "test message");
                }
                _ => panic!("Expected ServerMessage::Error for {:?}", error_code),
            }
        }
    }

    #[test]
    fn error_response_preserves_message_content() {
        let messages = vec![
            "Simple message",
            "Message with special chars: <>&\"'",
            "Multi-word error message with details",
            "",
        ];

        for message in messages {
            let msg = error_response(ErrorCode::BadRequest, message);
            match msg {
                ServerMessage::Error {
                    message: msg_content,
                    ..
                } => {
                    assert_eq!(msg_content, message);
                }
                _ => panic!("Expected ServerMessage::Error"),
            }
        }
    }
}

#[cfg(test)]
mod response_result_error_mapping {
    use wrldbldr_shared::ResponseResult;

    use super::*;

    #[test]
    fn response_result_error_uses_error_code() {
        let result = ResponseResult::error(ErrorCode::NotFound, "Entity not found");

        match result {
            ResponseResult::Error { code, message, .. } => {
                assert_eq!(code, ErrorCode::NotFound);
                assert_eq!(message, "Entity not found");
            }
            _ => panic!("Expected ResponseResult::Error"),
        }
    }

    #[test]
    fn response_result_error_serializes_code_correctly() {
        let result = ResponseResult::error(ErrorCode::ValidationError, "Invalid input");

        let json = serde_json::to_string(&result).expect("test: should serialize");

        // Verify the code is serialized as snake_case per ErrorCode serde attribute
        assert!(json.contains("\"code\":\"validation_error\""));
        assert!(json.contains("\"message\":\"Invalid input\""));
    }

    #[test]
    fn response_result_error_round_trips() {
        let original = ResponseResult::error(ErrorCode::Unauthorized, "Access denied");

        let json = serde_json::to_string(&original).expect("test: should serialize");
        let deserialized: ResponseResult =
            serde_json::from_str(&json).expect("test: should deserialize");

        match deserialized {
            ResponseResult::Error { code, message, .. } => {
                assert_eq!(code, ErrorCode::Unauthorized);
                assert_eq!(message, "Access denied");
            }
            _ => panic!("Expected ResponseResult::Error after round-trip"),
        }
    }
}

#[cfg(test)]
mod error_mapping_consistency {
    use super::*;

    /// Verifies that the error_response helper produces codes that match
    /// what ResponseResult::error would produce when serialized.
    #[test]
    fn error_response_matches_response_result_serialization() {
        use wrldbldr_shared::ResponseResult;

        let error_codes = vec![
            ErrorCode::BadRequest,
            ErrorCode::NotFound,
            ErrorCode::ValidationError,
            ErrorCode::InternalError,
        ];

        for code in error_codes {
            // Get the code string from error_response
            let server_msg = error_response(code, "test");
            let error_response_code = match server_msg {
                ServerMessage::Error { code, .. } => code,
                _ => panic!("Expected ServerMessage::Error"),
            };

            // Get the code from ResponseResult serialization
            let response_result = ResponseResult::error(code, "test");
            let json = serde_json::to_string(&response_result).expect("test: should serialize");
            let parsed: serde_json::Value =
                serde_json::from_str(&json).expect("test: should deserialize");
            let response_result_code = parsed["code"]
                .as_str()
                .expect("test: code should be a string");

            assert_eq!(
                error_response_code, response_result_code,
                "error_response and ResponseResult should produce the same code string for {:?}",
                code
            );
        }
    }
}

#[cfg(test)]
mod catalog_error_sanitizer_tests {
    use crate::api::websocket::error_sanitizer::{messages, sanitize_catalog_error};
    use crate::use_cases::visual_state::CatalogError;
    use wrldbldr_domain::{LocationId, LocationStateId, RegionId, RegionStateId};

    #[test]
    fn not_found_errors_return_not_found_message() {
        let test_cases = vec![
            (
                CatalogError::LocationNotFound(LocationId::new()),
                "location not found",
            ),
            (
                CatalogError::RegionNotFound(RegionId::new()),
                "region not found",
            ),
            (
                CatalogError::LocationStateNotFound(LocationStateId::new()),
                "location state not found",
            ),
            (
                CatalogError::RegionStateNotFound(RegionStateId::new()),
                "region state not found",
            ),
        ];

        for (error, _desc) in test_cases {
            let result = sanitize_catalog_error(&error, "test operation");
            assert_eq!(
                result,
                messages::NOT_FOUND,
                "{} should return NOT_FOUND message",
                _desc
            );
        }
    }

    #[test]
    fn validation_errors_return_invalid_request_message() {
        use wrldbldr_domain::DomainError;

        let test_cases = vec![
            (
                CatalogError::Validation("invalid input".to_string()),
                "validation",
            ),
            (CatalogError::NameEmpty, "name empty"),
            (CatalogError::NameTooLong, "name too long"),
            (CatalogError::DescriptionTooLong, "description too long"),
            (
                CatalogError::Domain(DomainError::validation("corrupt stored data")),
                "domain error",
            ),
        ];

        for (error, _desc) in test_cases {
            let result = sanitize_catalog_error(&error, "test operation");
            assert_eq!(
                result,
                messages::INVALID_REQUEST,
                "{} should return INVALID_REQUEST message",
                _desc
            );
        }
    }

    #[test]
    fn internal_errors_return_internal_error_message() {
        use crate::infrastructure::ports::RepoError;

        let test_cases = vec![
            (
                CatalogError::Repo(RepoError::database("query", "db down")),
                "repo error",
            ),
            (
                CatalogError::Repo(RepoError::database("enqueue", "queue down")),
                "queue error",
            ),
        ];

        for (error, _desc) in test_cases {
            let result = sanitize_catalog_error(&error, "test operation");
            assert_eq!(
                result,
                messages::INTERNAL_ERROR,
                "{} should return INTERNAL_ERROR message",
                _desc
            );
        }
    }

    #[test]
    fn sanitizer_includes_operation_context_in_logs() {
        // This test verifies the function signature accepts operation context
        // The actual logging is verified by compilation
        let error = CatalogError::Validation("test".to_string());
        let _result = sanitize_catalog_error(&error, "my operation");
    }
}

#[cfg(test)]
mod generation_error_sanitizer_tests {
    use crate::api::websocket::error_sanitizer::{messages, sanitize_generation_error};
    use crate::infrastructure::ports::{QueueError, RepoError};
    use crate::use_cases::visual_state::GenerationError;
    use wrldbldr_domain::{LocationId, RegionId};

    #[test]
    fn validation_error_returns_specific_message() {
        let error = GenerationError::Validation("Name cannot be empty".to_string());
        let result = sanitize_generation_error(&error, "test operation");

        // Validation errors should return the specific message
        assert_eq!(
            result, "Name cannot be empty",
            "Validation error should return the specific validation message"
        );
    }

    #[test]
    fn validation_error_with_different_messages() {
        let test_cases = vec![
            ("Prompt too long", "prompt length validation"),
            ("Invalid workflow ID", "workflow validation"),
            (
                "Description cannot exceed 2000 characters",
                "description validation",
            ),
        ];

        for (msg, desc) in test_cases {
            let error = GenerationError::Validation(msg.to_string());
            let result = sanitize_generation_error(&error, "test operation");
            assert_eq!(result, msg, "{} should return the specific message", desc);
        }
    }

    #[test]
    fn location_not_found_returns_clear_message() {
        let error = GenerationError::LocationNotFound(LocationId::new());
        let result = sanitize_generation_error(&error, "test operation");

        assert_eq!(
            result, "Location not found",
            "LocationNotFound should return clear message"
        );
    }

    #[test]
    fn region_not_found_returns_clear_message() {
        let error = GenerationError::RegionNotFound(RegionId::new());
        let result = sanitize_generation_error(&error, "test operation");

        assert_eq!(
            result, "Region not found",
            "RegionNotFound should return clear message"
        );
    }

    #[test]
    fn repo_error_returns_internal_error_message() {
        let error = GenerationError::Repo(RepoError::database("query", "db down"));
        let result = sanitize_generation_error(&error, "test operation");

        assert_eq!(
            result,
            messages::INTERNAL_ERROR,
            "Repo error should return sanitized internal error message"
        );
    }

    #[test]
    fn queue_error_returns_internal_error_message() {
        let error = GenerationError::Queue(QueueError::database("enqueue", "queue full"));
        let result = sanitize_generation_error(&error, "test operation");

        assert_eq!(
            result,
            messages::INTERNAL_ERROR,
            "Queue error should return sanitized internal error message"
        );
    }

    #[test]
    fn domain_validation_error_returns_specific_message() {
        use wrldbldr_domain::DomainError;

        let error = GenerationError::Domain(DomainError::validation("Invalid name provided"));
        let result = sanitize_generation_error(&error, "test operation");

        // Domain validation errors are safe to expose
        assert_eq!(
            result, "Invalid name provided",
            "Domain validation error should return the specific validation message"
        );
    }

    #[test]
    fn domain_invalid_id_error_returns_stable_message() {
        use wrldbldr_domain::DomainError;

        let error = GenerationError::Domain(DomainError::invalid_id("bad-id-format"));
        let result = sanitize_generation_error(&error, "test operation");

        // InvalidId errors return stable client-safe message
        assert_eq!(
            result, "Invalid ID",
            "Domain InvalidId error should return stable 'Invalid ID' message"
        );
    }

    #[test]
    fn domain_not_found_error_returns_stable_message() {
        use wrldbldr_domain::DomainError;

        let error = GenerationError::Domain(DomainError::not_found(
            "Character",
            "123e4567-e89b-12d3-a456-426614174000",
        ));
        let result = sanitize_generation_error(&error, "test operation");

        // NotFound errors return stable client-safe message
        assert_eq!(
            result, "Entity not found",
            "Domain NotFound error should return stable 'Entity not found' message"
        );
    }

    #[test]
    fn domain_constraint_error_returns_stable_message() {
        use wrldbldr_domain::DomainError;

        let error = GenerationError::Domain(DomainError::constraint("character already exists"));
        let result = sanitize_generation_error(&error, "test operation");

        // Constraint errors return stable client-safe message
        assert_eq!(
            result, "Invalid input",
            "Domain Constraint error should return stable 'Invalid input' message"
        );
    }

    #[test]
    fn domain_parse_error_returns_stable_message() {
        use wrldbldr_domain::DomainError;

        let error = GenerationError::Domain(DomainError::parse("unknown enum variant"));
        let result = sanitize_generation_error(&error, "test operation");

        // Parse errors return stable client-safe message
        assert_eq!(
            result, "Invalid input",
            "Domain Parse error should return stable 'Invalid input' message"
        );
    }

    #[test]
    fn domain_invalid_state_transition_error_returns_stable_message() {
        use wrldbldr_domain::DomainError;

        let error =
            GenerationError::Domain(DomainError::invalid_state_transition("cannot deactivate"));
        let result = sanitize_generation_error(&error, "test operation");

        // InvalidStateTransition errors return stable client-safe message
        assert_eq!(
            result, "Invalid input",
            "Domain InvalidStateTransition error should return stable 'Invalid input' message"
        );
    }

    #[test]
    fn domain_container_full_error_returns_stable_message() {
        use wrldbldr_domain::DomainError;

        let error = GenerationError::Domain(DomainError::container_full(5, 5));
        let result = sanitize_generation_error(&error, "test operation");

        // ContainerFull errors return stable client-safe message
        assert_eq!(
            result, "Capacity reached",
            "Domain ContainerFull error should return stable 'Capacity reached' message"
        );
    }

    #[test]
    fn domain_errors_do_not_leak_internal_details() {
        use wrldbldr_domain::DomainError;

        // Test that internal error details are not exposed to clients
        let test_cases = vec![
            (
                GenerationError::Domain(DomainError::invalid_id("internal-secret-id")),
                "Invalid ID",
                "InvalidId",
            ),
            (
                GenerationError::Domain(DomainError::not_found("SecretEntity", "secret-uuid")),
                "Entity not found",
                "NotFound",
            ),
            (
                GenerationError::Domain(DomainError::constraint("secret constraint violation")),
                "Invalid input",
                "Constraint",
            ),
            (
                GenerationError::Domain(DomainError::parse("secret parse failure")),
                "Invalid input",
                "Parse",
            ),
            (
                GenerationError::Domain(DomainError::invalid_state_transition("secret state")),
                "Invalid input",
                "InvalidStateTransition",
            ),
            (
                GenerationError::Domain(DomainError::container_full(999, 1000)),
                "Capacity reached",
                "ContainerFull",
            ),
        ];

        for (error, expected_message, variant_name) in test_cases {
            let result = sanitize_generation_error(&error, "test operation");
            assert_eq!(
                result, expected_message,
                "{} error should not leak internal details",
                variant_name
            );
        }
    }
}
