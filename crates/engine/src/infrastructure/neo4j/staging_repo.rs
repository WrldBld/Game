//! Neo4j staging repository implementation.
//!
//! Handles NPC staging for regions and pending staging approval.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use neo4rs::{query, Graph, Node, Row};

use wrldbldr_domain::MoodState;
use wrldbldr_domain::*;

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{ClockPort, RepoError, StagingRepo};

pub struct Neo4jStagingRepo {
    graph: Graph,
    clock: std::sync::Arc<dyn ClockPort>,
}

impl Neo4jStagingRepo {
    pub fn new(graph: Graph, clock: std::sync::Arc<dyn ClockPort>) -> Self {
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
                    game_time: $game_time,
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
            .param("game_time", now.to_rfc3339())
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

    /// Save a pending staging for DM approval.
    /// Creates the staging node first, then adds NPC relationships separately (no APOC dependency).
    async fn save_pending_staging(&self, staging: &Staging) -> Result<(), RepoError> {
        let npc_character_ids: Vec<String> = staging
            .npcs
            .iter()
            .map(|n| n.character_id.to_string())
            .collect();
        let npc_is_present: Vec<bool> = staging.npcs.iter().map(|n| n.is_present).collect();
        let npc_is_hidden_from_players: Vec<bool> = staging
            .npcs
            .iter()
            .map(|n| n.is_hidden_from_players)
            .collect();
        let npc_reasoning: Vec<String> = staging.npcs.iter().map(|n| n.reasoning.clone()).collect();
        let npc_mood: Vec<String> = staging.npcs.iter().map(|n| n.mood.to_string()).collect();
        let npc_has_incomplete_data: Vec<bool> = staging
            .npcs
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
                game_time: $game_time,
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
        .param("is_active", staging.is_active)
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
        current_game_time: DateTime<Utc>,
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
            let staging = row_to_staging_with_npcs(row, current_game_time)?;

            // Check if staging is expired
            if staging.is_expired(&current_game_time) {
                return Ok(None);
            }

            Ok(Some(staging))
        } else {
            Ok(None)
        }
    }

    /// Activate a staging, replacing any existing current staging for the region.
    async fn activate_staging(
        &self,
        staging_id: StagingId,
        region_id: RegionId,
    ) -> Result<(), RepoError> {
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

        self.graph
            .run(q)
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
            let mood_str: String = row.get("mood").unwrap_or_else(|_| "calm".to_string());
            Ok(mood_str.parse().unwrap_or(MoodState::Calm))
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
                let mood_str: String = row.get("mood").unwrap_or_else(|_| "calm".to_string());
                Ok(mood_str.parse().unwrap_or(MoodState::Calm))
            } else {
                Err(RepoError::not_found("Entity", "unknown"))
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
            return Err(RepoError::not_found("Entity", "unknown"));
        }

        Ok(())
    }
}

impl Neo4jStagingRepo {
    /// Load NPCs for a specific staging
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
        .map_err(|e| RepoError::database("query", e))?;
    let name: String = row
        .get("name")
        .map_err(|e| RepoError::database("query", e))?;
    let is_present: bool = row
        .get("is_present")
        .map_err(|e| RepoError::database("query", e))?;
    let is_hidden_from_players: bool = row.get("is_hidden_from_players").unwrap_or(false);
    let reasoning: String = row
        .get("reasoning")
        .map_err(|e| RepoError::database("query", e))?;

    let character_id = uuid::Uuid::parse_str(&character_id_str)
        .map(CharacterId::from)
        .map_err(|e| RepoError::database("query", format!("Invalid character_id: {}", e)))?;

    // Optional string fields
    let sprite_asset: Option<String> = row
        .get("sprite_asset")
        .ok()
        .filter(|s: &String| !s.is_empty());
    let portrait_asset: Option<String> = row
        .get("portrait_asset")
        .ok()
        .filter(|s: &String| !s.is_empty());

    // Parse mood - defaults to Calm if not present or invalid
    let mood_str: String = row.get("mood").unwrap_or_else(|_| "calm".to_string());
    let mood: MoodState = mood_str.parse().unwrap_or(MoodState::Calm);

    // Parse has_incomplete_data flag - defaults to false for existing data
    let has_incomplete_data: bool = row.get("has_incomplete_data").unwrap_or(false);

    Ok(StagedNpc {
        character_id,
        name,
        sprite_asset,
        portrait_asset,
        is_present,
        is_hidden_from_players,
        reasoning,
        mood,
        has_incomplete_data,
    })
}

