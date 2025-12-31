//! Observation Repository (Phase 23D)
//!
//! Neo4j repository for NPC observations.
//! Observations are stored as edges: `(PlayerCharacter)-[:OBSERVED_NPC {...}]->(Character)`

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;
use wrldbldr_common::datetime::parse_datetime_or;

use super::connection::Neo4jConnection;
use wrldbldr_domain::entities::{NpcObservation, ObservationSummary, ObservationType};
use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, RegionId};
use wrldbldr_engine_ports::outbound::{ClockPort, ObservationRepositoryPort};

/// Repository for NPC observations
pub struct Neo4jObservationRepository {
    connection: Neo4jConnection,
    clock: Arc<dyn ClockPort>,
}

impl Neo4jObservationRepository {
    pub fn new(connection: Neo4jConnection, clock: Arc<dyn ClockPort>) -> Self {
        Self { connection, clock }
    }

    /// Create or update an observation (upsert)
    ///
    /// If the PC already has an observation for this NPC, it will be updated.
    /// This ensures we always have the latest known location.
    pub async fn upsert(&self, observation: &NpcObservation) -> Result<()> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id}), (npc:Character {id: $npc_id})
            MERGE (pc)-[r:OBSERVED_NPC]->(npc)
            SET r.location_id = $location_id,
                r.region_id = $region_id,
                r.game_time = $game_time,
                r.observation_type = $observation_type,
                r.is_revealed_to_player = $is_revealed_to_player,
                r.notes = $notes,
                r.created_at = $created_at
            RETURN r",
        )
        .param("pc_id", observation.pc_id.to_string())
        .param("npc_id", observation.npc_id.to_string())
        .param("location_id", observation.location_id.to_string())
        .param("region_id", observation.region_id.to_string())
        .param("game_time", observation.game_time.to_rfc3339())
        .param("observation_type", observation.observation_type.to_string())
        .param("is_revealed_to_player", observation.is_revealed_to_player)
        .param("notes", observation.notes.clone().unwrap_or_default())
        .param("created_at", observation.created_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Upserted observation: PC {} observed NPC {} at region {}",
            observation.pc_id,
            observation.npc_id,
            observation.region_id
        );
        Ok(())
    }

    /// Get all observations for a PC
    pub async fn get_for_pc(&self, pc_id: PlayerCharacterId) -> Result<Vec<NpcObservation>> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[r:OBSERVED_NPC]->(npc:Character)
            RETURN r.location_id as location_id, r.region_id as region_id,
                   r.game_time as game_time, r.observation_type as observation_type,
                   coalesce(r.is_revealed_to_player, true) as is_revealed_to_player,
                   r.notes as notes, r.created_at as created_at,
                   npc.id as npc_id
            ORDER BY r.game_time DESC",
        )
        .param("pc_id", pc_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut observations = Vec::new();

        while let Some(row) = result.next().await? {
            let npc_id_str: String = row.get("npc_id")?;
            let location_id_str: String = row.get("location_id")?;
            let region_id_str: String = row.get("region_id")?;
            let game_time_str: String = row.get("game_time")?;
            let observation_type_str: String = row.get("observation_type")?;
            let is_revealed_to_player: bool = row.get("is_revealed_to_player").unwrap_or(true);
            let notes: String = row.get("notes").unwrap_or_default();
            let created_at_str: String = row.get("created_at")?;

            let observation = NpcObservation {
                pc_id,
                npc_id: CharacterId::from_uuid(uuid::Uuid::parse_str(&npc_id_str)?),
                location_id: LocationId::from_uuid(uuid::Uuid::parse_str(&location_id_str)?),
                region_id: RegionId::from_uuid(uuid::Uuid::parse_str(&region_id_str)?),
                game_time: parse_datetime_or(&game_time_str, self.clock.now()),
                observation_type: observation_type_str
                    .parse()
                    .unwrap_or(ObservationType::Direct),
                is_revealed_to_player,
                notes: if notes.is_empty() { None } else { Some(notes) },
                created_at: parse_datetime_or(&created_at_str, self.clock.now()),
            };

            observations.push(observation);
        }

        Ok(observations)
    }

    /// Get all observations with NPC details (for display)
    pub async fn get_summaries_for_pc(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<ObservationSummary>> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[r:OBSERVED_NPC]->(npc:Character)
            MATCH (loc:Location {id: r.location_id})
            MATCH (reg:Region {id: r.region_id})
             RETURN npc.id as npc_id, npc.name as npc_name, npc.portrait_asset as npc_portrait,
                    loc.name as location_name, reg.name as region_name,
                    r.game_time as game_time, r.observation_type as observation_type,
                    coalesce(r.is_revealed_to_player, true) as is_revealed_to_player,
                    r.notes as notes
             ORDER BY r.game_time DESC",
        )
        .param("pc_id", pc_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut summaries = Vec::new();

        while let Some(row) = result.next().await? {
            let npc_id: String = row.get("npc_id")?;
            let npc_name: String = row.get("npc_name")?;
            let npc_portrait: String = row.get("npc_portrait").unwrap_or_default();
            let location_name: String = row.get("location_name")?;
            let region_name: String = row.get("region_name")?;
            let game_time_str: String = row.get("game_time")?;
            let observation_type_str: String = row.get("observation_type")?;
            let is_revealed_to_player: bool = row.get("is_revealed_to_player").unwrap_or(true);
            let notes: String = row.get("notes").unwrap_or_default();

            let game_time = parse_datetime_or(&game_time_str, self.clock.now());

            summaries.push(ObservationSummary {
                npc_id,
                npc_name,
                npc_portrait: if npc_portrait.is_empty() {
                    None
                } else {
                    Some(npc_portrait)
                },
                is_revealed_to_player,
                location_name,
                region_name,
                game_time,
                observation_type: observation_type_str
                    .parse()
                    .unwrap_or(ObservationType::Direct),
                notes: if notes.is_empty() { None } else { Some(notes) },
                time_ago_description: None, // Caller can compute this from game time
            });
        }

        Ok(summaries)
    }

    /// Get the latest observation of a specific NPC by a PC
    pub async fn get_latest(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<Option<NpcObservation>> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[r:OBSERVED_NPC]->(npc:Character {id: $npc_id})
            RETURN r.location_id as location_id, r.region_id as region_id,
                   r.game_time as game_time, r.observation_type as observation_type,
                   coalesce(r.is_revealed_to_player, true) as is_revealed_to_player,
                   r.notes as notes, r.created_at as created_at",
        )
        .param("pc_id", pc_id.to_string())
        .param("npc_id", npc_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let location_id_str: String = row.get("location_id")?;
            let region_id_str: String = row.get("region_id")?;
            let game_time_str: String = row.get("game_time")?;
            let observation_type_str: String = row.get("observation_type")?;
            let is_revealed_to_player: bool = row.get("is_revealed_to_player").unwrap_or(true);
            let notes: String = row.get("notes").unwrap_or_default();
            let created_at_str: String = row.get("created_at")?;

            Ok(Some(NpcObservation {
                pc_id,
                npc_id,
                location_id: LocationId::from_uuid(uuid::Uuid::parse_str(&location_id_str)?),
                region_id: RegionId::from_uuid(uuid::Uuid::parse_str(&region_id_str)?),
                game_time: parse_datetime_or(&game_time_str, self.clock.now()),
                observation_type: observation_type_str
                    .parse()
                    .unwrap_or(ObservationType::Direct),
                is_revealed_to_player,
                notes: if notes.is_empty() { None } else { Some(notes) },
                created_at: parse_datetime_or(&created_at_str, self.clock.now()),
            }))
        } else {
            Ok(None)
        }
    }

    /// Delete an observation
    pub async fn delete(&self, pc_id: PlayerCharacterId, npc_id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[r:OBSERVED_NPC]->(npc:Character {id: $npc_id})
            DELETE r",
        )
        .param("pc_id", pc_id.to_string())
        .param("npc_id", npc_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Delete all observations for a PC
    pub async fn delete_all_for_pc(&self, pc_id: PlayerCharacterId) -> Result<()> {
        let q = query(
            "MATCH (pc:PlayerCharacter {id: $pc_id})-[r:OBSERVED_NPC]->()
            DELETE r",
        )
        .param("pc_id", pc_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Batch create observations (e.g., when PC enters a new region and sees multiple NPCs)
    pub async fn batch_upsert(&self, observations: &[NpcObservation]) -> Result<()> {
        for obs in observations {
            self.upsert(obs).await?;
        }
        Ok(())
    }
}

// Implement the port trait for hexagonal architecture compliance
#[async_trait]
impl ObservationRepositoryPort for Neo4jObservationRepository {
    async fn upsert(&self, observation: &NpcObservation) -> Result<()> {
        Neo4jObservationRepository::upsert(self, observation).await
    }

    async fn get_for_pc(&self, pc_id: PlayerCharacterId) -> Result<Vec<NpcObservation>> {
        Neo4jObservationRepository::get_for_pc(self, pc_id).await
    }

    async fn get_latest(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<Option<NpcObservation>> {
        Neo4jObservationRepository::get_latest(self, pc_id, npc_id).await
    }

    async fn delete(&self, pc_id: PlayerCharacterId, npc_id: CharacterId) -> Result<()> {
        Neo4jObservationRepository::delete(self, pc_id, npc_id).await
    }
}
