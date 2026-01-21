// Staging repo - clock field for future TTL calculations
#![allow(dead_code)]

//! Neo4j staging repository implementation.
//!
//! Handles NPC staging for regions and pending staging approval.

use crate::infrastructure::neo4j::Neo4jGraph;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use neo4rs::{query, Node, Row};

use wrldbldr_domain::MoodState;
use wrldbldr_domain::*;

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{ClockPort, RepoError, StagingRepo};

pub struct Neo4jStagingRepo {
    graph: Neo4jGraph,
    clock: std::sync::Arc<dyn ClockPort>,
}

impl Neo4jStagingRepo {
    pub fn new(graph: Neo4jGraph, clock: std::sync::Arc<dyn ClockPort>) -> Self {
        Self { graph, clock }
    }
}

#[async_trait]
impl StagingRepo for Neo4jStagingRepo {
    /// Get all staged NPCs in a region (from current active staging)
    async fn get_staged_npcs(&self, region_id: RegionId) -> Result<Vec<StagedNpc>, RepoError> {
        let q = query(
            "MATCH (r:Region {id: $region_id})-[:CURRENT_STAGING]->(s:Staging)-[rel:INCLUDES_NPC]->(c:Character)
            WHERE s.is_active = true AND rel.is_present = true
            RETURN c.id as character_id,
                   c.name as name,
                   c.sprite_asset as sprite_asset,
                   c.portrait_asset as portrait_asset,
                   rel.is_present as is_present,
                   COALESCE(rel.is_hidden_from_players, false) as is_hidden_from_players,
                   rel.reasoning as reasoning,
                   COALESCE(rel.mood, c.default_mood, 'calm') as mood,
                   COALESCE(rel.has_incomplete_data, false) as has_incomplete_data",
        )
        .param("region_id", region_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        let mut npcs = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            npcs.push(row_to_staged_npc(row)?);
        }

        Ok(npcs)
    }

