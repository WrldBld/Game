//! Visual state catalog use case.
//!
//! Provides CRUD operations for LocationState and RegionState entities,
//! plus generation of new visual states with assets.

use std::sync::Arc;
use thiserror::Error;

use wrldbldr_domain::{
    ActivationLogic, ActivationRule, AssetPath, Atmosphere, Description, LocationId, LocationState,
    LocationStateId, RegionId, RegionState, RegionStateId, StateName,
};

use crate::infrastructure::ports::{
    AssetRepo, ClockPort, ImageGenPort, LocationRepo, LocationStateRepo,
    QueuePort, RandomPort, RegionStateRepo, RepoError,
};
use crate::queue_types::AssetGenerationData;

/// Error type for visual state catalog operations
#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Location {0} not found")]
    LocationNotFound(LocationId),

    #[error("Region {0} not found")]
    RegionNotFound(RegionId),

    #[error("Location state {0} not found")]
    LocationStateNotFound(LocationStateId),

    #[error("Region state {0} not found")]
    RegionStateNotFound(RegionStateId),

    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),

    #[error("Image generation error: {0}")]
    ImageGen(String),

    #[error("Invalid activation rules JSON: {0}")]
    InvalidActivationRules(String),

    #[error("Invalid activation logic: {0}")]
    InvalidActivationLogic(String),
}

/// Visual state catalog data
#[derive(Debug, Clone)]
pub struct CatalogData {
    pub location_states: Vec<LocationState>,
    pub region_states: Vec<RegionState>,
}

/// Visual state details (combined)
#[derive(Debug, Clone)]
pub enum VisualStateDetails {
    LocationState(LocationState),
    RegionState(RegionState),
}

/// Generated visual state result
#[derive(Debug, Clone)]
pub struct GeneratedVisualState {
    pub location_state: Option<LocationState>,
    pub region_state: Option<RegionState>,
    pub generation_batch_id: String,
    pub is_complete: bool,
}

/// Visual state catalog use case.
///
/// Manages LocationState and RegionState entities, including
/// CRUD operations and asset generation.
pub struct VisualStateCatalog {
    location_repo: Arc<dyn LocationRepo>,
    location_state_repo: Arc<dyn LocationStateRepo>,
    region_state_repo: Arc<dyn RegionStateRepo>,
    image_gen: Arc<dyn ImageGenPort>,
    asset_repo: Arc<dyn AssetRepo>,
    queue: Arc<dyn QueuePort>,
    clock: Arc<dyn ClockPort>,
    random: Arc<dyn RandomPort>,
}

impl VisualStateCatalog {
    pub fn new(
        location_repo: Arc<dyn LocationRepo>,
        location_state_repo: Arc<dyn LocationStateRepo>,
        region_state_repo: Arc<dyn RegionStateRepo>,
        image_gen: Arc<dyn ImageGenPort>,
        asset_repo: Arc<dyn AssetRepo>,
        queue: Arc<dyn QueuePort>,
        clock: Arc<dyn ClockPort>,
        random: Arc<dyn RandomPort>,
    ) -> Self {
        Self {
            location_repo,
            location_state_repo,
            region_state_repo,
            image_gen,
            asset_repo,
            queue,
            clock,
            random,
        }
    }

    /// Get visual state catalog for a location/region
    pub async fn get_catalog(
        &self,
        location_id: Option<LocationId>,
        region_id: Option<RegionId>,
    ) -> Result<CatalogData, CatalogError> {
        let mut location_states = Vec::new();
        let mut region_states = Vec::new();

        if let Some(loc_id) = location_id {
            self.location_repo.get_location(loc_id).await?
                .ok_or(CatalogError::LocationNotFound(loc_id))?;

            location_states = self.location_state_repo.list_for_location(loc_id).await?;
        }

        if let Some(reg_id) = region_id {
            let _region = self.location_repo.get_region(reg_id).await?
                .ok_or(CatalogError::RegionNotFound(reg_id))?;

            region_states = self.region_state_repo.list_for_region(reg_id).await?;
        }

        Ok(CatalogData {
            location_states,
            region_states,
        })
    }