/// Parse a staging row that includes collected NPCs
fn row_to_staging_with_npcs(row: Row, fallback: DateTime<Utc>) -> Result<Staging, RepoError> {
    let node: Node = row
        .get("s")
        .map_err(|e| RepoError::database("query", e))?;

    let id: StagingId =
        parse_typed_id(&node, "id").map_err(|e| RepoError::database("query", e))?;
    let region_id: RegionId =
        parse_typed_id(&node, "region_id").map_err(|e| RepoError::database("query", e))?;
    let location_id: LocationId =
        parse_typed_id(&node, "location_id").map_err(|e| RepoError::database("query", e))?;
    let world_id: WorldId =
        parse_typed_id(&node, "world_id").map_err(|e| RepoError::database("query", e))?;

    let ttl_hours: i64 = node
        .get("ttl_hours")
        .map_err(|e| RepoError::database("query", e))?;
    let approved_by: String = node
        .get("approved_by")
        .map_err(|e| RepoError::database("query", e))?;
    let source_str: String = node
        .get("source")
        .map_err(|e| RepoError::database("query", e))?;
    let is_active: bool = node
        .get("is_active")
        .map_err(|e| RepoError::database("query", e))?;

    let game_time = node.get_datetime_or("game_time", fallback);
    let approved_at = node.get_datetime_or("approved_at", fallback);
    let source = source_str.parse().unwrap_or(StagingSource::RuleBased);
    let dm_guidance = node.get_optional_string("dm_guidance");

    // Parse collected NPCs from the row
    let npcs = parse_collected_npcs(&row)?;

    Ok(Staging {
        id,
        region_id,
        location_id,
        world_id,
        npcs,
        game_time,
        approved_at,
        ttl_hours: ttl_hours as i32,
        approved_by,
        source,
        dm_guidance,
        is_active,
        location_state_id: None,
        region_state_id: None,
        visual_state_source: VisualStateSource::default(),
        visual_state_reasoning: None,
    })
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

        let name: String = npc_map.get("name").unwrap_or_default();
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
        let mood_str: String = npc_map.get("mood").unwrap_or_else(|_| "calm".to_string());
        let mood: MoodState = mood_str.parse().unwrap_or(MoodState::Calm);
        let has_incomplete_data: bool = npc_map.get("has_incomplete_data").unwrap_or(false);

        npcs.push(StagedNpc {
            character_id,
            name,
            sprite_asset,
            portrait_asset,
            is_present,
            is_hidden_from_players,
            reasoning,
            mood,
            has_incomplete_data,
        });
    }

    Ok(npcs)
}

fn row_to_staging(row: Row, fallback: DateTime<Utc>) -> Result<Staging, RepoError> {
    let node: Node = row
        .get("s")
        .map_err(|e| RepoError::database("query", e))?;

    let id: StagingId =
        parse_typed_id(&node, "id").map_err(|e| RepoError::database("query", e))?;
    let region_id: RegionId =
        parse_typed_id(&node, "region_id").map_err(|e| RepoError::database("query", e))?;
    let location_id: LocationId =
        parse_typed_id(&node, "location_id").map_err(|e| RepoError::database("query", e))?;
    let world_id: WorldId =
        parse_typed_id(&node, "world_id").map_err(|e| RepoError::database("query", e))?;

    let ttl_hours: i64 = node
        .get("ttl_hours")
        .map_err(|e| RepoError::database("query", e))?;
    let approved_by: String = node
        .get("approved_by")
        .map_err(|e| RepoError::database("query", e))?;
    let source_str: String = node
        .get("source")
        .map_err(|e| RepoError::database("query", e))?;
    let is_active: bool = node
        .get("is_active")
        .map_err(|e| RepoError::database("query", e))?;

    let game_time = node.get_datetime_or("game_time", fallback);
    let approved_at = node.get_datetime_or("approved_at", fallback);
    let source = source_str.parse().unwrap_or(StagingSource::RuleBased);
    let dm_guidance = node.get_optional_string("dm_guidance");

    Ok(Staging {
        id,
        region_id,
        location_id,
        world_id,
        npcs: Vec::new(), // Loaded separately
        game_time,
        approved_at,
        ttl_hours: ttl_hours as i32,
        approved_by,
        source,
        dm_guidance,
        is_active,
        // Visual state fields - will be loaded from edges in future
        location_state_id: None,
        region_state_id: None,
        visual_state_source: VisualStateSource::default(),
        visual_state_reasoning: None,
    })
}
