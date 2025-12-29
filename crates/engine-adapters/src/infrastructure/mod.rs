//! Infrastructure layer - External adapters and implementations
//!
//! This layer contains:
//! - Persistence: Neo4j adapter for data storage
//! - HTTP: REST API routes
//! - WebSocket: Real-time communication with Player clients
//! - Ollama: LLM integration for AI-powered responses
//! - ComfyUI: Asset generation integration
//! - Config: Application configuration
//! - State: Shared application state (legacy - being replaced by adapter_state)
//! - Adapter State: New hexagonal-compliant state (AppState + infrastructure)
//! - Event Bus: Event publishing and subscription infrastructure
//! - Repositories: Additional persistence implementations
//! - Context Budget: Token budget enforcement for LLM prompts
//! - State Broadcast: Utilities for broadcasting state changes to WebSocket clients
//! - Clock: System time abstraction for testability

pub mod adapter_state;
pub mod clock;
pub mod comfyui;
pub mod config;
pub mod context_budget;
pub mod event_bus;
pub mod export;
pub mod file_storage;
pub mod http;
pub mod ollama;
pub mod persistence;
pub mod ports;
pub mod queues;
pub mod repositories;
pub mod settings_loader;
pub mod state;
pub mod state_broadcast;
pub mod suggestion_enqueue_adapter;
pub mod websocket;
pub mod websocket_event_subscriber;
pub mod world_connection_manager;
pub mod world_state_manager;

// Re-export clock adapter
pub use clock::SystemClock;

// Re-export file storage adapter
pub use file_storage::TokioFileStorageAdapter;

// Re-export world state manager types
pub use world_state_manager::{WaitingPc, WorldPendingStagingApproval, WorldStateManager};
// Re-export domain types used by world state
pub use wrldbldr_domain::value_objects::{
    ApprovalType, ConversationEntry, DirectorialNotes, PendingApprovalItem, Speaker,
};
// Re-export the port trait so callers can use trait methods on Arc<WorldStateManager>
pub use wrldbldr_engine_ports::outbound::WorldStatePort;

// Re-export world connection manager types
pub use world_connection_manager::{BroadcastError, DmInfo, WorldConnectionManager};

// Re-export settings loader
pub use settings_loader::load_settings_from_env;

// Re-export adapter state for hexagonal architecture
pub use adapter_state::AdapterState;