    /// Get details of a specific visual state
    pub async fn get_details(
        &self,
        location_state_id: Option<LocationStateId>,
        region_state_id: Option<RegionStateId>,
    ) -> Result<VisualStateDetails, CatalogError> {
        if let Some(ls_id) = location_state_id {
            let state = self.location_state_repo.get(ls_id).await?
                .ok_or(CatalogError::LocationStateNotFound(ls_id))?;
            return Ok(VisualStateDetails::LocationState(state));
        }

        if let Some(rs_id) = region_state_id {
            let state = self.region_state_repo.get(rs_id).await?
                .ok_or(CatalogError::RegionStateNotFound(rs_id))?;
            return Ok(VisualStateDetails::RegionState(state));
        }

        Err(CatalogError::Validation(
            "Either location_state_id or region_state_id must be provided".to_string(),
        ))
    }

    /// Create a new location state
    pub async fn create_location_state(
        &self,
        location_id: LocationId,
        name: String,
        description: Option<String>,
        backdrop_asset: Option<String>,
        atmosphere: Option<String>,
        ambient_sound: Option<String>,
        map_overlay: Option<String>,
        activation_rules_json: Option<serde_json::Value>,
        activation_logic_str: Option<String>,
        priority: i32,
        is_default: bool,
    ) -> Result<LocationState, CatalogError> {
        let location = self.location_repo.get_location(location_id).await?
            .ok_or(CatalogError::LocationNotFound(location_id))?;

        let now = self.clock.now();

        let activation_rules = self.parse_activation_rules(activation_rules_json)?;
        let activation_logic = self.parse_activation_logic(activation_logic_str)?;

        let mut state = LocationState::new(location_id, location.world_id(), name, now)
            .with_description(description.unwrap_or_default())
            .with_priority(priority)
            .with_is_default(is_default)
            .with_rules(activation_rules, activation_logic);

        if let Some(asset) = backdrop_asset.and_then(|p| AssetPath::new(p).ok()) {
            state = state.with_backdrop(asset);
        }
        if let Some(atm) = atmosphere.and_then(|a| Atmosphere::new(a).ok()) {
            state = state.with_atmosphere(atm);
        }
        if let Some(sound) = ambient_sound.and_then(|s| AssetPath::new(s).ok()) {
            state = state.with_ambient_sound(sound);
        }
        if let Some(overlay) = map_overlay.and_then(|m| AssetPath::new(m).ok()) {
            state = state.with_map_overlay(overlay);
        }

        self.location_state_repo.save(&state).await?;
        Ok(state)
    }

    /// Create a new region state
    pub async fn create_region_state(
        &self,
        region_id: RegionId,
        name: String,
        description: Option<String>,
        backdrop_asset: Option<String>,
        atmosphere: Option<String>,
        ambient_sound: Option<String>,
        activation_rules_json: Option<serde_json::Value>,
        activation_logic_str: Option<String>,
        priority: i32,
        is_default: bool,
    ) -> Result<RegionState, CatalogError> {
        let region = self.location_repo.get_region(region_id).await?
            .ok_or(CatalogError::RegionNotFound(region_id))?;

        // Fetch the location to get world_id (Region doesn't have world_id())
        let location = self.location_repo.get_location(region.location_id()).await?
            .ok_or(CatalogError::LocationNotFound(region.location_id()))?;

        let now = self.clock.now();

        let activation_rules = self.parse_activation_rules(activation_rules_json)?;
        let activation_logic = self.parse_activation_logic(activation_logic_str)?;

        let mut state = RegionState::new(
            region_id,
            region.location_id(),
            location.world_id(),
            name,
            now,
        )
        .with_description(description.unwrap_or_default())
        .with_priority(priority)
        .with_is_default(is_default)
        .with_rules(activation_rules, activation_logic);

        if let Some(asset) = backdrop_asset.and_then(|p| AssetPath::new(p).ok()) {
            state = state.with_backdrop(asset);
        }
        if let Some(atm) = atmosphere.and_then(|a| Atmosphere::new(a).ok()) {
            state = state.with_atmosphere(atm);
        }
        if let Some(sound) = ambient_sound.and_then(|s| AssetPath::new(s).ok()) {
            state = state.with_ambient_sound(sound);
        }

        self.region_state_repo.save(&state).await?;
        Ok(state)
    }

    /// Delete a visual state
    pub async fn delete(
        &self,
        location_state_id: Option<LocationStateId>,
        region_state_id: Option<RegionStateId>,
    ) -> Result<(), CatalogError> {
        if let Some(ls_id) = location_state_id {
            self.location_state_repo.delete(ls_id).await?;
            return Ok(());
        }

        if let Some(rs_id) = region_state_id {
            self.region_state_repo.delete(rs_id).await?;
            return Ok(());
        }

        Err(CatalogError::Validation(
            "Either location_state_id or region_state_id must be provided".to_string(),
        ))
    }