    /// Stage an NPC in a region (add to current staging or create new one)
    async fn stage_npc(
        &self,
        region_id: RegionId,
        character_id: CharacterId,
    ) -> Result<(), RepoError> {
        // First try to add to existing current staging
        let q = query(
            "MATCH (r:Region {id: $region_id})-[:CURRENT_STAGING]->(s:Staging)
            MATCH (c:Character {id: $character_id})
            MERGE (s)-[rel:INCLUDES_NPC]->(c)
            ON CREATE SET rel.is_present = true, rel.is_hidden_from_players = false, rel.reasoning = 'Manually staged', rel.mood = COALESCE(c.default_mood, 'calm')
            ON MATCH SET rel.is_present = true
            RETURN s.id as staging_id",
        )
        .param("region_id", region_id.to_string())
        .param("character_id", character_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
            .is_none()
        {
            // No current staging exists, create one
            let now = self.clock.now();
            let staging_id = StagingId::new();

            // Create new staging and link it
            // Get world_id via Region -> Location (location_id property) -> Location.world_id
            // Note: game_time_minutes defaults to 0 for manually staged NPCs (should be set properly in use case)
            let create_q = query(
                "MATCH (r:Region {id: $region_id})
                MATCH (c:Character {id: $character_id})
                MATCH (l:Location {id: r.location_id})
                WITH r, c, l.id as location_id, l.world_id as world_id
                CREATE (s:Staging {
                    id: $staging_id,
                    region_id: $region_id,
                    location_id: location_id,
                    world_id: world_id,
                    game_time_minutes: $game_time_minutes,
                    approved_at: $approved_at,
                    ttl_hours: 24,
                    approved_by: 'system',
                    source: 'DmCustomized',
                    is_active: true
                })
                CREATE (r)-[:CURRENT_STAGING]->(s)
                CREATE (r)-[:HAS_STAGING]->(s)
                CREATE (s)-[:INCLUDES_NPC {is_present: true, is_hidden_from_players: false, reasoning: 'Manually staged', mood: COALESCE(c.default_mood, 'calm')}]->(c)",
            )
            .param("region_id", region_id.to_string())
            .param("character_id", character_id.to_string())
            .param("staging_id", staging_id.to_string())
            .param("game_time_minutes", 0i64) // Default to epoch for manual staging
            .param("approved_at", now.to_rfc3339());

            self.graph
                .run(create_q)
                .await
                .map_err(|e| RepoError::database("query", e))?;
        }

        Ok(())
    }

    /// Remove an NPC from staging in a region
    async fn unstage_npc(
        &self,
        region_id: RegionId,
        character_id: CharacterId,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (r:Region {id: $region_id})-[:CURRENT_STAGING]->(s:Staging)-[rel:INCLUDES_NPC]->(c:Character {id: $character_id})
            SET rel.is_present = false",
        )
        .param("region_id", region_id.to_string())
        .param("character_id", character_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        Ok(())
    }

    /// Get pending stagings awaiting DM approval for a world
    async fn get_pending_staging(&self, world_id: WorldId) -> Result<Vec<Staging>, RepoError> {
        // Use COLLECT to fetch NPCs in single query (avoids N+1)
        let q = query(
            "MATCH (s:Staging {world_id: $world_id})
            WHERE s.is_active = false AND NOT EXISTS((s)<-[:CURRENT_STAGING]-())
            OPTIONAL MATCH (s)-[rel:INCLUDES_NPC]->(c:Character)
            WITH s, COLLECT({
                character_id: c.id,
                name: c.name,
                sprite_asset: c.sprite_asset,
                portrait_asset: c.portrait_asset,
                is_present: rel.is_present,
                is_hidden_from_players: COALESCE(rel.is_hidden_from_players, false),
                reasoning: rel.reasoning,
                mood: COALESCE(rel.mood, c.default_mood, 'calm')
            }) as npcs
            RETURN s, npcs
            ORDER BY s.approved_at DESC",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        let mut stagings = Vec::new();
        let now = self.clock.now();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let staging = row_to_staging_with_npcs(row, now)?;
            stagings.push(staging);
        }

        Ok(stagings)
    }

    /// Save a pending staging for DM approval and immediately activate it.
    /// Uses explicit transaction to ensure save and activate are atomic.
    async fn save_and_activate_pending_staging(
        &self,
        staging: &Staging,
        region_id: RegionId,
    ) -> Result<(), RepoError> {
        // Use explicit transaction to ensure save and activate are atomic
        let mut txn = self
            .graph
            .start_txn()
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // First, save the staging node and all NPC relationships
        let npc_character_ids: Vec<String> = staging
            .npcs()
            .iter()
            .map(|n| n.character_id.to_string())
            .collect();
        let npc_is_present: Vec<bool> = staging.npcs().iter().map(|n| n.is_present()).collect();
        let npc_is_hidden_from_players: Vec<bool> = staging
            .npcs()
            .iter()
            .map(|n| n.is_hidden_from_players())
            .collect();
        let npc_reasoning: Vec<String> = staging
            .npcs()
            .iter()
            .map(|n| n.reasoning.to_string())
            .collect();
        let npc_mood: Vec<String> = staging.npcs().iter().map(|n| n.mood.to_string()).collect();
        let npc_has_incomplete_data: Vec<bool> = staging
            .npcs()
            .iter()
            .map(|n| n.has_incomplete_data)
            .collect();

        let save_q = query(
            "MATCH (r:Region {id: $region_id})
            CREATE (s:Staging {
                id: $id,
                region_id: $region_id,
                location_id: $location_id,
                world_id: $world_id,
                game_time_minutes: $game_time_minutes,
                approved_at: $approved_at,
                ttl_hours: $ttl_hours,
                approved_by: $approved_by,
                source: $source,
                dm_guidance: $dm_guidance,
                is_active: $is_active
            })
            CREATE (r)-[:HAS_STAGING]->(s)
            WITH s
            UNWIND range(0, size($npc_character_ids) - 1) as i
            MATCH (c:Character {id: $npc_character_ids[i]})
            CREATE (s)-[:INCLUDES_NPC {
                is_present: $npc_is_present[i],
                is_hidden_from_players: $npc_is_hidden_from_players[i],
                reasoning: $npc_reasoning[i],
                mood: $npc_mood[i],
                has_incomplete_data: $npc_has_incomplete_data[i]
            }]->(c)",
        )
        .param("id", staging.id().to_string())
        .param("region_id", staging.region_id().to_string())
        .param("location_id", staging.location_id().to_string())
        .param("world_id", staging.world_id().to_string())
        .param("game_time_minutes", staging.game_time_minutes())
        .param("approved_at", staging.approved_at().to_rfc3339())
        .param("ttl_hours", staging.ttl_hours() as i64)
        .param("approved_by", staging.approved_by().to_string())
        .param("source", staging.source().to_string())
        .param(
            "dm_guidance",
            staging.dm_guidance().unwrap_or_default().to_string(),
        )
        .param("is_active", staging.is_active())
        .param("npc_character_ids", npc_character_ids)
        .param("npc_is_present", npc_is_present)
        .param("npc_is_hidden_from_players", npc_is_hidden_from_players)
        .param("npc_reasoning", npc_reasoning)
        .param("npc_mood", npc_mood)
        .param("npc_has_incomplete_data", npc_has_incomplete_data);

        txn.run(save_q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Then activate the staging (remove old current staging, set this one as current)
        let activate_q = query(
            "MATCH (r:Region {id: $region_id})
            OPTIONAL MATCH (r)-[old:CURRENT_STAGING]->(:Staging)
            DELETE old
            WITH r
            MATCH (s:Staging {id: $staging_id})
            SET s.is_active = true
            CREATE (r)-[:CURRENT_STAGING]->(s)",
        )
        .param("region_id", region_id.to_string())
        .param("staging_id", staging.id().to_string());

        txn.run(activate_q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Commit transaction
        txn.commit()
            .await
            .map_err(|e| RepoError::database("query", e))?;

        Ok(())
    }

    /// Save a pending staging for DM approval.
    /// Creates the staging node first, then adds NPC relationships separately (no APOC dependency).
    async fn save_pending_staging(&self, staging: &Staging) -> Result<(), RepoError> {
        let npc_character_ids: Vec<String> = staging
            .npcs()
            .iter()
            .map(|n| n.character_id.to_string())
            .collect();
        let npc_is_present: Vec<bool> = staging.npcs().iter().map(|n| n.is_present()).collect();
        let npc_is_hidden_from_players: Vec<bool> = staging
            .npcs()
            .iter()
            .map(|n| n.is_hidden_from_players())
            .collect();
        let npc_reasoning: Vec<String> = staging
            .npcs()
            .iter()
            .map(|n| n.reasoning.to_string())
            .collect();
        let npc_mood: Vec<String> = staging.npcs().iter().map(|n| n.mood.to_string()).collect();
        let npc_has_incomplete_data: Vec<bool> = staging
            .npcs()
            .iter()
            .map(|n| n.has_incomplete_data)
            .collect();

        // Create staging and all NPC relationships in one query (no APOC)
        let q = query(
            "MATCH (r:Region {id: $region_id})
            CREATE (s:Staging {
                id: $id,
                region_id: $region_id,
                location_id: $location_id,
                world_id: $world_id,
                game_time_minutes: $game_time_minutes,
                approved_at: $approved_at,
                ttl_hours: $ttl_hours,
                approved_by: $approved_by,
                source: $source,
                dm_guidance: $dm_guidance,
                is_active: $is_active
            })
            CREATE (r)-[:HAS_STAGING]->(s)
            WITH s
            UNWIND range(0, size($npc_character_ids) - 1) as i
            MATCH (c:Character {id: $npc_character_ids[i]})
            CREATE (s)-[:INCLUDES_NPC {
                is_present: $npc_is_present[i],
                is_hidden_from_players: $npc_is_hidden_from_players[i],
                reasoning: $npc_reasoning[i],
                mood: $npc_mood[i],
                has_incomplete_data: $npc_has_incomplete_data[i]
            }]->(c)",
        )
        .param("id", staging.id().to_string())
        .param("region_id", staging.region_id().to_string())
        .param("location_id", staging.location_id().to_string())
        .param("world_id", staging.world_id().to_string())
        .param("game_time_minutes", staging.game_time_minutes())
        .param("approved_at", staging.approved_at().to_rfc3339())
        .param("ttl_hours", staging.ttl_hours() as i64)
        .param("approved_by", staging.approved_by().to_string())
        .param("source", staging.source().to_string())
        .param(
            "dm_guidance",
            staging.dm_guidance().unwrap_or_default().to_string(),
        )
        .param("is_active", staging.is_active())
        .param("npc_character_ids", npc_character_ids)
        .param("npc_is_present", npc_is_present)
        .param("npc_is_hidden_from_players", npc_is_hidden_from_players)
        .param("npc_reasoning", npc_reasoning)
        .param("npc_mood", npc_mood)
        .param("npc_has_incomplete_data", npc_has_incomplete_data);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        Ok(())
    }

    /// Delete a pending staging
    async fn delete_pending_staging(&self, id: StagingId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (s:Staging {id: $id})
            DETACH DELETE s",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        Ok(())
    }

    /// Get active staging for a region, checking TTL expiry.
    /// Uses a single query with COLLECT to fetch staging and NPCs together (avoids N+1).
    async fn get_active_staging(
        &self,
        region_id: RegionId,
        current_game_time_minutes: i64,
    ) -> Result<Option<Staging>, RepoError> {
        let q = query(
            "MATCH (r:Region {id: $region_id})-[:CURRENT_STAGING]->(s:Staging)
            WHERE s.is_active = true
            OPTIONAL MATCH (s)-[rel:INCLUDES_NPC]->(c:Character)
            WITH s, COLLECT({
                character_id: c.id,
                name: c.name,
                sprite_asset: c.sprite_asset,
                portrait_asset: c.portrait_asset,
                is_present: rel.is_present,
                is_hidden_from_players: COALESCE(rel.is_hidden_from_players, false),
                reasoning: rel.reasoning,
                mood: COALESCE(rel.mood, c.default_mood, 'calm'),
                has_incomplete_data: COALESCE(rel.has_incomplete_data, false)
            }) as npcs
            RETURN s, npcs",
        )
        .param("region_id", region_id.to_string());

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
            let staging = row_to_staging_with_npcs(row, self.clock.now())?;

            // Check if staging is expired
            if staging.is_expired(current_game_time_minutes) {
                return Ok(None);
            }

            Ok(Some(staging))
        } else {
            Ok(None)
        }
    }

    /// Activate a staging, replacing any existing current staging for the region.
    /// Uses explicit transaction to ensure atomicity with save_pending_staging.
    async fn activate_staging(
        &self,
        staging_id: StagingId,
        region_id: RegionId,
    ) -> Result<(), RepoError> {
        // Use explicit transaction to ensure save and activate are atomic
        let mut txn = self
            .graph
            .start_txn()
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Remove existing CURRENT_STAGING relationship and add new one
        let q = query(
            "MATCH (r:Region {id: $region_id})
            OPTIONAL MATCH (r)-[old:CURRENT_STAGING]->(:Staging)
            DELETE old
            WITH r
            MATCH (s:Staging {id: $staging_id})
            SET s.is_active = true
            CREATE (r)-[:CURRENT_STAGING]->(s)",
        )
        .param("region_id", region_id.to_string())
        .param("staging_id", staging_id.to_string());

        txn.run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Commit transaction
        txn.commit()
            .await
            .map_err(|e| RepoError::database("query", e))?;

        Ok(())
    }

    /// Get staging history for a region (most recent first).
    async fn get_staging_history(
        &self,
        region_id: RegionId,
        limit: usize,
    ) -> Result<Vec<Staging>, RepoError> {
        // Get past stagings that are linked via HAS_STAGING but not CURRENT_STAGING
        // Use COLLECT to fetch NPCs in single query (avoids N+1)
        let q = query(
            "MATCH (r:Region {id: $region_id})-[:HAS_STAGING]->(s:Staging)
            WHERE NOT (r)-[:CURRENT_STAGING]->(s)
            OPTIONAL MATCH (s)-[rel:INCLUDES_NPC]->(c:Character)
            WITH s, COLLECT({
                character_id: c.id,
                name: c.name,
                sprite_asset: c.sprite_asset,
                portrait_asset: c.portrait_asset,
                is_present: rel.is_present,
                is_hidden_from_players: COALESCE(rel.is_hidden_from_players, false),
                reasoning: rel.reasoning,
                mood: COALESCE(rel.mood, c.default_mood, 'calm'),
                has_incomplete_data: COALESCE(rel.has_incomplete_data, false)
            }) as npcs
            RETURN s, npcs
            ORDER BY s.approved_at DESC
            LIMIT $limit",
        )
        .param("region_id", region_id.to_string())
        .param("limit", limit as i64);

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        let mut stagings = Vec::new();
        let now = self.clock.now();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let staging = row_to_staging_with_npcs(row, now)?;
            stagings.push(staging);
        }

        Ok(stagings)
    }

    // =========================================================================
    // Mood Operations (Tier 2 of three-tier emotional model)
    // =========================================================================

    /// Get an NPC's current mood in a region's active staging.
    /// Returns the NPC's default_mood if not staged or no mood override set.
    async fn get_npc_mood(
        &self,
        region_id: RegionId,
        npc_id: CharacterId,
    ) -> Result<MoodState, RepoError> {
        let q = query(
            "MATCH (r:Region {id: $region_id})-[:CURRENT_STAGING]->(s:Staging)-[rel:INCLUDES_NPC]->(c:Character {id: $npc_id})
            WHERE s.is_active = true
            RETURN COALESCE(rel.mood, c.default_mood, 'calm') as mood",
        )
        .param("region_id", region_id.to_string())
        .param("npc_id", npc_id.to_string());

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
            let mood_str: String = row.get("mood").map_err(|e| {
                RepoError::database(
                    "query",
                    format!(
                        "Missing mood for NPC {} in region {}: {}",
                        npc_id, region_id, e
                    ),
                )
            })?;
            let mood: MoodState = mood_str.parse().map_err(|e| {
                RepoError::database(
                    "parse",
                    format!(
                        "Invalid MoodState for NPC {} in region {}: '{}': {}",
                        npc_id, region_id, mood_str, e
                    ),
                )
            })?;
            Ok(mood)
        } else {
            // NPC not staged in this region, try to get their default mood
            let default_q = query(
                "MATCH (c:Character {id: $npc_id})
                RETURN COALESCE(c.default_mood, 'calm') as mood",
            )
            .param("npc_id", npc_id.to_string());

            let mut default_result = self
                .graph
                .execute(default_q)
                .await
                .map_err(|e| RepoError::database("query", e))?;

            if let Some(row) = default_result
                .next()
                .await
                .map_err(|e| RepoError::database("query", e))?
            {
                let mood_str: String = row.get("mood").map_err(|e| {
                    RepoError::database(
                        "query",
                        format!("Missing default_mood for NPC {}: {}", npc_id, e),
                    )
                })?;
                let mood: MoodState = mood_str.parse().map_err(|e| {
                    RepoError::database(
                        "parse",
                        format!(
                            "Invalid MoodState for NPC {}: '{}': {}",
                            npc_id, mood_str, e
                        ),
                    )
                })?;
                Ok(mood)
            } else {
                tracing::warn!(
                    region_id = %region_id,
                    npc_id = %npc_id,
                    "NPC mood not found in active staging"
                );
                Err(RepoError::not_found(
                    "NpcMood",
                    format!("region:{}/npc:{}", region_id, npc_id),
                ))
            }
        }
    }

