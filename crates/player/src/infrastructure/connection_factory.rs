//! Connection factory for creating game connections
//!
//! This module provides a factory for creating game connections to the Engine.
//! It encapsulates the infrastructure details of connection creation, keeping
//! the presentation layer free from infrastructure dependencies.

use std::sync::Arc;

use crate::ports::outbound::GameConnectionPort;

use super::websocket::{EngineClient, EngineGameConnection};

/// Factory for creating game connections
pub struct ConnectionFactory;

impl ConnectionFactory {
    /// Create a new game connection to the specified server URL
    ///
    /// # Arguments
    /// * `server_url` - The WebSocket URL of the Engine server
    ///
    /// # Returns
    /// An Arc-wrapped connection that implements GameConnectionPort
    pub fn create_game_connection(server_url: &str) -> Arc<dyn GameConnectionPort> {
        let client = EngineClient::new(server_url);
        Arc::new(EngineGameConnection::new(client))
    }
}
