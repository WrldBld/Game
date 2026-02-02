//! Neo4j observation repository implementation.
//!
//! Tracks PC observations of NPC locations (fog of war for investigation gameplay).
//! Observations are stored as edges: `(PlayerCharacter)-[:OBSERVED_NPC {...}]->(Character)`

use crate::infrastructure::neo4j::Neo4jGraph;
use async_trait::async_trait;
use neo4rs::query;
use wrldbldr_domain::common::{parse_datetime_or, StringExt};
use wrldbldr_domain::*;

use crate::infrastructure::ports::{ClockPort, ObservationRepo, PlayerCharacterRepo, RepoError, WorldRepo};

use super::helpers::{match_player_character_by_id};

pub struct Neo4jObservationRepo {
    graph: Neo4jGraph,
    clock: std::sync::Arc<dyn ClockPort>,
    player_character: std::sync::Arc<dyn PlayerCharacterRepo>,
    world: std::sync::Arc<dyn WorldRepo>,
}

impl Neo4jObservationRepo {
    pub fn new(
        graph: Neo4jGraph,
        clock: std::sync::Arc<dyn ClockPort>,
        player_character: std::sync::Arc<dyn PlayerCharacterRepo>,
        world: std::sync::Arc<dyn WorldRepo>,
    ) -> Self {
        Self {
            graph,
            clock,
            player_character,
            world,
        }
    }
}

