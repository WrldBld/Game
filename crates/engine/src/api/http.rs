//! HTTP routes.

use axum::{
    extract::{Path, State, Query},
    routing::{get, post, put},
    http::StatusCode,
    Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::WorldId;

use crate::app::App;
use crate::infrastructure::app_settings::AppSettings;
use crate::infrastructure::ports::RepoError;
use crate::use_cases::management::ManagementError;
use crate::use_cases::prompt_templates::PromptTemplateError;
use crate::use_cases::settings::SettingsError;
use crate::use_cases::world::WorldError;

/// Maximum HTTP request body size (10MB).
/// This should be enforced by middleware in production. For now, individual endpoints should validate their payloads.
const MAX_HTTP_BODY_SIZE: usize = 10 * 1024 * 1024;

// =============================================================================
// Prompt Template DTOs (wire format)
// =============================================================================

/// Information about a prompt template (metadata).
#[derive(serde::Serialize)]
struct TemplateInfo {
    key: String,
    label: String,
    description: String,
    category: String,
    default_value: String,
    env_var: String,
}

/// Information about a template override value.
#[derive(serde::Serialize)]
struct TemplateOverrideInfo {
    key: String,
    value: String,
}

/// Request to set a template override.
#[derive(serde::Serialize, serde::Deserialize)]
struct SetOverrideRequest {
    value: String,
}

/// Query parameters for template resolution.
#[derive(serde::Deserialize)]
struct ResolveParams {
    world_id: Option<Uuid>,
}

/// Resolved template value with override information.
#[derive(serde::Serialize, serde::Deserialize)]
struct ResolvedTemplate {
    key: String,
    value: String,
    /// Whether this is a world-specific override
    is_override: bool,
    /// Default value (for reset functionality)
    default_value: String,
}

/// Create all HTTP routes.
pub fn routes() -> Router<Arc<App>> {
    Router::new()
        .route("/", get(health))
        .route("/api/health", get(health))
        .route("/api/worlds", get(list_worlds))
        .route("/api/worlds/{id}", get(get_world))
        .route("/api/worlds/{id}/export", get(export_world))
        .route("/api/worlds/import", post(import_world))
        .route("/api/settings", get(get_settings).put(update_settings))
        .route("/api/settings/reset", post(reset_settings))
        .route("/api/settings/metadata", get(get_settings_metadata))
        .route(
            "/api/worlds/{id}/settings",
            get(get_world_settings).put(update_world_settings),
        )
        .route(
            "/api/worlds/{id}/settings/reset",
            post(reset_world_settings),
        )
        .route(
            "/api/rule-systems/{system_type}/presets/{variant}",
            get(get_rule_system_preset),
        )
        // Prompt template management
        .route("/api/prompt-templates", get(list_templates))
        .route("/api/prompt-templates/global", get(list_global_overrides))
        .route("/api/prompt-templates/global/{key}", put(set_global_override).delete(delete_global_override))
        .route("/api/prompt-templates/world/{world_id}", get(list_world_overrides))
        .route(
            "/api/prompt-templates/world/{world_id}/{key}",
            put(set_world_override).delete(delete_world_override),
        )
        .route("/api/prompt-templates/resolve/{key}", get(resolve_template))
    // Add more routes as needed
}

async fn health() -> &'static str {
    "OK"
}

async fn list_worlds(
    State(app): State<Arc<App>>,
) -> Result<Json<Vec<wrldbldr_domain::World>>, ApiError> {
    let worlds = app
        .use_cases
        .management
        .world
        .list()
        .await
        .map_err(map_management_error)?;
    Ok(Json(worlds))
}

async fn get_world(
    State(app): State<Arc<App>>,
    Path(id): Path<Uuid>,
) -> Result<Json<wrldbldr_domain::World>, ApiError> {
    let world = app
        .use_cases
        .management
        .world
        .get(wrldbldr_domain::WorldId::from_uuid(id))
        .await
        .map_err(map_management_error)?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(world))
}

