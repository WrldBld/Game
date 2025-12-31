//! Disposition Service - Application service for NPC disposition and relationship tracking
//!
//! P1.4: Character Disposition & Relationship Tracking
//!
//! This service manages:
//! - NPC disposition states toward individual PCs
//! - Long-term relationship level tracking
//! - Disposition updates from challenge outcomes and DM direction
//! - Default disposition management for NPCs
//!
//! ## Terminology
//!
//! - **Disposition**: How an NPC emotionally feels about a specific PC (Tier 1)
//! - **Relationship**: Social distance/familiarity between NPC and PC
//!
//! See `disposition.rs` in domain for the full Three-Tier Emotional Model.

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, instrument};

use wrldbldr_domain::value_objects::{
    DispositionLevel, InteractionOutcome, NpcDispositionState, RelationshipLevel,
};
use wrldbldr_domain::{CharacterId, PlayerCharacterId};
use wrldbldr_engine_ports::outbound::{
    CharacterDispositionPort, ClockPort, DispositionServicePort,
};

/// Disposition service trait defining the application use cases
#[async_trait]
pub trait DispositionService: Send + Sync {
    /// Get an NPC's disposition toward a specific PC
    /// Returns the disposition state if it exists, or creates a default one from the NPC's default_disposition
    async fn get_disposition(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<NpcDispositionState>;

    /// Set an NPC's disposition toward a PC (for DM control)
    async fn set_disposition(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        disposition: DispositionLevel,
        reason: Option<String>,
    ) -> Result<NpcDispositionState>;

    /// Apply an interaction outcome to update disposition and relationship
    async fn apply_interaction(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        outcome: InteractionOutcome,
    ) -> Result<NpcDispositionState>;

    /// Get dispositions for multiple NPCs in a scene
    async fn get_scene_dispositions(
        &self,
        npc_ids: &[CharacterId],
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>>;

    /// Get all NPC relationships for a PC (for DM panel)
    async fn get_all_relationships(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>>;

    /// Get an NPC's default disposition
    async fn get_default_disposition(&self, npc_id: CharacterId) -> Result<DispositionLevel>;

    /// Set an NPC's default disposition
    async fn set_default_disposition(
        &self,
        npc_id: CharacterId,
        disposition: DispositionLevel,
    ) -> Result<()>;

    /// Set an NPC's relationship level toward a PC (for DM control)
    async fn set_relationship(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        relationship: RelationshipLevel,
    ) -> Result<NpcDispositionState>;
}

/// Default implementation of DispositionService
#[derive(Clone)]
pub struct DispositionServiceImpl {
    character_disposition: Arc<dyn CharacterDispositionPort>,
    clock: Arc<dyn ClockPort>,
}

impl DispositionServiceImpl {
    /// Create a new DispositionServiceImpl with the given repository
    pub fn new(
        character_disposition: Arc<dyn CharacterDispositionPort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            character_disposition,
            clock,
        }
    }
}

#[async_trait]
impl DispositionService for DispositionServiceImpl {
    #[instrument(skip(self))]
    async fn get_disposition(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<NpcDispositionState> {
        debug!(npc_id = %npc_id, pc_id = %pc_id, "Getting NPC disposition toward PC");

        // Try to get existing disposition state
        if let Some(disposition_state) = self
            .character_disposition
            .get_disposition_toward_pc(npc_id, pc_id)
            .await?
        {
            return Ok(disposition_state);
        }

        // No existing disposition - create default from NPC's default_disposition
        let default_disposition = self
            .character_disposition
            .get_default_disposition(npc_id)
            .await?;
        let disposition_state = NpcDispositionState::new(npc_id, pc_id, self.clock.now())
            .with_disposition(default_disposition);

        // Persist the initial state
        self.character_disposition
            .set_disposition_toward_pc(&disposition_state)
            .await?;

        Ok(disposition_state)
    }

    #[instrument(skip(self))]
    async fn set_disposition(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        disposition: DispositionLevel,
        reason: Option<String>,
    ) -> Result<NpcDispositionState> {
        debug!(npc_id = %npc_id, pc_id = %pc_id, disposition = ?disposition, "Setting NPC disposition");

        // Get or create the disposition state
        let mut disposition_state =
            DispositionService::get_disposition(self, npc_id, pc_id).await?;

        // Update the disposition
        disposition_state.set_disposition(disposition, reason, self.clock.now());

        // Persist
        self.character_disposition
            .set_disposition_toward_pc(&disposition_state)
            .await?;

        Ok(disposition_state)
    }

    #[instrument(skip(self))]
    async fn apply_interaction(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        outcome: InteractionOutcome,
    ) -> Result<NpcDispositionState> {
        debug!(npc_id = %npc_id, pc_id = %pc_id, outcome = ?outcome, "Applying interaction outcome");

        let mut disposition_state =
            DispositionService::get_disposition(self, npc_id, pc_id).await?;

        let now = self.clock.now();
        match outcome {
            InteractionOutcome::Positive { magnitude, reason } => {
                disposition_state.adjust_sentiment(magnitude, Some(reason), now);
                disposition_state.add_relationship_points((magnitude * 5.0) as i32, now);
            }
            InteractionOutcome::Negative { magnitude, reason } => {
                disposition_state.adjust_sentiment(-magnitude, Some(reason), now);
                disposition_state.add_relationship_points((-magnitude * 5.0) as i32, now);
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
                    disposition_state.adjust_sentiment(
                        significance.success_delta(),
                        Some(reason),
                        now,
                    );
                    disposition_state.add_relationship_points(significance.success_points(), now);
                } else {
                    let reason = format!("Failed {} challenge", skill_name);
                    disposition_state.adjust_sentiment(
                        significance.failure_delta(),
                        Some(reason),
                        now,
                    );
                    disposition_state.add_relationship_points(significance.failure_points(), now);
                }
            }
        }

        // Persist
        self.character_disposition
            .set_disposition_toward_pc(&disposition_state)
            .await?;

        Ok(disposition_state)
    }

    #[instrument(skip(self))]
    async fn get_scene_dispositions(
        &self,
        npc_ids: &[CharacterId],
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>> {
        debug!(npc_count = npc_ids.len(), pc_id = %pc_id, "Getting scene dispositions");

        if npc_ids.is_empty() {
            return Ok(vec![]);
        }

        // Get existing dispositions
        let existing_dispositions = self
            .character_disposition
            .get_scene_dispositions(npc_ids, pc_id)
            .await?;

        // Create a set of NPCs that have existing dispositions
        let existing_npc_ids: std::collections::HashSet<_> =
            existing_dispositions.iter().map(|d| d.npc_id).collect();

        // For NPCs without existing dispositions, create defaults
        let mut all_dispositions = existing_dispositions;
        for &npc_id in npc_ids {
            if !existing_npc_ids.contains(&npc_id) {
                // Get default disposition and create initial state
                let default_disposition = self
                    .character_disposition
                    .get_default_disposition(npc_id)
                    .await?;
                let disposition_state = NpcDispositionState::new(npc_id, pc_id, self.clock.now())
                    .with_disposition(default_disposition);
                all_dispositions.push(disposition_state);
            }
        }

        Ok(all_dispositions)
    }

    #[instrument(skip(self))]
    async fn get_all_relationships(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>> {
        debug!(pc_id = %pc_id, "Getting all NPC relationships for PC");
        self.character_disposition
            .get_all_npc_dispositions_for_pc(pc_id)
            .await
    }

    #[instrument(skip(self))]
    async fn get_default_disposition(&self, npc_id: CharacterId) -> Result<DispositionLevel> {
        debug!(npc_id = %npc_id, "Getting NPC default disposition");
        self.character_disposition
            .get_default_disposition(npc_id)
            .await
    }

    #[instrument(skip(self))]
    async fn set_default_disposition(
        &self,
        npc_id: CharacterId,
        disposition: DispositionLevel,
    ) -> Result<()> {
        debug!(npc_id = %npc_id, disposition = ?disposition, "Setting NPC default disposition");
        self.character_disposition
            .set_default_disposition(npc_id, disposition)
            .await
    }

    #[instrument(skip(self))]
    async fn set_relationship(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        relationship: RelationshipLevel,
    ) -> Result<NpcDispositionState> {
        debug!(npc_id = %npc_id, pc_id = %pc_id, relationship = ?relationship, "Setting NPC relationship");

        let mut disposition_state =
            DispositionService::get_disposition(self, npc_id, pc_id).await?;
        disposition_state.relationship = relationship;

        // Adjust relationship_points to match the new level
        disposition_state.relationship_points = match relationship {
            RelationshipLevel::Nemesis => -60,
            RelationshipLevel::Enemy => -35,
            RelationshipLevel::Rival => -15,
            RelationshipLevel::Stranger => 0,
            RelationshipLevel::Acquaintance => 15,
            RelationshipLevel::Friend => 35,
            RelationshipLevel::Ally => 60,
        };

        self.character_disposition
            .set_disposition_toward_pc(&disposition_state)
            .await?;

        Ok(disposition_state)
    }
}

// =============================================================================
// Port Implementation
// =============================================================================

/// Implementation of the `DispositionServicePort` for `DispositionServiceImpl`.
///
/// This exposes the disposition service methods to infrastructure adapters.
#[async_trait]
impl DispositionServicePort for DispositionServiceImpl {
    async fn get_disposition(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<NpcDispositionState> {
        DispositionService::get_disposition(self, npc_id, pc_id).await
    }

    async fn set_disposition(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        disposition: DispositionLevel,
        reason: Option<String>,
    ) -> Result<NpcDispositionState> {
        DispositionService::set_disposition(self, npc_id, pc_id, disposition, reason).await
    }

    async fn apply_interaction(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        outcome: InteractionOutcome,
    ) -> Result<NpcDispositionState> {
        DispositionService::apply_interaction(self, npc_id, pc_id, outcome).await
    }

    async fn get_scene_dispositions(
        &self,
        npc_ids: &[CharacterId],
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>> {
        DispositionService::get_scene_dispositions(self, npc_ids, pc_id).await
    }

    async fn get_all_relationships(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>> {
        DispositionService::get_all_relationships(self, pc_id).await
    }

    async fn get_default_disposition(&self, npc_id: CharacterId) -> Result<DispositionLevel> {
        DispositionService::get_default_disposition(self, npc_id).await
    }

    async fn set_default_disposition(
        &self,
        npc_id: CharacterId,
        disposition: DispositionLevel,
    ) -> Result<()> {
        DispositionService::set_default_disposition(self, npc_id, disposition).await
    }

    async fn set_relationship(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        relationship: RelationshipLevel,
    ) -> Result<NpcDispositionState> {
        DispositionService::set_relationship(self, npc_id, pc_id, relationship).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock repository for testing would go here
    // For now, just a placeholder test
    #[test]
    fn test_disposition_service_created() {
        // This is a compile-time test that the types are correct
        assert!(true);
    }
}
