//! Neo4j implementation of the Flag repository.
//!
//! Flags are stored as relationships from World or PlayerCharacter nodes:
//! - World flags: (World)-[:HAS_FLAG {name: "flag_name"}]->()
//! - PC flags: (PlayerCharacter)-[:HAS_FLAG {name: "flag_name"}]->()

use async_trait::async_trait;
use neo4rs::{query, Graph};
use std::sync::Arc;

use wrldbldr_domain::{PlayerCharacterId, WorldId};

use crate::infrastructure::ports::{FlagRepo, RepoError};

pub struct Neo4jFlagRepo {
    graph: Arc<Graph>,
}

impl Neo4jFlagRepo {
    pub fn new(graph: Arc<Graph>) -> Self {
        Self { graph }
    }
}

#[async_trait]
impl FlagRepo for Neo4jFlagRepo {
    async fn get_world_flags(&self, world_id: WorldId) -> Result<Vec<String>, RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[f:HAS_FLAG]->()
             RETURN f.name AS name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let mut flags = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            if let Ok(name) = row.get::<String>("name") {
                flags.push(name);
            }
        }

        Ok(flags)
    }

    async fn get_pc_flags(&self, pc_id: PlayerCharacterId) -> Result<Vec<String>, RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[f:HAS_FLAG]->()
             RETURN f.name AS name",
        )
        .param("pc_id", pc_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let mut flags = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            if let Ok(name) = row.get::<String>("name") {
                flags.push(name);
            }
        }

        Ok(flags)
    }

    async fn set_world_flag(&self, world_id: WorldId, flag_name: &str) -> Result<(), RepoError> {
        // Use MERGE to create or update the flag relationship
        // We use a dummy Flag node as the target (could also use a self-relationship)
        let q = query(
            "MATCH (w:World {id: $world_id})
             MERGE (f:Flag {name: $flag_name, world_id: $world_id})
             MERGE (w)-[:HAS_FLAG {name: $flag_name}]->(f)",
        )
        .param("world_id", world_id.to_string())
        .param("flag_name", flag_name);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        Ok(())
    }

    async fn unset_world_flag(&self, world_id: WorldId, flag_name: &str) -> Result<(), RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[r:HAS_FLAG {name: $flag_name}]->(f:Flag)
             DELETE r
             WITH f
             WHERE NOT exists((f)<-[:HAS_FLAG]-())
             DELETE f",
        )
        .param("world_id", world_id.to_string())
        .param("flag_name", flag_name);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        Ok(())
    }

    async fn set_pc_flag(
        &self,
        pc_id: PlayerCharacterId,
        flag_name: &str,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})
             MERGE (f:Flag {name: $flag_name, pc_id: $pc_id})
             MERGE (pc)-[:HAS_FLAG {name: $flag_name}]->(f)",
        )
        .param("pc_id", pc_id.to_string())
        .param("flag_name", flag_name);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        Ok(())
    }

    async fn unset_pc_flag(
        &self,
        pc_id: PlayerCharacterId,
        flag_name: &str,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[r:HAS_FLAG {name: $flag_name}]->(f:Flag)
             DELETE r
             WITH f
             WHERE NOT exists((f)<-[:HAS_FLAG]-())
             DELETE f",
        )
        .param("pc_id", pc_id.to_string())
        .param("flag_name", flag_name);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;
        Ok(())
    }

    async fn is_world_flag_set(
        &self,
        world_id: WorldId,
        flag_name: &str,
    ) -> Result<bool, RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_FLAG {name: $flag_name}]->()
             RETURN count(*) > 0 AS is_set",
        )
        .param("world_id", world_id.to_string())
        .param("flag_name", flag_name);

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            Ok(row.get::<bool>("is_set").unwrap_or(false))
        } else {
            Ok(false)
        }
    }

    async fn is_pc_flag_set(
        &self,
        pc_id: PlayerCharacterId,
        flag_name: &str,
    ) -> Result<bool, RepoError> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[:HAS_FLAG {name: $flag_name}]->()
             RETURN count(*) > 0 AS is_set",
        )
        .param("pc_id", pc_id.to_string())
        .param("flag_name", flag_name);

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            Ok(row.get::<bool>("is_set").unwrap_or(false))
        } else {
            Ok(false)
        }
    }
}