    /// Set active visual state
    pub async fn set_active(
        &self,
        location_id: Option<LocationId>,
        location_state_id: Option<LocationStateId>,
        region_id: Option<RegionId>,
        region_state_id: Option<RegionStateId>,
    ) -> Result<(), CatalogError> {
        if let Some(loc_id) = location_id {
            if let Some(ls_id) = location_state_id {
                self.location_state_repo.set_active(loc_id, ls_id).await?;
            } else {
                self.location_state_repo.clear_active(loc_id).await?;
            }
            return Ok(());
        }

        if let Some(reg_id) = region_id {
            if let Some(rs_id) = region_state_id {
                self.region_state_repo.set_active(reg_id, rs_id).await?;
            } else {
                self.region_state_repo.clear_active(reg_id).await?;
            }
            return Ok(());
        }

        Err(CatalogError::Validation(
            "Either location_id or region_id must be provided".to_string(),
        ))
    }

    /// Update a location state with optional fields
    #[allow(clippy::too_many_arguments)]
    pub async fn update_location_state(
        &self,
        id: LocationStateId,
        name: Option<String>,
        description: Option<String>,
        backdrop_asset: Option<String>,
        atmosphere: Option<String>,
        ambient_sound: Option<String>,
        map_overlay: Option<String>,
        activation_rules_json: Option<serde_json::Value>,
        activation_logic_str: Option<String>,
        priority: Option<i32>,
        is_default: Option<bool>,
        generation_prompt: Option<String>,
        workflow_id: Option<String>,
    ) -> Result<LocationState, CatalogError> {
        // Load existing state
        let mut state = self.location_state_repo.get(id).await?
            .ok_or(CatalogError::LocationStateNotFound(id))?;

        // Validate and update name
        if let Some(n) = &name {
            if n.trim().is_empty() {
                return Err(CatalogError::Validation("name cannot be empty".to_string()));
            }
            if n.len() > 200 {
                return Err(CatalogError::Validation("name too long (max 200 chars)".to_string()));
            }
        }

        // Validate description length
        if let Some(desc) = &description {
            if desc.len() > 5000 {
                return Err(CatalogError::Validation("description too long (max 5000 chars)".to_string()));
            }
        }

        // Update asset paths
        let backdrop = backdrop_asset.and_then(|p| AssetPath::new(p).ok());
        let atm = atmosphere.and_then(|a| Atmosphere::new(a).ok());
        let sound = ambient_sound.and_then(|s| AssetPath::new(s).ok());
        let overlay = map_overlay.and_then(|m| AssetPath::new(m).ok());

        // Parse rules and logic
        let activation_rules = self.parse_activation_rules(activation_rules_json)?;
        let activation_logic = self.parse_activation_logic(activation_logic_str)?;

        // Reconstruct with updates
        let now = self.clock.now();
        state = LocationState::from_parts(
            state.id(),
            state.location_id(),
            state.world_id(),
            StateName::new(name.unwrap_or_else(|| state.name().to_string()))
                .unwrap_or_else(|_| StateName::default()),
            Description::new(description.unwrap_or_else(|| state.description().to_string()))
                .unwrap_or_default(),
            backdrop.or_else(|| state.backdrop_override().cloned()),
            atm.or_else(|| state.atmosphere_override().cloned()),
            sound.or_else(|| state.ambient_sound().cloned()),
            overlay.or_else(|| state.map_overlay().cloned()),
            activation_rules,
            activation_logic,
            priority.unwrap_or(state.priority()),
            is_default.unwrap_or(state.is_default()),
            generation_prompt.or_else(|| state.generation_prompt().map(String::from)),
            workflow_id.or_else(|| state.workflow_id().map(String::from)),
            state.created_at(),
            now,
        );

        self.location_state_repo.save(&state).await?;
        Ok(state)
    }

