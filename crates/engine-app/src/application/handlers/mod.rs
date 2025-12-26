//! Request Handlers - WebSocket request/response pattern handlers
//!
//! This module contains the `AppRequestHandler` implementation that routes
//! WebSocket requests to the appropriate services.

mod request_handler;

pub use request_handler::AppRequestHandler;
