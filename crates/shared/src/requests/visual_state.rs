//! Visual state catalog and generation requests.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Visual state request variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VisualStateRequest {
    /// Get visual state catalog for location/region
    GetCatalog {
        request: GetVisualStateCatalogRequest,
    },
    /// Get details of a specific visual state
    GetDetails {
        request: GetVisualStateDetailsRequest,
    },
    /// Create a new visual state
    Create { request: CreateVisualStateRequest },
    /// Update an existing visual state
    Update { request: UpdateVisualStateRequest },
    /// Delete a visual state
    Delete { request: DeleteVisualStateRequest },
    /// Set active visual state for location/region
    SetActive {
        request: SetActiveVisualStateRequest,
    },
    /// Generate a new visual state with assets
    Generate { request: GenerateVisualStateRequest },
}

/// Visual state type discriminator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualStateType {
    /// Location-level visual state (city-wide)
    Location,
    /// Region-level visual state (room/specific area)
    Region,
    #[serde(other)]
    Unknown,
}

/// Request to list all available visual states for a location/region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetVisualStateCatalogRequest {
    /// Location to query (for location states)
    #[serde(default)]
    pub location_id: Option<Uuid>,
    /// Region to query (for region states)
    #[serde(default)]
    pub region_id: Option<Uuid>,
}

/// Request to get details of a specific visual state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetVisualStateDetailsRequest {
    /// Location state ID
    #[serde(default)]
    pub location_state_id: Option<Uuid>,
    /// Region state ID
    #[serde(default)]
    pub region_state_id: Option<Uuid>,
}

/// Request to create a new visual state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVisualStateRequest {
    /// Visual state type
    pub state_type: VisualStateType,
    /// Target location ID (required for Location states)
    #[serde(default)]
    pub location_id: Option<Uuid>,
    /// Target region ID (required for Region states)
    #[serde(default)]
    pub region_id: Option<Uuid>,
    /// Name of this state
    pub name: String,
    /// Description for DM reference
    #[serde(default)]
    pub description: Option<String>,
    /// Backdrop asset override
    #[serde(default)]
    pub backdrop_asset: Option<String>,
    /// Atmosphere text override
    #[serde(default)]
    pub atmosphere: Option<String>,
    /// Ambient sound asset override
    #[serde(default)]
    pub ambient_sound: Option<String>,
    /// Map overlay asset (for location states)
    #[serde(default)]
    pub map_overlay: Option<String>,
    /// Activation rules (JSON-serialized for flexibility)
    #[serde(default)]
    pub activation_rules: Option<serde_json::Value>,
    /// Activation logic (All, Any, etc.)
    #[serde(default)]
    pub activation_logic: Option<String>,
    /// Priority (higher = preferred when multiple match)
    #[serde(default)]
    pub priority: i32,
    /// If true, this is the default state
    #[serde(default)]
    pub is_default: bool,
}

/// Request to update an existing visual state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateVisualStateRequest {
    /// Location state ID (if updating location state)
    #[serde(default)]
    pub location_state_id: Option<Uuid>,
    /// Region state ID (if updating region state)
    #[serde(default)]
    pub region_state_id: Option<Uuid>,
    /// New name
    #[serde(default)]
    pub name: Option<String>,
    /// New description
    #[serde(default)]
    pub description: Option<String>,
    /// New backdrop asset
    #[serde(default)]
    pub backdrop_asset: Option<String>,
    /// New atmosphere text
    #[serde(default)]
    pub atmosphere: Option<String>,
    /// New ambient sound
    #[serde(default)]
    pub ambient_sound: Option<String>,
    /// New map overlay (location only)
    #[serde(default)]
    pub map_overlay: Option<String>,
    /// New activation rules
    #[serde(default)]
    pub activation_rules: Option<serde_json::Value>,
    /// New activation logic
    #[serde(default)]
    pub activation_logic: Option<String>,
    /// New priority
    #[serde(default)]
    pub priority: Option<i32>,
    /// New is_default flag
    #[serde(default)]
    pub is_default: Option<bool>,
    /// New generation prompt
    #[serde(default)]
    pub generation_prompt: Option<String>,
    /// New workflow ID
    #[serde(default)]
    pub workflow_id: Option<String>,
}

/// Request to delete a visual state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteVisualStateRequest {
    /// Location state ID
    #[serde(default)]
    pub location_state_id: Option<Uuid>,
    /// Region state ID
    #[serde(default)]
    pub region_state_id: Option<Uuid>,
}

/// Request to set the active visual state for a location/region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetActiveVisualStateRequest {
    /// Location ID
    #[serde(default)]
    pub location_id: Option<Uuid>,
    /// Location state ID (to set active location state)
    #[serde(default)]
    pub location_state_id: Option<Uuid>,
    /// Region ID
    #[serde(default)]
    pub region_id: Option<Uuid>,
    /// Region state ID (to set active region state)
    #[serde(default)]
    pub region_state_id: Option<Uuid>,
}

/// Request to generate a new visual state with assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateVisualStateRequest {
    /// Visual state type
    pub state_type: VisualStateType,
    /// Target location ID (required for Location states)
    #[serde(default)]
    pub location_id: Option<Uuid>,
    /// Target region ID (required for Region states)
    #[serde(default)]
    pub region_id: Option<Uuid>,
    /// Name for the generated state
    pub name: String,
    /// Description/prompt for generation
    pub description: String,
    /// Generation prompt for images
    pub prompt: String,
    /// ComfyUI workflow to use
    pub workflow: String,
    /// Negative prompt (optional)
    #[serde(default)]
    pub negative_prompt: Option<String>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether to generate backdrop asset
    #[serde(default)]
    pub generate_backdrop: bool,
    /// Whether to generate map overlay (location only)
    #[serde(default)]
    pub generate_map: bool,
    /// Activation rules (optional)
    #[serde(default)]
    pub activation_rules: Option<serde_json::Value>,
    /// Activation logic (optional)
    #[serde(default)]
    pub activation_logic: Option<String>,
    /// Priority (optional, defaults to 0)
    #[serde(default)]
    pub priority: i32,
    /// Whether this is a default state
    #[serde(default)]
    pub is_default: bool,
}

