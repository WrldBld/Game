use std::sync::Arc;

use crate::api::connections::ConnectionManager;
use wrldbldr_domain::WorldId;
use wrldbldr_protocol::DirectorialContext;

/// IO dependencies for directorial updates (WS-state owned).
pub struct DirectorialUpdateContext<'a> {
    pub connections: &'a ConnectionManager,
}

/// Input for storing directorial context.
pub struct DirectorialUpdateInput {
    pub world_id: WorldId,
    pub context: DirectorialContext,
}

/// Use case for updating directorial context.
pub struct DirectorialUpdate {
    _marker: Arc<()>,
}

impl DirectorialUpdate {
    pub fn new() -> Self {
        Self { _marker: Arc::new(()) }
    }

    pub async fn execute(
        &self,
        ctx: &DirectorialUpdateContext<'_>,
        input: DirectorialUpdateInput,
    ) {
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

        ctx.connections
            .set_directorial_context(input.world_id, context);

        tracing::info!(
            world_id = %input.world_id,
            "Directorial context stored for world"
        );
    }
}