    /// Set an NPC's mood in a region's active staging.
    /// Creates or updates the mood property on the INCLUDES_NPC edge.
    async fn set_npc_mood(
        &self,
        region_id: RegionId,
        npc_id: CharacterId,
        mood: MoodState,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (r:Region {id: $region_id})-[:CURRENT_STAGING]->(s:Staging)-[rel:INCLUDES_NPC]->(c:Character {id: $npc_id})
            WHERE s.is_active = true
            SET rel.mood = $mood
            RETURN rel",
        )
        .param("region_id", region_id.to_string())
        .param("npc_id", npc_id.to_string())
        .param("mood", mood.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
            .is_none()
        {
            // NPC is not staged in this region
            tracing::warn!(
                region_id = %region_id,
                npc_id = %npc_id,
                mood = %mood,
                "Cannot set NPC mood: NPC not staged in region"
            );
            return Err(RepoError::not_found(
                "StagedNpc",
                format!("region:{}/npc:{}", region_id, npc_id),
            ));
        }

        Ok(())
    }
}

impl Neo4jStagingRepo {
    /// Load NPCs for a specific staging
    #[allow(dead_code)]
    async fn load_staging_npcs(&self, staging_id: StagingId) -> Result<Vec<StagedNpc>, RepoError> {
        let q = query(
            "MATCH (s:Staging {id: $staging_id})-[rel:INCLUDES_NPC]->(c:Character)
            RETURN c.id as character_id,
                   c.name as name,
                   c.sprite_asset as sprite_asset,
                   c.portrait_asset as portrait_asset,
                   rel.is_present as is_present,
                   COALESCE(rel.is_hidden_from_players, false) as is_hidden_from_players,
                   rel.reasoning as reasoning,
                   COALESCE(rel.mood, c.default_mood, 'calm') as mood,
                   COALESCE(rel.has_incomplete_data, false) as has_incomplete_data",
        )
        .param("staging_id", staging_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        let mut npcs = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            npcs.push(row_to_staged_npc(row)?);
        }

        Ok(npcs)
    }
}

