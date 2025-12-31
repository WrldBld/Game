//! CharacterLocationPort implementation

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;
use wrldbldr_domain::entities::{Character, FrequencyLevel};
use wrldbldr_domain::value_objects::{
    RegionFrequency, RegionRelationship, RegionRelationshipType, RegionShift,
};
use wrldbldr_domain::{CharacterId, LocationId, RegionId};
use wrldbldr_engine_ports::outbound::CharacterLocationPort;

use super::common::row_to_character;
use super::Neo4jCharacterRepository;

impl Neo4jCharacterRepository {
    // =========================================================================
    // Character-Location Relationships
    // =========================================================================

    /// Set character's home location
    pub(crate) async fn set_home_location_impl(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        description: Option<String>,
    ) -> Result<()> {
        // Remove existing home first
        self.remove_home_location_impl(character_id).await?;

        let q = query(
            "MATCH (c:Character {id: $character_id}), (l:Location {id: $location_id})
            CREATE (c)-[:HOME_LOCATION {description: $description}]->(l)",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string())
        .param("description", description.unwrap_or_default());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove character's home location
    pub(crate) async fn remove_home_location_impl(&self, character_id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:HOME_LOCATION]->()
            DELETE r",
        )
        .param("id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Set character's work location
    pub(crate) async fn set_work_location_impl(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        role: String,
        schedule: Option<String>,
    ) -> Result<()> {
        // Remove existing work first
        self.remove_work_location_impl(character_id).await?;

        let q = query(
            "MATCH (c:Character {id: $character_id}), (l:Location {id: $location_id})
            CREATE (c)-[:WORKS_AT {role: $role, schedule: $schedule}]->(l)",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string())
        .param("role", role)
        .param("schedule", schedule.unwrap_or_default());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove character's work location
    pub(crate) async fn remove_work_location_impl(&self, character_id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:WORKS_AT]->()
            DELETE r",
        )
        .param("id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add a frequented location
    pub(crate) async fn add_frequented_location_impl(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        frequency: FrequencyLevel,
        time_of_day: String,
        day_of_week: Option<String>,
        reason: Option<String>,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id}), (l:Location {id: $location_id})
            CREATE (c)-[:FREQUENTS {
                frequency: $frequency,
                time_of_day: $time_of_day,
                day_of_week: $day_of_week,
                reason: $reason,
                since: $since
            }]->(l)",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string())
        .param("frequency", frequency.to_string())
        .param("time_of_day", time_of_day)
        .param("day_of_week", day_of_week.unwrap_or_default())
        .param("reason", reason.unwrap_or_default())
        .param("since", self.clock.now_rfc3339());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove a frequented location
    pub(crate) async fn remove_frequented_location_impl(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:FREQUENTS]->(l:Location {id: $location_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add an avoided location
    pub(crate) async fn add_avoided_location_impl(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        reason: String,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id}), (l:Location {id: $location_id})
            CREATE (c)-[:AVOIDS {reason: $reason}]->(l)",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string())
        .param("reason", reason);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove an avoided location
    pub(crate) async fn remove_avoided_location_impl(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:AVOIDS]->(l:Location {id: $location_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("location_id", location_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get NPCs who might be at a location
    pub(crate) async fn get_npcs_at_location_impl(
        &self,
        location_id: LocationId,
        time_of_day: Option<&str>,
    ) -> Result<Vec<Character>> {
        // Build query based on whether time_of_day filter is provided
        let cypher = if time_of_day.is_some() {
            "MATCH (c:Character)-[r]->(l:Location {id: $location_id})
            WHERE (type(r) = 'HOME_LOCATION')
               OR (type(r) = 'WORKS_AT' AND (r.schedule IS NULL OR r.schedule = '' OR r.schedule = $time_of_day))
               OR (type(r) = 'FREQUENTS' AND (r.time_of_day = 'Any' OR r.time_of_day = $time_of_day))
            RETURN DISTINCT c"
        } else {
            "MATCH (c:Character)-[r]->(l:Location {id: $location_id})
            WHERE type(r) IN ['HOME_LOCATION', 'WORKS_AT', 'FREQUENTS']
            RETURN DISTINCT c"
        };

        let q = query(cypher)
            .param("location_id", location_id.to_string())
            .param("time_of_day", time_of_day.unwrap_or(""));

        let mut result = self.connection.graph().execute(q).await?;
        let mut characters = Vec::new();

        while let Some(row) = result.next().await? {
            characters.push(row_to_character(row)?);
        }

        Ok(characters)
    }

    // =========================================================================
    // Character-Region Relationships (Phase 23C)
    // =========================================================================

    /// Set character's home region
    pub(crate) async fn set_home_region_impl(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
    ) -> Result<()> {
        // Remove existing home region first
        self.remove_home_region_impl(character_id).await?;

        let q = query(
            "MATCH (c:Character {id: $character_id}), (r:Region {id: $region_id})
            CREATE (c)-[:HOME_REGION]->(r)",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Set home region for character {}: {}",
            character_id,
            region_id
        );
        Ok(())
    }

    /// Remove character's home region
    pub(crate) async fn remove_home_region_impl(&self, character_id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:HOME_REGION]->()
            DELETE r",
        )
        .param("id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Set character's work region with shift (day, night, always)
    pub(crate) async fn set_work_region_impl(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        shift: RegionShift,
    ) -> Result<()> {
        // Remove existing work region first
        self.remove_work_region_impl(character_id).await?;

        let q = query(
            "MATCH (c:Character {id: $character_id}), (r:Region {id: $region_id})
            CREATE (c)-[:WORKS_AT_REGION {shift: $shift}]->(r)",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string())
        .param("shift", shift.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Set work region for character {}: {} ({:?})",
            character_id,
            region_id,
            shift
        );
        Ok(())
    }

    /// Remove character's work region
    pub(crate) async fn remove_work_region_impl(&self, character_id: CharacterId) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $id})-[r:WORKS_AT_REGION]->()
            DELETE r",
        )
        .param("id", character_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add a frequented region
    pub(crate) async fn add_frequented_region_impl(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        frequency: RegionFrequency,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id}), (r:Region {id: $region_id})
            CREATE (c)-[:FREQUENTS_REGION {
                frequency: $frequency,
                since: $since
            }]->(r)",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string())
        .param("frequency", frequency.to_string())
        .param("since", self.clock.now_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Added frequented region for character {}: {} ({:?})",
            character_id,
            region_id,
            frequency
        );
        Ok(())
    }

    /// Remove a frequented region
    pub(crate) async fn remove_frequented_region_impl(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:FREQUENTS_REGION]->(reg:Region {id: $region_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Add an avoided region
    pub(crate) async fn add_avoided_region_impl(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        reason: String,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id}), (r:Region {id: $region_id})
            CREATE (c)-[:AVOIDS_REGION {reason: $reason}]->(r)",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string())
        .param("reason", reason);

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Added avoided region for character {}: {}",
            character_id,
            region_id
        );
        Ok(())
    }

    /// Remove an avoided region
    pub(crate) async fn remove_avoided_region_impl(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:AVOIDS_REGION]->(reg:Region {id: $region_id})
            DELETE r",
        )
        .param("character_id", character_id.to_string())
        .param("region_id", region_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// List all region relationships for a character
    pub(crate) async fn list_region_relationships_impl(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<RegionRelationship>> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r]->(reg:Region)
            WHERE type(r) IN ['HOME_REGION', 'WORKS_AT_REGION', 'FREQUENTS_REGION', 'AVOIDS_REGION']
            RETURN type(r) as rel_type, reg.id as region_id, reg.name as region_name,
                   r.shift as shift, r.frequency as frequency, r.reason as reason",
        )
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut relationships = Vec::new();

        while let Some(row) = result.next().await? {
            let rel_type: String = row.get("rel_type")?;
            let region_id_str: String = row.get("region_id")?;
            let region_name: String = row.get("region_name")?;

            let region_id = RegionId::from_uuid(uuid::Uuid::parse_str(&region_id_str)?);

            let relationship_type = match rel_type.as_str() {
                "HOME_REGION" => RegionRelationshipType::Home,
                "WORKS_AT_REGION" => {
                    let shift_str: String = row.get("shift").unwrap_or_default();
                    let shift = shift_str.parse().unwrap_or(RegionShift::Always);
                    RegionRelationshipType::WorksAt { shift }
                }
                "FREQUENTS_REGION" => {
                    let freq_str: String = row.get("frequency").unwrap_or_default();
                    let frequency = freq_str.parse().unwrap_or(RegionFrequency::Sometimes);
                    RegionRelationshipType::Frequents { frequency }
                }
                "AVOIDS_REGION" => {
                    let reason: String = row.get("reason").unwrap_or_default();
                    RegionRelationshipType::Avoids { reason }
                }
                _ => continue,
            };

            relationships.push(RegionRelationship {
                region_id,
                region_name,
                relationship_type,
            });
        }

        Ok(relationships)
    }

    /// Get all NPCs with any relationship to a region
    pub(crate) async fn get_npcs_related_to_region_impl(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<(Character, RegionRelationshipType)>> {
        let q = query(
            "MATCH (c:Character)-[r]->(reg:Region {id: $region_id})
            WHERE type(r) IN ['HOME_REGION', 'WORKS_AT_REGION', 'FREQUENTS_REGION', 'AVOIDS_REGION']
            RETURN c, type(r) as rel_type, r.shift as shift, r.frequency as frequency, r.reason as reason",
        )
        .param("region_id", region_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut npcs = Vec::new();

        while let Some(row) = result.next().await? {
            // Extract relationship data first (before consuming row for character)
            let rel_type: String = row.get("rel_type")?;
            let shift_str: String = row.get("shift").unwrap_or_default();
            let freq_str: String = row.get("frequency").unwrap_or_default();
            let reason: String = row.get("reason").unwrap_or_default();

            let relationship_type = match rel_type.as_str() {
                "HOME_REGION" => RegionRelationshipType::Home,
                "WORKS_AT_REGION" => {
                    let shift = shift_str.parse().unwrap_or(RegionShift::Always);
                    RegionRelationshipType::WorksAt { shift }
                }
                "FREQUENTS_REGION" => {
                    let frequency = freq_str.parse().unwrap_or(RegionFrequency::Sometimes);
                    RegionRelationshipType::Frequents { frequency }
                }
                "AVOIDS_REGION" => RegionRelationshipType::Avoids { reason },
                _ => continue,
            };

            let character = row_to_character(row)?;
            npcs.push((character, relationship_type));
        }

        Ok(npcs)
    }
}

#[async_trait]
impl CharacterLocationPort for Neo4jCharacterRepository {
    async fn set_home_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        description: Option<String>,
    ) -> Result<()> {
        self.set_home_location_impl(character_id, location_id, description)
            .await
    }

    async fn remove_home_location(&self, character_id: CharacterId) -> Result<()> {
        self.remove_home_location_impl(character_id).await
    }

    async fn set_work_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        role: String,
        schedule: Option<String>,
    ) -> Result<()> {
        self.set_work_location_impl(character_id, location_id, role, schedule)
            .await
    }

    async fn remove_work_location(&self, character_id: CharacterId) -> Result<()> {
        self.remove_work_location_impl(character_id).await
    }

    async fn add_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        frequency: FrequencyLevel,
        time_of_day: String,
        day_of_week: Option<String>,
        reason: Option<String>,
    ) -> Result<()> {
        self.add_frequented_location_impl(
            character_id,
            location_id,
            frequency,
            time_of_day,
            day_of_week,
            reason,
        )
        .await
    }