async fn export_world(
    State(app): State<Arc<App>>,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::use_cases::world::WorldExport>, ApiError> {
    let export = app
        .use_cases
        .world
        .export
        .execute(wrldbldr_domain::WorldId::from_uuid(id))
        .await
        .map_err(map_world_error)?;
    Ok(Json(export))
}

async fn import_world(
    State(app): State<Arc<App>>,
    Json(payload): Json<crate::use_cases::world::WorldExport>,
) -> Result<Json<ImportWorldResponse>, ApiError> {
    let world_id = app
        .use_cases
        .world
        .import
        .execute(payload)
        .await
        .map_err(map_world_error)?;
    Ok(Json(ImportWorldResponse {
        id: world_id.to_string(),
    }))
}

// =============================================================================
// Settings
// =============================================================================

async fn get_settings(State(app): State<Arc<App>>) -> Result<Json<AppSettings>, ApiError> {
    let settings = app
        .use_cases
        .settings
        .get_global()
        .await
        .map_err(map_settings_error)?;
    Ok(Json(settings))
}

async fn update_settings(
    State(app): State<Arc<App>>,
    Json(settings): Json<AppSettings>,
) -> Result<Json<AppSettings>, ApiError> {
    let updated = app
        .use_cases
        .settings
        .update_global(settings)
        .await
        .map_err(map_settings_error)?;
    Ok(Json(updated))
}

async fn reset_settings(State(app): State<Arc<App>>) -> Result<Json<AppSettings>, ApiError> {
    let settings = app
        .use_cases
        .settings
        .reset_global()
        .await
        .map_err(map_settings_error)?;
    Ok(Json(settings))
}

async fn get_settings_metadata(
    State(app): State<Arc<App>>,
) -> Result<Json<Vec<wrldbldr_shared::settings::SettingsFieldMetadata>>, ApiError> {
    Ok(Json(app.use_cases.settings.metadata()))
}

async fn get_world_settings(
    State(app): State<Arc<App>>,
    Path(id): Path<Uuid>,
) -> Result<Json<AppSettings>, ApiError> {
    let settings = app
        .use_cases
        .settings
        .get_for_world(wrldbldr_domain::WorldId::from_uuid(id))
        .await
        .map_err(map_settings_error)?;
    Ok(Json(settings))
}

async fn update_world_settings(
    State(app): State<Arc<App>>,
    Path(id): Path<Uuid>,
    Json(settings): Json<AppSettings>,
) -> Result<Json<AppSettings>, ApiError> {
    if let Some(world_id) = settings.world_id() {
        if world_id != wrldbldr_domain::WorldId::from_uuid(id) {
            return Err(ApiError::BadRequest(
                "world_id does not match path".to_string(),
            ));
        }
    }

    let updated = app
        .use_cases
        .settings
        .update_for_world(wrldbldr_domain::WorldId::from_uuid(id), settings)
        .await
        .map_err(map_settings_error)?;
    Ok(Json(updated))
}

async fn reset_world_settings(
    State(app): State<Arc<App>>,
    Path(id): Path<Uuid>,
) -> Result<Json<AppSettings>, ApiError> {
    let settings = app
        .use_cases
        .settings
        .reset_for_world(wrldbldr_domain::WorldId::from_uuid(id))
        .await
        .map_err(map_settings_error)?;
    Ok(Json(settings))
}

// =============================================================================
// Rule System Presets
// =============================================================================

#[derive(serde::Serialize)]
struct RuleSystemPresetDetails {
    variant: wrldbldr_domain::RuleSystemVariant,
    config: wrldbldr_domain::RuleSystemConfig,
}

async fn get_rule_system_preset(
    Path((system_type, variant)): Path<(String, String)>,
) -> Result<Json<RuleSystemPresetDetails>, ApiError> {
    let system_type = parse_rule_system_type(&system_type)?;
    let variant = parse_rule_system_variant(&variant)?;

    if system_type != wrldbldr_domain::RuleSystemType::Custom
        && variant.system_type() != system_type
    {
        return Err(ApiError::BadRequest(format!(
            "Variant {:?} does not belong to system type {:?}",
            variant, system_type
        )));
    }

    let config = wrldbldr_domain::RuleSystemConfig::from_variant(variant.clone());
    Ok(Json(RuleSystemPresetDetails { variant, config }))
}

fn parse_rule_system_type(value: &str) -> Result<wrldbldr_domain::RuleSystemType, ApiError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "d20" => Ok(wrldbldr_domain::RuleSystemType::D20),
        "d100" => Ok(wrldbldr_domain::RuleSystemType::D100),
        "narrative" => Ok(wrldbldr_domain::RuleSystemType::Narrative),
        "custom" => Ok(wrldbldr_domain::RuleSystemType::Custom),
        _ => Err(ApiError::BadRequest("Unknown rule system type".to_string())),
    }
}

