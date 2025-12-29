//! Sheet template service port - Interface for character sheet template operations
//!
//! This port abstracts character sheet template business logic from infrastructure.
//! It provides methods for managing character sheet templates that define the
//! structure and fields of character sheets in a world.
//!
//! # Design Notes
//!
//! Character sheet templates define the schema for character sheets, including
//! sections, fields, and validation rules. Each world can have multiple templates,
//! with one marked as the default.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::entities::{CharacterSheetTemplate, SheetTemplateId};
use wrldbldr_domain::WorldId;

/// Port for sheet template service operations.
///
/// This trait provides access to character sheet template management
/// functionality including CRUD operations and world-scoped queries.
///
/// # Usage
///
/// Infrastructure adapters should depend on this trait rather than importing
/// the service directly from engine-app, maintaining proper hexagonal
/// architecture boundaries.
#[async_trait]
pub trait SheetTemplateServicePort: Send + Sync {
    /// Create a new sheet template.
    ///
    /// # Arguments
    ///
    /// * `template` - The template to create
    ///
    /// # Errors
    ///
    /// Returns an error if the template cannot be created (e.g., validation failure).
    async fn create(&self, template: &CharacterSheetTemplate) -> Result<()>;

    /// Get a sheet template by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the template to retrieve
    ///
    /// # Returns
    ///
    /// `Ok(Some(template))` if found, `Ok(None)` if not found.
    async fn get(&self, id: &SheetTemplateId) -> Result<Option<CharacterSheetTemplate>>;

    /// Get the default template for a world.
    ///
    /// Returns the template marked as the default for the specified world.
    /// A world should have exactly one default template.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The ID of the world
    ///
    /// # Returns
    ///
    /// `Ok(Some(template))` if a default exists, `Ok(None)` if no default is set.
    async fn get_default_for_world(
        &self,
        world_id: &WorldId,
    ) -> Result<Option<CharacterSheetTemplate>>;

    /// List all templates for a world.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The ID of the world whose templates to list
    ///
    /// # Returns
    ///
    /// A vector of all templates in the world.
    async fn list_by_world(&self, world_id: &WorldId) -> Result<Vec<CharacterSheetTemplate>>;

    /// Update a sheet template.
    ///
    /// # Arguments
    ///
    /// * `template` - The template with updated fields
    ///
    /// # Errors
    ///
    /// Returns an error if the template doesn't exist or validation fails.
    async fn update(&self, template: &CharacterSheetTemplate) -> Result<()>;

    /// Delete a sheet template.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the template to delete
    ///
    /// # Errors
    ///
    /// Returns an error if the template doesn't exist or is in use.
    async fn delete(&self, id: &SheetTemplateId) -> Result<()>;

    /// Delete all templates for a world.
    ///
    /// This is typically used when deleting a world.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The ID of the world whose templates to delete
    async fn delete_all_for_world(&self, world_id: &WorldId) -> Result<()>;

    /// Check if a world has any templates.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The ID of the world to check
    ///
    /// # Returns
    ///
    /// `true` if the world has at least one template, `false` otherwise.
    async fn has_templates(&self, world_id: &WorldId) -> Result<bool>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    /// Mock implementation of SheetTemplateServicePort for testing.
    pub SheetTemplateServicePort {}

    #[async_trait]
    impl SheetTemplateServicePort for SheetTemplateServicePort {
        async fn create(&self, template: &CharacterSheetTemplate) -> Result<()>;
        async fn get(&self, id: &SheetTemplateId) -> Result<Option<CharacterSheetTemplate>>;
        async fn get_default_for_world(&self, world_id: &WorldId) -> Result<Option<CharacterSheetTemplate>>;
        async fn list_by_world(&self, world_id: &WorldId) -> Result<Vec<CharacterSheetTemplate>>;
        async fn update(&self, template: &CharacterSheetTemplate) -> Result<()>;
        async fn delete(&self, id: &SheetTemplateId) -> Result<()>;
        async fn delete_all_for_world(&self, world_id: &WorldId) -> Result<()>;
        async fn has_templates(&self, world_id: &WorldId) -> Result<bool>;
    }
}
