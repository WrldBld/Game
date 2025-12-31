//! StoryEvent repository implementation for Neo4j
//!
//! This module provides the persistence layer for StoryEvent entities,
//! split into focused sub-modules following the Interface Segregation Principle.

mod common;
mod crud_impl;
mod dialogue_impl;
mod edge_impl;
mod query_impl;
mod stored_types;

use super::connection::Neo4jConnection;

/// Repository for StoryEvent operations
///
/// Implements the following port traits:
/// - `StoryEventCrudPort` - Core CRUD + state management
/// - `StoryEventQueryPort` - Query operations
/// - `StoryEventEdgePort` - Edge relationship management
/// - `StoryEventDialoguePort` - Dialogue-specific operations
///
/// The super-trait `StoryEventRepositoryPort` is automatically satisfied via
/// blanket impl when all 4 traits are implemented.
pub struct Neo4jStoryEventRepository {
    pub(crate) connection: Neo4jConnection,
}

impl Neo4jStoryEventRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }
}
