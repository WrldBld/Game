//! Infrastructure layer - External adapters and implementations
//!
//! This layer contains:
//! - Persistence: Neo4j adapter for data storage
//! - HTTP: REST API routes
//! - WebSocket: Real-time communication with Player clients
//! - Ollama: LLM integration for AI-powered responses
//! - ComfyUI: Asset generation integration
//! - Config: Application configuration
//! - State: Shared application state
//! - State: Shared application state (world connection manager replaces sessions)
//! - Event Bus: Event publishing and subscription infrastructure
//! - Repositories: Additional persistence implementations
//! - Context Budget: Token budget enforcement for LLM prompts
//! - State Broadcast: Utilities for broadcasting state changes to WebSocket clients

pub mod comfyui;
pub mod config;
pub mod context_budget;
pub mod event_bus;
pub mod export;
pub mod http;
pub mod ollama;
pub mod persistence;
pub mod queue_workers;
pub mod queues;
pub mod repositories;
pub mod state;
pub mod suggestion_enqueue_adapter;
pub mod state_broadcast;
pub mod websocket;
pub mod websocket_event_subscriber;
pub mod websocket_helpers;
pub mod world_connection_manager;
pub mod world_connection_port_adapter;
pub mod world_state_manager;

// Re-export world state manager types
pub use world_state_manager::{
    WorldStateManager, ConversationEntry, Speaker,
    PendingApprovalItem, ApprovalType, 
    WorldPendingStagingApproval, WaitingPc,
};

// Re-export world connection manager types
pub use world_connection_manager::{
    BroadcastError, DmInfo, WorldConnectionManager,
};

// Re-export world connection port adapter
pub use world_connection_port_adapter::WorldConnectionPortAdapter;
