//! Mood Service - Application service for NPC mood and relationship tracking
//!
//! P1.4: Character Mood & Relationship Tracking
//!
//! This service manages:
//! - NPC mood states toward individual PCs
//! - Long-term relationship disposition tracking
//! - Mood updates from challenge outcomes and DM direction
//! - Default mood management for NPCs

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, instrument};

use wrldbldr_engine_ports::outbound::CharacterRepositoryPort;
use wrldbldr_domain::value_objects::{
    InteractionOutcome, MoodLevel, NpcMoodState, RelationshipLevel,
};
use wrldbldr_domain::{CharacterId, PlayerCharacterId};

/// Mood service trait defining the application use cases
#[async_trait]
pub trait MoodService: Send + Sync {
    /// Get an NPC's mood toward a specific PC
    /// Returns the mood state if it exists, or creates a default one from the NPC's default_mood
    async fn get_mood(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<NpcMoodState>;

    /// Set an NPC's mood toward a PC (for DM control)
    async fn set_mood(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        mood: MoodLevel,
        reason: Option<String>,
    ) -> Result<NpcMoodState>;

    /// Apply an interaction outcome to update mood and relationship
    async fn apply_interaction(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        outcome: InteractionOutcome,
    ) -> Result<NpcMoodState>;

    /// Get moods for multiple NPCs in a scene
    async fn get_scene_moods(
        &self,
        npc_ids: &[CharacterId],
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcMoodState>>;

    /// Get all NPC relationships for a PC (for DM panel)
    async fn get_all_relationships(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcMoodState>>;

    /// Get an NPC's default mood
    async fn get_default_mood(&self, npc_id: CharacterId) -> Result<MoodLevel>;

    /// Set an NPC's default mood
    async fn set_default_mood(&self, npc_id: CharacterId, mood: MoodLevel) -> Result<()>;

    /// Set an NPC's relationship level toward a PC (for DM control)
    async fn set_relationship(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        relationship: RelationshipLevel,
    ) -> Result<NpcMoodState>;
}

/// Default implementation of MoodService
#[derive(Clone)]
pub struct MoodServiceImpl {
    character_repo: Arc<dyn CharacterRepositoryPort>,
}

impl MoodServiceImpl {
    /// Create a new MoodServiceImpl with the given repository
    pub fn new(character_repo: Arc<dyn CharacterRepositoryPort>) -> Self {
        Self { character_repo }
    }
}

#[async_trait]
impl MoodService for MoodServiceImpl {
    #[instrument(skip(self))]
    async fn get_mood(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<NpcMoodState> {
        debug!(npc_id = %npc_id, pc_id = %pc_id, "Getting NPC mood toward PC");

        // Try to get existing mood state
        if let Some(mood_state) = self.character_repo.get_mood_toward_pc(npc_id, pc_id).await? {
            return Ok(mood_state);
        }

        // No existing mood - create default from NPC's default_mood
        let default_mood = self.character_repo.get_default_mood(npc_id).await?;
        let mood_state = NpcMoodState::new(npc_id, pc_id).with_mood(default_mood);

        // Persist the initial state
        self.character_repo.set_mood_toward_pc(&mood_state).await?;

        Ok(mood_state)
    }

    #[instrument(skip(self))]
    async fn set_mood(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        mood: MoodLevel,
        reason: Option<String>,
    ) -> Result<NpcMoodState> {
        debug!(npc_id = %npc_id, pc_id = %pc_id, mood = ?mood, "Setting NPC mood");

        // Get or create the mood state
        let mut mood_state = self.get_mood(npc_id, pc_id).await?;

        // Update the mood
        mood_state.set_mood(mood, reason);

        // Persist
        self.character_repo.set_mood_toward_pc(&mood_state).await?;

        Ok(mood_state)
    }

    #[instrument(skip(self))]
    async fn apply_interaction(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        outcome: InteractionOutcome,
    ) -> Result<NpcMoodState> {
        debug!(npc_id = %npc_id, pc_id = %pc_id, outcome = ?outcome, "Applying interaction outcome");

        let mut mood_state = self.get_mood(npc_id, pc_id).await?;

        match outcome {
            InteractionOutcome::Positive { magnitude, reason } => {
                mood_state.adjust_sentiment(magnitude, Some(reason));
                mood_state.add_relationship_points((magnitude * 5.0) as i32);
            }
            InteractionOutcome::Negative { magnitude, reason } => {
                mood_state.adjust_sentiment(-magnitude, Some(reason));
                mood_state.add_relationship_points((-magnitude * 5.0) as i32);
            }
            InteractionOutcome::Neutral => {
                // No change
            }
            InteractionOutcome::ChallengeResult {
                succeeded,
                skill_name,
                significance,
            } => {
                if succeeded {
                    let reason = format!("Succeeded at {} challenge", skill_name);
                    mood_state.adjust_sentiment(significance.success_delta(), Some(reason));
                    mood_state.add_relationship_points(significance.success_points());
                } else {
                    let reason = format!("Failed {} challenge", skill_name);
                    mood_state.adjust_sentiment(significance.failure_delta(), Some(reason));
                    mood_state.add_relationship_points(significance.failure_points());
                }
            }
        }

        // Persist
        self.character_repo.set_mood_toward_pc(&mood_state).await?;

        Ok(mood_state)
    }

    #[instrument(skip(self))]
    async fn get_scene_moods(
        &self,
        npc_ids: &[CharacterId],
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcMoodState>> {
        debug!(npc_count = npc_ids.len(), pc_id = %pc_id, "Getting scene moods");

        if npc_ids.is_empty() {
            return Ok(vec![]);
        }

        // Get existing moods
        let existing_moods = self.character_repo.get_scene_moods(npc_ids, pc_id).await?;

        // Create a set of NPCs that have existing moods
        let existing_npc_ids: std::collections::HashSet<_> =
            existing_moods.iter().map(|m| m.npc_id).collect();

        // For NPCs without existing moods, create defaults
        let mut all_moods = existing_moods;
        for &npc_id in npc_ids {
            if !existing_npc_ids.contains(&npc_id) {
                // Get default mood and create initial state
                let default_mood = self.character_repo.get_default_mood(npc_id).await?;
                let mood_state = NpcMoodState::new(npc_id, pc_id).with_mood(default_mood);
                all_moods.push(mood_state);
            }
        }

        Ok(all_moods)
    }

    #[instrument(skip(self))]
    async fn get_all_relationships(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcMoodState>> {
        debug!(pc_id = %pc_id, "Getting all NPC relationships for PC");
        self.character_repo.get_all_npc_moods_for_pc(pc_id).await
    }

    #[instrument(skip(self))]
    async fn get_default_mood(&self, npc_id: CharacterId) -> Result<MoodLevel> {
        debug!(npc_id = %npc_id, "Getting NPC default mood");
        self.character_repo.get_default_mood(npc_id).await
    }

    #[instrument(skip(self))]
    async fn set_default_mood(&self, npc_id: CharacterId, mood: MoodLevel) -> Result<()> {
        debug!(npc_id = %npc_id, mood = ?mood, "Setting NPC default mood");
        self.character_repo.set_default_mood(npc_id, mood).await
    }

    #[instrument(skip(self))]
    async fn set_relationship(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        relationship: RelationshipLevel,
    ) -> Result<NpcMoodState> {
        debug!(npc_id = %npc_id, pc_id = %pc_id, relationship = ?relationship, "Setting NPC relationship");

        let mut mood_state = self.get_mood(npc_id, pc_id).await?;
        mood_state.relationship = relationship;

        // Adjust relationship_points to match the new level
        mood_state.relationship_points = match relationship {
            RelationshipLevel::Nemesis => -60,
            RelationshipLevel::Enemy => -35,
            RelationshipLevel::Rival => -15,
            RelationshipLevel::Stranger => 0,
            RelationshipLevel::Acquaintance => 15,
            RelationshipLevel::Friend => 35,
            RelationshipLevel::Ally => 60,
        };

        self.character_repo.set_mood_toward_pc(&mood_state).await?;

        Ok(mood_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock repository for testing would go here
    // For now, just a placeholder test
    #[test]
    fn test_mood_service_created() {
        // This is a compile-time test that the types are correct
        assert!(true);
    }
}
