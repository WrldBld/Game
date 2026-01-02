//! Prompt Template HTTP Routes
//!
//! Provides REST API endpoints for managing configurable LLM prompt templates.
//! Mirrors the settings routes pattern.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use wrldbldr_domain::value_objects::PromptTemplateCategory;
use wrldbldr_domain::WorldId;
use wrldbldr_engine_ports::inbound::AppStatePort;
use wrldbldr_engine_ports::outbound::PromptTemplateSource;

/// Create the prompt template routes
pub fn prompt_template_routes() -> Router<Arc<dyn AppStatePort>> {
    Router::new()
        // Global prompt templates
        .route("/api/prompt-templates", get(get_prompt_templates))
        .route("/api/prompt-templates", put(update_prompt_templates))
        .route("/api/prompt-templates/reset", post(reset_prompt_templates))
        // Template metadata for UI
        .route(
            "/api/prompt-templates/metadata",
            get(get_prompt_template_metadata),
        )
        // Per-world prompt templates
        .route(
            "/api/worlds/{world_id}/prompt-templates",
            get(get_world_prompt_templates),
        )
        .route(
            "/api/worlds/{world_id}/prompt-templates",
            put(update_world_prompt_templates),
        )
        .route(
            "/api/worlds/{world_id}/prompt-templates/reset",
            post(reset_world_prompt_templates),
        )
        // Single template operations
        .route("/api/prompt-templates/{key}", get(get_prompt_template))
        .route(
            "/api/prompt-templates/{key}",
            delete(delete_prompt_template),
        )
        .route(
            "/api/worlds/{world_id}/prompt-templates/{key}",
            get(get_world_prompt_template),
        )
        .route(
            "/api/worlds/{world_id}/prompt-templates/{key}",
            delete(delete_world_prompt_template),
        )
}

// =============================================================================
// DTOs
// =============================================================================

/// A resolved prompt template as returned by the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplateDto {
    /// The template key
    pub key: String,
    /// The effective value (after priority resolution)
    pub value: String,
    /// Where this value came from
    pub source: String,
    /// Whether this template has an override (world or global)
    pub is_overridden: bool,
    /// The hard-coded default value
    pub default_value: String,
}

/// Request to update prompt templates (bulk update)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePromptTemplatesRequest {
    /// Map of key -> value for templates to update
    pub templates: Vec<PromptTemplateUpdate>,
}

/// A single template update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplateUpdate {
    /// Template key
    pub key: String,
    /// New value (or null to delete override)
    pub value: Option<String>,
}

/// Response containing all templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplatesResponse {
    /// All resolved templates
    pub templates: Vec<PromptTemplateDto>,
}

/// Metadata response with category grouping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplateMetadataResponse {
    /// All template metadata grouped by category
    pub categories: Vec<PromptTemplateCategoryGroup>,
}

/// A category group of templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplateCategoryGroup {
    /// Category identifier
    pub category: String,
    /// Human-readable category name
    pub display_name: String,
    /// Templates in this category
    pub templates: Vec<PromptTemplateMetadataDto>,
}

/// Template metadata for UI rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplateMetadataDto {
    /// Template key
    pub key: String,
    /// Human-readable label
    pub label: String,
    /// Description of what this template is used for
    pub description: String,
    /// Environment variable name for override
    pub env_var: String,
}

// =============================================================================
// Global Prompt Templates
// =============================================================================

/// Get all prompt templates (global context)
async fn get_prompt_templates(
    State(state): State<Arc<dyn AppStatePort>>,
) -> Json<PromptTemplatesResponse> {
    let resolved = state.prompt_template_use_case().get_all().await;

    let templates: Vec<PromptTemplateDto> = resolved
        .into_iter()
        .map(|r| PromptTemplateDto {
            key: r.key,
            value: r.value,
            source: r.source.as_str().to_string(),
            is_overridden: r.source != PromptTemplateSource::Default,
            default_value: r.default_value,
        })
        .collect();

    Json(PromptTemplatesResponse { templates })
}