// =============================================================================
// Row conversion helpers
// =============================================================================

fn row_to_staged_npc(row: Row) -> Result<StagedNpc, RepoError> {
    let character_id_str: String = row
        .get("character_id")
        .map_err(|e| RepoError::database("query", format!("Failed to get character_id: {}", e)))?;
    let name: String = row.get("name").map_err(|e| {
        RepoError::database(
            "query",
            format!(
                "Failed to get 'name' for character_id {}: {}",
                character_id_str, e
            ),
        )
    })?;
    let is_present: bool = row.get("is_present").map_err(|e| {
        RepoError::database(
            "query",
            format!(
                "Failed to get 'is_present' for character_id {}: {}",
                character_id_str, e
            ),
        )
    })?;
    let is_hidden_from_players: bool = row.get("is_hidden_from_players").unwrap_or(false);
    let reasoning: String = row.get("reasoning").map_err(|e| {
        RepoError::database(
            "query",
            format!(
                "Failed to get 'reasoning' for character_id {}: {}",
                character_id_str, e
            ),
        )
    })?;

    let character_id = uuid::Uuid::parse_str(&character_id_str)
        .map(CharacterId::from)
        .map_err(|e| {
            RepoError::database(
                "query",
                format!(
                    "Failed to parse CharacterId for character_id {}: {}",
                    character_id_str, e
                ),
            )
        })?;

    // Optional string fields
    let sprite_asset: Option<String> = row
        .get("sprite_asset")
        .ok()
        .filter(|s: &String| !s.is_empty());
    let portrait_asset: Option<String> = row
        .get("portrait_asset")
        .ok()
        .filter(|s: &String| !s.is_empty());

    // Parse mood - fail-fast on invalid values
    let mood_str: String = row.get("mood").map_err(|e| {
        RepoError::database(
            "query",
            format!("Missing mood for staged NPC {}: {}", character_id_str, e),
        )
    })?;
    let mood: MoodState = mood_str.parse().map_err(|e| {
        RepoError::database(
            "parse",
            format!(
                "Invalid MoodState for staged NPC {}: '{}': {}",
                character_id_str, mood_str, e
            ),
        )
    })?;

    // Parse has_incomplete_data flag - defaults to false for existing data
    let has_incomplete_data: bool = row.get("has_incomplete_data").unwrap_or(false);

    let presence = if is_hidden_from_players {
        NpcPresence::Hidden
    } else if is_present {
        NpcPresence::Visible
    } else {
        NpcPresence::Absent
    };

    let mut npc = StagedNpc::new(character_id, name, is_present, reasoning).with_presence(presence);
    npc.mood = mood;
    npc.has_incomplete_data = has_incomplete_data;
    if let Some(sprite_str) = sprite_asset {
        let sprite = AssetPath::new(sprite_str).map_err(|e| RepoError::database("parse", e))?;
        npc.sprite_asset = Some(sprite);
    }
    if let Some(portrait_str) = portrait_asset {
        let portrait = AssetPath::new(portrait_str).map_err(|e| RepoError::database("parse", e))?;
        npc.portrait_asset = Some(portrait);
    }
    Ok(npc)
}

