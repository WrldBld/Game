//! Visual state catalog use case.
//!
//! Provides CRUD operations for LocationState and RegionState entities,
//! plus generation of new visual states with assets.

use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

use wrldbldr_domain::{
    ActivationLogic, ActivationRule, AssetPath, Atmosphere, LocationId, LocationState,
    LocationStateId, RegionId, RegionState, RegionStateId,
};

use crate::infrastructure::ports::{
    AssetRepo, ClockPort, ImageGenPort, ImageRequest, LocationRepo, LocationStateRepo,
    RandomPort, RegionStateRepo, RepoError,
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
        clock: Arc<dyn ClockPort>,
        random: Arc<dyn RandomPort>,
    ) -> Self {
        Self {
            location_repo,
            location_state_repo,
            region_state_repo,
            image_gen,
            asset_repo,
            clock,
            random,
        }
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
            self.location_repo.get(loc_id).await?
                .ok_or(CatalogError::LocationNotFound(loc_id))?;

            location_states = self.location_state_repo.list_for_location(loc_id).await?;
        }

        if let Some(reg_id) = region_id {
            let region = self.location_repo.get_region(reg_id).await?
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
        let location = self.location_repo.get(location_id).await?
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

        let now = self.clock.now();

        let activation_rules = self.parse_activation_rules(activation_rules_json)?;
        let activation_logic = self.parse_activation_logic(activation_logic_str)?;

        let mut state = RegionState::new(
            region_id,
            region.location_id(),
            region.world_id(),
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
            return Ok();
        }

        Err(CatalogError::Validation(
            "Either location_id or region_id must be provided".to_string(),
        ))
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
            Some("All") => Ok(ActivationLogic::All),
            Some("Any") => Ok(ActivationLogic::Any),
            Some("AtLeast") => Ok(ActivationLogic::AtLeast(1)),
            None => Ok(ActivationLogic::All),
            Some(_) => Err(CatalogError::InvalidActivationLogic(format!(
                "Invalid activation logic. Must be 'All', 'Any', or 'AtLeast'"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_activation_rules_empty() {
        let catalog = VisualStateCatalog::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockImageGenPort::new()),
            Arc::new(MockAssetRepo::new()),
            Arc::new(SystemClock::new()),
            Arc::new(SystemRandom::new()),
        );
        let rules = catalog.parse_activation_rules(None);
        assert!(rules.is_ok());
        assert!(rules.unwrap().is_empty());
    }

    #[test]
    fn test_parse_activation_logic_valid() {
        let catalog = VisualStateStateCatalog::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockImageGenPort::new()),
            Arc::new(MockAssetRepo::new()),
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
            Arc::new(SystemClock::new()),
            Arc::new(SystemRandom::new()),
            );
        let result = catalog.parse_activation_logic(Some("Invalid".to_string()));
        assert!(result.is_err());
    }
}
