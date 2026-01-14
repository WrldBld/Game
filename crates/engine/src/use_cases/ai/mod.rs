use std::sync::Arc;

use uuid::Uuid;

use wrldbldr_domain::{CharacterId, LlmRequestData, LlmRequestType, SuggestionContext, WorldId};

use crate::infrastructure::ports::{QueueError, QueuePort, RepoError};
use crate::repositories::character::Character;
use crate::repositories::World;

/// Actantial role for NPC want relationships (domain representation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActantialRole {
    Helper,
    Opponent,
    Sender,
    Receiver,
    Unknown,
}

impl ActantialRole {
    /// Convert to JSON-compatible string representation.
    pub fn as_json_str(&self) -> &'static str {
        match self {
            ActantialRole::Helper => "\"Helper\"",
            ActantialRole::Opponent => "\"Opponent\"",
            ActantialRole::Sender => "\"Sender\"",
            ActantialRole::Receiver => "\"Receiver\"",
            ActantialRole::Unknown => "\"Unknown\"",
        }
    }
}

/// Input for content suggestion requests (domain representation).
#[derive(Debug, Clone, Default)]
pub struct SuggestionContextInput {
    pub entity_type: Option<String>,
    pub entity_name: Option<String>,
    pub world_setting: Option<String>,
    pub hints: Option<String>,
    pub additional_context: Option<String>,
    pub world_id: Option<Uuid>,
}

pub struct AiUseCases {
    pub suggestions: Arc<SuggestionOps>,
}

impl AiUseCases {
    pub fn new(suggestions: Arc<SuggestionOps>) -> Self {
        Self { suggestions }
    }
}

pub struct SuggestionOps {
    queue: Arc<dyn QueuePort>,
    world: Arc<World>,
    character: Arc<Character>,
}

impl SuggestionOps {
    pub fn new(queue: Arc<dyn QueuePort>, world: Arc<World>, character: Arc<Character>) -> Self {
        Self {
            queue,
            world,
            character,
        }
    }

    pub async fn enqueue_content_suggestion(
        &self,
        world_id: WorldId,
        suggestion_type: String,
        context: SuggestionContextInput,
    ) -> Result<SuggestionQueued, SuggestionError> {
        let world_setting = self
            .enrich_world_setting(world_id, context.world_setting.clone())
            .await?;

        let suggestion_context = SuggestionContext {
            entity_type: context.entity_type,
            entity_name: context.entity_name,
            world_setting,
            hints: context.hints,
            additional_context: context.additional_context,
            world_id: context.world_id.map(WorldId::from_uuid),
        };

        self.queue_suggestion(world_id, suggestion_type, None, Some(suggestion_context))
            .await
    }

    pub async fn cancel_content_suggestion(
        &self,
        request_id: &str,
    ) -> Result<bool, SuggestionError> {
        let cancelled = self
            .queue
            .cancel_pending_llm_request_by_callback_id(request_id)
            .await?;
        Ok(cancelled)
    }

    pub async fn suggest_want_description(
        &self,
        world_id: WorldId,
        npc_id: CharacterId,
        context: Option<String>,
    ) -> Result<SuggestionQueued, SuggestionError> {
        let world_setting = self.enrich_world_setting(world_id, None).await?;
        let suggestion_context = SuggestionContext {
            entity_type: Some("npc".to_string()),
            entity_name: self.load_npc_name(npc_id).await?,
            world_setting,
            hints: None,
            additional_context: context,
            world_id: Some(world_id),
        };

        self.queue_suggestion(
            world_id,
            "want_description".to_string(),
            Some(npc_id.to_string()),
            Some(suggestion_context),
        )
        .await
    }

