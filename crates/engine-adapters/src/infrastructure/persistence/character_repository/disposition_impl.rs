//! CharacterDispositionPort implementation

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;
use wrldbldr_common::datetime::parse_datetime_or;
use wrldbldr_domain::value_objects::{DispositionLevel, NpcDispositionState, RelationshipLevel};
use wrldbldr_domain::{CharacterId, PlayerCharacterId};
use wrldbldr_engine_ports::outbound::CharacterDispositionPort;

use super::Neo4jCharacterRepository;

impl Neo4jCharacterRepository {
    /// Get an NPC's disposition state toward a specific PC
    pub(crate) async fn get_disposition_toward_pc_impl(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<Option<NpcDispositionState>> {
        let q = query(
            "MATCH (npc:Character {id: $npc_id})-[r:DISPOSITION_TOWARD]->(pc:PlayerCharacter {id: $pc_id})
            RETURN r.disposition as disposition, r.relationship as relationship, r.sentiment as sentiment,
                   r.updated_at as updated_at, r.disposition_reason as disposition_reason, r.relationship_points as relationship_points",
        )
        .param("npc_id", npc_id.to_string())
        .param("pc_id", pc_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let disposition_str: String = row
                .get("disposition")
                .unwrap_or_else(|_| "Neutral".to_string());
            let relationship_str: String = row
                .get("relationship")
                .unwrap_or_else(|_| "Stranger".to_string());
            let sentiment: f64 = row.get("sentiment").unwrap_or(0.0);
            let updated_at_str: String = row.get("updated_at").unwrap_or_default();
            let disposition_reason: Option<String> = row.get("disposition_reason").ok();
            let relationship_points: i64 = row.get("relationship_points").unwrap_or(0);

            let updated_at = parse_datetime_or(&updated_at_str, self.clock.now());

            Ok(Some(NpcDispositionState {
                npc_id,
                pc_id,
                disposition: disposition_str.parse().unwrap_or(DispositionLevel::Neutral),
                relationship: relationship_str
                    .parse()
                    .unwrap_or(RelationshipLevel::Stranger),
                sentiment: sentiment as f32,
                updated_at,
                disposition_reason,
                relationship_points: relationship_points as i32,
            }))
        } else {
            Ok(None)
        }
    }

    /// Set/update an NPC's disposition state toward a specific PC
    pub(crate) async fn set_disposition_toward_pc_impl(
        &self,
        disposition_state: &NpcDispositionState,
    ) -> Result<()> {
        let q = query(
            "MATCH (npc:Character {id: $npc_id}), (pc:PlayerCharacter {id: $pc_id})
            MERGE (npc)-[r:DISPOSITION_TOWARD]->(pc)
            SET r.disposition = $disposition,
                r.relationship = $relationship,
                r.sentiment = $sentiment,
                r.updated_at = $updated_at,
                r.disposition_reason = $disposition_reason,
                r.relationship_points = $relationship_points
            RETURN npc.id as id",
        )
        .param("npc_id", disposition_state.npc_id.to_string())
        .param("pc_id", disposition_state.pc_id.to_string())
        .param("disposition", disposition_state.disposition.to_string())
        .param("relationship", disposition_state.relationship.to_string())
        .param("sentiment", disposition_state.sentiment as f64)
        .param("updated_at", disposition_state.updated_at.to_rfc3339())
        .param(
            "disposition_reason",
            disposition_state
                .disposition_reason
                .clone()
                .unwrap_or_default(),
        )
        .param(
            "relationship_points",
            disposition_state.relationship_points as i64,
        );

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Set disposition for NPC {} toward PC {}: {:?}",
            disposition_state.npc_id,
            disposition_state.pc_id,
            disposition_state.disposition
        );
        Ok(())
    }

