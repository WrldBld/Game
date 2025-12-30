//! Neo4j staging repository implementation
//!
//! # Neo4j Schema
//!
//! Nodes:
//! - `(:Staging)` - DM-approved NPC presence configuration
//!
//! Edges:
//! - `(Region)-[:CURRENT_STAGING]->(Staging)` - Active staging for region
//! - `(Region)-[:HAS_STAGING]->(Staging)` - Historical stagings
//! - `(Staging)-[:INCLUDES_NPC {is_present, reasoning}]->(Character)` - NPC presence

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::{StagedNpcRow, StagingRepositoryPort};
use crate::domain::entities::{StagedNpc, Staging, StagingSource};
use crate::domain::value_objects::{
    CharacterId, GameTime, LocationId, RegionId, StagingId, WorldId,
};

pub struct Neo4jStagingRepository {
    connection: Neo4jConnection,
}

impl Neo4jStagingRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Helper to convert a Neo4j row to a Staging entity (without NPCs)
    fn row_to_staging(row: Row) -> Result<Staging> {
        let node: neo4rs::Node = row.get("s")?;

        let id_str: String = node.get("id")?;
        let region_id_str: String = node.get("region_id")?;
        let location_id_str: String = node.get("location_id")?;
        let world_id_str: String = node.get("world_id")?;
        let game_time_str: String = node.get("game_time")?;
        let approved_at_str: String = node.get("approved_at")?;
        let ttl_hours: i64 = node.get("ttl_hours")?;
        let approved_by: String = node.get("approved_by")?;
        let source_str: String = node.get("source")?;
        let dm_guidance: String = node.get("dm_guidance").unwrap_or_default();
        let is_active: bool = node.get("is_active")?;

        let id = uuid::Uuid::parse_str(&id_str)?;
        let region_id = uuid::Uuid::parse_str(&region_id_str)?;
        let location_id = uuid::Uuid::parse_str(&location_id_str)?;
        let world_id = uuid::Uuid::parse_str(&world_id_str)?;

        let game_time = DateTime::parse_from_rfc3339(&game_time_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let approved_at = DateTime::parse_from_rfc3339(&approved_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let source = source_str.parse().unwrap_or(StagingSource::RuleBased);

        Ok(Staging {
            id: StagingId::from_uuid(id),
            region_id: RegionId::from_uuid(region_id),
            location_id: LocationId::from_uuid(location_id),
            world_id: WorldId::from_uuid(world_id),
            npcs: Vec::new(), // NPCs loaded separately via get_staged_npcs
            game_time,
            approved_at,
            ttl_hours: ttl_hours as i32,
            approved_by,
            source,
            dm_guidance: if dm_guidance.is_empty() {
                None
            } else {
                Some(dm_guidance)
            },
            is_active,
        })
    }
}

#[async_trait]
impl StagingRepositoryPort for Neo4jStagingRepository {
    async fn get_current(&self, region_id: RegionId) -> Result<Option<Staging>> {
        let q = query(
            "MATCH (r:Region {id: $region_id})-[:CURRENT_STAGING]->(s:Staging)
             WHERE s.is_active = true
             RETURN s",
        )
        .param("region_id", region_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let mut staging = Self::row_to_staging(row)?;
            // Load NPCs for this staging
            staging.npcs = self
                .get_staged_npcs(staging.id)
                .await?
                .into_iter()
                .map(|row| StagedNpc {
                    character_id: row.character_id,
                    name: row.name,
                    sprite_asset: row.sprite_asset,
                    portrait_asset: row.portrait_asset,
                    is_present: row.is_present,
                    reasoning: row.reasoning,
                })
                .collect();
            Ok(Some(staging))
        } else {
            Ok(None)
        }
    }

    async fn get_history(&self, region_id: RegionId, limit: u32) -> Result<Vec<Staging>> {
        let q = query(
            "MATCH (r:Region {id: $region_id})-[:HAS_STAGING]->(s:Staging)
             RETURN s
             ORDER BY s.approved_at DESC
             LIMIT $limit",
        )
        .param("region_id", region_id.to_string())
        .param("limit", limit as i64);

        let mut result = self.connection.graph().execute(q).await?;
        let mut stagings = Vec::new();

        while let Some(row) = result.next().await? {
            let staging = Self::row_to_staging(row)?;
            stagings.push(staging);
        }

        // Note: NPCs are not loaded for history to keep it lightweight
        // Use get() with specific ID if NPCs are needed
        Ok(stagings)
    }

    async fn get(&self, id: StagingId) -> Result<Option<Staging>> {
        let q = query(
            "MATCH (s:Staging {id: $id})
             RETURN s",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let mut staging = Self::row_to_staging(row)?;
            // Load NPCs for this staging
            staging.npcs = self
                .get_staged_npcs(staging.id)
                .await?
                .into_iter()
                .map(|row| StagedNpc {
                    character_id: row.character_id,
                    name: row.name,
                    sprite_asset: row.sprite_asset,
                    portrait_asset: row.portrait_asset,
                    is_present: row.is_present,
                    reasoning: row.reasoning,
                })
                .collect();
            Ok(Some(staging))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, staging: &Staging) -> Result<StagingId> {
        // Create the Staging node
        let q = query(
            "MATCH (r:Region {id: $region_id})
             CREATE (s:Staging {
                 id: $id,
                 region_id: $region_id,
                 location_id: $location_id,
                 world_id: $world_id,
                 game_time: $game_time,
                 approved_at: $approved_at,
                 ttl_hours: $ttl_hours,
                 approved_by: $approved_by,
                 source: $source,
                 dm_guidance: $dm_guidance,
                 is_active: $is_active
             })
             CREATE (r)-[:HAS_STAGING]->(s)
             RETURN s.id as id",
        )
        .param("id", staging.id.to_string())
        .param("region_id", staging.region_id.to_string())
        .param("location_id", staging.location_id.to_string())
        .param("world_id", staging.world_id.to_string())
        .param("game_time", staging.game_time.to_rfc3339())
        .param("approved_at", staging.approved_at.to_rfc3339())
        .param("ttl_hours", staging.ttl_hours as i64)
        .param("approved_by", staging.approved_by.clone())
        .param("source", staging.source.to_string())
        .param(
            "dm_guidance",
            staging.dm_guidance.clone().unwrap_or_default(),
        )
        .param("is_active", staging.is_active);

        self.connection
            .graph()
            .run(q)
            .await
            .context("Failed to create Staging node")?;

        // Create INCLUDES_NPC edges for each NPC
        for npc in &staging.npcs {
            let npc_q = query(
                "MATCH (s:Staging {id: $staging_id})
                 MATCH (c:Character {id: $character_id})
                 CREATE (s)-[:INCLUDES_NPC {
                     is_present: $is_present,
                     reasoning: $reasoning
                 }]->(c)",
            )
            .param("staging_id", staging.id.to_string())
            .param("character_id", npc.character_id.to_string())
            .param("is_present", npc.is_present)
            .param("reasoning", npc.reasoning.clone());

            self.connection
                .graph()
                .run(npc_q)
                .await
                .context("Failed to create INCLUDES_NPC edge")?;
        }

        tracing::debug!(
            staging_id = %staging.id,
            region_id = %staging.region_id,
            npc_count = staging.npcs.len(),
            "Saved staging"
        );

        Ok(staging.id)
    }

    async fn is_valid(&self, staging_id: StagingId, current_game_time: &GameTime) -> Result<bool> {
        let staging = self.get(staging_id).await?;

        match staging {
            Some(s) => {
                if !s.is_active {
                    return Ok(false);
                }
                Ok(!s.is_expired(&current_game_time.current()))
            }
            None => Ok(false),
        }
    }

    async fn invalidate_all(&self, region_id: RegionId) -> Result<()> {
        let q = query(
            "MATCH (r:Region {id: $region_id})-[:HAS_STAGING]->(s:Staging)
             SET s.is_active = false",
        )
        .param("region_id", region_id.to_string());

        self.connection
            .graph()
            .run(q)
            .await
            .context("Failed to invalidate stagings")?;

        // Also remove CURRENT_STAGING edge
        let remove_current_q = query(
            "MATCH (r:Region {id: $region_id})-[rel:CURRENT_STAGING]->()
             DELETE rel",
        )
        .param("region_id", region_id.to_string());

        self.connection
            .graph()
            .run(remove_current_q)
            .await
            .context("Failed to remove CURRENT_STAGING edge")?;

        tracing::debug!(region_id = %region_id, "Invalidated all stagings for region");

        Ok(())
    }

    async fn set_current(&self, staging_id: StagingId) -> Result<()> {
        // First, get the staging to find its region
        let staging = self
            .get(staging_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Staging not found: {}", staging_id))?;

        // Remove any existing CURRENT_STAGING edge for this region
        let remove_q = query(
            "MATCH (r:Region {id: $region_id})-[rel:CURRENT_STAGING]->()
             DELETE rel",
        )
        .param("region_id", staging.region_id.to_string());

        self.connection
            .graph()
            .run(remove_q)
            .await
            .context("Failed to remove existing CURRENT_STAGING edge")?;

        // Create new CURRENT_STAGING edge
        let create_q = query(
            "MATCH (r:Region {id: $region_id})
             MATCH (s:Staging {id: $staging_id})
             CREATE (r)-[:CURRENT_STAGING]->(s)",
        )
        .param("region_id", staging.region_id.to_string())
        .param("staging_id", staging_id.to_string());

        self.connection
            .graph()
            .run(create_q)
            .await
            .context("Failed to create CURRENT_STAGING edge")?;

        tracing::debug!(
            staging_id = %staging_id,
            region_id = %staging.region_id,
            "Set current staging"
        );

        Ok(())
    }

    async fn get_staged_npcs(&self, staging_id: StagingId) -> Result<Vec<StagedNpcRow>> {
        let q = query(
            "MATCH (s:Staging {id: $staging_id})-[r:INCLUDES_NPC]->(c:Character)
             RETURN c.id as character_id,
                    c.name as name,
                    c.sprite_asset as sprite_asset,
                    c.portrait_asset as portrait_asset,
                    r.is_present as is_present,
                    r.reasoning as reasoning",
        )
        .param("staging_id", staging_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut npcs = Vec::new();

        while let Some(row) = result.next().await? {
            let character_id_str: String = row.get("character_id")?;
            let name: String = row.get("name")?;
            let sprite_asset: String = row.get("sprite_asset").unwrap_or_default();
            let portrait_asset: String = row.get("portrait_asset").unwrap_or_default();
            let is_present: bool = row.get("is_present")?;
            let reasoning: String = row.get("reasoning")?;

            let character_id =
                CharacterId::from_uuid(uuid::Uuid::parse_str(&character_id_str)?);

            npcs.push(StagedNpcRow {
                character_id,
                name,
                sprite_asset: if sprite_asset.is_empty() {
                    None
                } else {
                    Some(sprite_asset)
                },
                portrait_asset: if portrait_asset.is_empty() {
                    None
                } else {
                    Some(portrait_asset)
                },
                is_present,
                reasoning,
            });
        }

        Ok(npcs)
    }
}
