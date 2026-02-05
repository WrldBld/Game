//! Visual state generation use case.
//!
//! Handles generation of new visual states with asset generation queueing.
//! This use case is separate from the catalog CRUD operations to avoid
//! "god object" growth in VisualStateCatalog.

use std::sync::Arc;
use thiserror::Error;

use wrldbldr_domain::{
    ActivationLogic, ActivationRule, BatchId, Description, DomainError, LocationId,
    LocationState, LocationStateId, RegionId, RegionState, RegionStateId,
};

use crate::infrastructure::ports::{
    ClockPort, LocationRepo, LocationStateRepo, QueuePort, RegionStateRepo, RepoError,
};
use crate::queue_types::AssetGenerationData;
use crate::use_cases::visual_state::validation::{
    validate_generation_prompt, validate_negative_prompt, validate_state_description,
    validate_state_name_required, validate_workflow_id, MAX_PROMPT_LENGTH,
};

/// Error type for visual state generation operations
#[derive(Debug, Error)]
pub enum GenerationError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Location {0} not found")]
    LocationNotFound(LocationId),

    #[error("Region {0} not found")]
    RegionNotFound(RegionId),

    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),

    #[error("Queue error: {0}")]
    Queue(#[from] crate::infrastructure::ports::QueueError),

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
}

/// Generated visual state result
#[derive(Debug, Clone)]
pub struct GeneratedVisualState {
    pub location_state: Option<LocationState>,
    pub region_state: Option<RegionState>,
    pub generation_batch_id: BatchId,
    pub is_complete: bool,
}

/// Target for visual state generation (Location or Region).
///
/// Using an enum ensures exactly one target is specified at compile time,
/// avoiding the invalid state where both or neither location_id/region_id is set.
#[derive(Debug, Clone)]
pub enum GenerateVisualStateTarget {
    Location(LocationId),
    Region(RegionId),
}

/// Input parameters for generating a visual state.
///
/// This struct encapsulates all parameters needed for visual state generation,
/// reducing call-site brittleness and centralizing validation.
#[derive(Debug, Clone)]
pub struct GenerateVisualStateInput {
    pub target: GenerateVisualStateTarget,
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub workflow_id: String,
    pub map_workflow_id: Option<String>,
    pub generate_backdrop: bool,
    pub generate_map: bool,
    pub activation_rules: Vec<ActivationRule>,
    pub activation_logic: ActivationLogic,
    pub priority: i32,
    pub is_default: bool,
}

impl GenerateVisualStateInput {
    /// Create a new input for generating a location state.
    #[allow(clippy::too_many_arguments)]
    pub fn for_location(
        location_id: LocationId,
        name: String,
        description: String,
        prompt: String,
        workflow_id: String,
        generate_backdrop: bool,
        generate_map: bool,
        activation_rules: Vec<ActivationRule>,
        activation_logic: ActivationLogic,
        priority: i32,
        is_default: bool,
    ) -> Self {
        Self {
            target: GenerateVisualStateTarget::Location(location_id),
            name,
            description,
            prompt,
            negative_prompt: None,
            workflow_id,
            map_workflow_id: None,
            generate_backdrop,
            generate_map,
            activation_rules,
            activation_logic,
            priority,
            is_default,
        }
    }

    /// Create a new input for generating a region state.
    #[allow(clippy::too_many_arguments)]
    pub fn for_region(
        region_id: RegionId,
        name: String,
        description: String,
        prompt: String,
        workflow_id: String,
        generate_backdrop: bool,
        activation_rules: Vec<ActivationRule>,
        activation_logic: ActivationLogic,
        priority: i32,
        is_default: bool,
    ) -> Self {
        Self {
            target: GenerateVisualStateTarget::Region(region_id),
            name,
            description,
            prompt,
            negative_prompt: None,
            workflow_id,
            map_workflow_id: None,
            generate_backdrop,
            generate_map: false, // Map generation not supported for regions
            activation_rules,
            activation_logic,
            priority,
            is_default,
        }
    }

    /// Set the map workflow ID for map generation.
    pub fn with_map_workflow(mut self, map_workflow_id: String) -> Self {
        self.map_workflow_id = Some(map_workflow_id);
        self
    }

    /// Set the negative prompt for generation.
    pub fn with_negative_prompt(mut self, negative_prompt: String) -> Self {
        self.negative_prompt = Some(negative_prompt);
        self
    }
}

/// Visual state generation use case.
///
/// Handles creation of new visual states with asset generation queueing.
/// This is separate from VisualStateCatalog which handles CRUD for existing states.
pub struct GenerateVisualState {
    location_repo: Arc<dyn LocationRepo>,
    location_state_repo: Arc<dyn LocationStateRepo>,
    region_state_repo: Arc<dyn RegionStateRepo>,
    queue: Arc<dyn QueuePort>,
    clock: Arc<dyn ClockPort>,
}

