//! HTTP REST API routes
//!
//! Most CRUD operations are now handled via WebSocket Request/Response pattern.
//! This module only contains routes that require HTTP-specific features:
//! - File uploads (multipart/form-data)
//! - Large file exports
//! - Configuration endpoints
//! - Health checks

mod asset_routes;
mod export_routes;
mod prompt_template_routes;
mod queue_routes;
mod rule_system_routes;
mod settings_routes;
mod workflow_routes;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use std::sync::Arc;

use crate::infrastructure::state::AppState;

/// Create all API routes
///
/// Note: Entity CRUD operations (World, Character, Location, Region, Scene,
/// Challenge, StoryEvent, NarrativeEvent, EventChain, Goal, Want, Skill,
/// SheetTemplate, Observation, Interaction) are now handled via WebSocket.
/// See `crates/protocol/src/requests.rs` for the RequestPayload enum.
pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Export routes (large file downloads)
        .route("/api/worlds/{id}/export", get(export_routes::export_world))
        .route(
            "/api/worlds/{id}/export/raw",
            get(export_routes::export_world_raw),
        )
        // Asset Gallery routes - Characters (file uploads)
        .route(
            "/api/characters/{character_id}/gallery",
            get(asset_routes::list_character_assets),
        )
        .route(
            "/api/characters/{character_id}/gallery",
            post(asset_routes::upload_character_asset),
        )
        .route(
            "/api/characters/{character_id}/gallery/{asset_id}/activate",
            put(asset_routes::activate_character_asset),
        )
        .route(
            "/api/characters/{character_id}/gallery/{asset_id}/label",
            put(asset_routes::update_character_asset_label),
        )
        .route(
            "/api/characters/{character_id}/gallery/{asset_id}",
            delete(asset_routes::delete_character_asset),
        )
        // Asset Gallery routes - Locations (file uploads)
        .route(
            "/api/locations/{location_id}/gallery",
            get(asset_routes::list_location_assets),
        )
        .route(
            "/api/locations/{location_id}/gallery",
            post(asset_routes::upload_location_asset),
        )
        .route(
            "/api/locations/{location_id}/gallery/{asset_id}/activate",
            put(asset_routes::activate_location_asset),
        )
        .route(
            "/api/locations/{location_id}/gallery/{asset_id}",
            delete(asset_routes::delete_location_asset),
        )
        // Asset Gallery routes - Items (file uploads)
        .route(
            "/api/items/{item_id}/gallery",
            get(asset_routes::list_item_assets),
        )
        .route(
            "/api/items/{item_id}/gallery",
            post(asset_routes::upload_item_asset),
        )
        .route(
            "/api/items/{item_id}/gallery/{asset_id}/activate",
            put(asset_routes::activate_item_asset),
        )
        .route(
            "/api/items/{item_id}/gallery/{asset_id}",
            delete(asset_routes::delete_item_asset),
        )
        // Unified generation queue snapshot
        .route(
            "/api/generation/queue",
            get(queue_routes::get_generation_queue),
        )
        // Generation Queue routes
        .route("/api/assets/generate", post(asset_routes::queue_generation))
        .route(
            "/api/worlds/{world_id}/assets/queue",
            get(asset_routes::list_queue),
        )
        .route("/api/assets/ready", get(asset_routes::list_ready_batches))
        .route("/api/assets/batch/{batch_id}", get(asset_routes::get_batch))
        .route(
            "/api/assets/batch/{batch_id}/assets",
            get(asset_routes::get_batch_assets),
        )
        .route(
            "/api/assets/batch/{batch_id}/select",
            post(asset_routes::select_from_batch),
        )
        .route(
            "/api/assets/batch/{batch_id}",
            delete(asset_routes::cancel_batch),
        )
        .route(
            "/api/assets/batch/{batch_id}/retry",
            post(asset_routes::retry_batch),
        )
        // Workflow Configuration routes
        .route("/api/workflows", get(workflow_routes::list_workflow_slots))
        .route(
            "/api/workflows/{slot}",
            get(workflow_routes::get_workflow_config),
        )
        .route(
            "/api/workflows/{slot}",
            post(workflow_routes::save_workflow_config),
        )
        .route(
            "/api/workflows/{slot}",
            delete(workflow_routes::delete_workflow_config),
        )
        .route(
            "/api/workflows/{slot}/defaults",
            patch(workflow_routes::update_workflow_defaults),
        )
        .route(
            "/api/workflows/analyze",
            post(workflow_routes::analyze_workflow),
        )
        .route(
            "/api/workflows/export",
            get(workflow_routes::export_workflows),
        )
        .route(
            "/api/workflows/import",
            post(workflow_routes::import_workflows),
        )
        .route(
            "/api/workflows/{slot}/test",
            post(workflow_routes::test_workflow),
        )
        // Rule System routes (read-only presets)
        .route(
            "/api/rule-systems",
            get(rule_system_routes::list_rule_systems),
        )
        .route(
            "/api/rule-systems/{system_type}",
            get(rule_system_routes::get_rule_system),
        )
        .route(
            "/api/rule-systems/{system_type}/presets",
            get(rule_system_routes::list_presets),
        )
        .route(
            "/api/rule-systems/{system_type}/presets/{variant}",
            get(rule_system_routes::get_preset),
        )
        // Queue health check
        .merge(queue_routes::create_queue_routes())
        // Settings routes
        .merge(settings_routes::settings_routes())
        // Prompt template routes
        .merge(prompt_template_routes::prompt_template_routes())
}
