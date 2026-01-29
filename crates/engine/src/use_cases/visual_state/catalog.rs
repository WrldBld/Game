//! Visual state catalog use case.
//!
//! Provides CRUD operations for LocationState and RegionState entities,
//! plus generation of new visual states with assets.

use std::sync::Arc;
use thiserror::Error;

use wrldbldr_domain::{
    ActivationLogic, ActivationRule, AssetPath, Atmosphere, BatchId, Description, DomainError, LocationId,
    LocationState, LocationStateId, RegionId, RegionState, RegionStateId, StateName,
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

    #[error("Queue error: {0}")]
    Queue(#[from] crate::infrastructure::ports::QueueError),

    #[error("Invalid activation rules JSON: {0}")]
    InvalidActivationRules(String),

    #[error("Invalid activation logic: {0}")]
    InvalidActivationLogic(String),

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
    pub generation_batch_id: BatchId,
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
        activation_rules: Option<Vec<ActivationRule>>,
        activation_logic: Option<ActivationLogic>,
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
                return Err(CatalogError::NameEmpty);
            }
            if n.len() > 200 {
                return Err(CatalogError::NameTooLong);
            }
        }

        // Validate description length
        if let Some(desc) = &description {
            if desc.len() > 5000 {
                return Err(CatalogError::DescriptionTooLong);
            }
        }

        // Update asset paths
        let backdrop = backdrop_asset.and_then(|p| AssetPath::new(p).ok());
        let atm = atmosphere.and_then(|a| Atmosphere::new(a).ok());
        let sound = ambient_sound.and_then(|s| AssetPath::new(s).ok());
        let overlay = map_overlay.and_then(|m| AssetPath::new(m).ok());

        // Parse rules and logic
        let activation_rules = activation_rules.unwrap_or_else(|| state.activation_rules().to_vec());
        let activation_logic = activation_logic.unwrap_or_else(|| state.activation_logic());

        // Reconstruct with updates
        let now = self.clock.now();
        state = LocationState::from_storage(
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
        // Load existing state
        let mut state = self.region_state_repo.get(id).await?
            .ok_or(CatalogError::RegionStateNotFound(id))?;

        // Validate name
        if let Some(n) = &name {
            if n.trim().is_empty() {
                return Err(CatalogError::NameEmpty);
            }
            if n.len() > 200 {
                return Err(CatalogError::NameTooLong);
            }
        }

        // Validate description length
        if let Some(desc) = &description {
            if desc.len() > 5000 {
                return Err(CatalogError::DescriptionTooLong);
            }
        }

        // Update asset paths
        let backdrop = backdrop_asset.and_then(|p| AssetPath::new(p).ok());
        let atm = atmosphere.and_then(|a| Atmosphere::new(a).ok());
        let sound = ambient_sound.and_then(|s| AssetPath::new(s).ok());

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
        map_workflow_id: Option<String>,
        _tags: Vec<String>,  // Tags reserved for future use
        generate_backdrop: bool,  // Whether to generate backdrop asset
        generate_map: bool,  // Whether to generate map asset
        activation_rules: Vec<ActivationRule>,
        activation_logic: ActivationLogic,
        priority: i32,
        is_default: bool,
    ) -> Result<GeneratedVisualState, CatalogError> {
        // Determine target and get world_id
        let (world_id, location_state, region_state) =         if let Some(loc_id) = location_id {
            let location = self.location_repo.get_location(loc_id).await?
                .ok_or(CatalogError::LocationNotFound(loc_id))?;

            let state_id = LocationStateId::new();
            let now = self.clock.now();

            let state = LocationState::new_with_id(state_id, loc_id, location.world_id(), &name, now)?
                .with_description(Description::new(description.clone())?)
                .with_generation_prompt(prompt.clone())
                .with_workflow_id(workflow_id.clone())
                .with_rules(activation_rules.clone(), activation_logic)
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
            let now = self.clock.now();

            let description = Description::new(description.clone())?;

            let state = RegionState::new_with_id(state_id, reg_id, region.location_id(), location.world_id(), &name, now)?
                .with_description(description)
                .with_generation_prompt(prompt.clone())
                .with_workflow_id(workflow_id.clone())
                .with_rules(activation_rules.clone(), activation_logic)
                .with_priority(priority)
                .with_is_default(is_default);

            self.region_state_repo.save(&state).await?;

            (location.world_id(), None, Some(state))
        } else {
            return Err(CatalogError::Validation(
                "Either location_id or region_id must be provided".to_string(),
            ));
        };

        // Create generation batch ID as typed BatchId
        let batch_id = BatchId::new();

        // Enqueue asset generation if requested
        if generate_backdrop {
            let entity_id: String = location_state
                .as_ref()
                .map(|s| s.id().to_string())
                .or_else(|| region_state.as_ref().map(|s| s.id().to_string()))
                .ok_or_else(|| CatalogError::Validation("Failed to determine state ID".to_string()))?;

            let entity_type: String = location_state.as_ref()
                .map(|_| "LocationState".to_string())
                .or_else(|| region_state.as_ref().map(|_| "RegionState".to_string()))
                .ok_or_else(|| CatalogError::Validation("Failed to determine entity type".to_string()))?;

            let data = AssetGenerationData {
                world_id: Some(world_id),
                entity_type,
                entity_id,
                workflow_id,
                prompt: prompt.clone(),
                count: 1,
            };

            self.queue.enqueue_asset_generation(&data).await?;
        }

        // Enqueue map generation if requested
        if generate_map {
            // Map generation only supported for location states (has map_overlay field)
            if location_state.is_none() {
                return Err(CatalogError::Validation(
                    "Map generation is only supported for location states".to_string(),
                ));
            }

            // Map workflow must be configured
            let map_workflow_id = map_workflow_id.ok_or_else(|| {
                CatalogError::Validation(
                    "Map workflow not configured. Please set map_workflow in request.".to_string(),
                )
            })?;

            let entity_id: String = location_state
                .as_ref()
                .map(|s| s.id().to_string())
                .ok_or_else(|| CatalogError::Validation("Failed to determine state ID".to_string()))?;

            let data = AssetGenerationData {
                world_id: Some(world_id),
                entity_type: "LocationStateMapOverlay".to_string(),
                entity_id,
                workflow_id: map_workflow_id,
                prompt: format!("Map overlay: {}", prompt),
                count: 1,
            };

            self.queue.enqueue_asset_generation(&data).await?;
        }

        Ok(GeneratedVisualState {
            location_state,
            region_state,
            generation_batch_id: batch_id,
            is_complete: false, // Assets are queued but not yet generated
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_domain::{Location, LocationName, LocationType, WorldId};
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

    #[tokio::test]
    async fn generate_map_without_workflow_returns_validation_error() {
        let mut mock_queue = MockQueueForTesting::new();
        // Enqueue should not be called
        mock_queue.expect_enqueue_asset_generation().never();

        let catalog = VisualStateCatalog::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockImageGenPort::new()),
            Arc::new(MockAssetRepo::new()),
            Arc::new(mock_queue),
            Arc::new(SystemClock::new()),
            Arc::new(SystemRandom::new()),
        );

        // Create a mock location
        let location_id = LocationId::new();
        let world_id = WorldId::new();
        let mut mock_location_repo = MockLocationRepo::new();
        mock_location_repo
            .expect_get_location()
            .returning(move |_| {
                Ok(Some(Location::new(
                    world_id,
                    LocationName::new("Test Location").unwrap(),
                    LocationType::Interior,
                )))
            });

        let catalog = VisualStateCatalog::new(
            Arc::new(mock_location_repo),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockImageGenPort::new()),
            Arc::new(MockAssetRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
            Arc::new(SystemRandom::new()),
        );

        // Try to generate map without workflow - should fail
        let result = catalog
            .generate_visual_state(
                Some(location_id),
                None,
                "Test State".to_string(),
                "Test description".to_string(),
                "Test prompt".to_string(),
                "backdrop_v1".to_string(),
                None, // map_workflow_id not provided
                vec![],
                false, // generate_backdrop false
                true,  // generate_map true - should fail
                vec![],
                ActivationLogic::All,
                0,
                false,
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CatalogError::Validation(msg) => {
                assert!(msg.contains("map_workflow"));
            }
            _ => panic!("Expected CatalogError::Validation"),
        }
    }

    #[tokio::test]
    async fn generate_map_with_workflow_enqueues_asset() {
        let mut mock_queue = MockQueueForTesting::new();
        let world_id = WorldId::new();
        let location_id = LocationId::new();

        // Enqueue should be called for map generation
        mock_queue
            .expect_enqueue_asset_generation()
            .times(1)
            .withf(move |data| {
                data.workflow_id == "map_v1"
                    && data.entity_type == "LocationStateMapOverlay"
                    && data.prompt.contains("Map overlay:")
            })
            .returning(|_| Ok(()));

        // Mock location repo
        let mut mock_location_repo = MockLocationRepo::new();
        mock_location_repo
            .expect_get_location()
            .returning(move |_| {
                Ok(Some(Location::new(
                    world_id,
                    LocationName::new("Test Location").unwrap(),
                    LocationType::Interior,
                )))
            });

        let catalog = VisualStateCatalog::new(
            Arc::new(mock_location_repo),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockImageGenPort::new()),
            Arc::new(MockAssetRepo::new()),
            Arc::new(mock_queue),
            Arc::new(SystemClock::new()),
            Arc::new(SystemRandom::new()),
        );

        // Generate with map workflow provided
        let result = catalog
            .generate_visual_state(
                Some(location_id),
                None,
                "Test State".to_string(),
                "Test description".to_string(),
                "Test prompt".to_string(),
                "backdrop_v1".to_string(),
                Some("map_v1".to_string()), // map_workflow provided
                vec![],
                false, // generate_backdrop false
                true,  // generate_map true - should succeed
                vec![],
                ActivationLogic::All,
                0,
                false,
            )
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.is_complete);
    }

    #[tokio::test]
    async fn generate_map_for_region_state_fails_validation() {
        let mut mock_queue = MockQueueForTesting::new();
        // Enqueue should not be called
        mock_queue.expect_enqueue_asset_generation().never();

        let catalog = VisualStateCatalog::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockImageGenPort::new()),
            Arc::new(MockAssetRepo::new()),
            Arc::new(mock_queue),
            Arc::new(SystemClock::new()),
            Arc::new(SystemRandom::new()),
        );

        // Try to generate map for region state - should fail
        let result = catalog
            .generate_visual_state(
                None,
                Some(RegionId::new()),
                "Test State".to_string(),
                "Test description".to_string(),
                "Test prompt".to_string(),
                "backdrop_v1".to_string(),
                Some("map_v1".to_string()),
                vec![],
                false,
                true, // generate_map true for region - should fail
                vec![],
                ActivationLogic::All,
                0,
                false,
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CatalogError::Validation(msg) => {
                assert!(msg.contains("Map generation is only supported for location states"));
            }
            _ => panic!("Expected CatalogError::Validation"),
        }
    }
}