fn parse_rule_system_variant(value: &str) -> Result<wrldbldr_domain::RuleSystemVariant, ApiError> {
    let raw = value.trim();
    let lower = raw.to_ascii_lowercase().replace('_', "");
    let variant = match lower.as_str() {
        "dnd5e" => wrldbldr_domain::RuleSystemVariant::Dnd5e,
        "pathfinder2e" => wrldbldr_domain::RuleSystemVariant::Pathfinder2e,
        "genericd20" => wrldbldr_domain::RuleSystemVariant::GenericD20,
        "callofcthulhu7e" => wrldbldr_domain::RuleSystemVariant::CallOfCthulhu7e,
        "runequest" => wrldbldr_domain::RuleSystemVariant::RuneQuest,
        "genericd100" => wrldbldr_domain::RuleSystemVariant::GenericD100,
        "kidsonbikes" => wrldbldr_domain::RuleSystemVariant::KidsOnBikes,
        "fatecore" => wrldbldr_domain::RuleSystemVariant::FateCore,
        "poweredbyapocalypse" => wrldbldr_domain::RuleSystemVariant::PoweredByApocalypse,
        "bladesinthedark" => wrldbldr_domain::RuleSystemVariant::BladesInTheDark,
        "custom" => wrldbldr_domain::RuleSystemVariant::Custom("Custom".to_string()),
        _ => {
            if lower.starts_with("custom(") && lower.ends_with(')') {
                let inner = raw
                    .trim_start_matches("Custom(")
                    .trim_start_matches("custom(")
                    .trim_end_matches(')')
                    .trim_matches('"')
                    .trim();
                if inner.is_empty() {
                    return Err(ApiError::BadRequest(
                        "Custom variant requires a name".to_string(),
                    ));
                }
                wrldbldr_domain::RuleSystemVariant::Custom(inner.to_string())
            } else {
                return Err(ApiError::BadRequest(
                    "Unknown rule system variant".to_string(),
                ));
            }
        }
    };

    Ok(variant)
}

#[derive(serde::Serialize)]
struct ImportWorldResponse {
    id: String,
}

// =============================================================================
// Prompt Template Management
// =============================================================================

async fn list_templates(
    State(app): State<Arc<App>>,
) -> Result<Json<Vec<TemplateInfo>>, ApiError> {
    let templates = app
        .use_cases
        .prompt_templates
        .all_template_metadata()
        .into_iter()
        .map(|m| TemplateInfo {
            key: m.key,
            label: m.label,
            description: m.description,
            category: m.category.as_str().to_string(),
            default_value: m.default_value,
            env_var: m.env_var,
        })
        .collect();
    Ok(Json(templates))
}

async fn list_global_overrides(
    State(app): State<Arc<App>>,
) -> Result<Json<Vec<TemplateOverrideInfo>>, ApiError> {
    let overrides = app
        .use_cases
        .prompt_templates
        .list_global_overrides()
        .await
        .map_err(map_prompt_template_error)?
        .into_iter()
        .map(|o| TemplateOverrideInfo {
            key: o.key,
            value: o.value,
        })
        .collect();
    Ok(Json(overrides))
}

