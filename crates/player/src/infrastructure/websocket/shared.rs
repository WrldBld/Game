//! Shared helpers for the platform-specific WebSocket clients.
//!
//! This module is intentionally runtime-agnostic (no tokio, no web-sys) so it can
//! be used by both the desktop and WASM implementations.

use wrldbldr_shared::{ResponseResult, ServerMessage};

// Reconnection constants (kept here so desktop + wasm stay in sync)
pub const INITIAL_RETRY_DELAY_MS: u64 = 1_000;
pub const MAX_RETRY_DELAY_MS: u64 = 30_000;
pub const MAX_RETRY_ATTEMPTS: u32 = 10;
pub const BACKOFF_MULTIPLIER: f64 = 2.0;

/// Parsed server message with `Response` lifted out for easier handling.
#[derive(Debug)]
pub enum ParsedServerMessage {
    Response {
        request_id: String,
        result: ResponseResult,
    },
    Other(Box<ServerMessage>),
}

pub fn parse_server_message(text: &str) -> Result<ParsedServerMessage, serde_json::Error> {
    let msg: ServerMessage = serde_json::from_str(text)?;
    Ok(match msg {
        ServerMessage::Response { request_id, result } => {
            ParsedServerMessage::Response { request_id, result }
        }
        other => ParsedServerMessage::Other(Box::new(other)),
    })
}
