//! Prompt template service port - Interface for prompt template operations
//!
//! This port abstracts prompt template business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.
//!
//! Provides methods for:
//! - Global templates: get_all, set_global, delete_global, reset_global
//! - World templates: get_all_for_world, set_for_world, delete_for_world, reset_for_world
//! - Resolution: resolve_with_source, resolve_for_world_with_source
//! - Metadata: get_metadata

use async_trait::async_trait;
use wrldbldr_domain::value_objects::PromptTemplateMetadata;
use wrldbldr_domain::WorldId;

use super::{PromptTemplateError, ResolvedPromptTemplate};

/// Port for prompt template service operations
///
/// This trait defines the operations for retrieving, managing, and rendering
/// prompt templates used for LLM interactions.
///
/// Templates follow a priority resolution order:
/// 1. World-specific DB override (if world_id provided)
/// 2. Global DB override
/// 3. Environment variable (WRLDBLDR_PROMPT_{KEY})
/// 4. Hard-coded default
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait PromptTemplateServicePort: Send + Sync {
    // =========================================================================
    // Global Template Operations
    // =========================================================================

    /// Get all templates with their resolved values (global context)
    ///
    /// Returns all known prompt templates with their effective values
    /// resolved using the priority chain: Global DB → Env → Default
    async fn get_all(&self) -> Vec<ResolvedPromptTemplate>;

    /// Set a global template override
    ///
    /// Stores a global override in the database. This takes precedence
    /// over environment variables and defaults.
    ///
    /// # Arguments
    ///
    /// * `key` - The template key/name
    /// * `value` - The override value to store
    async fn set_global(&self, key: &str, value: &str) -> Result<(), PromptTemplateError>;

    /// Delete a global template override
    ///
    /// Removes the global database override for a template, causing it
    /// to fall back to environment variable or default.
    ///
    /// # Arguments
    ///
    /// * `key` - The template key/name to delete the override for
    async fn delete_global(&self, key: &str) -> Result<(), PromptTemplateError>;

    /// Reset all global template overrides
    ///
    /// Deletes all global database overrides, causing all templates
    /// to fall back to environment variables or defaults.
    async fn reset_global(&self) -> Result<(), PromptTemplateError>;

    // =========================================================================
    // World-Specific Template Operations
    // =========================================================================

    /// Get all templates with their resolved values for a specific world
    ///
    /// Returns all known prompt templates with their effective values
    /// resolved using the priority chain: World DB → Global DB → Env → Default
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to get templates for
    async fn get_all_for_world(&self, world_id: WorldId) -> Vec<ResolvedPromptTemplate>;

    /// Set a world-specific template override
    ///
    /// Stores a world-specific override in the database. This takes
    /// precedence over global overrides, environment variables, and defaults.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to set the override for
    /// * `key` - The template key/name
    /// * `value` - The override value to store
    async fn set_for_world(
        &self,
        world_id: WorldId,
        key: &str,
        value: &str,
    ) -> Result<(), PromptTemplateError>;

    /// Delete a world-specific template override
    ///
    /// Removes the world-specific database override for a template,
    /// causing it to fall back to global override, environment variable, or default.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to delete the override from
    /// * `key` - The template key/name to delete the override for
    async fn delete_for_world(
        &self,
        world_id: WorldId,
        key: &str,
    ) -> Result<(), PromptTemplateError>;

    /// Reset all world-specific template overrides
    ///
    /// Deletes all world-specific database overrides for a given world,
    /// causing all templates to fall back to global overrides,
    /// environment variables, or defaults.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to reset overrides for
    async fn reset_for_world(&self, world_id: WorldId) -> Result<(), PromptTemplateError>;

    // =========================================================================
    // Resolution Operations
    // =========================================================================

    /// Resolve a template with source information (global context)
    ///
    /// Returns the resolved template value along with metadata about
    /// where the value came from (global override, env, or default).
    ///
    /// Priority: Global DB → Env → Default
    ///
    /// # Arguments
    ///
    /// * `key` - The template key/name to resolve
    async fn resolve_with_source(&self, key: &str) -> ResolvedPromptTemplate;

    /// Resolve a template for a specific world with source information
    ///
    /// Returns the resolved template value along with metadata about
    /// where the value came from (world override, global override, env, or default).
    ///
    /// Priority: World DB → Global DB → Env → Default
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world context for resolution
    /// * `key` - The template key/name to resolve
    async fn resolve_for_world_with_source(
        &self,
        world_id: WorldId,
        key: &str,
    ) -> ResolvedPromptTemplate;

    // =========================================================================
    // Metadata Operations
    // =========================================================================

    /// Get template metadata for all known templates
    ///
    /// Returns metadata including keys, labels, descriptions, categories,
    /// default values, and environment variable names for all templates.
    /// Useful for UI rendering and documentation.
    fn get_metadata(&self) -> Vec<PromptTemplateMetadata>;
}
