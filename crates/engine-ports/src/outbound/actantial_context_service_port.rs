//! Actantial Context Service Port
//!
//! Defines the interface for building character motivation and social stance context
//! for LLM prompts. This port abstracts the actantial model service, allowing adapters
//! to retrieve rich motivational context for NPCs without depending on the application layer.
//!
//! # Actantial Model
//!
//! The actantial model (from Greimas' narrative theory) structures character motivations:
//! - **Wants**: What the character desires (with targets, intensity, visibility)
//! - **Helpers/Opponents**: Who aids or opposes the character's goals
//! - **Senders/Receivers**: Who motivates the want and who benefits
//!
//! # Usage
//!
//! Adapters use this port to get context for LLM prompt construction:
//!
//! ```ignore
//! let context = actantial_service.get_context(character_id).await?;
//! let motivations = context.to_motivations_context();
//! let social_stance = context.to_social_stance_context();
//! ```

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::value_objects::ActantialContext;
use wrldbldr_domain::CharacterId;

/// Port for retrieving actantial context (motivations and social views) for characters.
///
/// This port provides read-only access to the actantial model data aggregated
/// into context structures suitable for LLM consumption.
///
/// # Implementation Notes
///
/// Implementors should aggregate data from character, want, and goal repositories
/// to build the complete actantial context, resolving all targets and actors
/// to their display names.
#[async_trait]
pub trait ActantialContextServicePort: Send + Sync {
    /// Get the full actantial context for a character.
    ///
    /// Returns the complete context including all wants, their targets,
    /// actantial actors (helpers, opponents, senders, receivers), and
    /// aggregated social views.
    ///
    /// # Arguments
    ///
    /// * `character_id` - The character to get context for
    ///
    /// # Returns
    ///
    /// * `Ok(ActantialContext)` - The complete actantial context
    /// * `Err(_)` - If the character is not found or data retrieval fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// let context = service.get_context(character_id).await?;
    ///
    /// // Convert to LLM-ready formats
    /// let motivations = context.to_motivations_context();
    /// let social_stance = context.to_social_stance_context();
    ///
    /// // Or format directly for prompt insertion
    /// let prompt_text = context.to_llm_string(include_secrets);
    /// ```
    async fn get_context(&self, character_id: CharacterId) -> Result<ActantialContext>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    /// Mock implementation of ActantialContextServicePort for testing
    pub ActantialContextServicePort {}

    #[async_trait]
    impl ActantialContextServicePort for ActantialContextServicePort {
        async fn get_context(&self, character_id: CharacterId) -> Result<ActantialContext>;
    }
}
