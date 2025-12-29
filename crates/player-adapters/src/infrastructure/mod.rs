//! Infrastructure layer - External adapters

pub mod api;
pub mod connection_factory;
pub mod http_client;
pub mod message_translator;
pub mod platform;
pub mod session_type_converters;
pub mod storage;
pub mod websocket;

// Re-export ConnectionFactory for convenience
pub use connection_factory::ConnectionFactory;

// Re-export converters for use by adapter implementations
pub use session_type_converters::{
    adhoc_outcomes_to_proto, approval_decision_to_proto, approved_npc_info_to_proto,
    challenge_outcome_decision_to_proto, dice_input_to_proto, directorial_context_to_proto,
    participant_role_to_proto,
};

// Re-export message translator for ServerMessage → PlayerEvent conversion
pub use message_translator::translate as translate_server_message;

// Test-only infrastructure fakes (ports/adapters).
// Available for integration testing from other crates as well
pub mod testing;