    /// Update a region state with optional fields
    #[allow(clippy::too_many_arguments)]
    pub async fn update_region_state(
        &self,
        id: RegionStateId,
        name: Option<String>,
        description: Option<String>,
        backdrop_asset: Option<String>,
        atmosphere: Option<String>,
        ambient_sound: Option<String>,
        activation_rules_json: Option<serde_json::Value>,
        activation_logic_str: Option<String>,
        priority: Option<i32>,
        is_default: Option<bool>,
        generation_prompt: Option<String>,
        workflow_id: Option<String>,
    ) -> Result<RegionState, CatalogError> {
        // Load existing state
        let mut state = self.region_state_repo.get(id).await?
            .ok_or(CatalogError::RegionStateNotFound(id))?;

        // Validate and update name
        if let Some(n) = &name {
            if n.trim().is_empty() {
                return Err(CatalogError::Validation("name cannot be empty".to_string()));
            }
            if n.len() > 200 {
                return Err(CatalogError::Validation("name too long (max 200 chars)".to_string()));
            }
        }

        // Validate description length
        if let Some(desc) = &description {
            if desc.len() > 5000 {
                return Err(CatalogError::Validation("description too long (max 5000 chars)".to_string()));
            }
        }

        // Update asset paths
        let backdrop = backdrop_asset.and_then(|p| AssetPath::new(p).ok());
        let atm = atmosphere.and_then(|a| Atmosphere::new(a).ok());
        let sound = ambient_sound.and_then(|s| AssetPath::new(s).ok());

        // Parse rules and logic
        let activation_rules = self.parse_activation_rules(activation_rules_json)?;
        let activation_logic = self.parse_activation_logic(activation_logic_str)?;

        // Reconstruct with updates
        let now = self.clock.now();
        state = RegionState::from_parts(
            state.id(),
            state.region_id(),
            state.location_id(),
            state.world_id(),
            StateName::new(name.unwrap_or_else(|| state.name().to_string()))
                .unwrap_or_else(|_| StateName::default()),
            Description::new(description.unwrap_or_else(|| state.description().to_string()))
                .unwrap_or_default(),
            backdrop.or_else(|| state.backdrop_override().cloned()),
            atm.or_else(|| state.atmosphere_override().cloned()),
            sound.or_else(|| state.ambient_sound().cloned()),
            activation_rules,
            activation_logic,
            priority.unwrap_or(state.priority()),
            is_default.unwrap_or(state.is_default()),
            generation_prompt.or_else(|| state.generation_prompt().map(String::from)),
            workflow_id.or_else(|| state.workflow_id().map(String::from)),
            state.created_at(),
            now,
        );

        self.region_state_repo.save(&state).await?;
        Ok(state)
    }

    /// Generate a new visual state with asset generation
    #[allow(clippy::too_many_arguments)]
    pub async fn generate_visual_state(
        &self,
        location_id: Option<LocationId>,
        region_id: Option<RegionId>,
        name: String,
        description: String,
        prompt: String,
        workflow_id: String,
        _tags: Vec<String>,  // Tags reserved for future use
        generate_backdrop: bool,  // Whether to generate backdrop asset
        _generate_map: bool,  // Map generation not yet implemented
        activation_rules_json: Option<serde_json::Value>,
        activation_logic_str: Option<String>,
        priority: i32,
        is_default: bool,
    ) -> Result<GeneratedVisualState, CatalogError> {
        // Determine target and get world_id
        let (world_id, location_state, region_state) = if let Some(loc_id) = location_id {
            let location = self.location_repo.get_location(loc_id).await?
                .ok_or(CatalogError::LocationNotFound(loc_id))?;

            let state_id = LocationStateId::new();
            let activation_rules = self.parse_activation_rules(activation_rules_json)?;
            let activation_logic = self.parse_activation_logic(activation_logic_str)?;
            let now = self.clock.now();

            let state = LocationState::new_with_id(state_id, loc_id, location.world_id(), &name, now)
                .with_description(description.clone())
                .with_generation_prompt(prompt.clone())
                .with_workflow_id(workflow_id.clone())
                .with_rules(activation_rules, activation_logic)
                .with_priority(priority)
                .with_is_default(is_default);

            self.location_state_repo.save(&state).await?;

            (location.world_id(), Some(state), None)
        } else if let Some(reg_id) = region_id {
            let region = self.location_repo.get_region(reg_id).await?
                .ok_or(CatalogError::RegionNotFound(reg_id))?;

            // Fetch the location to get world_id
            let location = self.location_repo.get_location(region.location_id()).await?
                .ok_or(CatalogError::LocationNotFound(region.location_id()))?;

            let state_id = RegionStateId::new();
            let activation_rules = self.parse_activation_rules(activation_rules_json)?;
            let activation_logic = self.parse_activation_logic(activation_logic_str)?;
            let now = self.clock.now();

            let state = RegionState::new_with_id(state_id, reg_id, region.location_id(), location.world_id(), &name, now)
                .with_description(description.clone())
                .with_generation_prompt(prompt.clone())
                .with_workflow_id(workflow_id.clone())
                .with_rules(activation_rules, activation_logic)
                .with_priority(priority)
                .with_is_default(is_default);

            self.region_state_repo.save(&state).await?;

            (location.world_id(), None, Some(state))
        } else {
            return Err(CatalogError::Validation(
                "Either location_id or region_id must be provided".to_string(),
            ));
        };

        // Create generation batch ID
        let batch_id = uuid::Uuid::new_v4().to_string();

        // Enqueue asset generation if requested
        if generate_backdrop {
            let entity_id = location_state
                .as_ref()
                .map(|s| s.id().to_string())
                .or_else(|| region_state.as_ref().map(|s| s.id().to_string()))
                .ok_or_else(|| CatalogError::Validation("Failed to determine state ID".to_string()))?;

            let entity_type = location_state.as_ref()
                .map(|_| "LocationState".to_string())
                .or_else(|| region_state.as_ref().map(|_| "RegionState".to_string()))
                .ok_or_else(|| CatalogError::Validation("Failed to determine entity type".to_string()))?;

            let data = AssetGenerationData {
                world_id: Some(world_id),
                entity_type,
                entity_id,
                workflow_id,
                prompt,
                count: 1,
            };

            self.queue.enqueue_asset_generation(&data).await
                .map_err(|e| CatalogError::ImageGen(e.to_string()))?;
        }

        // Note: Map generation would be queued separately here when implemented

        Ok(GeneratedVisualState {
            location_state,
            region_state,
            generation_batch_id: batch_id,
            is_complete: false, // Assets are queued but not yet generated
        })
    }

