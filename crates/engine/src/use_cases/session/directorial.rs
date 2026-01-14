use std::sync::Arc;

use crate::infrastructure::ports::{DirectorialContext, DirectorialContextPort, NpcMotivation};
use wrldbldr_domain::WorldId;

/// IO dependencies for directorial updates (WS-state owned).
pub struct DirectorialUpdateContext<'a> {
    pub context_store: &'a dyn DirectorialContextPort,
}

/// Input for storing directorial context.
pub struct DirectorialUpdateInput {
    pub world_id: WorldId,
    pub context: DirectorialContext,
}

impl DirectorialUpdateInput {
    /// Create input from protocol types (API layer conversion helper).
    pub fn from_protocol(
        world_id: WorldId,
        proto_context: wrldbldr_protocol::DirectorialContext,
    ) -> Self {
        Self {
            world_id,
            context: DirectorialContext {
                scene_notes: proto_context.scene_notes,
                tone: proto_context.tone,
                npc_motivations: proto_context
                    .npc_motivations
                    .into_iter()
                    .map(|m| NpcMotivation {
                        character_id: m.character_id,
                        emotional_guidance: m.emotional_guidance,
                        immediate_goal: m.immediate_goal,
                        secret_agenda: m.secret_agenda,
                    })
                    .collect(),
                forbidden_topics: proto_context.forbidden_topics,
            },
        }
    }
}

/// Use case for updating directorial context.
pub struct DirectorialUpdate {
    _marker: Arc<()>,
}

impl DirectorialUpdate {
    pub fn new() -> Self {
        Self {
            _marker: Arc::new(()),
        }
    }

    pub async fn execute(&self, ctx: &DirectorialUpdateContext<'_>, input: DirectorialUpdateInput) {
        let context = input.context;
        tracing::info!(
            world_id = %input.world_id,
            scene_notes = %context.scene_notes,
            tone = %context.tone,
            npc_motivation_count = context.npc_motivations.len(),
            forbidden_topic_count = context.forbidden_topics.len(),
            "Directorial context stored"
        );

        for motivation in &context.npc_motivations {
            tracing::debug!(
                world_id = %input.world_id,
                character_id = %motivation.character_id,
                emotional_guidance = %motivation.emotional_guidance,
                immediate_goal = %motivation.immediate_goal,
                has_secret_agenda = motivation.secret_agenda.is_some(),
                "NPC motivation in directorial context"
            );
        }

        if !context.forbidden_topics.is_empty() {
            tracing::debug!(
                world_id = %input.world_id,
                forbidden_topics = ?context.forbidden_topics,
                "Forbidden topics in directorial context"
            );
        }

        ctx.context_store.set_context(input.world_id, context);

        tracing::info!(
            world_id = %input.world_id,
            "Directorial context stored for world"
        );
    }
}
