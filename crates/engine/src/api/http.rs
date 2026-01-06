//! HTTP routes.

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
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
        // Add more routes as needed
}

async fn health() -> &'static str {
    "OK"
}

async fn list_worlds(
    State(app): State<Arc<App>>,
) -> Result<Json<Vec<wrldbldr_domain::World>>, ApiError> {
    let worlds = app.entities.world.list_all().await?;
    Ok(Json(worlds))
}

async fn get_world(
    State(app): State<Arc<App>>,
    Path(id): Path<Uuid>,
) -> Result<Json<wrldbldr_domain::World>, ApiError> {
    let world = app
        .entities
        .world
        .get(wrldbldr_domain::WorldId::from_uuid(id))
        .await?
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

#[derive(Debug)]
pub enum ApiError {
    NotFound,
    Internal(String),
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::NotFound => (axum::http::StatusCode::NOT_FOUND, "Not found"),
            ApiError::Internal(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal error"),
        };
        (status, message).into_response()
    }
}

impl From<crate::infrastructure::ports::RepoError> for ApiError {
    fn from(e: crate::infrastructure::ports::RepoError) -> Self {
        ApiError::Internal(e.to_string())
    }
}