async fn set_global_override(
    State(app): State<Arc<App>>,
    Path(key): Path<String>,
    Json(req): Json<SetOverrideRequest>,
) -> Result<StatusCode, ApiError> {
    app.use_cases
        .prompt_templates
        .set_global_override(key, req.value)
        .await
        .map_err(map_prompt_template_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_global_override(
    State(app): State<Arc<App>>,
    Path(key): Path<String>,
) -> Result<StatusCode, ApiError> {
    app.use_cases
        .prompt_templates
        .delete_global_override(key)
        .await
        .map_err(map_prompt_template_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_world_overrides(
    State(app): State<Arc<App>>,
    Path(world_id): Path<Uuid>,
) -> Result<Json<Vec<TemplateOverrideInfo>>, ApiError> {
    let overrides = app
        .use_cases
        .prompt_templates
        .list_world_overrides(WorldId::from_uuid(world_id))
        .await
        .map_err(map_prompt_template_error)?
        .into_iter()
        .map(|o| TemplateOverrideInfo {
            key: o.key,
            value: o.value,
        })
        .collect();
    Ok(Json(overrides))
}

async fn set_world_override(
    State(app): State<Arc<App>>,
    Path((world_id, key)): Path<(Uuid, String)>,
    Json(req): Json<SetOverrideRequest>,
) -> Result<StatusCode, ApiError> {
    app.use_cases
        .prompt_templates
        .set_world_override(WorldId::from_uuid(world_id), key, req.value)
        .await
        .map_err(map_prompt_template_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_world_override(
    State(app): State<Arc<App>>,
    Path((world_id, key)): Path<(Uuid, String)>,
) -> Result<StatusCode, ApiError> {
    app.use_cases
        .prompt_templates
        .delete_world_override(WorldId::from_uuid(world_id), key)
        .await
        .map_err(map_prompt_template_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn resolve_template(
    State(app): State<Arc<App>>,
    Path(key): Path<String>,
    Query(params): Query<ResolveParams>,
) -> Result<Json<ResolvedTemplate>, ApiError> {
    let world_id = params.world_id.map(WorldId::from_uuid);

    // Get the resolved value
    let value = app
        .use_cases
        .prompt_templates
        .resolve_template(world_id, &key)
        .await
        .map_err(map_prompt_template_error)?
        .ok_or(ApiError::NotFound)?;

    // Get metadata for default value
    let metadata = app
        .use_cases
        .prompt_templates
        .get_template_metadata(&key)
        .ok_or(ApiError::NotFound)?;

    // Check if this is a world override by comparing with global value
    let is_override = if let Some(wid) = world_id {
        let world_value = app
            .use_cases
            .prompt_templates
            .resolve_template(Some(wid), &key)
            .await
            .map_err(map_prompt_template_error)?;
        let global_value = app
            .use_cases
            .prompt_templates
            .resolve_template(None, &key)
            .await
            .map_err(map_prompt_template_error)?;
        world_value != global_value
    } else {
        false
    };

    Ok(Json(ResolvedTemplate {
        key,
        value,
        is_override,
        default_value: metadata.default_value,
    }))
}

fn map_prompt_template_error(e: PromptTemplateError) -> ApiError {
    match e {
        PromptTemplateError::Repo(RepoError::NotFound { .. }) | PromptTemplateError::NotFound(_) => {
            ApiError::NotFound
        }
        PromptTemplateError::UnknownKey(_) => ApiError::NotFound,
        e => {
            tracing::error!(error = %e, "Prompt template operation failed");
            ApiError::Internal(e.to_string())
        }
    }
}

// =============================================================================
// Error Mapping Helpers
// =============================================================================

fn map_management_error(e: ManagementError) -> ApiError {
    match e {
        ManagementError::NotFound { .. } => ApiError::NotFound,
        ManagementError::InvalidInput(msg) => ApiError::BadRequest(msg),
        ManagementError::Domain(ref de) => {
            if matches!(de, wrldbldr_domain::DomainError::Validation(_)) {
                ApiError::BadRequest(e.to_string())
            } else {
                tracing::error!(error = %e, "Management operation failed");
                ApiError::Internal(e.to_string())
            }
        }
        e => {
            tracing::error!(error = %e, "Management operation failed");
            ApiError::Internal(e.to_string())
        }
    }
}

fn map_world_error(e: WorldError) -> ApiError {
    match e {
        WorldError::NotFound => ApiError::NotFound,
        WorldError::ExportFailed(msg) | WorldError::ImportFailed(msg) => ApiError::BadRequest(msg),
        e => {
            tracing::error!(error = %e, "World operation failed");
            ApiError::Internal(e.to_string())
        }
    }
}

fn map_settings_error(e: SettingsError) -> ApiError {
    match e {
        SettingsError::Repo(RepoError::NotFound { .. }) => ApiError::NotFound,
        SettingsError::Repo(RepoError::ConstraintViolation(msg)) => ApiError::BadRequest(msg),
        e => {
            tracing::error!(error = %e, "Settings operation failed");
            ApiError::Internal(e.to_string())
        }
    }
}

#[cfg(test)]
mod prompt_template_tests {
    use super::*;
    use axum::body::Body;
    use crate::api::websocket::test_support::build_test_app;
    use crate::infrastructure::ports::MockPromptTemplateRepo;
    use chrono::Utc;
    use tower::ServiceExt;

    // Helper function to read JSON body (shared with main tests module)
    async fn read_body_json<T: serde::de::DeserializeOwned>(
        response: axum::response::Response,
    ) -> T {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("test helper: failed to read response body");
        serde_json::from_slice(&body).expect("test helper: failed to parse JSON response")
    }

    #[tokio::test]
    async fn list_templates_returns_entries() {
        let prompt_repo = MockPromptTemplateRepo::new();
        let mut repos = TestAppRepos::new(MockWorldRepo::new());
        repos.prompt_templates = Some(prompt_repo);

        let app = build_test_app(repos, Utc::now());
        let router: Router = routes().with_state(app);

        let response = router
            .into_service()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/prompt-templates")
                    .method(axum::http::Method::GET)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let templates: Vec<TemplateInfo> = read_body_json(response).await;
        assert!(!templates.is_empty());
    }

    #[tokio::test]
    async fn resolve_template_returns_value() {
        let mut prompt_repo = MockPromptTemplateRepo::new();
        prompt_repo
            .expect_resolve_template()
            .returning(|_, _| Ok(Some("resolved value".to_string())));

        let mut repos = TestAppRepos::new(MockWorldRepo::new());
        repos.prompt_templates = Some(prompt_repo);

        let app = build_test_app(repos, Utc::now());
        let router: Router = routes().with_state(app);

        let response = router
            .into_service()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/prompt-templates/resolve/dialogue.response_format")
                    .method(axum::http::Method::GET)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let template: ResolvedTemplate = read_body_json(response).await;
        assert_eq!(template.key, "dialogue.response_format");
        assert_eq!(template.value, "resolved value");
        assert_eq!(template.is_override, false);
        assert!(!template.default_value.is_empty());
    }

    #[tokio::test]
    async fn resolve_template_returns_404_when_not_found() {
        let mut prompt_repo = MockPromptTemplateRepo::new();
        prompt_repo
            .expect_resolve_template()
            .returning(|_, _| Ok(None));

        let mut repos = TestAppRepos::new(MockWorldRepo::new());
        repos.prompt_templates = Some(prompt_repo);

        let app = build_test_app(repos, Utc::now());
        let router: Router = routes().with_state(app);

        let response = router
            .into_service()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/prompt-templates/resolve/unknown.key")
                    .method(axum::http::Method::GET)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn set_global_override_works() {
        let mut prompt_repo = MockPromptTemplateRepo::new();
        prompt_repo
            .expect_set_global_override()
            .returning(|_, _| Ok(()));

        let mut repos = TestAppRepos::new(MockWorldRepo::new());
        repos.prompt_templates = Some(prompt_repo);

        let app = build_test_app(repos, Utc::now());
        let router: Router = routes().with_state(app);

        let payload = SetOverrideRequest {
            value: "custom format".to_string(),
        };

        let response = router
            .into_service()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/prompt-templates/global/dialogue.response_format")
                    .method(axum::http::Method::PUT)
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&payload).expect("test: payload should serialize"),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // PUT endpoint returns 204 No Content, not 200 OK
        assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);
    }
}

#[derive(Debug)]
pub enum ApiError {
    NotFound,
    BadRequest(String),
    Internal(String),
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiError::NotFound => (axum::http::StatusCode::NOT_FOUND, "Not found").into_response(),
            ApiError::BadRequest(msg) => {
                tracing::warn!(error = %msg, "Bad request");
                (axum::http::StatusCode::BAD_REQUEST, msg).into_response()
            }
            ApiError::Internal(msg) => {
                tracing::error!(error = %msg, "Internal API error");
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal error",
                )
                    .into_response()
            }
        }
    }
}

impl From<crate::infrastructure::ports::RepoError> for ApiError {
    fn from(e: crate::infrastructure::ports::RepoError) -> Self {
        ApiError::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use chrono::Utc;
    use tower::ServiceExt;

    use crate::api::websocket::test_support::TestAppRepos;
    use crate::infrastructure::ports::MockWorldRepo;

    fn build_router_with_repos(mut repos: TestAppRepos) -> Router {
        repos
            .settings_repo
            .expect_get_global()
            .returning(|| Ok(None))
            .times(0..=1);

        let app = crate::api::websocket::test_support::build_test_app(repos, Utc::now());
        routes().with_state(app)
    }

    async fn read_body_json<T: serde::de::DeserializeOwned>(
        response: axum::response::Response,
    ) -> T {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("test helper: failed to read response body");
        serde_json::from_slice(&body).expect("test helper: failed to parse JSON response")
    }

    #[tokio::test]
    async fn get_settings_returns_defaults() {
        let repos = TestAppRepos::new(MockWorldRepo::new());
        let router = build_router_with_repos(repos);

        let response = router
            .into_service()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/settings")
                    .method(axum::http::Method::GET)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let settings: AppSettings = read_body_json(response).await;
        assert_eq!(settings, AppSettings::default());
    }

    #[tokio::test]
    async fn get_settings_metadata_returns_entries() {
        let repos = TestAppRepos::new(MockWorldRepo::new());
        let router = build_router_with_repos(repos);

        let response = router
            .into_service()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/settings/metadata")
                    .method(axum::http::Method::GET)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let metadata: Vec<wrldbldr_shared::settings::SettingsFieldMetadata> =
            read_body_json(response).await;
        assert!(!metadata.is_empty());
    }

    #[tokio::test]
    async fn update_world_settings_rejects_mismatched_world_id() {
        let mut repos = TestAppRepos::new(MockWorldRepo::new());
        repos.settings_repo.expect_save_for_world().times(0);

        let app = crate::api::websocket::test_support::build_test_app(repos, Utc::now());
        let router: Router = routes().with_state(app);

        let world_id = Uuid::new_v4();
        let other_world_id = Uuid::new_v4();
        let settings = AppSettings::default();
        let settings =
            settings.with_world_id(Some(wrldbldr_domain::WorldId::from_uuid(other_world_id)));

        let response = router
            .into_service()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/api/worlds/{}/settings", world_id))
                    .method(axum::http::Method::PUT)
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&settings).expect("test: settings should serialize"),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_rule_system_preset_returns_config() {
        let repos = TestAppRepos::new(MockWorldRepo::new());
        let router = build_router_with_repos(repos);

        let response = router
            .into_service()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/rule-systems/D20/presets/Dnd5e")
                    .method(axum::http::Method::GET)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let preset: serde_json::Value = read_body_json(response).await;
        assert_eq!(preset["variant"], "Dnd5e");
        assert_eq!(preset["config"]["system_type"], "D20");
    }

    #[tokio::test]
    async fn import_world_returns_id() {
        let mut world_repo = MockWorldRepo::new();
        world_repo.expect_save().times(1).returning(|_| Ok(()));

        let repos = TestAppRepos::new(world_repo);
        let router = build_router_with_repos(repos);

        let now = Utc::now();
        let mut world = wrldbldr_domain::World::new(
            wrldbldr_domain::WorldName::new("Test World").unwrap(),
            now,
        );
        let _ = world.set_description(wrldbldr_domain::Description::new("Desc").unwrap(), now);
        let export = crate::use_cases::world::WorldExport {
            world: world.clone(),
            locations: Vec::new(),
            regions: Vec::new(),
            characters: Vec::new(),
            items: Vec::new(),
            narrative_events: Vec::new(),
            format_version: 1,
        };

        let response = router
            .into_service()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/worlds/import")
                    .method(axum::http::Method::POST)
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&export).expect("test: export should serialize"),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let payload: serde_json::Value = read_body_json(response).await;
        assert_eq!(payload["id"], world.id().to_string());
    }

}
