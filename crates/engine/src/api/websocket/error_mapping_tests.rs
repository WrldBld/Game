//! Tests for error code mapping in WebSocket handlers.
//!
//! Verifies that use case errors are correctly mapped to protocol ErrorCodes,
//! and that the error_response helper produces correct ServerMessage values.
//!
//! Note: Enum variants serialize as PascalCase (Rust default per ADR-010).

use wrldbldr_shared::{ErrorCode, ServerMessage};

use super::error_response;

#[cfg(test)]
mod error_code_serialization {
    use super::*;

    #[test]
    fn error_code_serializes_to_pascal_case() {
        // Client errors - PascalCase per ADR-010 (Rust default)
        assert_eq!(
            serde_json::to_string(&ErrorCode::BadRequest).unwrap(),
            "\"BadRequest\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::Unauthorized).unwrap(),
            "\"Unauthorized\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::Forbidden).unwrap(),
            "\"Forbidden\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::NotFound).unwrap(),
            "\"NotFound\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::Conflict).unwrap(),
            "\"Conflict\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::ValidationError).unwrap(),
            "\"ValidationError\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::RateLimitExceeded).unwrap(),
            "\"RateLimitExceeded\""
        );

        // Server errors
        assert_eq!(
            serde_json::to_string(&ErrorCode::InternalError).unwrap(),
            "\"InternalError\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::ServiceUnavailable).unwrap(),
            "\"ServiceUnavailable\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::Timeout).unwrap(),
            "\"Timeout\""
        );
    }

    #[test]
    fn error_code_deserializes_from_pascal_case() {
        assert_eq!(
            serde_json::from_str::<ErrorCode>("\"BadRequest\"").unwrap(),
            ErrorCode::BadRequest
        );
        assert_eq!(
            serde_json::from_str::<ErrorCode>("\"NotFound\"").unwrap(),
            ErrorCode::NotFound
        );
        assert_eq!(
            serde_json::from_str::<ErrorCode>("\"ValidationError\"").unwrap(),
            ErrorCode::ValidationError
        );
        assert_eq!(
            serde_json::from_str::<ErrorCode>("\"InternalError\"").unwrap(),
            ErrorCode::InternalError
        );
    }

    #[test]
    fn unknown_error_code_deserializes_to_unknown() {
        // Forward compatibility: unknown codes deserialize to Unknown
        assert_eq!(
            serde_json::from_str::<ErrorCode>("\"SomeFutureError\"").unwrap(),
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
                assert_eq!(code, "NotFound");
                assert_eq!(message, "Character not found");
            }
            _ => panic!("Expected ServerMessage::Error, got {:?}", msg),
        }
    }

    #[test]
    fn error_response_handles_all_error_codes() {
        // PascalCase per ADR-010 (Rust default for enum variants)
        let test_cases = vec![
            (ErrorCode::BadRequest, "BadRequest"),
            (ErrorCode::Unauthorized, "Unauthorized"),
            (ErrorCode::Forbidden, "Forbidden"),
            (ErrorCode::NotFound, "NotFound"),
            (ErrorCode::Conflict, "Conflict"),
            (ErrorCode::ValidationError, "ValidationError"),
            (ErrorCode::RateLimitExceeded, "RateLimitExceeded"),
            (ErrorCode::InternalError, "InternalError"),
            (ErrorCode::ServiceUnavailable, "ServiceUnavailable"),
            (ErrorCode::Timeout, "Timeout"),
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

        let json = serde_json::to_string(&result).unwrap();

        // Verify the code is serialized as PascalCase (Rust default per ADR-010)
        assert!(json.contains("\"code\":\"ValidationError\""));
        assert!(json.contains("\"message\":\"Invalid input\""));
    }

    #[test]
    fn response_result_error_round_trips() {
        let original = ResponseResult::error(ErrorCode::Unauthorized, "Access denied");

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ResponseResult = serde_json::from_str(&json).unwrap();

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
            let json = serde_json::to_string(&response_result).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            let response_result_code = parsed["code"].as_str().unwrap();

            assert_eq!(
                error_response_code, response_result_code,
                "error_response and ResponseResult should produce the same code string for {:?}",
                code
            );
        }
    }
}
