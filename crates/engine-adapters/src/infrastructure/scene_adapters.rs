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

/// Adapter for DM action queue operations.
///
/// This adapter implements `SceneDmActionQueuePort` by delegating to `DmActionEnqueuePort`.
/// It converts scene-specific DM actions to the generic queue format.
pub struct SceneDmActionQueueAdapter {
    dm_action_queue: Arc<dyn wrldbldr_engine_ports::outbound::DmActionEnqueuePort>,
    clock: Arc<dyn wrldbldr_engine_ports::outbound::ClockPort>,
}

impl SceneDmActionQueueAdapter {
    pub fn new(
        dm_action_queue: Arc<dyn wrldbldr_engine_ports::outbound::DmActionEnqueuePort>,
        clock: Arc<dyn wrldbldr_engine_ports::outbound::ClockPort>,
    ) -> Self {
        Self {
            dm_action_queue,
            clock,
        }
    }
}

#[async_trait::async_trait]
impl SceneDmActionQueuePort for SceneDmActionQueueAdapter {
    async fn enqueue_action(
        &self,
        world_id: &WorldId,
        dm_id: String,
        action: SceneDmAction,
    ) -> Result<(), String> {
        use wrldbldr_engine_ports::outbound::{
            DmActionEnqueueRequest, DmActionEnqueueType, DmEnqueueDecision, SceneApprovalDecision,
        };

        tracing::debug!(
            world_id = %world_id,
            dm_id = %dm_id,
            action = ?action,
            "Enqueueing DM action"
        );

        // Convert SceneDmAction to DmActionEnqueueType
        let action_type = match action {
            SceneDmAction::ApprovalDecision {
                request_id,
                decision,
            } => {
                let enqueue_decision = match decision {
                    SceneApprovalDecision::Approve => DmEnqueueDecision::Approve,
                    SceneApprovalDecision::Reject { reason } => {
                        DmEnqueueDecision::Reject { reason }
                    }
                    SceneApprovalDecision::ApproveWithEdits { modified_text } => {
                        DmEnqueueDecision::ApproveWithEdits { modified_text }
                    }
                };
                DmActionEnqueueType::ApprovalDecision {
                    request_id,
                    decision: enqueue_decision,
                }
            }
        };

        // Create the enqueue request
        let request = DmActionEnqueueRequest {
            world_id: *world_id,
            dm_id,
            action_type,
            timestamp: self.clock.now(),
        };

        // Enqueue via the port
        self.dm_action_queue
            .enqueue(request)
            .await
            .map_err(|e| e.to_string())?;

        tracing::info!(world_id = %world_id, "DM action successfully enqueued");
        Ok(())
    }
}