    /// Helper: Parse activation rules from JSON
    fn parse_activation_rules(
        &self,
        json: Option<serde_json::Value>,
    ) -> Result<Vec<ActivationRule>, CatalogError> {
        match json {
            Some(value) => {
                serde_json::from_value(value)
                    .map_err(|e| CatalogError::InvalidActivationRules(e.to_string()))
            }
            None => Ok(Vec::new()),
        }
    }

    /// Helper: Parse activation logic from string
    fn parse_activation_logic(
        &self,
        logic: Option<String>,
    ) -> Result<ActivationLogic, CatalogError> {
        match logic.as_deref() {
            Some(s) if s == "All" => Ok(ActivationLogic::All),
            Some(s) if s == "Any" => Ok(ActivationLogic::Any),
            Some(s) if s == "AtLeast" => Ok(ActivationLogic::AtLeast(1)),
            None => Ok(ActivationLogic::All),
            Some(_) => Err(CatalogError::InvalidActivationLogic(
                "Invalid activation logic. Must be 'All', 'Any', or 'AtLeast'".to_string()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::{
        MockAssetRepo, MockLocationRepo, MockLocationStateRepo, MockRegionStateRepo,
    };
    use crate::infrastructure::clock::{SystemClock, SystemRandom};
    use crate::test_fixtures::image_mocks::PlaceholderImageGen as MockImageGenPort;
    use crate::test_fixtures::queue_mocks::MockQueueForTesting;

    #[test]
    fn test_parse_activation_rules_empty() {
        let catalog = VisualStateCatalog::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockImageGenPort::new()),
            Arc::new(MockAssetRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
            Arc::new(SystemRandom::new()),
        );
        let rules = catalog.parse_activation_rules(None);
        assert!(rules.is_ok());
        assert!(rules.unwrap().is_empty());
    }

    #[test]
    fn test_parse_activation_logic_valid() {
        let catalog = VisualStateCatalog::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockImageGenPort::new()),
            Arc::new(MockAssetRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
            Arc::new(SystemRandom::new()),
        );
        assert!(catalog.parse_activation_logic(Some("All".to_string())).is_ok());
        assert!(catalog.parse_activation_logic(Some("Any".to_string())).is_ok());
        assert!(catalog.parse_activation_logic(None).is_ok());
    }

    #[test]
    fn test_parse_activation_logic_invalid() {
        let catalog = VisualStateCatalog::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockImageGenPort::new()),
            Arc::new(MockAssetRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
            Arc::new(SystemRandom::new()),
        );
        let result = catalog.parse_activation_logic(Some("Invalid".to_string()));
        assert!(result.is_err());
    }
}