    /// Get disposition states for multiple NPCs toward a PC (for scene context)
    pub(crate) async fn get_scene_dispositions_impl(
        &self,
        npc_ids: &[CharacterId],
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>> {
        if npc_ids.is_empty() {
            return Ok(vec![]);
        }

        let npc_id_strings: Vec<String> = npc_ids.iter().map(|id| id.to_string()).collect();

        let q = query(
            "MATCH (npc:Character)-[r:DISPOSITION_TOWARD]->(pc:PlayerCharacter {id: $pc_id})
            WHERE npc.id IN $npc_ids
            RETURN npc.id as npc_id, r.disposition as disposition, r.relationship as relationship,
                   r.sentiment as sentiment, r.updated_at as updated_at,
                   r.disposition_reason as disposition_reason, r.relationship_points as relationship_points",
        )
        .param("pc_id", pc_id.to_string())
        .param("npc_ids", npc_id_strings);

        let mut result = self.connection.graph().execute(q).await?;
        let mut dispositions = Vec::new();

        while let Some(row) = result.next().await? {
            let npc_id_str: String = row.get("npc_id")?;
            let disposition_str: String = row
                .get("disposition")
                .unwrap_or_else(|_| "Neutral".to_string());
            let relationship_str: String = row
                .get("relationship")
                .unwrap_or_else(|_| "Stranger".to_string());
            let sentiment: f64 = row.get("sentiment").unwrap_or(0.0);
            let updated_at_str: String = row.get("updated_at").unwrap_or_default();
            let disposition_reason: Option<String> = row.get("disposition_reason").ok();
            let relationship_points: i64 = row.get("relationship_points").unwrap_or(0);

            let npc_uuid = uuid::Uuid::parse_str(&npc_id_str)?;
            let updated_at = parse_datetime_or(&updated_at_str, self.clock.now());

            dispositions.push(NpcDispositionState {
                npc_id: CharacterId::from_uuid(npc_uuid),
                pc_id,
                disposition: disposition_str.parse().unwrap_or(DispositionLevel::Neutral),
                relationship: relationship_str
                    .parse()
                    .unwrap_or(RelationshipLevel::Stranger),
                sentiment: sentiment as f32,
                updated_at,
                disposition_reason,
                relationship_points: relationship_points as i32,
            });
        }

        Ok(dispositions)
    }

    /// Get all NPCs who have a relationship with a PC (for DM panel)
    pub(crate) async fn get_all_npc_dispositions_for_pc_impl(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>> {
        let q = query(
            "MATCH (npc:Character)-[r:DISPOSITION_TOWARD]->(pc:PlayerCharacter {id: $pc_id})
            RETURN npc.id as npc_id, r.disposition as disposition, r.relationship as relationship,
                   r.sentiment as sentiment, r.updated_at as updated_at,
                   r.disposition_reason as disposition_reason, r.relationship_points as relationship_points
            ORDER BY r.updated_at DESC",
        )
        .param("pc_id", pc_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut dispositions = Vec::new();

        while let Some(row) = result.next().await? {
            let npc_id_str: String = row.get("npc_id")?;
            let disposition_str: String = row
                .get("disposition")
                .unwrap_or_else(|_| "Neutral".to_string());
            let relationship_str: String = row
                .get("relationship")
                .unwrap_or_else(|_| "Stranger".to_string());
            let sentiment: f64 = row.get("sentiment").unwrap_or(0.0);
            let updated_at_str: String = row.get("updated_at").unwrap_or_default();
            let disposition_reason: Option<String> = row.get("disposition_reason").ok();
            let relationship_points: i64 = row.get("relationship_points").unwrap_or(0);

            let npc_uuid = uuid::Uuid::parse_str(&npc_id_str)?;
            let updated_at = parse_datetime_or(&updated_at_str, self.clock.now());

            dispositions.push(NpcDispositionState {
                npc_id: CharacterId::from_uuid(npc_uuid),
                pc_id,
                disposition: disposition_str.parse().unwrap_or(DispositionLevel::Neutral),
                relationship: relationship_str
                    .parse()
                    .unwrap_or(RelationshipLevel::Stranger),
                sentiment: sentiment as f32,
                updated_at,
                disposition_reason,
                relationship_points: relationship_points as i32,
            });
        }

        Ok(dispositions)
    }

    /// Get the NPC's default/global disposition (from Character node)
    pub(crate) async fn get_default_disposition_impl(
        &self,
        npc_id: CharacterId,
    ) -> Result<DispositionLevel> {
        let q = query(
            "MATCH (c:Character {id: $id})
            RETURN c.default_disposition as default_disposition",
        )
        .param("id", npc_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let disposition_str: String = row
                .get("default_disposition")
                .unwrap_or_else(|_| "Neutral".to_string());
            Ok(disposition_str.parse().unwrap_or(DispositionLevel::Neutral))
        } else {
            Ok(DispositionLevel::Neutral)
        }
    }

    /// Set the NPC's default/global disposition (on Character node)
    pub(crate) async fn set_default_disposition_impl(
        &self,
        npc_id: CharacterId,
        disposition: DispositionLevel,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})
            SET c.default_disposition = $disposition
            RETURN c.id as id",
        )
        .param("id", npc_id.to_string())
        .param("disposition", disposition.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Set default disposition for NPC {}: {:?}",
            npc_id,
            disposition
        );
        Ok(())
    }
}

#[async_trait]
impl CharacterDispositionPort for Neo4jCharacterRepository {
    async fn get_disposition_toward_pc(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<Option<NpcDispositionState>> {
        self.get_disposition_toward_pc_impl(npc_id, pc_id).await
    }

    async fn set_disposition_toward_pc(
        &self,
        disposition_state: &NpcDispositionState,
    ) -> Result<()> {
        self.set_disposition_toward_pc_impl(disposition_state).await
    }

    async fn get_scene_dispositions(
        &self,
        npc_ids: &[CharacterId],
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>> {
        self.get_scene_dispositions_impl(npc_ids, pc_id).await
    }

    async fn get_all_npc_dispositions_for_pc(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>> {
        self.get_all_npc_dispositions_for_pc_impl(pc_id).await
    }

    async fn get_default_disposition(&self, npc_id: CharacterId) -> Result<DispositionLevel> {
        self.get_default_disposition_impl(npc_id).await
    }

    async fn set_default_disposition(
        &self,
        npc_id: CharacterId,
        disposition: DispositionLevel,
    ) -> Result<()> {
        self.set_default_disposition_impl(npc_id, disposition).await
    }
}
