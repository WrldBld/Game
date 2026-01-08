use std::sync::Arc;

use uuid::Uuid;

use wrldbldr_domain::{CharacterId, PlayerCharacterId, WorldId};

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
}

impl HandlePlayerAction {
    pub fn new(start_conversation: Arc<StartConversation>) -> Self {
        Self { start_conversation }
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
        let action_id = Uuid::new_v4();

        let mut result = PlayerActionProcessed {
            action_id,
            action_type: action_type.clone(),
            player_id: user_id.clone(),
            world_id,
            queue_depth: 1,
            conversation_id: None,
            npc_name: None,
        };

        if action_type == "talk" {
            let npc_id = target_npc.ok_or(PlayerActionError::MissingTalkTarget)?;
            let dialogue_text = dialogue.ok_or(PlayerActionError::MissingTalkDialogue)?;

            let conversation = self
                .start_conversation
                .execute(world_id, pc_id, npc_id, user_id, dialogue_text)
                .await
                .map_err(PlayerActionError::Conversation)?;

            result.conversation_id = Some(conversation.conversation_id);
            result.npc_name = Some(conversation.npc_name);
        }

        Ok(result)
    }
}

#[derive(Debug)]
pub struct PlayerActionProcessed {
    pub action_id: Uuid,
    pub action_type: String,
    pub player_id: String,
    pub world_id: WorldId,
    pub queue_depth: usize,
    pub conversation_id: Option<Uuid>,
    pub npc_name: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PlayerActionError {
    #[error("Talk action requires a target NPC ID")]
    MissingTalkTarget,
    #[error("Talk action requires dialogue")]
    MissingTalkDialogue,
    #[error("Conversation failed: {0}")]
    Conversation(#[from] ConversationError),
}
