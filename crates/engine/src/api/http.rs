//! HTTP routes.

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::app::App;

/// Create all HTTP routes.
pub fn routes() -> Router<Arc<App>> {
    Router::new()
        .route("/", get(health))
        .route("/api/health", get(health))
        .route("/api/worlds", get(list_worlds))
        .route("/api/worlds/{id}", get(get_world))
        .route("/api/worlds/{id}/export", get(export_world))
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
        .map_err(|e| ApiError::Internal(e.to_string()))?;
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
        .map_err(|e| ApiError::Internal(e.to_string()))?
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
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(export))
}

// =============================================================================
// Settings
// =============================================================================

async fn get_settings(State(app): State<Arc<App>>) -> Result<Json<wrldbldr_domain::AppSettings>, ApiError> {
    let settings = app
        .use_cases
        .settings
        .ops
        .get_global()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(settings))
}

async fn update_settings(
    State(app): State<Arc<App>>,
    Json(settings): Json<wrldbldr_domain::AppSettings>,
) -> Result<Json<wrldbldr_domain::AppSettings>, ApiError> {
    let updated = app
        .use_cases
        .settings
        .ops
        .update_global(settings)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(updated))
}

async fn reset_settings(
    State(app): State<Arc<App>>,
) -> Result<Json<wrldbldr_domain::AppSettings>, ApiError> {
    let settings = app
        .use_cases
        .settings
        .ops
        .reset_global()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(settings))
}

async fn get_settings_metadata(
    State(app): State<Arc<App>>,
) -> Result<Json<Vec<wrldbldr_domain::SettingsFieldMetadata>>, ApiError> {
    Ok(Json(app.use_cases.settings.ops.metadata()))
}

async fn get_world_settings(
    State(app): State<Arc<App>>,
    Path(id): Path<Uuid>,
) -> Result<Json<wrldbldr_domain::AppSettings>, ApiError> {
    let settings = app
        .use_cases
        .settings
        .ops
        .get_for_world(wrldbldr_domain::WorldId::from_uuid(id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(settings))
}

async fn update_world_settings(
    State(app): State<Arc<App>>,
    Path(id): Path<Uuid>,
    Json(settings): Json<wrldbldr_domain::AppSettings>,
) -> Result<Json<wrldbldr_domain::AppSettings>, ApiError> {
    if let Some(world_id) = settings.world_id {
        if world_id != wrldbldr_domain::WorldId::from_uuid(id) {
            return Err(ApiError::BadRequest("world_id does not match path".to_string()));
        }
    }

    let updated = app
        .use_cases
        .settings
        .ops
        .update_for_world(wrldbldr_domain::WorldId::from_uuid(id), settings)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(updated))
}

async fn reset_world_settings(
    State(app): State<Arc<App>>,
    Path(id): Path<Uuid>,
) -> Result<Json<wrldbldr_domain::AppSettings>, ApiError> {
    let settings = app
        .use_cases
        .settings
        .ops
        .reset_for_world(wrldbldr_domain::WorldId::from_uuid(id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
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
                return Err(ApiError::BadRequest("Unknown rule system variant".to_string()));
            }
        }
    };

    Ok(variant)
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
            ApiError::NotFound => {
                (axum::http::StatusCode::NOT_FOUND, "Not found").into_response()
            }
            ApiError::BadRequest(msg) => {
                (axum::http::StatusCode::BAD_REQUEST, msg).into_response()
            }
            ApiError::Internal(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal error",
            )
                .into_response(),
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
            .returning(|| Ok(None));

        let app = crate::api::websocket::test_support::build_test_app(repos, Utc::now());
        routes().with_state(app)
    }

    async fn read_body_json<T: serde::de::DeserializeOwned>(
        response: axum::response::Response,
    ) -> T {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
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
        let settings: wrldbldr_domain::AppSettings = read_body_json(response).await;
        assert_eq!(settings, wrldbldr_domain::AppSettings::default());
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
        let metadata: Vec<wrldbldr_domain::SettingsFieldMetadata> =
            read_body_json(response).await;
        assert!(!metadata.is_empty());
    }

    #[tokio::test]
    async fn update_world_settings_rejects_mismatched_world_id() {
        let mut repos = TestAppRepos::new(MockWorldRepo::new());
        repos
            .settings_repo
            .expect_save_for_world()
            .times(0);

        let app = crate::api::websocket::test_support::build_test_app(repos, Utc::now());
        let router: Router = routes().with_state(app);

        let world_id = Uuid::new_v4();
        let other_world_id = Uuid::new_v4();
        let mut settings = wrldbldr_domain::AppSettings::default();
        settings.world_id = Some(wrldbldr_domain::WorldId::from_uuid(other_world_id));

        let response = router
            .into_service()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/api/worlds/{}/settings", world_id))
                    .method(axum::http::Method::PUT)
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&settings).unwrap()))
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
        assert_eq!(preset["variant"], "dnd5e");
        assert_eq!(preset["config"]["system_type"], "d20");
    }
}
