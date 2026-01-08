//! Simple test fixtures used across unit tests.

use crate::ports::outbound::ApiError;

pub fn api_request_failed(msg: &str) -> ApiError {
    ApiError::RequestFailed(msg.to_string())
}
