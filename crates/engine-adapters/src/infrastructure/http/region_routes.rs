//! Region API routes
//!
//! Endpoints for managing regions within locations.
//! Regions are sub-areas within a Location, each with their own backdrop.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use wrldbldr_engine_app::application::dto::{CharacterResponseDto, CreateRegionRequestDto, MapBoundsDto, RegionResponseDto};
use wrldbldr_domain::entities::{MapBounds, Region, RegionConnection, RegionExit};
use crate::infrastructure::state::AppState;
use wrldbldr_domain::{
    LocationId, RegionId, RegionRelationshipType, RegionShift, WorldId,
};

// =============================================================================
// Request/Response DTOs specific to routes
// =============================================================================

#[derive(Debug, Serialize)]
pub struct SpawnPointResponseDto {
    pub region: RegionResponseDto,
    pub location_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateRegionConnectionRequestDto {
    pub to_region_id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_bidirectional")]
    pub bidirectional: bool,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub lock_description: Option<String>,
}

fn default_bidirectional() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct RegionConnectionResponseDto {
    pub from_region_id: String,
    pub to_region_id: String,
    pub description: Option<String>,
    pub bidirectional: bool,
    pub is_locked: bool,
    pub lock_description: Option<String>,
}

impl From<RegionConnection> for RegionConnectionResponseDto {
    fn from(c: RegionConnection) -> Self {
        Self {
            from_region_id: c.from_region.to_string(),
            to_region_id: c.to_region.to_string(),
            description: c.description,
            bidirectional: c.bidirectional,
            is_locked: c.is_locked,
            lock_description: c.lock_description,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateRegionExitRequestDto {
    pub to_location_id: String,
    pub arrival_region_id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_bidirectional")]
    pub bidirectional: bool,
}

#[derive(Debug, Serialize)]
pub struct RegionExitResponseDto {
    pub from_region_id: String,
    pub to_location_id: String,
    pub arrival_region_id: String,
    pub description: Option<String>,
    pub bidirectional: bool,
}

impl From<RegionExit> for RegionExitResponseDto {
    fn from(e: RegionExit) -> Self {
        Self {
            from_region_id: e.from_region.to_string(),
            to_location_id: e.to_location.to_string(),
            arrival_region_id: e.arrival_region_id.to_string(),
            description: e.description,
            bidirectional: e.bidirectional,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateRegionRequestDto {
    pub name: Option<String>,
    pub description: Option<String>,
    pub backdrop_asset: Option<String>,
    pub atmosphere: Option<String>,
    pub map_bounds: Option<MapBoundsDto>,
    pub is_spawn_point: Option<bool>,
    pub order: Option<u32>,
}

// =============================================================================
// Region CRUD
// =============================================================================

/// List all regions in a location
///
/// GET /api/locations/{location_id}/regions
pub async fn list_regions(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
) -> Result<Json<Vec<RegionResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;
    let location_id = LocationId::from_uuid(uuid);

    let regions = state
        .repository
        .regions()
        .list_by_location(location_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(regions.into_iter().map(RegionResponseDto::from).collect()))
}

/// Get a region by ID
///
/// GET /api/regions/{region_id}
pub async fn get_region(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
) -> Result<Json<RegionResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let region_id = RegionId::from_uuid(uuid);

    let region = state
        .repository
        .regions()
        .get(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Region not found".to_string()))?;

    Ok(Json(RegionResponseDto::from(region)))
}

/// Create a region in a location
///
/// POST /api/locations/{location_id}/regions
pub async fn create_region(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    Json(req): Json<CreateRegionRequestDto>,
) -> Result<(StatusCode, Json<RegionResponseDto>), (StatusCode, String)> {
    let uuid = Uuid::parse_str(&location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;
    let location_id = LocationId::from_uuid(uuid);

    // Verify the location exists
    let location = state
        .repository
        .locations()
        .get(location_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Location not found".to_string()))?;

    let mut region = Region::new(location_id, &req.name);
    region = region.with_description(&req.description);

    if let Some(backdrop) = req.backdrop_asset {
        region = region.with_backdrop(backdrop);
    }
    if let Some(atmosphere) = req.atmosphere {
        region = region.with_atmosphere(atmosphere);
    }
    if let Some(bounds) = req.map_bounds {
        region = region.with_map_bounds(MapBounds::from(bounds));
    }
    if req.is_spawn_point {
        region = region.as_spawn_point();
    }
    region = region.with_order(req.order);

    state
        .repository
        .regions()
        .create(&region)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(RegionResponseDto::from(region))))
}

/// Update a region
///
/// PATCH /api/regions/{region_id}
pub async fn update_region(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
    Json(req): Json<UpdateRegionRequestDto>,
) -> Result<Json<RegionResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let region_id = RegionId::from_uuid(uuid);

    let mut region = state
        .repository
        .regions()
        .get(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Region not found".to_string()))?;

    if let Some(name) = req.name {
        region.name = name;
    }
    if let Some(description) = req.description {
        region.description = description;
    }
    if let Some(backdrop) = req.backdrop_asset {
        region.backdrop_asset = Some(backdrop);
    }
    if let Some(atmosphere) = req.atmosphere {
        region.atmosphere = Some(atmosphere);
    }
    if let Some(bounds) = req.map_bounds {
        region.map_bounds = Some(MapBounds::from(bounds));
    }
    if let Some(is_spawn) = req.is_spawn_point {
        region.is_spawn_point = is_spawn;
    }
    if let Some(order) = req.order {
        region.order = order;
    }

    state
        .repository
        .regions()
        .update(&region)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(RegionResponseDto::from(region)))
}

/// Delete a region
///
/// DELETE /api/regions/{region_id}
pub async fn delete_region(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let region_id = RegionId::from_uuid(uuid);

    state
        .repository
        .regions()
        .delete(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// Spawn Points
// =============================================================================

/// List all spawn points in a world
///
/// GET /api/worlds/{world_id}/spawn-points
pub async fn list_spawn_points(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<RegionResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    let regions = state
        .repository
        .regions()
        .list_spawn_points(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(regions.into_iter().map(RegionResponseDto::from).collect()))
}

// =============================================================================
// Region Connections
// =============================================================================

/// List connections from a region
///
/// GET /api/regions/{region_id}/connections
pub async fn list_region_connections(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
) -> Result<Json<Vec<RegionConnectionResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let region_id = RegionId::from_uuid(uuid);

    let connections = state
        .repository
        .regions()
        .get_connections(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        connections
            .into_iter()
            .map(RegionConnectionResponseDto::from)
            .collect(),
    ))
}

/// Create a connection between regions
///
/// POST /api/regions/{region_id}/connections
pub async fn create_region_connection(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
    Json(req): Json<CreateRegionConnectionRequestDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    let from_uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let to_uuid = Uuid::parse_str(&req.to_region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid to_region_id".to_string()))?;

    let from_region = RegionId::from_uuid(from_uuid);
    let to_region = RegionId::from_uuid(to_uuid);

    let connection = RegionConnection {
        from_region,
        to_region,
        description: req.description,
        bidirectional: req.bidirectional,
        is_locked: req.is_locked,
        lock_description: req.lock_description,
    };

    state
        .repository
        .regions()
        .create_connection(&connection)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

/// Delete a connection between regions
///
/// DELETE /api/regions/{from_region_id}/connections/{to_region_id}
pub async fn delete_region_connection(
    State(state): State<Arc<AppState>>,
    Path((from_region_id, to_region_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let from_uuid = Uuid::parse_str(&from_region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid from region ID".to_string()))?;
    let to_uuid = Uuid::parse_str(&to_region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid to region ID".to_string()))?;

    let from_region = RegionId::from_uuid(from_uuid);
    let to_region = RegionId::from_uuid(to_uuid);

    state
        .repository
        .regions()
        .delete_connection(from_region, to_region)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Unlock a connection between regions
///
/// POST /api/regions/{from_region_id}/connections/{to_region_id}/unlock
pub async fn unlock_region_connection(
    State(state): State<Arc<AppState>>,
    Path((from_region_id, to_region_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let from_uuid = Uuid::parse_str(&from_region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid from region ID".to_string()))?;
    let to_uuid = Uuid::parse_str(&to_region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid to region ID".to_string()))?;

    let from_region = RegionId::from_uuid(from_uuid);
    let to_region = RegionId::from_uuid(to_uuid);

    state
        .repository
        .regions()
        .unlock_connection(from_region, to_region)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

// =============================================================================
// Region Exits
// =============================================================================

/// List exits from a region
///
/// GET /api/regions/{region_id}/exits
pub async fn list_region_exits(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
) -> Result<Json<Vec<RegionExitResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let region_id = RegionId::from_uuid(uuid);

    let exits = state
        .repository
        .regions()
        .get_exits(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(exits.into_iter().map(RegionExitResponseDto::from).collect()))
}

/// Create an exit from a region to another location
///
/// POST /api/regions/{region_id}/exits
pub async fn create_region_exit(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
    Json(req): Json<CreateRegionExitRequestDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    let from_uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let to_location_uuid = Uuid::parse_str(&req.to_location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid to_location_id".to_string()))?;
    let arrival_uuid = Uuid::parse_str(&req.arrival_region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid arrival_region_id".to_string()))?;

    let from_region = RegionId::from_uuid(from_uuid);
    let to_location = LocationId::from_uuid(to_location_uuid);
    let arrival_region = RegionId::from_uuid(arrival_uuid);

    let exit = RegionExit {
        from_region,
        to_location,
        arrival_region_id: arrival_region,
        description: req.description,
        bidirectional: req.bidirectional,
    };

    state
        .repository
        .regions()
        .create_exit(&exit)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

/// Delete an exit from a region to a location
///
/// DELETE /api/regions/{region_id}/exits/{location_id}
pub async fn delete_region_exit(
    State(state): State<Arc<AppState>>,
    Path((region_id, location_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let region_uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let location_uuid = Uuid::parse_str(&location_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid location ID".to_string()))?;

    let from_region = RegionId::from_uuid(region_uuid);
    let to_location = LocationId::from_uuid(location_uuid);

    state
        .repository
        .regions()
        .delete_exit(from_region, to_location)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// NPC-Region Relationships (Phase 23C)
// =============================================================================

/// DTO for NPC with their relationship to a region
#[derive(Debug, Serialize)]
pub struct NpcWithRegionRelationshipDto {
    pub character: CharacterResponseDto,
    #[serde(flatten)]
    pub relationship: RegionRelationshipTypeDto,
}

/// DTO for region relationship type (matches character_routes)
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RegionRelationshipTypeDto {
    Home,
    WorksAt { shift: String },
    Frequents { frequency: String },
    Avoids { reason: String },
}

impl From<RegionRelationshipType> for RegionRelationshipTypeDto {
    fn from(rel: RegionRelationshipType) -> Self {
        match rel {
            RegionRelationshipType::Home => RegionRelationshipTypeDto::Home,
            RegionRelationshipType::WorksAt { shift } => RegionRelationshipTypeDto::WorksAt {
                shift: shift.to_string(),
            },
            RegionRelationshipType::Frequents { frequency } => {
                RegionRelationshipTypeDto::Frequents {
                    frequency: frequency.to_string(),
                }
            }
            RegionRelationshipType::Avoids { reason } => RegionRelationshipTypeDto::Avoids { reason },
        }
    }
}

/// List all NPCs related to a region
///
/// GET /api/regions/{region_id}/npcs
pub async fn list_region_npcs(
    State(state): State<Arc<AppState>>,
    Path(region_id): Path<String>,
) -> Result<Json<Vec<NpcWithRegionRelationshipDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&region_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid region ID".to_string()))?;
    let region_id = RegionId::from_uuid(uuid);

    let npcs = state
        .repository
        .characters()
        .get_npcs_related_to_region(region_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        npcs.into_iter()
            .map(|(character, rel_type)| NpcWithRegionRelationshipDto {
                character: CharacterResponseDto::from(character),
                relationship: RegionRelationshipTypeDto::from(rel_type),
            })
            .collect(),
    ))
}