/// Parse a staging row that includes collected NPCs
fn row_to_staging_with_npcs(row: Row, fallback: DateTime<Utc>) -> Result<Staging, RepoError> {
    let node: Node = row
        .get("s")
        .map_err(|e| RepoError::database("query", format!("Failed to get 's' node: {}", e)))?;

    let id: StagingId = parse_typed_id(&node, "id")
        .map_err(|e| RepoError::database("query", format!("Failed to parse StagingId: {}", e)))?;
    let region_id: RegionId = parse_typed_id(&node, "region_id").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to parse RegionId for Staging {}: {}", id, e),
        )
    })?;
    let location_id: LocationId = parse_typed_id(&node, "location_id").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to parse LocationId for Staging {}: {}", id, e),
        )
    })?;
    let world_id: WorldId = parse_typed_id(&node, "world_id").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to parse WorldId for Staging {}: {}", id, e),
        )
    })?;

    let ttl_hours: i64 = node.get("ttl_hours").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'ttl_hours' for Staging {}: {}", id, e),
        )
    })?;
    let approved_by: String = node.get("approved_by").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'approved_by' for Staging {}: {}", id, e),
        )
    })?;
    let source_str: String = node.get("source").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'source' for Staging {}: {}", id, e),
        )
    })?;
    let is_active: bool = node.get("is_active").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'is_active' for Staging {}: {}", id, e),
        )
    })?;

    // Load game time as minutes (new format) or default to 0 for backwards compatibility
    let game_time_minutes = node.get_i64_or("game_time_minutes", 0);
    let approved_at = node.get_datetime_or("approved_at", fallback);
    let source: StagingSource = source_str.parse().map_err(|e| {
        RepoError::database(
            "parse",
            format!(
                "Invalid StagingSource for staging {}: '{}': {}",
                id, source_str, e
            ),
        )
    })?;
    let dm_guidance = node.get_optional_string("dm_guidance");

    // Parse collected NPCs from the row
    let npcs = parse_collected_npcs(&row)?;

    Ok(Staging::from_stored(
        id,
        region_id,
        location_id,
        world_id,
        npcs,
        game_time_minutes,
        approved_at,
        ttl_hours as i32,
        approved_by,
        source,
        dm_guidance,
        is_active,
        None, // location_state_id
        None, // region_state_id
        VisualStateSource::default(),
        None, // visual_state_reasoning
    ))
}

