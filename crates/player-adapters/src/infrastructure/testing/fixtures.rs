//! Simple test fixtures used across unit tests.

use wrldbldr_player_ports::outbound::ApiError;

pub fn api_request_failed(msg: &str) -> ApiError {
    ApiError::RequestFailed(msg.to_string())
}