#[async_trait]
impl ObservationRepo for Neo4jObservationRepo {
    /// Get all observations for a PC
    async fn get_observations(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcObservation>, RepoError> {
        let q = query(&format!(
            "{}-[r:OBSERVED_NPC]->(npc:Character)
            RETURN r.location_id as location_id, r.region_id as region_id,
                   r.game_time as game_time, r.observation_type as observation_type,
                   coalesce(r.is_revealed_to_player, true) as is_revealed_to_player,
                   r.notes as notes, r.created_at as created_at,
                   npc.id as npc_id
            ORDER BY r.game_time DESC",
            match_player_character_by_id().replace("$id", "$pc_id")
        ))
        .param("pc_id", pc_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        let mut observations = Vec::new();
        let now = self.clock.now();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let npc_id_str: String = row
                .get("npc_id")
                .map_err(|e| RepoError::database("query", e))?;
            let location_id_str: String = row
                .get("location_id")
                .map_err(|e| RepoError::database("query", e))?;
            let region_id_str: String = row
                .get("region_id")
                .map_err(|e| RepoError::database("query", e))?;
            let game_time_str: String = row
                .get("game_time")
                .map_err(|e| RepoError::database("query", e))?;
            let observation_type_str: String = row
                .get("observation_type")
                .map_err(|e| RepoError::database("query", e))?;
            let is_revealed_to_player: bool = row.get("is_revealed_to_player").unwrap_or(true);
            let notes: String = row.get("notes").unwrap_or_default();
            let created_at_str: String = row
                .get("created_at")
                .map_err(|e| RepoError::database("query", e))?;

            let observation = NpcObservation::from_storage(
                pc_id,
                CharacterId::from_uuid(
                    uuid::Uuid::parse_str(&npc_id_str)
                        .map_err(|e| RepoError::database("query", e))?,
                ),
                LocationId::from_uuid(
                    uuid::Uuid::parse_str(&location_id_str)
                        .map_err(|e| RepoError::database("query", e))?,
                ),
                RegionId::from_uuid(
                    uuid::Uuid::parse_str(&region_id_str)
                        .map_err(|e| RepoError::database("query", e))?,
                ),
                parse_datetime_or(&game_time_str, now),
                ObservationType::from_key(&observation_type_str)
                    .unwrap_or(ObservationType::Direct),
                is_revealed_to_player,
                notes.into_option(),
                parse_datetime_or(&created_at_str, now),
            );

            observations.push(observation);
        }

        Ok(observations)
    }

    /// Delete an observation between a PC and NPC
    async fn delete_observation(
        &self,
        pc_id: PlayerCharacterId,
        target_id: CharacterId,
    ) -> Result<(), RepoError> {
        let q = query(&format!(
            "{}-[r:OBSERVED_NPC]->(npc:Character {{id: $npc_id}})
            DELETE r",
            match_player_character_by_id().replace("$id", "$pc_id")
        ))
        .param("pc_id", pc_id.to_string())
        .param("npc_id", target_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        tracing::debug!("Deleted observation from PC {} to NPC {}", pc_id, target_id);
        Ok(())
    }

    /// Save an observation (upsert - updates if exists)
    async fn save_observation(&self, observation: &NpcObservation) -> Result<(), RepoError> {
        let q = query(&format!(
            "{}, (npc:Character {{id: $npc_id}})
            MERGE (pc)-[r:OBSERVED_NPC]->(npc)
            SET r.location_id = $location_id,
                r.region_id = $region_id,
                r.game_time = $game_time,
                r.observation_type = $observation_type,
                r.is_revealed_to_player = $is_revealed_to_player,
                r.notes = $notes,
                r.created_at = $created_at",
            match_player_character_by_id().replace("$id", "$pc_id")
        ))
        .param("pc_id", observation.pc_id().to_string())
        .param("npc_id", observation.npc_id().to_string())
        .param("location_id", observation.location_id().to_string())
        .param("region_id", observation.region_id().to_string())
        .param("game_time", observation.game_time().to_rfc3339())
        .param(
            "observation_type",
            observation.observation_type().as_key(),
        )
        .param("is_revealed_to_player", observation.is_revealed_to_player())
        .param("notes", observation.notes().unwrap_or_default().to_string())
        .param("created_at", observation.created_at().to_rfc3339());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        Ok(())
    }

    /// Check if a PC has observed a specific NPC
    async fn has_observed(
        &self,
        pc_id: PlayerCharacterId,
        target_id: CharacterId,
    ) -> Result<bool, RepoError> {
        let q = query(&format!(
            "{}-[r:OBSERVED_NPC]->(npc:Character {{id: $npc_id}})
            RETURN count(r) > 0 as has_observed",
            match_player_character_by_id().replace("$id", "$pc_id")
        ))
        .param("pc_id", pc_id.to_string())
        .param("npc_id", target_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let has_observed: bool = row.get("has_observed").unwrap_or(false);
            Ok(has_observed)
        } else {
            Ok(false)
        }
    }

    /// Save deduced information from a challenge outcome.
    /// If npc_id is provided, creates a Deduced observation; otherwise creates a JournalEntry.
    async fn save_deduced_info(
        &self,
        pc_id: PlayerCharacterId,
        info: String,
        npc_id: Option<CharacterId>,
        location_id: Option<LocationId>,
        region_id: Option<RegionId>,
    ) -> Result<(), RepoError> {
        let now = self.clock.now();

        // If npc_id provided, create a Deduced observation instead of JournalEntry
        if let Some(npc) = npc_id {
            // Validate required fields for observation
            let location_id = location_id.ok_or_else(|| {
                RepoError::constraint("location_id required when npc_id is provided for deduced info")
            })?;
            let region_id = region_id.ok_or_else(|| {
                RepoError::constraint("region_id required when npc_id is provided for deduced info")
            })?;

            // Get world game time for the observation
            let player = self
                .player_character
                .get(pc_id)
                .await
                .map_err(|e| RepoError::database("get player character", e))?
                .ok_or_else(|| RepoError::not_found("PlayerCharacter", pc_id.to_string()))?;

            let world = self
                .world
                .get(player.world_id())
                .await
                .map_err(|e| RepoError::database("get world", e))?
                .ok_or_else(|| {
                    RepoError::not_found("World", player.world_id().to_string())
                })?;

            let game_time = world.game_time().to_datetime();

            let observation = wrldbldr_domain::NpcObservation::deduced(
                pc_id,
                npc,
                location_id,
                region_id,
                game_time,
                Some(info),
                now,
            );

            self.save_observation(&observation).await?;
        } else {
            // No npc_id: create JournalEntry for general deduced info
            let entry_id = uuid::Uuid::new_v4().to_string();

            let q = query(&format!(
                "{} CREATE (je:JournalEntry {{
                    id: $entry_id,
                    content: $info,
                    entry_type: 'deduced',
                    created_at: $created_at
                }})
                CREATE (pc)-[:HAS_JOURNAL_ENTRY]->(je)",
                match_player_character_by_id().replace("$id", "$pc_id")
            ))
            .param("pc_id", pc_id.to_string())
            .param("entry_id", entry_id)
            .param("info", info)
            .param("created_at", now.to_rfc3339());

            self.graph
                .run(q)
                .await
                .map_err(|e| RepoError::database("query", e))?;
        }

        Ok(())
    }
}