/// Parse NPCs from a COLLECT result
fn parse_collected_npcs(row: &Row) -> Result<Vec<StagedNpc>, RepoError> {
    // COLLECT returns a list of maps
    let npcs_data: Vec<neo4rs::BoltMap> = row
        .get("npcs")
        .map_err(|e| RepoError::database("query", format!("Failed to get npcs: {}", e)))?;

    let mut npcs = Vec::with_capacity(npcs_data.len());
    for npc_map in npcs_data {
        // Skip null entries (from OPTIONAL MATCH with no NPCs)
        let character_id_str: Option<String> = npc_map.get("character_id").ok();
        let character_id_str = match character_id_str {
            Some(id) => id,
            None => continue, // Skip null NPC entries
        };

        let character_id = uuid::Uuid::parse_str(&character_id_str)
            .map(CharacterId::from)
            .map_err(|e| RepoError::database("query", format!("Invalid character_id: {}", e)))?;

        let name: String = npc_map.get("name").map_err(|e| {
            RepoError::database(
                "query",
                format!(
                    "Missing required NPC name for character_id {}: {}",
                    character_id_str, e
                ),
            )
        })?;
        let sprite_asset: Option<String> = npc_map
            .get("sprite_asset")
            .ok()
            .filter(|s: &String| !s.is_empty());
        let portrait_asset: Option<String> = npc_map
            .get("portrait_asset")
            .ok()
            .filter(|s: &String| !s.is_empty());
        let is_present: bool = npc_map.get("is_present").unwrap_or(true);
        let is_hidden_from_players: bool = npc_map.get("is_hidden_from_players").unwrap_or(false);
        let reasoning: String = npc_map.get("reasoning").unwrap_or_default();
        let mood_str: String = npc_map.get("mood").map_err(|e| {
            RepoError::database(
                "query",
                format!("Missing mood for collected NPC {}: {}", character_id_str, e),
            )
        })?;
        let mood: MoodState = mood_str.parse().map_err(|e| {
            RepoError::database(
                "parse",
                format!(
                    "Invalid MoodState for collected NPC {}: '{}': {}",
                    character_id_str, mood_str, e
                ),
            )
        })?;
        let has_incomplete_data: bool = npc_map.get("has_incomplete_data").unwrap_or(false);

        let presence = if is_hidden_from_players {
            NpcPresence::Hidden
        } else if is_present {
            NpcPresence::Visible
        } else {
            NpcPresence::Absent
        };

        let mut npc =
            StagedNpc::new(character_id, name, is_present, reasoning).with_presence(presence);
        npc.mood = mood;
        npc.has_incomplete_data = has_incomplete_data;
        if let Some(sprite_str) = sprite_asset {
            let sprite = AssetPath::new(sprite_str).map_err(|e| RepoError::database("parse", e))?;
            npc.sprite_asset = Some(sprite);
        }
        if let Some(portrait_str) = portrait_asset {
            let portrait =
                AssetPath::new(portrait_str).map_err(|e| RepoError::database("parse", e))?;
            npc.portrait_asset = Some(portrait);
        }
        npcs.push(npc);
    }

    Ok(npcs)
}