/// Update prompt templates (global)
async fn update_prompt_templates(
    State(state): State<Arc<dyn AppStatePort>>,
    Json(request): Json<UpdatePromptTemplatesRequest>,
) -> Result<Json<PromptTemplatesResponse>, (StatusCode, String)> {
    for update in &request.templates {
        match &update.value {
            Some(value) => {
                state
                    .prompt_template_use_case()
                    .set_global(&update.key, value)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
            None => {
                // Delete the override
                state
                    .prompt_template_use_case()
                    .delete_global(&update.key)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
        }
    }

    // Return updated templates
    let resolved = state.prompt_template_use_case().get_all().await;
    let templates: Vec<PromptTemplateDto> = resolved
        .into_iter()
        .map(|r| PromptTemplateDto {
            key: r.key,
            value: r.value,
            source: r.source.as_str().to_string(),
            is_overridden: r.source != PromptTemplateSource::Default,
            default_value: r.default_value,
        })
        .collect();

    Ok(Json(PromptTemplatesResponse { templates }))
}

/// Reset all global prompt template overrides
async fn reset_prompt_templates(
    State(state): State<Arc<dyn AppStatePort>>,
) -> Result<Json<PromptTemplatesResponse>, (StatusCode, String)> {
    state
        .prompt_template_use_case()
        .reset_global()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return templates (now all defaults/env)
    let resolved = state.prompt_template_use_case().get_all().await;
    let templates: Vec<PromptTemplateDto> = resolved
        .into_iter()
        .map(|r| PromptTemplateDto {
            key: r.key,
            value: r.value,
            source: r.source.as_str().to_string(),
            is_overridden: r.source != PromptTemplateSource::Default,
            default_value: r.default_value,
        })
        .collect();

    Ok(Json(PromptTemplatesResponse { templates }))
}

/// Get a single prompt template (global context)
async fn get_prompt_template(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(key): Path<String>,
) -> Json<PromptTemplateDto> {
    let resolved = state
        .prompt_template_use_case()
        .resolve_with_source(&key)
        .await;

    Json(PromptTemplateDto {
        key: resolved.key,
        value: resolved.value,
        source: resolved.source.as_str().to_string(),
        is_overridden: resolved.source != PromptTemplateSource::Default,
        default_value: resolved.default_value,
    })
}

/// Delete a single global prompt template override
async fn delete_prompt_template(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(key): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .prompt_template_use_case()
        .delete_global(&key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// Template Metadata
// =============================================================================

/// Get template metadata for UI rendering
async fn get_prompt_template_metadata(
    State(state): State<Arc<dyn AppStatePort>>,
) -> Json<PromptTemplateMetadataResponse> {
    let metadata = state.prompt_template_use_case().get_metadata();

    // Group by category
    let mut categories: std::collections::HashMap<
        PromptTemplateCategory,
        Vec<PromptTemplateMetadataDto>,
    > = std::collections::HashMap::new();

    for m in metadata {
        categories
            .entry(m.category)
            .or_default()
            .push(PromptTemplateMetadataDto {
                key: m.key,
                label: m.label,
                description: m.description,
                env_var: m.env_var,
            });
    }

    let category_groups: Vec<PromptTemplateCategoryGroup> = vec![
        PromptTemplateCategory::Dialogue,
        PromptTemplateCategory::Staging,
        PromptTemplateCategory::Outcomes,
        PromptTemplateCategory::Suggestions,
        PromptTemplateCategory::Summarization,
    ]
    .into_iter()
    .filter_map(|cat| {
        categories
            .get(&cat)
            .map(|templates| PromptTemplateCategoryGroup {
                category: cat.as_str().to_string(),
                display_name: cat.display_name().to_string(),
                templates: templates.clone(),
            })
    })
    .collect();

    Json(PromptTemplateMetadataResponse {
        categories: category_groups,
    })
}

// =============================================================================
// Per-World Prompt Templates
// =============================================================================

/// Get all prompt templates for a specific world
async fn get_world_prompt_templates(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(world_id): Path<String>,
) -> Result<Json<PromptTemplatesResponse>, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid world ID format".to_string(),
        )
    })?;
    let world_id = WorldId::from_uuid(world_uuid);

    let resolved = state
        .prompt_template_use_case()
        .get_all_for_world(world_id)
        .await;

    let templates: Vec<PromptTemplateDto> = resolved
        .into_iter()
        .map(|r| PromptTemplateDto {
            key: r.key,
            value: r.value,
            source: r.source.as_str().to_string(),
            is_overridden: r.source != PromptTemplateSource::Default,
            default_value: r.default_value,
        })
        .collect();

    Ok(Json(PromptTemplatesResponse { templates }))
}

