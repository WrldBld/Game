use std::sync::Arc;

use uuid::Uuid;

use wrldbldr_domain::{CharacterId, WorldId};

use crate::queue_types::{LlmRequestData, LlmRequestType, SuggestionContext};

use crate::infrastructure::ports::{
    CharacterRepo, LlmModelDescriptor, LlmModelsPort, QueueError, QueuePort, RepoError, SettingsRepo,
    WorldRepo,
};

// =============================================================================
// Model Resolution Use Case
// =============================================================================

/// Result of resolving the LLM model with fallback logic.
#[derive(Debug, Clone)]
pub struct ResolvedModel {
    /// The resolved model ID to use
    pub model_id: String,
    /// Where this model came from (for UI display)
    pub source: ModelSource,
    /// Optional warning if fallback was used
    pub warning: Option<String>,
}

/// Source of the resolved model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSource {
    /// DM explicitly selected this model in settings
    DmPreference,
    /// From environment variable default
    EnvDefault,
    /// First available from /v1/models
    Discovered,
}

impl std::fmt::Display for ModelSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelSource::DmPreference => write!(f, "DM preference"),
            ModelSource::EnvDefault => write!(f, "environment default"),
            ModelSource::Discovered => write!(f, "auto-discovered"),
        }
    }
}

/// Input for model resolution.
#[derive(Debug, Clone)]
pub struct ResolveModelInput {
    /// World ID for per-world settings lookup
    pub world_id: Option<WorldId>,
    /// Optional override for the env default (for testing)
    pub env_default_override: Option<String>,
}

/// Use case for resolving the LLM model with fallback logic.
///
/// Fallback order:
/// 1. DM setting (if Specific { model_id } and available)
/// 2. Environment default model (if set and available)
/// 3. First available model from /v1/models
pub struct ResolveLlmModel {
    models_port: Arc<dyn LlmModelsPort>,
    settings_repo: Arc<dyn SettingsRepo>,
}

impl ResolveLlmModel {
    pub fn new(
        models_port: Arc<dyn LlmModelsPort>,
        settings_repo: Arc<dyn SettingsRepo>,
    ) -> Self {
        Self {
            models_port,
            settings_repo,
        }
    }

    /// Resolve the model to use, following the fallback chain.
    ///
    /// Fallback order:
    /// 1. DM setting (if Specific { model_id })
    /// 2. Environment default model (if set)
    /// 3. First available model from /v1/models
    ///
    /// If /v1/models fails, falls back to DM preference or env default with a warning.
    pub async fn execute(&self, input: ResolveModelInput) -> Result<ResolvedModel, ModelResolutionError> {
        // Get settings for DM preference and env default first
        let dm_preference = self.fetch_dm_preference(input.world_id).await;
        let env_default = self.fetch_env_default(input.env_default_override).await;

        // Try to fetch available models from the provider
        let available_models_result = self.models_port.list_models().await;

        match available_models_result {
            Ok(models) if !models.is_empty() => {
                // Normal path: use available models for validation and fallback
                self.resolve_with_available_models(models, dm_preference, env_default).await
            }
            Ok(_) => {
                // Empty models list - fall back to DM/env defaults
                tracing::warn!("LLM provider returned empty model list, using fallback");
                self.resolve_without_available_models(dm_preference, env_default, "Provider returned no models").await
            }
            Err(e) => {
                // Discovery failed - fall back to DM/env defaults
                tracing::warn!(
                    error = %e,
                    "Failed to discover LLM models from provider, using fallback"
                );
                self.resolve_without_available_models(dm_preference, env_default, &e.to_string()).await
            }
        }
    }

