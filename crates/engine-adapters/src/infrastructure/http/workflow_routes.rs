//! Workflow configuration REST API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use wrldbldr_domain_types::{
    analyze_workflow, auto_detect_prompt_mappings, validate_workflow, WorkflowSlot,
};
use wrldbldr_engine_ports::inbound::AppStatePort;

use super::workflow_helpers::{export_configs, import_configs, prepare_workflow};
use wrldbldr_protocol::{
    parse_workflow_slot, AnalyzeWorkflowRequestDto, CreateWorkflowConfigRequestDto,
    ImportWorkflowsRequestDto, ImportWorkflowsResponseDto, TestWorkflowRequestDto,
    TestWorkflowResponseDto, UpdateWorkflowDefaultsRequestDto, WorkflowAnalysisResponseDto,
    WorkflowConfigFullResponseDto, WorkflowSlotCategoryDto, WorkflowSlotStatusDto,
    WorkflowSlotsResponseDto,
};

// Import conversion functions from adapters
use crate::infrastructure::dto_conversions::{
    workflow_config_to_full_response_dto, workflow_config_to_response_dto,
};

// ============================================================================
// Route Handlers
// ============================================================================

/// List all workflow slots grouped by category
pub async fn list_workflow_slots(
    State(state): State<Arc<dyn AppStatePort>>,
) -> Result<Json<WorkflowSlotsResponseDto>, (StatusCode, String)> {
    let configs = state
        .workflow_service()
        .list_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Group slots by category, maintaining order
    let category_order = [
        "Character Assets",
        "Location Assets",
        "Item Assets",
        "Map Assets",
    ];
    let mut categories: Vec<WorkflowSlotCategoryDto> = category_order
        .iter()
        .map(|name| WorkflowSlotCategoryDto {
            name: name.to_string(),
            slots: Vec::new(),
        })
        .collect();

    for slot in WorkflowSlot::all() {
        let config = configs.iter().find(|c| c.slot == *slot);
        let (width, height) = slot.default_dimensions();
        let category_name = slot.category();

        let status = WorkflowSlotStatusDto {
            slot: slot.as_str().to_string(),
            display_name: slot.display_name().to_string(),
            default_width: width,
            default_height: height,
            configured: config.is_some(),
            config: config.map(|c| {
                let analysis = analyze_workflow(&c.workflow_json);
                workflow_config_to_response_dto(c, &analysis)
            }),
        };

        // Find the category and add the slot
        if let Some(category) = categories.iter_mut().find(|c| c.name == category_name) {
            category.slots.push(status);
        }
    }

    Ok(Json(WorkflowSlotsResponseDto { categories }))
}

/// Get a workflow configuration by slot
pub async fn get_workflow_config(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(slot): Path<String>,
) -> Result<Json<WorkflowConfigFullResponseDto>, (StatusCode, String)> {
    let workflow_slot = parse_workflow_slot(&slot).map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    let config = state
        .workflow_service()
        .get_by_slot(workflow_slot)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("No workflow configured for slot: {}", slot),
            )
        })?;

    let analysis = analyze_workflow(&config.workflow_json);
    Ok(Json(workflow_config_to_full_response_dto(&config, analysis)))
}

/// Create or update a workflow configuration
pub async fn save_workflow_config(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(slot): Path<String>,
    Json(req): Json<CreateWorkflowConfigRequestDto>,
) -> Result<(StatusCode, Json<WorkflowConfigFullResponseDto>), (StatusCode, String)> {
    let workflow_slot = parse_workflow_slot(&slot).map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    // Validate the workflow JSON
    validate_workflow(&req.workflow_json).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Delegate entity mutation to the service
    let (config, is_update) = state
        .workflow_service()
        .create_or_update(
            workflow_slot,
            req.name,
            req.workflow_json,
            req.prompt_mappings.into_iter().map(Into::into).collect(),
            req.input_defaults.into_iter().map(Into::into).collect(),
            req.locked_inputs,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let status = if is_update {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };

    let analysis = analyze_workflow(&config.workflow_json);
    Ok((status, Json(workflow_config_to_full_response_dto(&config, analysis))))
}

/// Delete a workflow configuration
pub async fn delete_workflow_config(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(slot): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let workflow_slot = parse_workflow_slot(&slot).map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    let deleted = state
        .workflow_service()
        .delete_by_slot(workflow_slot)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            format!("No workflow configured for slot: {}", slot),
        ))
    }
}

