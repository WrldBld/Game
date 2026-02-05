//! Visual state catalog use case.
//!
//! Provides CRUD operations for LocationState and RegionState entities.
//! For asset generation, use the `GenerateVisualState` use case.

use std::sync::Arc;
use thiserror::Error;

use wrldbldr_domain::{
    ActivationLogic, ActivationRule, AssetPath, Atmosphere, Description, DomainError, LocationId,
    LocationState, LocationStateId, RegionId, RegionState, RegionStateId, StateName,
};

use crate::infrastructure::ports::{
    ClockPort, LocationRepo, LocationStateRepo, RegionStateRepo, RepoError,
};
use crate::use_cases::visual_state::validation::{
    validate_generation_prompt, validate_state_description, validate_state_name_required,
    validate_workflow_id,
};

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

    #[error("Name cannot be empty")]
    NameEmpty,

    #[error("Name too long (max 200 characters)")]
    NameTooLong,

    #[error("Description too long (max 5000 characters)")]
    DescriptionTooLong,

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
}

/// Visual state catalog data
#[derive(Debug, Clone)]
pub struct CatalogData {
    pub location_states: Vec<LocationState>,
    pub region_states: Vec<RegionState>,
}

/// Active state IDs for a location/region
///
/// Used to determine which visual states are currently active without
/// fetching full state entities.
#[derive(Debug, Clone, Default)]
pub struct ActiveStateIds {
    pub location_state_id: Option<LocationStateId>,
    pub region_state_id: Option<RegionStateId>,
}

/// Visual state details (combined)
#[derive(Debug, Clone)]
pub enum VisualStateDetails {
    LocationState(LocationState),
    RegionState(RegionState),
}

/// Visual state catalog use case.
///
/// Manages LocationState and RegionState entities, including
/// CRUD operations. For asset generation, use `GenerateVisualState`.
pub struct VisualStateCatalog {
    location_repo: Arc<dyn LocationRepo>,
    location_state_repo: Arc<dyn LocationStateRepo>,
    region_state_repo: Arc<dyn RegionStateRepo>,
    clock: Arc<dyn ClockPort>,
}

