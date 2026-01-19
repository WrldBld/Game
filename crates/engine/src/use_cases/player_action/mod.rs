// Player action - fields for future player action features
#![allow(dead_code)]

use std::sync::Arc;

use wrldbldr_domain::{ActionId, CharacterId, ConversationId, PlayerCharacterId, WorldId};

use crate::infrastructure::ports::{ClockPort, QueueError, QueuePort};
use crate::queue_types::PlayerActionData;

use crate::use_cases::conversation::{ConversationError, StartConversation};

pub struct PlayerActionUseCases {
    pub handle: Arc<HandlePlayerAction>,
}

impl PlayerActionUseCases {
    pub fn new(handle: Arc<HandlePlayerAction>) -> Self {
        Self { handle }
    }
}

pub struct HandlePlayerAction {
    start_conversation: Arc<StartConversation>,
    queue: Arc<dyn QueuePort>,
    clock: Arc<dyn ClockPort>,
}

impl HandlePlayerAction {
    pub fn new(
        start_conversation: Arc<StartConversation>,
        queue: Arc<dyn QueuePort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            start_conversation,
            queue,
            clock,
        }
    }

    pub async fn execute(
        &self,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
        user_id: String,
        action_type: String,
        target_npc: Option<CharacterId>,
        dialogue: Option<String>,
    ) -> Result<PlayerActionProcessed, PlayerActionError> {
        if action_type == "talk" {
            let npc_id = target_npc.ok_or(PlayerActionError::MissingTalkTarget)?;
            let dialogue_text = dialogue
                .map(|text| text.trim().to_string())
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| "Hello".to_string());

            let conversation = self
                .start_conversation
                .execute(world_id, pc_id, npc_id, user_id.clone(), dialogue_text)
                .await
                .map_err(PlayerActionError::Conversation)?;

            return Ok(PlayerActionProcessed {
                action_id: ActionId::from(conversation.action_queue_id),
                action_type,
                player_id: user_id,
                world_id,
                queue_depth: 1,
                conversation_id: Some(conversation.conversation_id),
                npc_name: Some(conversation.npc_name),
            });
        }

        let action_data = PlayerActionData {
            world_id,
            player_id: user_id.clone(),
            pc_id: Some(pc_id),
            action_type: action_type.clone(),
            target: target_npc.map(|id| id.to_string()),
            dialogue,
            timestamp: self.clock.now(),
            conversation_id: None,
        };

        let action_id = self.queue.enqueue_player_action(&action_data).await?;

        let queue_depth = self.queue.get_pending_count("player_action").await?;

        Ok(PlayerActionProcessed {
            action_id: ActionId::from(action_id),
            action_type,
            player_id: user_id,
            world_id,
            queue_depth,
            conversation_id: None,
            npc_name: None,
        })
    }
}

#[derive(Debug)]
pub struct PlayerActionProcessed {
    pub action_id: ActionId,
    pub action_type: String,
    pub player_id: String,
    pub world_id: WorldId,
    pub queue_depth: usize,
    pub conversation_id: Option<ConversationId>,
    pub npc_name: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PlayerActionError {
    #[error("Talk action requires a target NPC ID")]
    MissingTalkTarget,
    #[error("Conversation failed: {0}")]
    Conversation(#[from] ConversationError),
    #[error("Queue error: {0}")]
    Queue(#[from] QueueError),
}