impl GenerateVisualState {
    pub fn new(
        location_repo: Arc<dyn LocationRepo>,
        location_state_repo: Arc<dyn LocationStateRepo>,
        region_state_repo: Arc<dyn RegionStateRepo>,
        queue: Arc<dyn QueuePort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            location_repo,
            location_state_repo,
            region_state_repo,
            queue,
            clock,
        }
    }

    /// Generate a new visual state with asset generation
    pub async fn execute(
        &self,
        input: GenerateVisualStateInput,
    ) -> Result<GeneratedVisualState, GenerationError> {
        // Validate name using shared helper (required for generation)
        let result = validate_state_name_required(&input.name);
        if !result.is_valid() {
            return Err(GenerationError::Validation(
                result.error_message().unwrap_or("Invalid name").to_string(),
            ));
        }

        // Validate description using shared helper
        let result = validate_state_description(Some(&input.description));
        if !result.is_valid() {
            return Err(GenerationError::Validation(
                result.error_message().unwrap_or("Invalid description").to_string(),
            ));
        }

        // Validate generation prompt using shared helper
        let result = validate_generation_prompt(Some(&input.prompt));
        if !result.is_valid() {
            return Err(GenerationError::Validation(
                result.error_message().unwrap_or("Invalid generation prompt").to_string(),
            ));
        }

        // Validate workflow_id using shared helper
        let result = validate_workflow_id(Some(&input.workflow_id));
        if !result.is_valid() {
            return Err(GenerationError::Validation(
                result.error_message().unwrap_or("Invalid workflow ID").to_string(),
            ));
        }

        // Validate negative_prompt using shared helper
        let result = validate_negative_prompt(input.negative_prompt.as_deref());
        if !result.is_valid() {
            return Err(GenerationError::Validation(
                result.error_message().unwrap_or("Invalid negative prompt").to_string(),
            ));
        }

        // Validate map generation constraints BEFORE any state creation or persistence
        if input.generate_map {
            // Map generation only supported for location states (has map_overlay field)
            match &input.target {
                GenerateVisualStateTarget::Region(_) => {
                    return Err(GenerationError::Validation(
                        "Map generation is only supported for location states".to_string(),
                    ));
                }
                GenerateVisualStateTarget::Location(_) => {}
            }

            // Map workflow must be configured
            let map_workflow_id = input.map_workflow_id.clone().ok_or_else(|| {
                GenerationError::Validation(
                    "Map workflow not configured. Please set map_workflow in request.".to_string(),
                )
            })?;

            // Validate map workflow ID using shared helper
            let result = validate_workflow_id(Some(&map_workflow_id));
            if !result.is_valid() {
                return Err(GenerationError::Validation(
                    result.error_message().unwrap_or("Invalid map workflow ID").to_string(),
                ));
            }

            // Validate the derived map prompt length (includes "Map overlay: " prefix)
            let map_prompt = format!("Map overlay: {}", input.prompt);
            if map_prompt.chars().count() > MAX_PROMPT_LENGTH {
                return Err(GenerationError::Validation(format!(
                    "Map generation prompt too long (max {} chars including 'Map overlay: ' prefix)",
                    MAX_PROMPT_LENGTH
                )));
            }
        }

        // Determine target and get world_id
        // Construct state in memory first - don't persist yet
        let (world_id, location_state, region_state) = match &input.target {
            GenerateVisualStateTarget::Location(loc_id) => {
                let location = self.location_repo.get_location(*loc_id).await?
                    .ok_or(GenerationError::LocationNotFound(*loc_id))?;

                let state_id = LocationStateId::new();
                let now = self.clock.now();

                let state = LocationState::new_with_id(state_id, *loc_id, location.world_id(), &input.name, now)?
                    .with_description(Description::new(input.description.clone())?)
                    .with_generation_prompt(input.prompt.clone())
                    .with_workflow_id(input.workflow_id.clone())
                    .with_rules(input.activation_rules.clone(), input.activation_logic)
                    .with_priority(input.priority)
                    .with_is_default(input.is_default);

                (location.world_id(), Some(state), None)
            }
            GenerateVisualStateTarget::Region(reg_id) => {
                let region = self.location_repo.get_region(*reg_id).await?
                    .ok_or(GenerationError::RegionNotFound(*reg_id))?;

                // Fetch the location to get world_id
                let location = self.location_repo.get_location(region.location_id()).await?
                    .ok_or(GenerationError::LocationNotFound(region.location_id()))?;

                let state_id = RegionStateId::new();
                let now = self.clock.now();

                let description = Description::new(input.description.clone())?;

                let state = RegionState::new_with_id(state_id, *reg_id, region.location_id(), location.world_id(), &input.name, now)?
                    .with_description(description)
                    .with_generation_prompt(input.prompt.clone())
                    .with_workflow_id(input.workflow_id.clone())
                    .with_rules(input.activation_rules.clone(), input.activation_logic)
                    .with_priority(input.priority)
                    .with_is_default(input.is_default);

                (location.world_id(), None, Some(state))
            }
        };

        // Create generation batch ID as typed BatchId
        let batch_id = BatchId::new();

        // If no assets are requested, save state and return immediately with is_complete: true
        if !input.generate_backdrop && !input.generate_map {
            // Save the state for non-asset generation
            if let Some(ref ls) = location_state {
                self.location_state_repo.save(ls).await?;
            }
            if let Some(ref rs) = region_state {
                self.region_state_repo.save(rs).await?;
            }

            return Ok(GeneratedVisualState {
                location_state,
                region_state,
                generation_batch_id: batch_id,
                is_complete: true, // No assets to generate, so generation is complete
            });
        }

        // Save the state BEFORE enqueueing so queue workers always see persisted state
        // If save fails, we fail fast without any side effects
        if let Some(ref ls) = location_state {
            self.location_state_repo.save(ls).await?;
        }
        if let Some(ref rs) = region_state {
            self.region_state_repo.save(rs).await?;
        }

        // Track enqueued items for rollback on failure
        let mut enqueued_items: Vec<wrldbldr_domain::QueueItemId> = Vec::new();
        let mut enqueue_error: Option<crate::infrastructure::ports::QueueError> = None;

        // Enqueue asset generation if requested
        // State is already persisted, so queue workers will see it
        if input.generate_backdrop {
            let entity_id: String = location_state
                .as_ref()
                .map(|s| s.id().to_string())
                .or_else(|| region_state.as_ref().map(|s| s.id().to_string()))
                .ok_or_else(|| GenerationError::Validation("Failed to determine state ID".to_string()))?;

            let entity_type: String = location_state.as_ref()
                .map(|_| "LocationState".to_string())
                .or_else(|| region_state.as_ref().map(|_| "RegionState".to_string()))
                .ok_or_else(|| GenerationError::Validation("Failed to determine entity type".to_string()))?;

            let data = AssetGenerationData {
                world_id: Some(world_id),
                entity_type,
                entity_id,
                workflow_id: input.workflow_id.clone(),
                prompt: input.prompt.clone(),
                negative_prompt: input.negative_prompt.clone(),
                count: 1,
                model_id: None, // TODO: Get from settings
                prompt_prefix: None, // TODO: Get from settings
                prompt_suffix: None, // TODO: Get from settings
                score_tokens_enabled: None, // TODO: Get from settings
                pose_asset_id: None, // TODO: Get from settings
                controlnet_mappings: None, // TODO: Get from settings
            };

            match self.queue.enqueue_asset_generation(&data).await {
                Ok(item_id) => {
                    enqueued_items.push(item_id);
                }
                Err(e) => {
                    enqueue_error = Some(e);
                }
            }
        }

        // Enqueue map generation if requested (validation already done above) and no error yet
        if enqueue_error.is_none() && input.generate_map {
            // Get map workflow ID - validation ensures this is Some when generate_map is true
            let map_workflow_id = match &input.map_workflow_id {
                Some(id) => id.clone(),
                None => {
                    // This is unreachable because validation already ensured map_workflow_id is Some
                    // when generate_map is true. If we get here, it's a logic bug.
                    unreachable!("Map workflow ID was validated but is now missing")
                }
            };
                let entity_id: String = location_state
                    .as_ref()
                    .map(|s| s.id().to_string())
                    .ok_or_else(|| GenerationError::Validation("Failed to determine state ID".to_string()))?;

                let data = AssetGenerationData {
                    world_id: Some(world_id),
                    entity_type: "LocationStateMapOverlay".to_string(),
                    entity_id,
                    workflow_id: map_workflow_id,
                    prompt: format!("Map overlay: {}", input.prompt),
                    negative_prompt: input.negative_prompt.clone(),
                    count: 1,
                    model_id: None, // TODO: Get from settings
                    prompt_prefix: None, // TODO: Get from settings
                    prompt_suffix: None, // TODO: Get from settings
                    score_tokens_enabled: None, // TODO: Get from settings
                    pose_asset_id: None, // TODO: Get from settings
                    controlnet_mappings: None, // TODO: Get from settings
                };

                match self.queue.enqueue_asset_generation(&data).await {
                    Ok(item_id) => {
                        enqueued_items.push(item_id);
                    }
                    Err(e) => {
                        enqueue_error = Some(e);
                    }
                }
        }

        // If there was an enqueue error, we need to:
        // 1. Mark previously enqueued items as failed (best-effort)
        // 2. Delete the saved state (best-effort rollback)
        // 3. Return the error
        if let Some(error) = enqueue_error {
            // Mark enqueued items as failed
            for item_id in &enqueued_items {
                if let Err(e) = self.queue.mark_failed(*item_id, "Partial enqueue rollback: subsequent enqueue failed").await {
                    tracing::warn!(error = %e, item_id = %item_id, "Failed to mark queue item as failed during partial enqueue rollback");
                }
            }

            // Delete the saved state (best-effort rollback)
            // We attempt to clean up the orphaned state since enqueue failed
            let delete_result = match (&location_state, &region_state) {
                (Some(ls), None) => self.location_state_repo.delete(ls.id()).await,
                (None, Some(rs)) => self.region_state_repo.delete(rs.id()).await,
                _ => unreachable!("Exactly one of location_state or region_state must be set"),
            };
            if let Err(e) = delete_result {
                tracing::warn!(error = %e, "Failed to delete state during enqueue rollback - state may be orphaned");
            }

            return Err(GenerationError::Queue(error));
        }

        // All enqueues succeeded - state was already saved before enqueueing
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
        MockLocationRepo, MockLocationStateRepo, MockRegionStateRepo,
    };
    use crate::infrastructure::clock::SystemClock;
    use crate::test_fixtures::queue_mocks::MockQueueForTesting;
    use crate::use_cases::visual_state::validation::{
        MAX_PROMPT_LENGTH, MAX_STATE_DESCRIPTION, MAX_STATE_NAME, MAX_WORKFLOW_LENGTH,
    };

    #[tokio::test]
    async fn generate_with_empty_name_returns_validation_error() {
        let location_id = LocationId::new();

        let generator = GenerateVisualState::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
        );

        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "".to_string(), // empty name
            description: "Test description".to_string(),
            prompt: "Test prompt".to_string(),
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: None,
            generate_backdrop: false,
            generate_map: false,
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Validation(msg) => {
                assert!(msg.contains("cannot be empty"));
            }
            _ => panic!("Expected GenerationError::Validation"),
        }
    }

    #[tokio::test]
    async fn generate_with_name_too_long_returns_validation_error() {
        let location_id = LocationId::new();

        let generator = GenerateVisualState::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
        );

        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "A".repeat(MAX_STATE_NAME + 1), // name too long
            description: "Test description".to_string(),
            prompt: "Test prompt".to_string(),
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: None,
            generate_backdrop: false,
            generate_map: false,
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Validation(msg) => {
                assert!(msg.contains("Name too long"));
            }
            _ => panic!("Expected GenerationError::Validation"),
        }
    }

    #[tokio::test]
    async fn generate_with_description_too_long_returns_validation_error() {
        let location_id = LocationId::new();

        let generator = GenerateVisualState::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
        );

        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "Test State".to_string(),
            description: "A".repeat(MAX_STATE_DESCRIPTION + 1), // description too long
            prompt: "Test prompt".to_string(),
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: None,
            generate_backdrop: false,
            generate_map: false,
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Validation(msg) => {
                assert!(msg.contains("Description too long"));
            }
            _ => panic!("Expected GenerationError::Validation"),
        }
    }

    #[tokio::test]
    async fn generate_with_prompt_too_long_returns_validation_error() {
        let location_id = LocationId::new();

        let generator = GenerateVisualState::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
        );

        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "Test State".to_string(),
            description: "Test description".to_string(),
            prompt: "A".repeat(MAX_PROMPT_LENGTH + 1), // prompt too long
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: None,
            generate_backdrop: false,
            generate_map: false,
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Validation(msg) => {
                assert!(msg.contains("Generation prompt too long"));
            }
            _ => panic!("Expected GenerationError::Validation"),
        }
    }

    #[tokio::test]
    async fn generate_with_workflow_id_too_long_returns_validation_error() {
        let location_id = LocationId::new();

        let generator = GenerateVisualState::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
        );

        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "Test State".to_string(),
            description: "Test description".to_string(),
            prompt: "Test prompt".to_string(),
            negative_prompt: None,
            workflow_id: "A".repeat(MAX_WORKFLOW_LENGTH + 1), // workflow_id too long
            map_workflow_id: None,
            generate_backdrop: false,
            generate_map: false,
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Validation(msg) => {
                assert!(msg.contains("Workflow ID too long"));
            }
            _ => panic!("Expected GenerationError::Validation"),
        }
    }

    #[tokio::test]
    async fn generate_map_with_map_workflow_id_too_long_returns_validation_error() {
        let location_id = LocationId::new();

        let generator = GenerateVisualState::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
        );

        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "Test State".to_string(),
            description: "Test description".to_string(),
            prompt: "Test prompt".to_string(),
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: Some("A".repeat(MAX_WORKFLOW_LENGTH + 1)), // map_workflow_id too long
            generate_backdrop: false,
            generate_map: true,
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Validation(msg) => {
                assert!(msg.contains("Workflow ID too long"));
            }
            _ => panic!("Expected GenerationError::Validation"),
        }
    }

    #[tokio::test]
    async fn generate_map_with_prompt_just_under_limit_but_map_prefix_pushes_over_returns_validation_error() {
        // "Map overlay: " prefix is 13 characters
        // If prompt is MAX_PROMPT_LENGTH - 10 chars, adding prefix makes it exceed
        let location_id = LocationId::new();

        let generator = GenerateVisualState::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
        );

        // Prompt is valid on its own (just under limit), but "Map overlay: " prefix pushes it over
        let prompt = "A".repeat(MAX_PROMPT_LENGTH - 10);
        assert!(prompt.chars().count() < MAX_PROMPT_LENGTH); // Verify prompt itself is valid

        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "Test State".to_string(),
            description: "Test description".to_string(),
            prompt,
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: Some("map_v1".to_string()),
            generate_backdrop: false,
            generate_map: true, // Map generation enabled - should fail due to combined length
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Validation(msg) => {
                assert!(msg.contains("Map generation prompt too long"));
                assert!(msg.contains("Map overlay"));
            }
            _ => panic!("Expected GenerationError::Validation"),
        }
    }

    #[tokio::test]
    async fn generate_map_without_workflow_returns_validation_error() {
        // Validation happens before any repo calls, so no mocks needed
        let location_id = LocationId::new();

        // No repo expectations - validation should fail before any calls
        let generator = GenerateVisualState::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
        );

        // Try to generate map without workflow - should fail validation before any persistence
        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "Test State".to_string(),
            description: "Test description".to_string(),
            prompt: "Test prompt".to_string(),
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: None, // map_workflow_id not provided
            generate_backdrop: false,
            generate_map: true, // generate_map true - should fail validation
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Validation(msg) => {
                assert!(msg.contains("map_workflow"));
            }
            _ => panic!("Expected GenerationError::Validation"),
        }
    }

    #[tokio::test]
    async fn generate_map_with_workflow_succeeds() {
        let world_id = WorldId::new();
        let location_id = LocationId::new();

        // Mock location repo
        let mut mock_location_repo = MockLocationRepo::new();
        mock_location_repo
            .expect_get_location()
            .returning(move |_| {
                Ok(Some(Location::new(
                    world_id,
                    LocationName::new("Test Location").expect("valid location name"),
                    LocationType::Interior,
                )))
            });

        // Mock location_state_repo.save - called when creating location state
        let mut mock_location_state_repo = MockLocationStateRepo::new();
        mock_location_state_repo
            .expect_save()
            .returning(|_| Ok(()));

        let generator = GenerateVisualState::new(
            Arc::new(mock_location_repo),
            Arc::new(mock_location_state_repo),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
        );

        // Generate with map workflow provided
        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "Test State".to_string(),
            description: "Test description".to_string(),
            prompt: "Test prompt".to_string(),
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: Some("map_v1".to_string()), // map_workflow provided
            generate_backdrop: false,
            generate_map: true, // generate_map true - should succeed
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        assert!(result.is_ok());
        let result = result.expect("result should be Ok");
        assert!(!result.is_complete);
    }

    #[tokio::test]
    async fn generate_map_for_region_state_fails_validation() {
        let region_id = RegionId::new();

        // Validation happens before any repo calls, so no mocks needed
        let generator = GenerateVisualState::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
        );

        // Try to generate map for region state - should fail validation before any persistence
        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Region(region_id),
            name: "Test State".to_string(),
            description: "Test description".to_string(),
            prompt: "Test prompt".to_string(),
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: Some("map_v1".to_string()),
            generate_backdrop: false,
            generate_map: true, // generate_map true for region - should fail validation
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Validation(msg) => {
                assert!(msg.contains("Map generation is only supported for location states"));
            }
            _ => panic!("Expected GenerationError::Validation"),
        }
    }

    #[tokio::test]
    async fn generate_no_assets_returns_is_complete_true() {
        let world_id = WorldId::new();
        let location_id = LocationId::new();

        // Mock location repo
        let mut mock_location_repo = MockLocationRepo::new();
        mock_location_repo
            .expect_get_location()
            .returning(move |_| {
                Ok(Some(Location::new(
                    world_id,
                    LocationName::new("Test Location").expect("valid location name"),
                    LocationType::Interior,
                )))
            });

        // Mock location_state_repo.save - called when creating location state
        let mut mock_location_state_repo = MockLocationStateRepo::new();
        mock_location_state_repo
            .expect_save()
            .returning(|_| Ok(()));

        let generator = GenerateVisualState::new(
            Arc::new(mock_location_repo),
            Arc::new(mock_location_state_repo),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
        );

        // Generate with no assets requested
        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "Test State".to_string(),
            description: "Test description".to_string(),
            prompt: "Test prompt".to_string(),
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: None,
            generate_backdrop: false,
            generate_map: false,
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        assert!(result.is_ok());
        let result = result.expect("result should be Ok");
        assert!(result.is_complete, "is_complete should be true when no assets are requested");
    }

    #[tokio::test]
    async fn generate_queue_failure_rolls_back_saved_state() {
        // With save-before-enqueue ordering, if enqueue fails, the saved state is deleted (best-effort rollback)
        use crate::infrastructure::ports::{QueueError, QueueItem};
        use async_trait::async_trait;
        use crate::queue_types::{ApprovalRequestData, LlmRequestData, PlayerActionData, QueueType};

        let world_id = WorldId::new();
        let location_id = LocationId::new();

        // Mock location repo
        let mut mock_location_repo = MockLocationRepo::new();
        mock_location_repo
            .expect_get_location()
            .returning(move |_| {
                Ok(Some(Location::new(
                    world_id,
                    LocationName::new("Test Location").expect("valid location name"),
                    LocationType::Interior,
                )))
            });

        // Mock location_state_repo - save succeeds, delete succeeds (rollback)
        let mut mock_location_state_repo = MockLocationStateRepo::new();
        mock_location_state_repo
            .expect_save()
            .returning(|_| Ok(()));
        mock_location_state_repo
            .expect_delete()
            .returning(|_| Ok(()));

        // Create a mock queue that always fails enqueue
        struct AlwaysFailingQueue;

        #[async_trait]
        impl QueuePort for AlwaysFailingQueue {
            async fn enqueue_player_action(
                &self,
                _data: &PlayerActionData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
            async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn enqueue_llm_request(
                &self,
                _data: &LlmRequestData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
            async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn enqueue_dm_approval(
                &self,
                _data: &ApprovalRequestData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
            async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn enqueue_asset_generation(
                &self,
                _data: &AssetGenerationData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Err(QueueError::database("enqueue", "queue full"))
            }
            async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn mark_complete(&self, _id: wrldbldr_domain::QueueItemId) -> Result<(), QueueError> {
                Ok(())
            }
            async fn mark_failed(&self, _id: wrldbldr_domain::QueueItemId, _error: &str) -> Result<(), QueueError> {
                Ok(())
            }
            async fn get_pending_count(&self, _queue_type: QueueType) -> Result<usize, QueueError> {
                Ok(0)
            }
            async fn list_by_type(
                &self,
                _queue_type: QueueType,
                _limit: usize,
            ) -> Result<Vec<QueueItem>, QueueError> {
                Ok(vec![])
            }
            async fn set_result_json(
                &self,
                _id: wrldbldr_domain::QueueItemId,
                _result_json: &str,
            ) -> Result<(), QueueError> {
                Ok(())
            }
            async fn cancel_pending_llm_request_by_callback_id(
                &self,
                _callback_id: &str,
            ) -> Result<bool, QueueError> {
                Ok(false)
            }
            async fn get_approval_request(
                &self,
                _id: wrldbldr_domain::QueueItemId,
            ) -> Result<Option<ApprovalRequestData>, QueueError> {
                Ok(None)
            }
            async fn get_generation_read_state(
                &self,
                _user_id: &str,
                _world_id: wrldbldr_domain::WorldId,
            ) -> Result<Option<(Vec<String>, Vec<String>)>, QueueError> {
                Ok(None)
            }
            async fn upsert_generation_read_state(
                &self,
                _user_id: &str,
                _world_id: wrldbldr_domain::WorldId,
                _read_batches: &[String],
                _read_suggestions: &[String],
            ) -> Result<(), QueueError> {
                Ok(())
            }
            async fn delete_by_callback_id(&self, _callback_id: &str) -> Result<bool, QueueError> {
                Ok(false)
            }
            async fn advance_phased_llm_request(
                &self,
                _item_id: wrldbldr_domain::QueueItemId,
                _next_request: &LlmRequestData,
                _phase_key: &str,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
        }

        let generator = GenerateVisualState::new(
            Arc::new(mock_location_repo),
            Arc::new(mock_location_state_repo),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(AlwaysFailingQueue),
            Arc::new(SystemClock::new()),
        );

        // Generate with backdrop - save succeeds, enqueue fails, delete (rollback) succeeds
        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "Test State".to_string(),
            description: "Test description".to_string(),
            prompt: "Test prompt".to_string(),
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: None,
            generate_backdrop: true,
            generate_map: false,
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        // Should fail with queue error - state was saved then rolled back
        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Queue(_) => {
                // Expected - queue error should be returned
            }
            other => panic!("Expected GenerationError::Queue, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn generate_partial_enqueue_failure_rolls_back_first_item() {
        use crate::infrastructure::ports::{QueueError, QueueItem};
        use async_trait::async_trait;
        use crate::queue_types::{ApprovalRequestData, LlmRequestData, PlayerActionData, QueueType};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc as StdArc;

        let world_id = WorldId::new();
        let location_id = LocationId::new();

        // Mock location repo
        let mut mock_location_repo = MockLocationRepo::new();
        mock_location_repo
            .expect_get_location()
            .returning(move |_| {
                Ok(Some(Location::new(
                    world_id,
                    LocationName::new("Test Location").expect("valid location name"),
                    LocationType::Interior,
                )))
            });

        // Mock location_state_repo - save succeeds, delete succeeds (rollback)
        let mut mock_location_state_repo = MockLocationStateRepo::new();
        mock_location_state_repo
            .expect_save()
            .returning(|_| Ok(()));
        mock_location_state_repo
            .expect_delete()
            .returning(|_| Ok(()));

        // Track enqueue calls and mark_failed calls
        let enqueue_count = StdArc::new(AtomicUsize::new(0));
        let mark_failed_count = StdArc::new(AtomicUsize::new(0));
        let enqueue_count_clone = StdArc::clone(&enqueue_count);
        let mark_failed_count_clone = StdArc::clone(&mark_failed_count);

        // Create a mock queue that succeeds on first enqueue but fails on second
        struct PartialFailingQueue {
            enqueue_count: StdArc<AtomicUsize>,
            mark_failed_count: StdArc<AtomicUsize>,
        }

        #[async_trait]
        impl QueuePort for PartialFailingQueue {
            async fn enqueue_player_action(
                &self,
                _data: &PlayerActionData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
            async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn enqueue_llm_request(
                &self,
                _data: &LlmRequestData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
            async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn enqueue_dm_approval(
                &self,
                _data: &ApprovalRequestData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
            async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn enqueue_asset_generation(
                &self,
                _data: &AssetGenerationData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                let count = self.enqueue_count.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    // First enqueue (backdrop) succeeds
                    Ok(wrldbldr_domain::QueueItemId::new())
                } else {
                    // Second enqueue (map) fails
                    Err(QueueError::database("enqueue", "queue full"))
                }
            }
            async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn mark_complete(&self, _id: wrldbldr_domain::QueueItemId) -> Result<(), QueueError> {
                Ok(())
            }
            async fn mark_failed(&self, _id: wrldbldr_domain::QueueItemId, _error: &str) -> Result<(), QueueError> {
                self.mark_failed_count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            async fn get_pending_count(&self, _queue_type: QueueType) -> Result<usize, QueueError> {
                Ok(0)
            }
            async fn list_by_type(
                &self,
                _queue_type: QueueType,
                _limit: usize,
            ) -> Result<Vec<QueueItem>, QueueError> {
                Ok(vec![])
            }
            async fn set_result_json(
                &self,
                _id: wrldbldr_domain::QueueItemId,
                _result_json: &str,
            ) -> Result<(), QueueError> {
                Ok(())
            }
            async fn cancel_pending_llm_request_by_callback_id(
                &self,
                _callback_id: &str,
            ) -> Result<bool, QueueError> {
                Ok(false)
            }
            async fn get_approval_request(
                &self,
                _id: wrldbldr_domain::QueueItemId,
            ) -> Result<Option<ApprovalRequestData>, QueueError> {
                Ok(None)
            }
            async fn get_generation_read_state(
                &self,
                _user_id: &str,
                _world_id: wrldbldr_domain::WorldId,
            ) -> Result<Option<(Vec<String>, Vec<String>)>, QueueError> {
                Ok(None)
            }
            async fn upsert_generation_read_state(
                &self,
                _user_id: &str,
                _world_id: wrldbldr_domain::WorldId,
                _read_batches: &[String],
                _read_suggestions: &[String],
            ) -> Result<(), QueueError> {
                Ok(())
            }
            async fn delete_by_callback_id(&self, _callback_id: &str) -> Result<bool, QueueError> {
                Ok(false)
            }
            async fn advance_phased_llm_request(
                &self,
                _item_id: wrldbldr_domain::QueueItemId,
                _next_request: &LlmRequestData,
                _phase_key: &str,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
        }

        let generator = GenerateVisualState::new(
            Arc::new(mock_location_repo),
            Arc::new(mock_location_state_repo),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(PartialFailingQueue { 
                enqueue_count: enqueue_count_clone, 
                mark_failed_count: mark_failed_count_clone 
            }),
            Arc::new(SystemClock::new()),
        );

        // Generate with both backdrop and map - backdrop enqueue succeeds, map enqueue fails
        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "Test State".to_string(),
            description: "Test description".to_string(),
            prompt: "Test prompt".to_string(),
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: Some("map_v1".to_string()),
            generate_backdrop: true,
            generate_map: true,
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        // Should fail with queue error
        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Queue(_) => {
                // Expected - queue error should be returned
            }
            other => panic!("Expected GenerationError::Queue, got {:?}", other),
        }

        // mark_failed should have been called for the first (backdrop) enqueued item
        assert_eq!(mark_failed_count.load(Ordering::SeqCst), 1, "mark_failed should be called once for the first enqueued item");
        // Both enqueue attempts should have been made
        assert_eq!(enqueue_count.load(Ordering::SeqCst), 2, "both enqueue attempts should have been made");
    }

    #[tokio::test]
    async fn generate_save_failure_before_enqueue_returns_error_no_enqueue() {
        // With save-before-enqueue ordering, if save fails, no enqueue happens
        // This test verifies that save failure is caught early without side effects
        use crate::infrastructure::ports::{QueueError, QueueItem};
        use async_trait::async_trait;
        use crate::queue_types::{ApprovalRequestData, LlmRequestData, PlayerActionData, QueueType};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc as StdArc;

        let world_id = WorldId::new();
        let location_id = LocationId::new();

        // Mock location repo
        let mut mock_location_repo = MockLocationRepo::new();
        mock_location_repo
            .expect_get_location()
            .returning(move |_| {
                Ok(Some(Location::new(
                    world_id,
                    LocationName::new("Test Location").expect("valid location name"),
                    LocationType::Interior,
                )))
            });

        // Mock location_state_repo - save fails
        let mut mock_location_state_repo = MockLocationStateRepo::new();
        mock_location_state_repo
            .expect_save()
            .returning(|_| Err(RepoError::database("save", "database connection lost")));

        // Track enqueue calls (should be 0 since save fails first)
        let enqueue_count = StdArc::new(AtomicUsize::new(0));
        let enqueue_count_clone = StdArc::clone(&enqueue_count);

        // Create a mock queue that tracks enqueue attempts
        struct TrackingQueue {
            enqueue_count: StdArc<AtomicUsize>,
        }

        #[async_trait]
        impl QueuePort for TrackingQueue {
            async fn enqueue_player_action(
                &self,
                _data: &PlayerActionData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
            async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn enqueue_llm_request(
                &self,
                _data: &LlmRequestData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
            async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn enqueue_dm_approval(
                &self,
                _data: &ApprovalRequestData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
            async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn enqueue_asset_generation(
                &self,
                _data: &AssetGenerationData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                self.enqueue_count.fetch_add(1, Ordering::SeqCst);
                Ok(wrldbldr_domain::QueueItemId::new())
            }
            async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn mark_complete(&self, _id: wrldbldr_domain::QueueItemId) -> Result<(), QueueError> {
                Ok(())
            }
            async fn mark_failed(&self, _id: wrldbldr_domain::QueueItemId, _error: &str) -> Result<(), QueueError> {
                Ok(())
            }
            async fn get_pending_count(&self, _queue_type: QueueType) -> Result<usize, QueueError> {
                Ok(0)
            }
            async fn list_by_type(
                &self,
                _queue_type: QueueType,
                _limit: usize,
            ) -> Result<Vec<QueueItem>, QueueError> {
                Ok(vec![])
            }
            async fn set_result_json(
                &self,
                _id: wrldbldr_domain::QueueItemId,
                _result_json: &str,
            ) -> Result<(), QueueError> {
                Ok(())
            }
            async fn cancel_pending_llm_request_by_callback_id(
                &self,
                _callback_id: &str,
            ) -> Result<bool, QueueError> {
                Ok(false)
            }
            async fn get_approval_request(
                &self,
                _id: wrldbldr_domain::QueueItemId,
            ) -> Result<Option<ApprovalRequestData>, QueueError> {
                Ok(None)
            }
            async fn get_generation_read_state(
                &self,
                _user_id: &str,
                _world_id: wrldbldr_domain::WorldId,
            ) -> Result<Option<(Vec<String>, Vec<String>)>, QueueError> {
                Ok(None)
            }
            async fn upsert_generation_read_state(
                &self,
                _user_id: &str,
                _world_id: wrldbldr_domain::WorldId,
                _read_batches: &[String],
                _read_suggestions: &[String],
            ) -> Result<(), QueueError> {
                Ok(())
            }
            async fn delete_by_callback_id(&self, _callback_id: &str) -> Result<bool, QueueError> {
                Ok(false)
            }
            async fn advance_phased_llm_request(
                &self,
                _item_id: wrldbldr_domain::QueueItemId,
                _next_request: &LlmRequestData,
                _phase_key: &str,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
        }

        let generator = GenerateVisualState::new(
            Arc::new(mock_location_repo),
            Arc::new(mock_location_state_repo),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(TrackingQueue { enqueue_count: enqueue_count_clone }),
            Arc::new(SystemClock::new()),
        );

        // Generate with backdrop - save fails, so enqueue should never be called
        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "Test State".to_string(),
            description: "Test description".to_string(),
            prompt: "Test prompt".to_string(),
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: None,
            generate_backdrop: true,
            generate_map: false,
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        // Should fail with repo error
        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Repo(_) => {
                // Expected - repo error should be returned
            }
            other => panic!("Expected GenerationError::Repo, got {:?}", other),
        }

        // Enqueue should NOT have been called since save failed first
        assert_eq!(enqueue_count.load(Ordering::SeqCst), 0, "enqueue should not be called when save fails first");
    }

    #[tokio::test]
    async fn generate_enqueue_failure_deletes_saved_state() {
        // When save succeeds but enqueue fails, the saved state should be deleted (best-effort rollback)
        use crate::infrastructure::ports::{QueueError, QueueItem};
        use async_trait::async_trait;
        use crate::queue_types::{ApprovalRequestData, LlmRequestData, PlayerActionData, QueueType};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc as StdArc;

        let world_id = WorldId::new();
        let location_id = LocationId::new();

        // Mock location repo
        let mut mock_location_repo = MockLocationRepo::new();
        mock_location_repo
            .expect_get_location()
            .returning(move |_| {
                Ok(Some(Location::new(
                    world_id,
                    LocationName::new("Test Location").expect("valid location name"),
                    LocationType::Interior,
                )))
            });

        // Mock location_state_repo - save succeeds, delete succeeds
        let mut mock_location_state_repo = MockLocationStateRepo::new();
        mock_location_state_repo
            .expect_save()
            .returning(|_| Ok(()));
        mock_location_state_repo
            .expect_delete()
            .returning(|_| Ok(()));

        // Track mark_failed calls
        let mark_failed_count = StdArc::new(AtomicUsize::new(0));
        let mark_failed_count_clone = StdArc::clone(&mark_failed_count);

        // Create a mock queue that always fails enqueue
        struct AlwaysFailingQueue {
            mark_failed_count: StdArc<AtomicUsize>,
        }

        #[async_trait]
        impl QueuePort for AlwaysFailingQueue {
            async fn enqueue_player_action(
                &self,
                _data: &PlayerActionData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
            async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn enqueue_llm_request(
                &self,
                _data: &LlmRequestData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
            async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn enqueue_dm_approval(
                &self,
                _data: &ApprovalRequestData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
            async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn enqueue_asset_generation(
                &self,
                _data: &AssetGenerationData,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Err(QueueError::database("enqueue", "queue full"))
            }
            async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError> {
                Ok(None)
            }
            async fn mark_complete(&self, _id: wrldbldr_domain::QueueItemId) -> Result<(), QueueError> {
                Ok(())
            }
            async fn mark_failed(&self, _id: wrldbldr_domain::QueueItemId, _error: &str) -> Result<(), QueueError> {
                self.mark_failed_count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            async fn get_pending_count(&self, _queue_type: QueueType) -> Result<usize, QueueError> {
                Ok(0)
            }
            async fn list_by_type(
                &self,
                _queue_type: QueueType,
                _limit: usize,
            ) -> Result<Vec<QueueItem>, QueueError> {
                Ok(vec![])
            }
            async fn set_result_json(
                &self,
                _id: wrldbldr_domain::QueueItemId,
                _result_json: &str,
            ) -> Result<(), QueueError> {
                Ok(())
            }
            async fn cancel_pending_llm_request_by_callback_id(
                &self,
                _callback_id: &str,
            ) -> Result<bool, QueueError> {
                Ok(false)
            }
            async fn get_approval_request(
                &self,
                _id: wrldbldr_domain::QueueItemId,
            ) -> Result<Option<ApprovalRequestData>, QueueError> {
                Ok(None)
            }
            async fn get_generation_read_state(
                &self,
                _user_id: &str,
                _world_id: wrldbldr_domain::WorldId,
            ) -> Result<Option<(Vec<String>, Vec<String>)>, QueueError> {
                Ok(None)
            }
            async fn upsert_generation_read_state(
                &self,
                _user_id: &str,
                _world_id: wrldbldr_domain::WorldId,
                _read_batches: &[String],
                _read_suggestions: &[String],
            ) -> Result<(), QueueError> {
                Ok(())
            }
            async fn delete_by_callback_id(&self, _callback_id: &str) -> Result<bool, QueueError> {
                Ok(false)
            }
            async fn advance_phased_llm_request(
                &self,
                _item_id: wrldbldr_domain::QueueItemId,
                _next_request: &LlmRequestData,
                _phase_key: &str,
            ) -> Result<wrldbldr_domain::QueueItemId, QueueError> {
                Ok(wrldbldr_domain::QueueItemId::new())
            }
        }

        let generator = GenerateVisualState::new(
            Arc::new(mock_location_repo),
            Arc::new(mock_location_state_repo),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(AlwaysFailingQueue { mark_failed_count: mark_failed_count_clone }),
            Arc::new(SystemClock::new()),
        );

        // Generate with backdrop - save succeeds, enqueue fails
        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "Test State".to_string(),
            description: "Test description".to_string(),
            prompt: "Test prompt".to_string(),
            negative_prompt: None,
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: None,
            generate_backdrop: true,
            generate_map: false,
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        // Should fail with queue error
        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Queue(_) => {
                // Expected - queue error should be returned
            }
            other => panic!("Expected GenerationError::Queue, got {:?}", other),
        }

        // mark_failed should NOT be called since no items were successfully enqueued
        assert_eq!(mark_failed_count.load(Ordering::SeqCst), 0, "mark_failed should not be called when enqueue fails immediately");
    }

    #[tokio::test]
    async fn generate_with_negative_prompt_too_long_returns_validation_error() {
        use crate::use_cases::visual_state::validation::MAX_NEGATIVE_PROMPT_LENGTH;

        let location_id = LocationId::new();

        let generator = GenerateVisualState::new(
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockLocationStateRepo::new()),
            Arc::new(MockRegionStateRepo::new()),
            Arc::new(MockQueueForTesting::new()),
            Arc::new(SystemClock::new()),
        );

        let input = GenerateVisualStateInput {
            target: GenerateVisualStateTarget::Location(location_id),
            name: "Test State".to_string(),
            description: "Test description".to_string(),
            prompt: "Test prompt".to_string(),
            negative_prompt: Some("A".repeat(MAX_NEGATIVE_PROMPT_LENGTH + 1)), // negative_prompt too long
            workflow_id: "backdrop_v1".to_string(),
            map_workflow_id: None,
            generate_backdrop: false,
            generate_map: false,
            activation_rules: vec![],
            activation_logic: ActivationLogic::All,
            priority: 0,
            is_default: false,
        };

        let result = generator.execute(input).await;

        assert!(result.is_err());
        match result.expect_err("should fail") {
            GenerationError::Validation(msg) => {
                assert!(msg.contains("Negative prompt too long"));
            }
            _ => panic!("Expected GenerationError::Validation"),
        }
    }
}
