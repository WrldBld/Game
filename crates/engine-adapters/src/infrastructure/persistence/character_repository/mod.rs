//! Character repository implementation for Neo4j
//!
//! # Graph-First Design (Phase 0.C)
//!
//! This repository uses Neo4j edges for all relationships:
//! - Wants: `(Character)-[:HAS_WANT]->(Want)` + `(Want)-[:TARGETS]->(target)`
//! - Inventory: `(Character)-[:POSSESSES]->(Item)`
//! - Location: `HOME_LOCATION`, `WORKS_AT`, `FREQUENTS`, `AVOIDS`
//! - Actantial: `VIEWS_AS_HELPER`, `VIEWS_AS_OPPONENT`, etc.
//!
//! # Module Structure
//!
//! The repository is split into focused modules following Interface Segregation:
//! - `stored_types`: Persistence serde models (StatBlockStored, ArchetypeChangeStored)
//! - `common`: Shared helpers like row_to_character
//! - `crud_impl`: CharacterCrudPort implementation
//! - `want_impl`: CharacterWantPort implementation
//! - `actantial_impl`: CharacterActantialPort implementation
//! - `inventory_impl`: CharacterInventoryPort implementation
//! - `location_impl`: CharacterLocationPort implementation
//! - `disposition_impl`: CharacterDispositionPort implementation

use std::sync::Arc;

use anyhow::Result;
use wrldbldr_domain::entities::Character;
use wrldbldr_domain::value_objects::RegionRelationshipType;
use wrldbldr_domain::{CharacterId, RegionId, WorldId};
use wrldbldr_engine_ports::outbound::ClockPort;

use super::connection::Neo4jConnection;

mod actantial_impl;
mod common;
mod crud_impl;
mod disposition_impl;
mod inventory_impl;
mod location_impl;
pub(crate) mod stored_types;
mod want_impl;

/// Repository for Character operations
pub struct Neo4jCharacterRepository {
    connection: Neo4jConnection,
    clock: Arc<dyn ClockPort>,
}

impl Neo4jCharacterRepository {
    pub fn new(connection: Neo4jConnection, clock: Arc<dyn ClockPort>) -> Self {
        Self { connection, clock }
    }

    // =========================================================================
    // Public API (for direct calls, e.g., from world_snapshot.rs)
    // These delegate to the _impl methods in the submodules
    // =========================================================================

    /// List all characters in a world
    pub async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Character>> {
        self.list_by_world_impl(world_id).await
    }

    /// Get a character by ID
    pub async fn get(&self, id: CharacterId) -> Result<Option<Character>> {
        self.get_impl(id).await
    }

    /// Get all NPCs with any relationship to a region
    pub async fn get_npcs_related_to_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<(Character, RegionRelationshipType)>> {
        self.get_npcs_related_to_region_impl(region_id).await
    }
}