    async fn remove_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        self.remove_frequented_location_impl(character_id, location_id)
            .await
    }

    async fn add_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        reason: String,
    ) -> Result<()> {
        self.add_avoided_location_impl(character_id, location_id, reason)
            .await
    }

    async fn remove_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()> {
        self.remove_avoided_location_impl(character_id, location_id)
            .await
    }

    async fn get_npcs_at_location(
        &self,
        location_id: LocationId,
        time_of_day: Option<String>,
    ) -> Result<Vec<Character>> {
        self.get_npcs_at_location_impl(location_id, time_of_day.as_deref())
            .await
    }

    async fn get_region_relationships(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<RegionRelationship>> {
        self.list_region_relationships_impl(character_id).await
    }

    async fn set_home_region(&self, character_id: CharacterId, region_id: RegionId) -> Result<()> {
        self.set_home_region_impl(character_id, region_id).await
    }

    async fn set_work_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        shift: RegionShift,
    ) -> Result<()> {
        self.set_work_region_impl(character_id, region_id, shift)
            .await
    }

    async fn remove_region_relationship(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        relationship_type: String,
    ) -> Result<()> {
        match relationship_type.to_lowercase().as_str() {
            "home" => self.remove_home_region_impl(character_id).await,
            "work" => self.remove_work_region_impl(character_id).await,
            "frequents" => {
                self.remove_frequented_region_impl(character_id, region_id)
                    .await
            }
            "avoids" => {
                self.remove_avoided_region_impl(character_id, region_id)
                    .await
            }
            _ => Err(anyhow::anyhow!(
                "Unknown relationship type: {}. Must be one of: home, work, frequents, avoids",
                relationship_type
            )),
        }
    }
}