    /// Fetch DM preference from settings.
    async fn fetch_dm_preference(&self, world_id: Option<WorldId>) -> Option<wrldbldr_shared::settings::LlmModelPreference> {
        if let Some(wid) = world_id {
            self.settings_repo
                .get_for_world(wid)
                .await
                .ok()
                .flatten()
                .map(|s| s.llm_model_preference().clone())
        } else {
            self.settings_repo
                .get_global()
                .await
                .ok()
                .flatten()
                .map(|s| s.llm_model_preference().clone())
        }
    }

    /// Fetch environment default from settings.
    async fn fetch_env_default(&self, override_value: Option<String>) -> Option<String> {
        if let Some(override_val) = override_value {
            return Some(override_val);
        }
        self.settings_repo
            .get_global()
            .await
            .ok()
            .flatten()
            .and_then(|s| s.llm_model_env_default().map(String::from))
    }

    /// Resolve model when provider list is available.
    async fn resolve_with_available_models(
        &self,
        available_models: Vec<LlmModelDescriptor>,
        dm_preference: Option<wrldbldr_shared::settings::LlmModelPreference>,
        env_default: Option<String>,
    ) -> Result<ResolvedModel, ModelResolutionError> {
        // Fallback 1: DM preference (if Specific and available)
        if let Some(preference) = &dm_preference {
            if let Some(model_id) = preference.model_id() {
                if self.model_is_available(&available_models, model_id) {
                    tracing::info!(
                        model_id = %model_id,
                        source = "dm_preference",
                        "Resolved LLM model from DM preference"
                    );
                    return Ok(ResolvedModel {
                        model_id: model_id.to_string(),
                        source: ModelSource::DmPreference,
                        warning: None,
                    });
                } else {
                    tracing::warn!(
                        requested_model = %model_id,
                        available_models = ?available_models.iter().map(|m| &m.id).collect::<Vec<_>>(),
                        "DM preferred model not available, falling back"
                    );
                }
            }
        }

        // Fallback 2: Environment default (if set and available)
        if let Some(ref env_model) = env_default {
            if self.model_is_available(&available_models, env_model) {
                let warning = dm_preference
                    .as_ref()
                    .and_then(|p| p.model_id())
                    .map(|requested| format!("DM preferred model '{}' not available, using environment default", requested));

                tracing::info!(
                    model_id = %env_model,
                    source = "env_default",
                    "Resolved LLM model from environment default"
                );
                return Ok(ResolvedModel {
                    model_id: env_model.clone(),
                    source: ModelSource::EnvDefault,
                    warning,
                });
            } else {
                tracing::warn!(
                    env_model = %env_model,
                    available_models = ?available_models.iter().map(|m| &m.id).collect::<Vec<_>>(),
                    "Environment default model not available, falling back to first available"
                );
            }
        }

        // Fallback 3: First available model
        let first_model = available_models
            .first()
            .expect("available_models is not empty");

        let warning = if let Some(pref) = dm_preference.as_ref().and_then(|p| p.model_id()) {
            Some(format!(
                "Preferred model '{}' not available, using '{}'",
                pref,
                first_model.id
            ))
        } else if let Some(ref env) = env_default {
            Some(format!(
                "Environment default '{}' not available, using '{}'",
                env,
                first_model.id
            ))
        } else {
            None
        };

        tracing::info!(
            model_id = %first_model.id,
            source = "discovered",
            "Resolved LLM model from available models list"
        );

        Ok(ResolvedModel {
            model_id: first_model.id.clone(),
            source: ModelSource::Discovered,
            warning,
        })
    }

