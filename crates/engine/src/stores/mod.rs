//! In-memory state storage modules.
//!
//! Stores manage runtime state that doesn't belong in the database:
//! - `SessionStore` - WebSocket connection tracking
//! - `PendingStagingStore` - Approval workflow state
//! - `DirectorialStore` - DM context state
//! - `TimeSuggestionStore` - Time suggestion cache

pub mod directorial;
pub mod pending_staging;
pub mod session;
pub mod time_suggestion;

// Re-export store types
pub use directorial::DirectorialContextStore;
pub use pending_staging::PendingStagingStore;
pub use session::SessionStore;
pub use time_suggestion::TimeSuggestionStore;