/// Update prompt templates for a specific world
async fn update_world_prompt_templates(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(world_id): Path<String>,
    Json(request): Json<UpdatePromptTemplatesRequest>,
) -> Result<Json<PromptTemplatesResponse>, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid world ID format".to_string(),
        )
    })?;
    let world_id = WorldId::from_uuid(world_uuid);

    for update in &request.templates {
        match &update.value {
            Some(value) => {
                state
                    .prompt_template_use_case()
                    .set_for_world(world_id, &update.key, value)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
            None => {
                state
                    .prompt_template_use_case()
                    .delete_for_world(world_id, &update.key)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
        }
    }

    // Return updated templates
    let resolved = state
        .prompt_template_use_case()
        .get_all_for_world(world_id)
        .await;
    let templates: Vec<PromptTemplateDto> = resolved
        .into_iter()
        .map(|r| PromptTemplateDto {
            key: r.key,
            value: r.value,
            source: r.source.as_str().to_string(),
            is_overridden: r.source != PromptTemplateSource::Default,
            default_value: r.default_value,
        })
        .collect();

    Ok(Json(PromptTemplatesResponse { templates }))
}

/// Reset all world-specific prompt template overrides
async fn reset_world_prompt_templates(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(world_id): Path<String>,
) -> Result<Json<PromptTemplatesResponse>, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid world ID format".to_string(),
        )
    })?;
    let world_id = WorldId::from_uuid(world_uuid);

    state
        .prompt_template_use_case()
        .reset_for_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return templates (now global/env/defaults)
    let resolved = state
        .prompt_template_use_case()
        .get_all_for_world(world_id)
        .await;
    let templates: Vec<PromptTemplateDto> = resolved
        .into_iter()
        .map(|r| PromptTemplateDto {
            key: r.key,
            value: r.value,
            source: r.source.as_str().to_string(),
            is_overridden: r.source != PromptTemplateSource::Default,
            default_value: r.default_value,
        })
        .collect();

    Ok(Json(PromptTemplatesResponse { templates }))
}

/// Get a single prompt template for a specific world
async fn get_world_prompt_template(
    State(state): State<Arc<dyn AppStatePort>>,
    Path((world_id, key)): Path<(String, String)>,
) -> Result<Json<PromptTemplateDto>, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid world ID format".to_string(),
        )
    })?;
    let world_id = WorldId::from_uuid(world_uuid);

    let resolved = state
        .prompt_template_use_case()
        .resolve_for_world_with_source(world_id, &key)
        .await;

    Ok(Json(PromptTemplateDto {
        key: resolved.key,
        value: resolved.value,
        source: resolved.source.as_str().to_string(),
        is_overridden: resolved.source != PromptTemplateSource::Default,
        default_value: resolved.default_value,
    }))
}

/// Delete a world-specific prompt template override
async fn delete_world_prompt_template(
    State(state): State<Arc<dyn AppStatePort>>,
    Path((world_id, key)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid world ID format".to_string(),
        )
    })?;
    let world_id = WorldId::from_uuid(world_uuid);

    state
        .prompt_template_use_case()
        .delete_for_world(world_id, &key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