#[allow(dead_code)]
fn row_to_staging(row: Row, fallback: DateTime<Utc>) -> Result<Staging, RepoError> {
    let node: Node = row
        .get("s")
        .map_err(|e| RepoError::database("query", format!("Failed to get 's' node: {}", e)))?;

    let id: StagingId = parse_typed_id(&node, "id")
        .map_err(|e| RepoError::database("query", format!("Failed to parse StagingId: {}", e)))?;
    let region_id: RegionId = parse_typed_id(&node, "region_id").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to parse RegionId for Staging {}: {}", id, e),
        )
    })?;
    let location_id: LocationId = parse_typed_id(&node, "location_id").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to parse LocationId for Staging {}: {}", id, e),
        )
    })?;
    let world_id: WorldId = parse_typed_id(&node, "world_id").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to parse WorldId for Staging {}: {}", id, e),
        )
    })?;

    let ttl_hours: i64 = node.get("ttl_hours").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'ttl_hours' for Staging {}: {}", id, e),
        )
    })?;
    let approved_by: String = node.get("approved_by").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'approved_by' for Staging {}: {}", id, e),
        )
    })?;
    let source_str: String = node.get("source").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'source' for Staging {}: {}", id, e),
        )
    })?;
    let is_active: bool = node.get("is_active").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'is_active' for Staging {}: {}", id, e),
        )
    })?;

    // Load game time as minutes (new format) or default to 0 for backwards compatibility
    let game_time_minutes = node.get_i64_or("game_time_minutes", 0);
    let approved_at = node.get_datetime_or("approved_at", fallback);
    let source: StagingSource = source_str.parse().map_err(|e| {
        RepoError::database(
            "parse",
            format!(
                "Invalid StagingSource for staging {}: '{}': {}",
                id, source_str, e
            ),
        )
    })?;
    let dm_guidance = node.get_optional_string("dm_guidance");

    Ok(Staging::from_stored(
        id,
        region_id,
        location_id,
        world_id,
        Vec::new(), // NPCs loaded separately
        game_time_minutes,
        approved_at,
        ttl_hours as i32,
        approved_by,
        source,
        dm_guidance,
        is_active,
        None, // location_state_id - will be loaded from edges in future
        None, // region_state_id
        VisualStateSource::default(),
        None, // visual_state_reasoning
    ))
}
