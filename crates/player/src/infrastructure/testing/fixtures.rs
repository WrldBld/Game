//! Simple test fixtures used across unit tests.

use crate::application::ports::outbound::ApiError;
use crate::domain::entities::PlayerAction;

pub fn api_request_failed(msg: &str) -> ApiError {
    ApiError::RequestFailed(msg.to_string())
}

pub fn action_custom(text: &str) -> PlayerAction {
    PlayerAction::custom(text)
}

