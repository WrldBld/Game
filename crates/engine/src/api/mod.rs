//! API layer - HTTP and WebSocket entry points.

pub mod connections;
pub mod http;
pub mod websocket;

pub use connections::{ConnectionManager, SharedConnectionManager};
