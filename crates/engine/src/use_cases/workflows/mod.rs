//! Workflow management use cases.
//!
//! Provides CRUD operations for workflow configurations and analysis.

use std::sync::Arc;

use wrldbldr_shared::workflow::{WorkflowConfiguration, WorkflowSlot};

use crate::infrastructure::ports::{ClockPort, QueuePort, RepoError, WorkflowConfigRepo};
use crate::queue_types::AssetGenerationData;

/// Errors that can occur during workflow operations.
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),

    #[error("Workflow configuration not found for slot: {0}")]
    NotFound(WorkflowSlot),

    #[error("Invalid workflow slot: {0}")]
    InvalidSlot(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Queue error: {0}")]
    Queue(#[from] crate::infrastructure::ports::QueueError),
}

/// Use case for listing all workflow configurations.
pub struct ListWorkflows {
    repo: Arc<dyn WorkflowConfigRepo>,
}

impl ListWorkflows {
    /// Create a new ListWorkflows use case.
    pub fn new(repo: Arc<dyn WorkflowConfigRepo>) -> Self {
        Self { repo }
    }

    /// Execute the use case.
    ///
    /// Returns a list of all configured workflow slots with their configuration status.
    pub async fn execute(&self,
    ) -> Result<Vec<(WorkflowSlot, Option<WorkflowConfiguration>)>, WorkflowError> {
        let configured = self.repo.list_all().await?;
        let configured_slots: std::collections::HashSet<WorkflowSlot> =
            configured.iter().map(|c| c.slot).collect();

        let mut result = Vec::new();
        for slot in WorkflowSlot::all() {
            let config = if configured_slots.contains(slot) {
                self.repo.get_by_slot(*slot).await?
            } else {
                None
            };
            result.push((*slot, config));
        }

        Ok(result)
    }
}

/// Use case for getting a workflow configuration by slot.
pub struct GetWorkflow {
    repo: Arc<dyn WorkflowConfigRepo>,
}

impl GetWorkflow {
    /// Create a new GetWorkflow use case.
    pub fn new(repo: Arc<dyn WorkflowConfigRepo>) -> Self {
        Self { repo }
    }

    /// Execute the use case.
    pub async fn execute(
        &self,
        slot: WorkflowSlot,
    ) -> Result<Option<WorkflowConfiguration>, WorkflowError> {
        self.repo.get_by_slot(slot).await.map_err(WorkflowError::from)
    }
}

/// Use case for saving a workflow configuration.
pub struct SaveWorkflow {
    repo: Arc<dyn WorkflowConfigRepo>,
    clock: Arc<dyn ClockPort>,
}

impl SaveWorkflow {
    /// Create a new SaveWorkflow use case.
    pub fn new(repo: Arc<dyn WorkflowConfigRepo>, clock: Arc<dyn ClockPort>) -> Self {
        Self { repo, clock }
    }

    /// Execute the use case.
    ///
    /// Saves a workflow configuration. If one already exists for this slot, it is replaced.
    pub async fn execute(
        &self,
        slot: WorkflowSlot,
        name: String,
        workflow_json: serde_json::Value,
        prompt_mappings: Vec<wrldbldr_shared::workflow::PromptMapping>,
        input_defaults: Vec<wrldbldr_shared::workflow::InputDefault>,
        locked_inputs: Vec<String>,
    ) -> Result<WorkflowConfiguration, WorkflowError> {
        // Validate the workflow JSON
        if let Err(e) = wrldbldr_shared::workflow::validate_workflow(&workflow_json) {
            return Err(WorkflowError::Validation(e));
        }

        let now = self.clock.now().to_rfc3339();

        let config = WorkflowConfiguration {
            id: uuid::Uuid::new_v4().to_string(),
            slot,
            name,
            workflow_json,
            prompt_mappings,
            input_defaults,
            locked_inputs,
            created_at: now.clone(),
            updated_at: now,
        };

        self.repo.save(&config).await?;

        Ok(config)
    }
}

/// Use case for deleting a workflow configuration.
pub struct DeleteWorkflow {
    repo: Arc<dyn WorkflowConfigRepo>,
}

impl DeleteWorkflow {
    /// Create a new DeleteWorkflow use case.
    pub fn new(repo: Arc<dyn WorkflowConfigRepo>) -> Self {
        Self { repo }
    }

    /// Execute the use case.
    pub async fn execute(&self, slot: WorkflowSlot) -> Result<(), WorkflowError> {
        // Check if the workflow exists
        if !self.repo.exists(slot).await? {
            return Err(WorkflowError::NotFound(slot));
        }

        self.repo.delete(slot).await.map_err(WorkflowError::from)
    }
}

/// Use case for updating workflow defaults.
pub struct UpdateWorkflowDefaults {
    repo: Arc<dyn WorkflowConfigRepo>,
    clock: Arc<dyn ClockPort>,
}

impl UpdateWorkflowDefaults {
    /// Create a new UpdateWorkflowDefaults use case.
    pub fn new(repo: Arc<dyn WorkflowConfigRepo>, clock: Arc<dyn ClockPort>) -> Self {
        Self { repo, clock }
    }

    /// Execute the use case.
    ///
    /// Updates only the input_defaults and locked_inputs of a workflow configuration.
    pub async fn execute(
        &self,
        slot: WorkflowSlot,
        input_defaults: Vec<wrldbldr_shared::workflow::InputDefault>,
        locked_inputs: Option<Vec<String>>,
    ) -> Result<WorkflowConfiguration, WorkflowError> {
        let mut config = self
            .repo
            .get_by_slot(slot)
            .await?
            .ok_or(WorkflowError::NotFound(slot))?;

        config.input_defaults = input_defaults;
        if let Some(locked) = locked_inputs {
            config.locked_inputs = locked;
        }
        config.updated_at = self.clock.now().to_rfc3339();

        self.repo.save(&config).await?;

        Ok(config)
    }
}

/// Use case for testing a workflow configuration.
pub struct TestWorkflow {
    repo: Arc<dyn WorkflowConfigRepo>,
    queue: Arc<dyn QueuePort>,
}

impl TestWorkflow {
    /// Create a new TestWorkflow use case.
    pub fn new(repo: Arc<dyn WorkflowConfigRepo>, queue: Arc<dyn QueuePort>) -> Self {
        Self { repo, queue }
    }

    /// Execute the use case.
    ///
    /// Queues a test generation using the specified workflow configuration.
    pub async fn execute(
        &self,
        slot: WorkflowSlot,
        prompt: String,
        negative_prompt: Option<String>,
    ) -> Result<wrldbldr_domain::QueueItemId, WorkflowError> {
        let config = self
            .repo
            .get_by_slot(slot)
            .await?
            .ok_or(WorkflowError::NotFound(slot))?;

        let data = AssetGenerationData {
            world_id: None,
            entity_type: "WorkflowTest".to_string(),
            entity_id: config.id.clone(),
            workflow_id: slot.as_str().to_string(),
            prompt,
            negative_prompt,
            count: 1,
            model_id: None,
            prompt_prefix: None,
            prompt_suffix: None,
            score_tokens_enabled: None,
            pose_asset_id: None,
            controlnet_mappings: None,
        };

        let item_id = self.queue.enqueue_asset_generation(&data).await?;

        Ok(item_id)
    }
}

/// Container for all workflow use cases.
pub struct WorkflowUseCases {
    pub list: Arc<ListWorkflows>,
    pub get: Arc<GetWorkflow>,
    pub save: Arc<SaveWorkflow>,
    pub delete: Arc<DeleteWorkflow>,
    pub update_defaults: Arc<UpdateWorkflowDefaults>,
    pub test: Arc<TestWorkflow>,
}

impl WorkflowUseCases {
    /// Create all workflow use cases.
    pub fn new(
        repo: Arc<dyn WorkflowConfigRepo>,
        clock: Arc<dyn ClockPort>,
        queue: Arc<dyn QueuePort>,
    ) -> Self {
        Self {
            list: Arc::new(ListWorkflows::new(repo.clone())),
            get: Arc::new(GetWorkflow::new(repo.clone())),
            save: Arc::new(SaveWorkflow::new(repo.clone(), clock.clone())),
            delete: Arc::new(DeleteWorkflow::new(repo.clone())),
            update_defaults: Arc::new(UpdateWorkflowDefaults::new(repo.clone(), clock.clone())),
            test: Arc::new(TestWorkflow::new(repo, queue)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::{
        MockClockPort, MockQueuePort, MockWorkflowConfigRepo, ImageRequest,
    };
    use chrono::Utc;
    use std::sync::Arc;
    use wrldbldr_domain::QueueItemId;

    fn create_test_workflow_json() -> serde_json::Value {
        serde_json::json!({
            "1": {
                "class_type": "CheckpointLoaderSimple",
                "inputs": {
                    "ckpt_name": "model.safetensors"
                }
            },
            "2": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "text": "positive prompt"
                }
            }
        })
    }

    // =============================================================================
    // ListWorkflows Tests
    // =============================================================================

    #[tokio::test]
    async fn list_workflows_returns_all_slots() {
        let mut repo = MockWorkflowConfigRepo::new();
        repo.expect_list_all().returning(|| Ok(Vec::new()));

        let use_case = ListWorkflows::new(Arc::new(repo));
        let result = use_case.execute().await.unwrap();

        // Should return all workflow slots
        assert_eq!(result.len(), WorkflowSlot::all().len());

        // All slots should be present
        for (i, slot) in WorkflowSlot::all().iter().enumerate() {
            assert_eq!(result[i].0, *slot);
        }
    }

    #[tokio::test]
    async fn list_workflows_includes_configured_status() {
        let mut repo = MockWorkflowConfigRepo::new();
        let config = WorkflowConfiguration::new(
            WorkflowSlot::CharacterPortrait,
            "Test Config",
            create_test_workflow_json(),
            Utc::now().to_rfc3339(),
        );

        repo.expect_list_all()
            .returning(move || Ok(vec![config.clone()]));
        repo.expect_get_by_slot()
            .with(mockall::predicate::eq(WorkflowSlot::CharacterPortrait))
            .returning(move |_| Ok(Some(config.clone())));

        let use_case = ListWorkflows::new(Arc::new(repo));
        let result = use_case.execute().await.unwrap();

        // Find the character_portrait slot
        let portrait_slot = result
            .iter()
            .find(|(slot, _)| *slot == WorkflowSlot::CharacterPortrait);
        assert!(portrait_slot.is_some());
        assert!(portrait_slot.unwrap().1.is_some());
    }

    // =============================================================================
    // GetWorkflow Tests
    // =============================================================================

    #[tokio::test]
    async fn get_workflow_returns_config_when_exists() {
        let mut repo = MockWorkflowConfigRepo::new();
        let config = WorkflowConfiguration::new(
            WorkflowSlot::CharacterPortrait,
            "Test Config",
            create_test_workflow_json(),
            Utc::now().to_rfc3339(),
        );

        repo.expect_get_by_slot()
            .with(mockall::predicate::eq(WorkflowSlot::CharacterPortrait))
            .returning(move |_| Ok(Some(config.clone())));

        let use_case = GetWorkflow::new(Arc::new(repo));
        let result = use_case.execute(WorkflowSlot::CharacterPortrait).await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Test Config");
    }

    #[tokio::test]
    async fn get_workflow_returns_none_when_not_exists() {
        let mut repo = MockWorkflowConfigRepo::new();
        repo.expect_get_by_slot()
            .with(mockall::predicate::eq(WorkflowSlot::CharacterPortrait))
            .returning(|_| Ok(None));

        let use_case = GetWorkflow::new(Arc::new(repo));
        let result = use_case.execute(WorkflowSlot::CharacterPortrait).await.unwrap();

        assert!(result.is_none());
    }

    // =============================================================================
    // SaveWorkflow Tests
    // =============================================================================

    #[tokio::test]
    async fn save_workflow_creates_new_config() {
        let mut repo = MockWorkflowConfigRepo::new();
        repo.expect_save().times(1).returning(|_| Ok(()));

        let mut clock = MockClockPort::new();
        clock.expect_now().returning(Utc::now);

        let use_case = SaveWorkflow::new(Arc::new(repo), Arc::new(clock));
        let result = use_case
            .execute(
                WorkflowSlot::CharacterPortrait,
                "New Workflow".to_string(),
                create_test_workflow_json(),
                vec![],
                vec![],
                vec![],
            )
            .await;

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.slot, WorkflowSlot::CharacterPortrait);
        assert_eq!(config.name, "New Workflow");
    }

    #[tokio::test]
    async fn save_workflow_validates_workflow_json() {
        let repo = MockWorkflowConfigRepo::new();

        let mut clock = MockClockPort::new();
        clock.expect_now().returning(Utc::now);

        let use_case = SaveWorkflow::new(Arc::new(repo), Arc::new(clock));
        let result = use_case
            .execute(
                WorkflowSlot::CharacterPortrait,
                "Invalid Workflow".to_string(),
                serde_json::json!({"invalid": "workflow"}), // No class_type
                vec![],
                vec![],
                vec![],
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WorkflowError::Validation(_)));
    }

    // =============================================================================
    // DeleteWorkflow Tests
    // =============================================================================

    #[tokio::test]
    async fn delete_workflow_removes_config() {
        let mut repo = MockWorkflowConfigRepo::new();
        repo.expect_exists()
            .with(mockall::predicate::eq(WorkflowSlot::CharacterPortrait))
            .returning(|_| Ok(true));
        repo.expect_delete()
            .with(mockall::predicate::eq(WorkflowSlot::CharacterPortrait))
            .times(1)
            .returning(|_| Ok(()));

        let use_case = DeleteWorkflow::new(Arc::new(repo));
        let result = use_case.execute(WorkflowSlot::CharacterPortrait).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_workflow_returns_error_when_not_exists() {
        let mut repo = MockWorkflowConfigRepo::new();
        repo.expect_exists()
            .with(mockall::predicate::eq(WorkflowSlot::CharacterPortrait))
            .returning(|_| Ok(false));

        let use_case = DeleteWorkflow::new(Arc::new(repo));
        let result = use_case.execute(WorkflowSlot::CharacterPortrait).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WorkflowError::NotFound(_)));
    }

    // =============================================================================
    // UpdateWorkflowDefaults Tests
    // =============================================================================

    #[tokio::test]
    async fn update_defaults_modifies_config() {
        let mut repo = MockWorkflowConfigRepo::new();
        let config = WorkflowConfiguration::new(
            WorkflowSlot::CharacterPortrait,
            "Test Config",
            create_test_workflow_json(),
            Utc::now().to_rfc3339(),
        );

        repo.expect_get_by_slot()
            .with(mockall::predicate::eq(WorkflowSlot::CharacterPortrait))
            .returning(move |_| Ok(Some(config.clone())));
        repo.expect_save().times(1).returning(|_| Ok(()));

        let mut clock = MockClockPort::new();
        clock.expect_now().returning(Utc::now);

        let use_case = UpdateWorkflowDefaults::new(Arc::new(repo), Arc::new(clock));

        let defaults = vec![wrldbldr_shared::workflow::InputDefault::new(
            "1",
            "seed",
            serde_json::json!(42),
        )];

        let result = use_case
            .execute(WorkflowSlot::CharacterPortrait, defaults, Some(vec!["1.seed".to_string()]))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_defaults_returns_error_when_not_found() {
        let mut repo = MockWorkflowConfigRepo::new();
        repo.expect_get_by_slot()
            .with(mockall::predicate::eq(WorkflowSlot::CharacterPortrait))
            .returning(|_| Ok(None));

        let mut clock = MockClockPort::new();
        clock.expect_now().returning(Utc::now);

        let use_case = UpdateWorkflowDefaults::new(Arc::new(repo), Arc::new(clock));
        let result = use_case
            .execute(WorkflowSlot::CharacterPortrait, vec![], None)
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WorkflowError::NotFound(_)));
    }

    // =============================================================================
    // TestWorkflow Tests
    // =============================================================================

    #[tokio::test]
    async fn test_workflow_queues_generation() {
        let mut repo = MockWorkflowConfigRepo::new();
        let config = WorkflowConfiguration::new(
            WorkflowSlot::CharacterPortrait,
            "Test Config",
            create_test_workflow_json(),
            Utc::now().to_rfc3339(),
        );

        repo.expect_get_by_slot()
            .with(mockall::predicate::eq(WorkflowSlot::CharacterPortrait))
            .returning(move |_| Ok(Some(config.clone())));

        let mut queue = MockQueuePort::new();
        queue
            .expect_enqueue_asset_generation()
            .times(1)
            .returning(|_| Ok(QueueItemId::new()));

        let use_case = TestWorkflow::new(Arc::new(repo), Arc::new(queue));
        let result = use_case
            .execute(
                WorkflowSlot::CharacterPortrait,
                "test prompt".to_string(),
                Some("negative prompt".to_string()),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_workflow_returns_error_when_not_found() {
        let mut repo = MockWorkflowConfigRepo::new();
        repo.expect_get_by_slot()
            .with(mockall::predicate::eq(WorkflowSlot::CharacterPortrait))
            .returning(|_| Ok(None));

        let queue = MockQueuePort::new();

        let use_case = TestWorkflow::new(Arc::new(repo), Arc::new(queue));
        let result = use_case
            .execute(
                WorkflowSlot::CharacterPortrait,
                "test prompt".to_string(),
                None,
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WorkflowError::NotFound(_)));
    }
}
