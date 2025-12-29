//! HTTP middleware for the engine API
//!
//! This module provides middleware components for HTTP routes:
//! - Authentication (currently X-User-Id header, future: JWT)
//!
//! Note: Most operations use WebSocket protocol where authentication
//! is handled during JoinWorld. HTTP middleware is for REST endpoints
//! that require authentication (file uploads, exports, etc.).

mod auth;

pub use auth::{AuthenticatedUser, Auth};
