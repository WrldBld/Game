//! Error to ServerMessage conversion for WebSocket handlers
//!
//! Provides the IntoServerError trait that converts any ErrorCode
//! implementing type into a ServerMessage::Error.
//!
//! This module exists to maintain proper hexagonal architecture boundaries:
//! - The `ErrorCode` trait in engine-app only provides `code()` method
//! - The adapters layer (this module) handles protocol-specific conversion
//! - Handlers import `IntoServerError` to get `.into_server_error()` method

use std::fmt::Display;
use wrldbldr_engine_app::application::use_cases::ErrorCode;
use wrldbldr_protocol::ServerMessage;

/// Extension trait for converting use case errors to ServerMessage
///
/// This trait is implemented for any type that implements both `ErrorCode`
/// (from engine-app) and `Display`. It provides the bridge between
/// domain-layer error handling and protocol-layer message formatting.
///
/// # Example
///
/// ```rust,ignore
/// use crate::infrastructure::websocket::IntoServerError;
///
/// // In handler:
/// match use_case.do_thing(ctx, input).await {
///     Ok(result) => convert_to_message(result),
///     Err(e) => e.into_server_error(), // Uses IntoServerError trait
/// }
/// ```
pub trait IntoServerError {
    /// Convert this error into a ServerMessage::Error
    fn into_server_error(&self) -> ServerMessage;
}

impl<T: ErrorCode + Display + ?Sized> IntoServerError for T {
    fn into_server_error(&self) -> ServerMessage {
        ServerMessage::Error {
            code: self.code().to_string(),
            message: self.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_engine_app::application::use_cases::MovementError;

    #[test]
    fn test_into_server_error() {
        let err = MovementError::NotConnected;
        let server_msg = err.into_server_error();

        match server_msg {
            ServerMessage::Error { code, message } => {
                assert_eq!(code, "NOT_CONNECTED");
                assert_eq!(message, "Not connected to a world");
            }
            _ => panic!("Expected Error message"),
        }
    }
}