/// Update just the defaults of a workflow configuration (without re-uploading the workflow JSON)
pub async fn update_workflow_defaults(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(slot): Path<String>,
    Json(req): Json<UpdateWorkflowDefaultsRequestDto>,
) -> Result<Json<WorkflowConfigFullResponseDto>, (StatusCode, String)> {
    let workflow_slot = parse_workflow_slot(&slot).map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    // Delegate entity mutation to the service
    let config = state
        .workflow_service()
        .update_defaults(
            workflow_slot,
            req.input_defaults.into_iter().map(Into::into).collect(),
            req.locked_inputs,
        )
        .await
        .map_err(|e| {
            // Check if it's a "not found" error
            let msg = e.to_string();
            if msg.contains("No workflow configured") {
                (StatusCode::NOT_FOUND, msg)
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        })?;

    let analysis = analyze_workflow(&config.workflow_json);
    Ok(Json(workflow_config_to_full_response_dto(&config, analysis)))
}

/// Analyze a workflow JSON without saving
pub async fn analyze_workflow_handler(
    Json(req): Json<AnalyzeWorkflowRequestDto>,
) -> Result<Json<WorkflowAnalysisResponseDto>, (StatusCode, String)> {
    // Validate first
    if let Err(e) = validate_workflow(&req.workflow_json) {
        return Err((StatusCode::BAD_REQUEST, e));
    }

    let analysis = analyze_workflow(&req.workflow_json);
    let auto_mappings = auto_detect_prompt_mappings(&req.workflow_json);

    Ok(Json(WorkflowAnalysisResponseDto {
        is_valid: analysis.is_valid(),
        analysis: analysis.into(),
        suggested_prompt_mappings: auto_mappings.into_iter().map(Into::into).collect(),
    }))
}

/// Export all workflow configurations
pub async fn export_workflows(
    State(state): State<Arc<dyn AppStatePort>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let configs = state
        .workflow_service()
        .list_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let export = export_configs(&configs, state.clock().now());
    Ok(Json(export))
}

pub async fn import_workflows(
    State(state): State<Arc<dyn AppStatePort>>,
    Json(req): Json<ImportWorkflowsRequestDto>,
) -> Result<Json<ImportWorkflowsResponseDto>, (StatusCode, String)> {
    // Parse the import data
    let configs = import_configs(&req.data)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Delegate import logic to the service
    let (imported, skipped) = state
        .workflow_service()
        .import_configs(configs, req.replace_existing)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ImportWorkflowsResponseDto { imported, skipped }))
}

pub async fn test_workflow(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(slot): Path<String>,
    Json(req): Json<TestWorkflowRequestDto>,
) -> Result<Json<TestWorkflowResponseDto>, (StatusCode, String)> {
    let workflow_slot = parse_workflow_slot(&slot).map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    let config = state
        .workflow_service()
        .get_by_slot(workflow_slot)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("No workflow configured for slot: {}", slot),
            )
        })?;

    // Prepare the workflow with the test prompt
    let prepared_workflow = prepare_workflow(
        &config,
        &req.prompt,
        req.negative_prompt.as_deref(),
        &[], // No overrides for test
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Queue the workflow with ComfyUI
    let queue_result = state
        .comfyui()
        .queue_prompt(prepared_workflow)
        .await
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;

    Ok(Json(TestWorkflowResponseDto {
        prompt_id: queue_result.prompt_id,
        queue_position: 0, // Queue position not available from ComfyUI response
    }))
}
