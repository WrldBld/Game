//! NarrativeEvent repository implementation for Neo4j
//!
//! This module provides the persistence layer for NarrativeEvent entities,
//! split into focused sub-modules following the Interface Segregation Principle.

mod common;
mod crud_impl;
mod npc_impl;
mod query_impl;
mod stored_types;
mod tie_impl;

use std::sync::Arc;

use super::connection::Neo4jConnection;
use wrldbldr_engine_ports::outbound::ClockPort;

/// Repository for NarrativeEvent operations
///
/// Implements the following port traits:
/// - `NarrativeEventCrudPort` - Core CRUD + state management
/// - `NarrativeEventTiePort` - Scene/Location/Act relationships
/// - `NarrativeEventNpcPort` - Featured NPC management
/// - `NarrativeEventQueryPort` - Query by relationships
///
/// The super-trait `NarrativeEventRepositoryPort` is automatically satisfied via
/// blanket impl when all 4 traits are implemented.
pub struct Neo4jNarrativeEventRepository {
    pub(crate) connection: Neo4jConnection,
    pub(crate) clock: Arc<dyn ClockPort>,
}

impl Neo4jNarrativeEventRepository {
    pub fn new(connection: Neo4jConnection, clock: Arc<dyn ClockPort>) -> Self {
        Self { connection, clock }
    }
}