impl VisualStateCatalog {
    pub fn new(
        location_repo: Arc<dyn LocationRepo>,
        location_state_repo: Arc<dyn LocationStateRepo>,
        region_state_repo: Arc<dyn RegionStateRepo>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            location_repo,
            location_state_repo,
            region_state_repo,
            clock,
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

    /// Get active state IDs for a location and/or region.
    ///
    /// This method centralizes active-state lookups to avoid direct repository
    /// access from API handlers. Returns optional IDs for location and region
    /// states that are currently active.
    pub async fn get_active_state_ids(
        &self,
        location_id: Option<LocationId>,
        region_id: Option<RegionId>,
    ) -> Result<ActiveStateIds, CatalogError> {
        let mut active_ids = ActiveStateIds::default();

        if let Some(loc_id) = location_id {
            active_ids.location_state_id = self
                .location_state_repo
                .get_active(loc_id)
                .await?
                .map(|state| state.id());
        }

        if let Some(reg_id) = region_id {
            active_ids.region_state_id = self
                .region_state_repo
                .get_active(reg_id)
                .await?
                .map(|state| state.id());
        }

        Ok(active_ids)
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
        activation_rules: Vec<ActivationRule>,
        activation_logic: ActivationLogic,
        priority: i32,
        is_default: bool,
    ) -> Result<LocationState, CatalogError> {
        let location = self.location_repo.get_location(location_id).await?
            .ok_or(CatalogError::LocationNotFound(location_id))?;

        let now = self.clock.now();

        let mut state = LocationState::new(location_id, location.world_id(), name, now)?
            .with_description(Description::new(description.unwrap_or_default())?)
            .with_priority(priority)
            .with_is_default(is_default)
            .with_rules(activation_rules, activation_logic);

        if let Some(path) = backdrop_asset {
            match AssetPath::new(path.clone()) {
                Ok(asset) => state = state.with_backdrop(asset),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        value = %path,
                        "Invalid backdrop asset path; skipping"
                    );
                }
            }
        }
        if let Some(value) = atmosphere {
            match Atmosphere::new(value.clone()) {
                Ok(atm) => state = state.with_atmosphere(atm),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        value = %value,
                        "Invalid atmosphere value; skipping"
                    );
                }
            }
        }
        if let Some(path) = ambient_sound {
            match AssetPath::new(path.clone()) {
                Ok(sound) => state = state.with_ambient_sound(sound),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        value = %path,
                        "Invalid ambient sound asset path; skipping"
                    );
                }
            }
        }
        if let Some(path) = map_overlay {
            match AssetPath::new(path.clone()) {
                Ok(overlay) => state = state.with_map_overlay(overlay),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        value = %path,
                        "Invalid map overlay asset path; skipping"
                    );
                }
            }
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
        activation_rules: Vec<ActivationRule>,
        activation_logic: ActivationLogic,
        priority: i32,
        is_default: bool,
    ) -> Result<RegionState, CatalogError> {
        let region = self.location_repo.get_region(region_id).await?
            .ok_or(CatalogError::RegionNotFound(region_id))?;

        // Fetch the location to get world_id (Region doesn't have world_id())
        let location = self.location_repo.get_location(region.location_id()).await?
            .ok_or(CatalogError::LocationNotFound(region.location_id()))?;

        let now = self.clock.now();

        let description = Description::new(description.unwrap_or_default())?;

        let mut state = RegionState::new(
            region_id,
            region.location_id(),
            location.world_id(),
            name,
            now,
        )?
        .with_description(description)
        .with_priority(priority)
        .with_is_default(is_default)
        .with_rules(activation_rules, activation_logic);

        if let Some(path) = backdrop_asset {
            match AssetPath::new(path.clone()) {
                Ok(asset) => state = state.with_backdrop(asset),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        value = %path,
                        "Invalid backdrop asset path; skipping"
                    );
                }
            }
        }
        if let Some(value) = atmosphere {
            match Atmosphere::new(value.clone()) {
                Ok(atm) => state = state.with_atmosphere(atm),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        value = %value,
                        "Invalid atmosphere value; skipping"
                    );
                }
            }
        }
        if let Some(path) = ambient_sound {
            match AssetPath::new(path.clone()) {
                Ok(sound) => state = state.with_ambient_sound(sound),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        value = %path,
                        "Invalid ambient sound asset path; skipping"
                    );
                }
            }
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
        activation_rules: Option<Vec<ActivationRule>>,
        activation_logic: Option<ActivationLogic>,
        priority: Option<i32>,
        is_default: Option<bool>,
        generation_prompt: Option<String>,
        workflow_id: Option<String>,
    ) -> Result<LocationState, CatalogError> {
        // Validate name and description length using shared validation helpers
        if let Some(n) = &name {
            let result = validate_state_name_required(n);
            if !result.is_valid() {
                return Err(CatalogError::Validation(
                    result.error_message().unwrap_or("Invalid name").to_string(),
                ));
            }
        }

        let result = validate_state_description(description.as_deref());
        if !result.is_valid() {
            return Err(CatalogError::Validation(
                result.error_message().unwrap_or("Invalid description").to_string(),
            ));
        }

        // Validate generation fields
        let result = validate_generation_prompt(generation_prompt.as_deref());
        if !result.is_valid() {
            return Err(CatalogError::Validation(
                result.error_message().unwrap_or("Invalid generation prompt").to_string(),
            ));
        }

        let result = validate_workflow_id(workflow_id.as_deref());
        if !result.is_valid() {
            return Err(CatalogError::Validation(
                result.error_message().unwrap_or("Invalid workflow ID").to_string(),
            ));
        }

        // Load existing state
        let mut state = self.location_state_repo.get(id).await?
            .ok_or(CatalogError::LocationStateNotFound(id))?;

        // Update asset paths
        let backdrop = match backdrop_asset {
            Some(path) => Some(
                AssetPath::new(path.clone())
                    .map_err(|e| CatalogError::Validation(format!("Invalid backdrop asset path: {}", e)))?,
            ),
            None => None,
        };
        let atm = match atmosphere {
            Some(value) => Some(
                Atmosphere::new(value.clone())
                    .map_err(|e| CatalogError::Validation(format!("Invalid atmosphere value: {}", e)))?,
            ),
            None => None,
        };
        let sound = match ambient_sound {
            Some(path) => Some(
                AssetPath::new(path.clone())
                    .map_err(|e| CatalogError::Validation(format!("Invalid ambient sound path: {}", e)))?,
            ),
            None => None,
        };
        let overlay = match map_overlay {
            Some(path) => Some(
                AssetPath::new(path.clone())
                    .map_err(|e| CatalogError::Validation(format!("Invalid map overlay path: {}", e)))?,
            ),
            None => None,
        };

        // Parse rules and logic
        let activation_rules = activation_rules.unwrap_or_else(|| state.activation_rules().to_vec());
        let activation_logic = activation_logic.unwrap_or_else(|| state.activation_logic());

        // Build validated name and description (fail-fast on invalid input)
        let name_value = match name {
            Some(n) => StateName::new(n)
                .map_err(|e| CatalogError::Validation(format!("Invalid state name: {}", e)))?,
            None => StateName::new(state.name().to_string())
                .map_err(CatalogError::Domain)?,
        };
        let desc_value = match description {
            Some(d) => Description::new(d)
                .map_err(|e| CatalogError::Validation(format!("Invalid description: {}", e)))?,
            None => Description::new(state.description().to_string())
                .map_err(CatalogError::Domain)?,
        };

        // Reconstruct with updates
        let now = self.clock.now();
        state = LocationState::from_storage(
            state.id(),
            state.location_id(),
            state.world_id(),
            name_value,
            desc_value,
            backdrop.or_else(|| state.backdrop_override().cloned()),
            atm.or_else(|| state.atmosphere_override().cloned()),
            sound.or_else(|| state.ambient_sound().cloned()),
            overlay.or_else(|| state.map_overlay().cloned()),
            activation_rules,
            activation_logic,
            priority.unwrap_or_else(|| state.priority()),
            is_default.unwrap_or_else(|| state.is_default()),
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
        activation_rules: Option<Vec<ActivationRule>>,
        activation_logic: Option<ActivationLogic>,
        priority: Option<i32>,
        is_default: Option<bool>,
        generation_prompt: Option<String>,
        workflow_id: Option<String>,
    ) -> Result<RegionState, CatalogError> {
        // Validate name and description length using shared validation helpers
        if let Some(n) = &name {
            let result = validate_state_name_required(n);
            if !result.is_valid() {
                return Err(CatalogError::Validation(
                    result.error_message().unwrap_or("Invalid name").to_string(),
                ));
            }
        }

        let result = validate_state_description(description.as_deref());
        if !result.is_valid() {
            return Err(CatalogError::Validation(
                result.error_message().unwrap_or("Invalid description").to_string(),
            ));
        }

        // Validate generation fields
        let result = validate_generation_prompt(generation_prompt.as_deref());
        if !result.is_valid() {
            return Err(CatalogError::Validation(
                result.error_message().unwrap_or("Invalid generation prompt").to_string(),
            ));
        }

        let result = validate_workflow_id(workflow_id.as_deref());
        if !result.is_valid() {
            return Err(CatalogError::Validation(
                result.error_message().unwrap_or("Invalid workflow ID").to_string(),
            ));
        }

        // Load existing state
        let mut state = self.region_state_repo.get(id).await?
            .ok_or(CatalogError::RegionStateNotFound(id))?;

        // Update asset paths
        let backdrop = match backdrop_asset {
            Some(path) => Some(
                AssetPath::new(path.clone())
                    .map_err(|e| CatalogError::Validation(format!("Invalid backdrop asset path: {}", e)))?,
            ),
            None => None,
        };
        let atm = match atmosphere {
            Some(value) => Some(
                Atmosphere::new(value.clone())
                    .map_err(|e| CatalogError::Validation(format!("Invalid atmosphere value: {}", e)))?,
            ),
            None => None,
        };
        let sound = match ambient_sound {
            Some(path) => Some(
                AssetPath::new(path.clone())
                    .map_err(|e| CatalogError::Validation(format!("Invalid ambient sound path: {}", e)))?,
            ),
            None => None,
        };

        // Parse rules and logic
        let activation_rules = activation_rules.unwrap_or_else(|| state.activation_rules().to_vec());
        let activation_logic = activation_logic.unwrap_or_else(|| state.activation_logic());

        // Reconstruct with updates
        let now = self.clock.now();
        state = RegionState::from_storage(
            state.id(),
            state.region_id(),
            state.location_id(),
            state.world_id(),
            StateName::new(name.unwrap_or_else(|| state.name().to_string()))?,
            Description::new(description.unwrap_or_else(|| state.description().to_string()))?,
            backdrop.or_else(|| state.backdrop_override().cloned()),
            atm.or_else(|| state.atmosphere_override().cloned()),
            sound.or_else(|| state.ambient_sound().cloned()),
            activation_rules,
            activation_logic,
            priority.unwrap_or_else(|| state.priority()),
            is_default.unwrap_or_else(|| state.is_default()),
            generation_prompt.or_else(|| state.generation_prompt().map(String::from)),
            workflow_id.or_else(|| state.workflow_id().map(String::from)),
            state.created_at(),
            now,
        );

        self.region_state_repo.save(&state).await?;
        Ok(state)
    }
}