/// Visual state catalog data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualStateCatalogData {
    /// All location states for the location
    #[serde(default)]
    pub location_states: Vec<LocationStateData>,
    /// All region states for the region
    #[serde(default)]
    pub region_states: Vec<RegionStateData>,
}

/// Location state data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationStateData {
    /// State ID
    pub id: Uuid,
    /// Location ID
    pub location_id: Uuid,
    /// Name
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
    /// Backdrop asset
    #[serde(default)]
    pub backdrop_override: Option<String>,
    /// Atmosphere text
    #[serde(default)]
    pub atmosphere_override: Option<String>,
    /// Ambient sound
    #[serde(default)]
    pub ambient_sound: Option<String>,
    /// Map overlay
    #[serde(default)]
    pub map_overlay: Option<String>,
    /// Priority
    pub priority: i32,
    /// Is default
    pub is_default: bool,
    /// Is currently active
    #[serde(default)]
    pub is_active: bool,
    /// Activation rules (for display)
    #[serde(default)]
    pub activation_rules: Option<serde_json::Value>,
    /// Activation logic
    #[serde(default)]
    pub activation_logic: Option<String>,
    /// Generation prompt (for reference)
    #[serde(default)]
    pub generation_prompt: Option<String>,
    /// ComfyUI workflow ID (for reference)
    #[serde(default)]
    pub workflow_id: Option<String>,
    /// Created at timestamp
    pub created_at: String,
    /// Updated at timestamp
    pub updated_at: String,
}

/// Region state data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionStateData {
    /// State ID
    pub id: Uuid,
    /// Region ID
    pub region_id: Uuid,
    /// Location ID (denormalized)
    pub location_id: Uuid,
    /// Name
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
    /// Backdrop asset
    #[serde(default)]
    pub backdrop_override: Option<String>,
    /// Atmosphere text
    #[serde(default)]
    pub atmosphere_override: Option<String>,
    /// Ambient sound
    #[serde(default)]
    pub ambient_sound: Option<String>,
    /// Priority
    pub priority: i32,
    /// Is default
    pub is_default: bool,
    /// Is currently active
    #[serde(default)]
    pub is_active: bool,
    /// Activation rules (for display)
    #[serde(default)]
    pub activation_rules: Option<serde_json::Value>,
    /// Activation logic
    #[serde(default)]
    pub activation_logic: Option<String>,
    /// Generation prompt (for reference)
    #[serde(default)]
    pub generation_prompt: Option<String>,
    /// ComfyUI workflow ID (for reference)
    #[serde(default)]
    pub workflow_id: Option<String>,
    /// Created at timestamp
    pub created_at: String,
    /// Updated at timestamp
    pub updated_at: String,
}

/// Generated visual state result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedVisualStateData {
    /// The created state (location or region)
    #[serde(default)]
    pub location_state: Option<LocationStateData>,
    #[serde(default)]
    pub region_state: Option<RegionStateData>,
    /// Generation batch ID (for tracking progress)
    pub generation_batch_id: String,
    /// Whether all assets are generated
    pub is_complete: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_visual_state_catalog_request_serialization() {
        let req = GetVisualStateCatalogRequest {
            location_id: Some(Uuid::new_v4()),
            region_id: Some(Uuid::new_v4()),
        };

        let json = serde_json::to_string(&req).unwrap();
        let decoded: GetVisualStateCatalogRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.location_id, req.location_id);
        assert_eq!(decoded.region_id, req.region_id);
    }

    #[test]
    fn test_create_visual_state_request_validation() {
        let req = CreateVisualStateRequest {
            state_type: VisualStateType::Location,
            location_id: Some(Uuid::new_v4()),
            region_id: None,
            name: "Test State".to_string(),
            description: None,
            backdrop_asset: None,
            atmosphere: None,
            ambient_sound: None,
            map_overlay: None,
            activation_rules: None,
            activation_logic: None,
            priority: 0,
            is_default: false,
        };

        // Valid location state with location_id
        assert!(matches!(req.state_type, VisualStateType::Location));
        assert!(req.location_id.is_some());
        assert!(req.region_id.is_none());
    }

    #[test]
    fn test_generate_visual_state_request_serialization() {
        let req = GenerateVisualStateRequest {
            state_type: VisualStateType::Region,
            location_id: None,
            region_id: Some(Uuid::new_v4()),
            name: "Evening".to_string(),
            description: "Warm evening atmosphere".to_string(),
            prompt: "tavern interior warm lighting evening".to_string(),
            workflow: "backdrop_v2".to_string(),
            negative_prompt: None,
            tags: vec!["evening".to_string(), "tavern".to_string()],
            generate_backdrop: true,
            generate_map: false,
            activation_rules: None,
            activation_logic: None,
            priority: 10,
            is_default: false,
        };

        let json = serde_json::to_string(&req).unwrap();
        let decoded: GenerateVisualStateRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.name, "Evening");
        assert_eq!(decoded.generate_backdrop, true);
        assert_eq!(decoded.tags.len(), 2);
    }
}
