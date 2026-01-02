//! Scene Adapters
//!
//! Implements scene-related outbound ports:
//! - DirectorialContextDtoRepositoryPort
//! - SceneDmActionQueuePort
//!
//! These are thin adapters that convert between DTO types and repository types.

use std::sync::Arc;

use wrldbldr_domain::value_objects::{DirectorialNotes, DomainNpcMotivation, PacingGuidance, ToneGuidance};
use wrldbldr_domain::WorldId;
use wrldbldr_engine_ports::outbound::{
    DirectorialContextData, DirectorialContextDtoRepositoryPort,
    DirectorialContextRepositoryPort, SceneDmAction, SceneDmActionQueuePort,
};

/// Adapter that implements DirectorialContextDtoRepositoryPort by wrapping
/// the domain-facing DirectorialContextRepositoryPort.
pub struct DirectorialContextAdapter {
    repo: Arc<dyn DirectorialContextRepositoryPort>,
}

impl DirectorialContextAdapter {
    pub fn new(repo: Arc<dyn DirectorialContextRepositoryPort>) -> Self {
        Self { repo }
    }

    /// Parse a tone string into ToneGuidance
    fn parse_tone(s: &str) -> ToneGuidance {
        let lower = s.to_lowercase();
        if lower.contains("tense") || lower.contains("suspense") {
            ToneGuidance::Tense
        } else if lower.contains("humor") || lower.contains("comedic") || lower.contains("light") {
            ToneGuidance::Comedic
        } else if lower.contains("serious") || lower.contains("grave") || lower.contains("solemn") {
            ToneGuidance::Serious
        } else if lower.contains("romantic") || lower.contains("intimate") {
            ToneGuidance::Romantic
        } else if lower.contains("mysterious") || lower.contains("enigmatic") {
            ToneGuidance::Mysterious
        } else if lower.contains("action") || lower.contains("exciting") || lower.contains("thrilling") {
            ToneGuidance::Exciting
        } else if lower.contains("creepy") || lower.contains("spooky") || lower.contains("unsettling") {
            ToneGuidance::Creepy
        } else if lower.contains("contempl") || lower.contains("reflect") || lower.contains("quiet") {
            ToneGuidance::Contemplative
        } else if lower.contains("lightheart") || lower.contains("fun") {
            ToneGuidance::Lighthearted
        } else {
            ToneGuidance::Neutral
        }
    }

    /// Convert use-case DTO to domain value object
    fn dto_to_domain(context: &DirectorialContextData) -> DirectorialNotes {
        let npc_motivations = context
            .npc_motivations
            .iter()
            .map(|m| {
                (
                    m.character_id.to_string(),
                    DomainNpcMotivation::new(
                        m.emotional_state.clone().unwrap_or_default(),
                        m.motivation.clone(),
                    ),
                )
            })
            .collect();

        let pacing = context
            .pacing
            .as_ref()
            .map(|p| match p.as_str() {
                "fast" => PacingGuidance::Fast,
                "slow" => PacingGuidance::Slow,
                "building" => PacingGuidance::Building,
                "urgent" => PacingGuidance::Urgent,
                _ => PacingGuidance::Natural,
            })
            .unwrap_or(PacingGuidance::Natural);

        let tone = context
            .scene_mood
            .as_ref()
            .map(|s| Self::parse_tone(s))
            .unwrap_or_default();

        DirectorialNotes {
            general_notes: context.dm_notes.clone().unwrap_or_default(),
            tone,
            npc_motivations,
            forbidden_topics: Vec::new(),
            allowed_tools: Vec::new(),
            suggested_beats: Vec::new(),
            pacing,
        }
    }
}

#[async_trait::async_trait]
impl DirectorialContextDtoRepositoryPort for DirectorialContextAdapter {
    async fn save(
        &self,
        world_id: &WorldId,
        context: &DirectorialContextData,
    ) -> Result<(), String> {
        let notes = Self::dto_to_domain(context);
        self.repo.save(world_id, &notes).await.map_err(|e| e.to_string())
    }
}

/// Placeholder adapter for DM action queue.
///
/// This is a no-op implementation that logs actions but doesn't persist them.
/// Real implementation would integrate with an actual queue system.
pub struct DmActionQueuePlaceholder;

impl DmActionQueuePlaceholder {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DmActionQueuePlaceholder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SceneDmActionQueuePort for DmActionQueuePlaceholder {
    async fn enqueue_action(
        &self,
        world_id: &WorldId,
        dm_id: String,
        action: SceneDmAction,
    ) -> Result<(), String> {
        tracing::debug!(
            world_id = %world_id,
            dm_id = %dm_id,
            action = ?action,
            "DM action enqueued (placeholder - no-op)"
        );
        Ok(())
    }
}
