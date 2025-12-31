//! CharacterActantialPort implementation

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};
use wrldbldr_common::datetime::parse_datetime_or;
use wrldbldr_domain::entities::{ActantialRole, ActantialView};
use wrldbldr_domain::value_objects::ActantialTarget;
use wrldbldr_domain::{CharacterId, PlayerCharacterId, WantId};
use wrldbldr_engine_ports::outbound::CharacterActantialPort;

use super::Neo4jCharacterRepository;

impl Neo4jCharacterRepository {
    /// Add an actantial view
    pub(crate) async fn add_actantial_view_impl(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        view: &ActantialView,
    ) -> Result<()> {
        let edge_type = match role {
            ActantialRole::Helper => "VIEWS_AS_HELPER",
            ActantialRole::Opponent => "VIEWS_AS_OPPONENT",
            ActantialRole::Sender => "VIEWS_AS_SENDER",
            ActantialRole::Receiver => "VIEWS_AS_RECEIVER",
        };

        let cypher = format!(
            "MATCH (s:Character {{id: $subject_id}}), (t:Character {{id: $target_id}})
            CREATE (s)-[:{} {{
                want_id: $want_id,
                reason: $reason,
                assigned_at: $assigned_at
            }}]->(t)",
            edge_type
        );

        let q = query(&cypher)
            .param("subject_id", subject_id.to_string())
            .param("target_id", target_id.to_string())
            .param("want_id", view.want_id.to_string())
            .param("reason", view.reason.clone())
            .param("assigned_at", view.assigned_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get all actantial views for a character (toward both NPCs and PCs)
    pub(crate) async fn get_actantial_views_impl(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<(ActantialRole, ActantialTarget, ActantialView)>> {
        // Query views toward NPCs
        let q_npc = query(
            "MATCH (s:Character {id: $id})-[r]->(t:Character)
            WHERE type(r) IN ['VIEWS_AS_HELPER', 'VIEWS_AS_OPPONENT', 'VIEWS_AS_SENDER', 'VIEWS_AS_RECEIVER']
            RETURN type(r) as role_type, t.id as target_id, 'NPC' as target_type,
                   r.want_id as want_id, r.reason as reason, r.assigned_at as assigned_at",
        )
        .param("id", character_id.to_string());

        // Query views toward PCs
        let q_pc = query(
            "MATCH (s:Character {id: $id})-[r]->(t:PlayerCharacter)
            WHERE type(r) IN ['VIEWS_AS_HELPER', 'VIEWS_AS_OPPONENT', 'VIEWS_AS_SENDER', 'VIEWS_AS_RECEIVER']
            RETURN type(r) as role_type, t.id as target_id, 'PC' as target_type,
                   r.want_id as want_id, r.reason as reason, r.assigned_at as assigned_at",
        )
        .param("id", character_id.to_string());

        let mut views = Vec::new();

        // Process NPC views
        let mut result = self.connection.graph().execute(q_npc).await?;
        while let Some(row) = result.next().await? {
            if let Some(view) = self.parse_actantial_view_row(&row)? {
                views.push(view);
            }
        }

        // Process PC views
        let mut result = self.connection.graph().execute(q_pc).await?;
        while let Some(row) = result.next().await? {
            if let Some(view) = self.parse_actantial_view_row(&row)? {
                views.push(view);
            }
        }

        Ok(views)
    }

    /// Helper to parse actantial view row
    pub(crate) fn parse_actantial_view_row(
        &self,
        row: &Row,
    ) -> Result<Option<(ActantialRole, ActantialTarget, ActantialView)>> {
        let role_type: String = row.get("role_type")?;
        let target_id_str: String = row.get("target_id")?;
        let target_type: String = row.get("target_type")?;
        let want_id_str: String = row.get("want_id")?;
        let reason: String = row.get("reason")?;
        let assigned_at_str: String = row.get("assigned_at")?;

        let role = match role_type.as_str() {
            "VIEWS_AS_HELPER" => ActantialRole::Helper,
            "VIEWS_AS_OPPONENT" => ActantialRole::Opponent,
            "VIEWS_AS_SENDER" => ActantialRole::Sender,
            "VIEWS_AS_RECEIVER" => ActantialRole::Receiver,
            _ => return Ok(None),
        };

        let target_uuid = uuid::Uuid::parse_str(&target_id_str)?;
        let target = match target_type.as_str() {
            "NPC" => ActantialTarget::Npc(target_uuid),
            "PC" => ActantialTarget::Pc(target_uuid),
            _ => return Ok(None),
        };

        let want_id = WantId::from_uuid(uuid::Uuid::parse_str(&want_id_str)?);
        let assigned_at = parse_datetime_or(&assigned_at_str, self.clock.now());

        Ok(Some((
            role,
            target,
            ActantialView {
                want_id,
                reason,
                assigned_at,
            },
        )))
    }

    /// Remove an actantial view
    pub(crate) async fn remove_actantial_view_impl(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        want_id: WantId,
    ) -> Result<()> {
        let edge_type = match role {
            ActantialRole::Helper => "VIEWS_AS_HELPER",
            ActantialRole::Opponent => "VIEWS_AS_OPPONENT",
            ActantialRole::Sender => "VIEWS_AS_SENDER",
            ActantialRole::Receiver => "VIEWS_AS_RECEIVER",
        };

        let cypher = format!(
            "MATCH (s:Character {{id: $subject_id}})-[r:{} {{want_id: $want_id}}]->(t:Character {{id: $target_id}})
            DELETE r",
            edge_type
        );

        let q = query(&cypher)
            .param("subject_id", subject_id.to_string())
            .param("target_id", target_id.to_string())
            .param("want_id", want_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add an actantial view toward a PC
    pub(crate) async fn add_actantial_view_to_pc_impl(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: PlayerCharacterId,
        view: &ActantialView,
    ) -> Result<()> {
        let edge_type = match role {
            ActantialRole::Helper => "VIEWS_AS_HELPER",
            ActantialRole::Opponent => "VIEWS_AS_OPPONENT",
            ActantialRole::Sender => "VIEWS_AS_SENDER",
            ActantialRole::Receiver => "VIEWS_AS_RECEIVER",
        };

        let cypher = format!(
            "MATCH (s:Character {{id: $subject_id}}), (t:PlayerCharacter {{id: $target_id}})
            CREATE (s)-[:{} {{
                want_id: $want_id,
                reason: $reason,
                assigned_at: $assigned_at
            }}]->(t)",
            edge_type
        );

        let q = query(&cypher)
            .param("subject_id", subject_id.to_string())
            .param("target_id", target_id.to_string())
            .param("want_id", view.want_id.to_string())
            .param("reason", view.reason.clone())
            .param("assigned_at", view.assigned_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove an actantial view toward a PC
    pub(crate) async fn remove_actantial_view_to_pc_impl(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: PlayerCharacterId,
        want_id: WantId,
    ) -> Result<()> {
        let edge_type = match role {
            ActantialRole::Helper => "VIEWS_AS_HELPER",
            ActantialRole::Opponent => "VIEWS_AS_OPPONENT",
            ActantialRole::Sender => "VIEWS_AS_SENDER",
            ActantialRole::Receiver => "VIEWS_AS_RECEIVER",
        };

        let cypher = format!(
            "MATCH (s:Character {{id: $subject_id}})-[r:{} {{want_id: $want_id}}]->(t:PlayerCharacter {{id: $target_id}})
            DELETE r",
            edge_type
        );

        let q = query(&cypher)
            .param("subject_id", subject_id.to_string())
            .param("target_id", target_id.to_string())
            .param("want_id", want_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }
}

#[async_trait]
impl CharacterActantialPort for Neo4jCharacterRepository {
    async fn add_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        view: &ActantialView,
    ) -> Result<()> {
        self.add_actantial_view_impl(subject_id, role, target_id, view)
            .await
    }

    async fn add_actantial_view_to_pc(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: PlayerCharacterId,
        view: &ActantialView,
    ) -> Result<()> {
        self.add_actantial_view_to_pc_impl(subject_id, role, target_id, view)
            .await
    }

    async fn get_actantial_views(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<(ActantialRole, ActantialTarget, ActantialView)>> {
        self.get_actantial_views_impl(character_id).await
    }

    async fn remove_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        want_id: WantId,
    ) -> Result<()> {
        self.remove_actantial_view_impl(subject_id, role, target_id, want_id)
            .await
    }

    async fn remove_actantial_view_to_pc(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: PlayerCharacterId,
        want_id: WantId,
    ) -> Result<()> {
        self.remove_actantial_view_to_pc_impl(subject_id, role, target_id, want_id)
            .await
    }
}
