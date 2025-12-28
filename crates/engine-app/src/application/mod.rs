//! Application layer - Use cases and orchestration
//!
//! This layer contains:
//! - Use Cases: Complex workflows with side-effects (movement, staging, etc.)
//! - Services: Domain service implementations
//! - DTOs: Data transfer objects for API boundaries
//! - Handlers: WebSocket request/response handlers

pub mod dto;
pub mod handlers;
pub mod services;
pub mod use_cases;