    /// Resolve model when provider list is unavailable (fallback mode).
    async fn resolve_without_available_models(
        &self,
        dm_preference: Option<wrldbldr_shared::settings::LlmModelPreference>,
        env_default: Option<String>,
        reason: &str,
    ) -> Result<ResolvedModel, ModelResolutionError> {
        // Fallback 1: DM preference (use even if we can't validate)
        if let Some(preference) = dm_preference {
            if let Some(model_id) = preference.model_id() {
                tracing::info!(
                    model_id = %model_id,
                    source = "dm_preference",
                    reason = %reason,
                    "Resolved LLM model from DM preference (discovery unavailable)"
                );
                return Ok(ResolvedModel {
                    model_id: model_id.to_string(),
                    source: ModelSource::DmPreference,
                    warning: Some(format!("Model discovery unavailable ({}), using DM preference without validation", reason)),
                });
            }
        }

        // Fallback 2: Environment default
        if let Some(env_model) = env_default {
            tracing::info!(
                model_id = %env_model,
                source = "env_default",
                reason = %reason,
                "Resolved LLM model from environment default (discovery unavailable)"
            );
            return Ok(ResolvedModel {
                model_id: env_model,
                source: ModelSource::EnvDefault,
                warning: Some(format!("Model discovery unavailable ({}), using environment default without validation", reason)),
            });
        }

        // No fallback available
        tracing::error!(
            reason = %reason,
            "Cannot resolve LLM model: discovery failed and no fallback configured"
        );
        Err(ModelResolutionError::DiscoveryFailed(format!(
            "Failed to discover models ({}), and no DM preference or environment default configured",
            reason
        )))
    }

    /// List all available models (for UI display).
    pub async fn list_available_models(&self) -> Result<Vec<LlmModelDescriptor>, ModelResolutionError> {
        self.models_port.list_models().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to list available LLM models");
            ModelResolutionError::DiscoveryFailed(e.to_string())
        })
    }

    fn model_is_available(&self, models: &[LlmModelDescriptor], model_id: &str) -> bool {
        models.iter().any(|m| m.id == model_id)
    }
}

/// Errors that can occur during model resolution.
#[derive(Debug, thiserror::Error)]
pub enum ModelResolutionError {
    #[error("Failed to discover available models: {0}")]
    DiscoveryFailed(String),
    
    #[error("No LLM models available from provider")]
    NoModelsAvailable,
    
    #[error("Settings error: {0}")]
    Settings(#[from] crate::infrastructure::ports::RepoError),
}

// =============================================================================
// Actantial Role (existing)
// =============================================================================

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
    pub world_id: Option<WorldId>,
}

pub struct AiUseCases {
    pub suggestions: Arc<SuggestionOps>,
    pub resolve_model: Arc<ResolveLlmModel>,
}

impl AiUseCases {
    pub fn new(
        suggestions: Arc<SuggestionOps>,
        resolve_model: Arc<ResolveLlmModel>,
    ) -> Self {
        Self {
            suggestions,
            resolve_model,
        }
    }
}

pub struct SuggestionOps {
    queue: Arc<dyn QueuePort>,
    world: Arc<dyn WorldRepo>,
    character: Arc<dyn CharacterRepo>,
    settings: Arc<dyn SettingsRepo>,
}

impl SuggestionOps {
    pub fn new(
        queue: Arc<dyn QueuePort>,
        world: Arc<dyn WorldRepo>,
        character: Arc<dyn CharacterRepo>,
        settings: Arc<dyn SettingsRepo>,
    ) -> Self {
        Self {
            queue,
            world,
            character,
            settings,
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
            world_id: context.world_id,
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

        // Resolve temperature from settings at enqueue time to avoid drift
        let temperature = self.resolve_temperature_suggestion(world_id).await;

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
            temperature: Some(temperature),
            phase_meta: None, // Suggestions don't use phased pipeline
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

    /// Resolve temperature for generic content suggestions.
    ///
    /// Falls back through: per-world setting -> global setting -> default.
    async fn resolve_temperature_suggestion(&self, world_id: WorldId) -> f32 {
        // Try per-world setting first
        if let Ok(Some(settings)) = self.settings.get_for_world(world_id).await {
            return settings.llm_temperature_suggestion();
        }

        // Fall back to global setting
        if let Ok(Some(settings)) = self.settings.get_global().await {
            return settings.llm_temperature_suggestion();
        }

        // Final fallback to default
        0.8
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
