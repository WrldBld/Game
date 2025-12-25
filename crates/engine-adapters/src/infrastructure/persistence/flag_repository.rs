//! Flag repository implementation for Neo4j
//!
//! ## Graph Design
//!
//! Flags are stored as properties on edges:
//! - World-scoped: `(World)-[:HAS_FLAG {name: "flag_name", value: true}]->(World)` (self-loop)
//! - PC-scoped: `(PlayerCharacter)-[:HAS_FLAG {name: "flag_name", value: true}]->(PlayerCharacter)` (self-loop)
//!
//! Using self-loops allows storing multiple flags as separate edges with different names.

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;

use super::connection::Neo4jConnection;
use wrldbldr_engine_ports::outbound::FlagRepositoryPort;
use wrldbldr_domain::{PlayerCharacterId, WorldId};

/// Repository for Game Flag operations
pub struct Neo4jFlagRepository {
    connection: Neo4jConnection,
}

impl Neo4jFlagRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl FlagRepositoryPort for Neo4jFlagRepository {
    // -------------------------------------------------------------------------
    // World-scoped Flags
    // -------------------------------------------------------------------------

    async fn set_world_flag(&self, world_id: WorldId, flag_name: &str, value: bool) -> Result<()> {
        let q = query(
            "MATCH (w:World {id: $world_id})
            MERGE (w)-[f:HAS_FLAG {name: $flag_name}]->(w)
            SET f.value = $value, f.updated_at = datetime()",
        )
        .param("world_id", world_id.to_string())
        .param("flag_name", flag_name)
        .param("value", value);

        self.connection.graph().run(q).await?;
        tracing::debug!("Set world flag '{}' = {} for world {}", flag_name, value, world_id);
        Ok(())
    }

    async fn get_world_flag(&self, world_id: WorldId, flag_name: &str) -> Result<bool> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[f:HAS_FLAG {name: $flag_name}]->(w)
            RETURN f.value as value",
        )
        .param("world_id", world_id.to_string())
        .param("flag_name", flag_name);

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let value: bool = row.get("value").unwrap_or(false);
            return Ok(value);
        }
        Ok(false) // Flag not set = false
    }

    async fn get_world_flags(&self, world_id: WorldId) -> Result<Vec<(String, bool)>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[f:HAS_FLAG]->(w)
            RETURN f.name as name, f.value as value",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut flags = Vec::new();

        while let Some(row) = result.next().await? {
            let name: String = row.get("name")?;
            let value: bool = row.get("value").unwrap_or(false);
            flags.push((name, value));
        }

        Ok(flags)
    }

    // -------------------------------------------------------------------------
    // PC-scoped Flags
    // -------------------------------------------------------------------------

    async fn set_pc_flag(&self, pc_id: PlayerCharacterId, flag_name: &str, value: bool) -> Result<()> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})
            MERGE (pc)-[f:HAS_FLAG {name: $flag_name}]->(pc)
            SET f.value = $value, f.updated_at = datetime()",
        )
        .param("pc_id", pc_id.to_string())
        .param("flag_name", flag_name)
        .param("value", value);

        self.connection.graph().run(q).await?;
        tracing::debug!("Set PC flag '{}' = {} for PC {}", flag_name, value, pc_id);
        Ok(())
    }

    async fn get_pc_flag(&self, pc_id: PlayerCharacterId, flag_name: &str) -> Result<bool> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[f:HAS_FLAG {name: $flag_name}]->(pc)
            RETURN f.value as value",
        )
        .param("pc_id", pc_id.to_string())
        .param("flag_name", flag_name);

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let value: bool = row.get("value").unwrap_or(false);
            return Ok(value);
        }
        Ok(false) // Flag not set = false
    }

    async fn get_pc_flags(&self, pc_id: PlayerCharacterId) -> Result<Vec<(String, bool)>> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[f:HAS_FLAG]->(pc)
            RETURN f.name as name, f.value as value",
        )
        .param("pc_id", pc_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut flags = Vec::new();

        while let Some(row) = result.next().await? {
            let name: String = row.get("name")?;
            let value: bool = row.get("value").unwrap_or(false);
            flags.push((name, value));
        }

        Ok(flags)
    }
}