    pub async fn suggest_deflection_behavior(
        &self,
        world_id: WorldId,
        npc_id: CharacterId,
        want_id: String,
        want_description: String,
    ) -> Result<SuggestionQueued, SuggestionError> {
        let world_setting = self.enrich_world_setting(world_id, None).await?;
        let extra = format!("want_id={}", want_id);
        let suggestion_context = SuggestionContext {
            entity_type: Some("npc".to_string()),
            entity_name: self.load_npc_name(npc_id).await?,
            world_setting,
            hints: Some(want_description),
            additional_context: Some(extra),
            world_id: Some(world_id),
        };

        self.queue_suggestion(
            world_id,
            "deflection_behavior".to_string(),
            Some(npc_id.to_string()),
            Some(suggestion_context),
        )
        .await
    }

    pub async fn suggest_behavioral_tells(
        &self,
        world_id: WorldId,
        npc_id: CharacterId,
        want_id: String,
        want_description: String,
    ) -> Result<SuggestionQueued, SuggestionError> {
        let world_setting = self.enrich_world_setting(world_id, None).await?;
        let extra = format!("want_id={}", want_id);
        let suggestion_context = SuggestionContext {
            entity_type: Some("npc".to_string()),
            entity_name: self.load_npc_name(npc_id).await?,
            world_setting,
            hints: Some(want_description),
            additional_context: Some(extra),
            world_id: Some(world_id),
        };

        self.queue_suggestion(
            world_id,
            "behavioral_tells".to_string(),
            Some(npc_id.to_string()),
            Some(suggestion_context),
        )
        .await
    }

    pub async fn suggest_actantial_reason(
        &self,
        world_id: WorldId,
        npc_id: CharacterId,
        want_id: String,
        target_id: String,
        role: ActantialRole,
    ) -> Result<SuggestionQueued, SuggestionError> {
        let world_setting = self.enrich_world_setting(world_id, None).await?;
        let extra = format!(
            "npc_id={}; want_id={}; target_id={}; role={}",
            npc_id,
            want_id,
            target_id,
            role.as_json_str()
        );
        let suggestion_context = SuggestionContext {
            entity_type: Some("npc".to_string()),
            entity_name: self.load_npc_name(npc_id).await?,
            world_setting,
            hints: None,
            additional_context: Some(extra),
            world_id: Some(world_id),
        };

        self.queue_suggestion(
            world_id,
            "actantial_reason".to_string(),
            Some(npc_id.to_string()),
            Some(suggestion_context),
        )
        .await
    }

    async fn queue_suggestion(
        &self,
        world_id: WorldId,
        field_type: String,
        entity_id: Option<String>,
        suggestion_context: Option<SuggestionContext>,
    ) -> Result<SuggestionQueued, SuggestionError> {
        let callback_id = Uuid::new_v4().to_string();

        let llm_request = LlmRequestData {
            request_type: LlmRequestType::Suggestion {
                field_type: field_type.clone(),
                entity_id: entity_id.clone(),
            },
            world_id,
            pc_id: None,
            prompt: None,
            suggestion_context,
            callback_id: callback_id.clone(),
            conversation_id: None,
        };

        self.queue.enqueue_llm_request(&llm_request).await?;

        Ok(SuggestionQueued {
            world_id,
            request_id: callback_id,
            field_type,
            entity_id,
        })
    }

    async fn enrich_world_setting(
        &self,
        world_id: WorldId,
        world_setting: Option<String>,
    ) -> Result<Option<String>, SuggestionError> {
        let trimmed = world_setting.as_deref().unwrap_or("").trim();
        if !trimmed.is_empty() {
            return Ok(world_setting);
        }

        let world_name = self
            .world
            .get(world_id)
            .await?
            .map(|world| world.name().as_str().to_string());
        Ok(world_name)
    }

    async fn load_npc_name(&self, npc_id: CharacterId) -> Result<Option<String>, SuggestionError> {
        let name = self
            .character
            .get(npc_id)
            .await?
            .map(|npc| npc.name().to_string());
        Ok(name)
    }
}

#[derive(Debug, Clone)]
pub struct SuggestionQueued {
    pub world_id: WorldId,
    pub request_id: String,
    pub field_type: String,
    pub entity_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SuggestionError {
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Queue error: {0}")]
    Queue(#[from] QueueError),
}
