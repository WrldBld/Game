//! Interaction service port - Interface for interaction operations
//!
//! This port abstracts interaction business logic from infrastructure adapters.
//! It exposes query methods for retrieving interaction templates and their targets.
//!
//! # Design Notes
//!
//! This port is designed for use by infrastructure adapters that need to query
//! interaction information for scenes. It focuses on read operations used by
//! prompt builders and scene renderers.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::entities::{InteractionTarget, InteractionTemplate};
use wrldbldr_domain::{InteractionId, SceneId};

/// Port for interaction service operations used by infrastructure adapters.
///
/// This trait provides read-only access to interaction template data for use in
/// building prompts, gathering context, and scene rendering.
///
/// # Usage
///
/// Infrastructure adapters should depend on this trait rather than importing
/// the service directly from engine-app, maintaining proper hexagonal
/// architecture boundaries.
#[async_trait]
pub trait InteractionServicePort: Send + Sync {
    /// Get an interaction template by ID.
    ///
    /// Returns `Ok(None)` if the interaction is not found.
    async fn get_interaction(&self, id: InteractionId) -> Result<Option<InteractionTemplate>>;

    /// List all interactions for a specific scene.
    ///
    /// Returns interactions in display order.
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<InteractionTemplate>>;

    /// Get an interaction with its resolved target.
    ///
    /// This method loads the interaction and resolves its target entity,
    /// which may involve additional graph traversals for TARGETS_* edges.
    ///
    /// Returns `Ok(None)` if the interaction is not found.
    async fn get_interaction_with_target(
        &self,
        id: InteractionId,
    ) -> Result<Option<(InteractionTemplate, InteractionTarget)>>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    /// Mock implementation of InteractionServicePort for testing.
    pub InteractionServicePort {}

    #[async_trait]
    impl InteractionServicePort for InteractionServicePort {
        async fn get_interaction(&self, id: InteractionId) -> Result<Option<InteractionTemplate>>;
        async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<InteractionTemplate>>;
        async fn get_interaction_with_target(
            &self,
            id: InteractionId,
        ) -> Result<Option<(InteractionTemplate, InteractionTarget)>>;
    }
}
