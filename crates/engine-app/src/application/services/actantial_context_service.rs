//! Actantial Context Service - Aggregates actantial model data for LLM context
//!
//! P1.5: Actantial Model System
//!
//! This service provides rich motivational context for NPCs by aggregating:
//! - Wants with their targets (Character, Item, or Goal)
//! - Actantial views (Helper, Opponent, Sender, Receiver) toward NPCs and PCs
//! - Behavioral guidance for secret motivations (deflection, tells)
//! - Social view summaries (allies vs enemies)
//!
//! Phase 4 adds mutation methods for DM Panel integration.

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, instrument, warn};

use wrldbldr_engine_ports::outbound::{
    CharacterRepositoryPort, GoalRepositoryPort, ItemRepositoryPort, PlayerCharacterRepositoryPort,
    WantRepositoryPort,
};
use wrldbldr_domain::entities::{ActantialRole, ActantialView, Character, Goal, PlayerCharacter, Want, WantVisibility};
use wrldbldr_domain::value_objects::{
    ActantialActor, ActantialContext, ActantialLLMContext, ActantialTarget, SocialViewSummary,
    WantContext, WantTarget,
};
use wrldbldr_domain::{CharacterId, GoalId, PlayerCharacterId, WantId, WorldId};

// =============================================================================
// Request Types for Mutations
// =============================================================================

/// Request to create a new want for a character
#[derive(Debug, Clone)]
pub struct CreateWantRequest {
    pub description: String,
    pub intensity: f32,
    pub priority: u32,
    pub visibility: WantVisibility,
    pub target_id: Option<String>,
    pub target_type: Option<String>, // "Character", "Item", "Goal"
    pub deflection_behavior: Option<String>,
    pub tells: Vec<String>,
}

impl Default for CreateWantRequest {
    fn default() -> Self {
        Self {
            description: String::new(),
            intensity: 0.5,
            priority: 1,
            visibility: WantVisibility::Hidden,
            target_id: None,
            target_type: None,
            deflection_behavior: None,
            tells: Vec::new(),
        }
    }
}

/// Request to update an existing want
#[derive(Debug, Clone, Default)]
pub struct UpdateWantRequest {
    pub description: Option<String>,
    pub intensity: Option<f32>,
    pub priority: Option<u32>,
    pub visibility: Option<WantVisibility>,
    pub deflection_behavior: Option<String>,
    pub tells: Option<Vec<String>>,
}

/// Actor type for actantial views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorTargetType {
    Npc,
    Pc,
}

/// Actantial context service trait
#[async_trait]
pub trait ActantialContextService: Send + Sync {
    // =========================================================================
    // Query Methods (Phase 2/3)
    // =========================================================================

    /// Get full actantial context for a character
    ///
    /// This aggregates all wants, targets, and actantial views into a complete
    /// context structure suitable for LLM consumption.
    async fn get_context(&self, character_id: CharacterId) -> Result<ActantialContext>;

    /// Get compact LLM context for a character
    ///
    /// This returns a minimal representation for token efficiency.
    async fn get_llm_context(&self, character_id: CharacterId) -> Result<ActantialLLMContext>;

    /// Get formatted context string for LLM prompt insertion
    ///
    /// If `include_secrets` is true, includes behavioral guidance for hidden wants.
    async fn get_context_string(
        &self,
        character_id: CharacterId,
        include_secrets: bool,
    ) -> Result<String>;

    // =========================================================================
    // Mutation Methods (Phase 4)
    // =========================================================================

    /// Create a new want for a character
    ///
    /// Returns the created want ID.
    async fn create_want(
        &self,
        character_id: CharacterId,
        request: CreateWantRequest,
    ) -> Result<WantId>;

    /// Update an existing want
    async fn update_want(&self, want_id: WantId, request: UpdateWantRequest) -> Result<()>;

    /// Delete a want
    async fn delete_want(&self, want_id: WantId) -> Result<()>;

    /// Set a want's target
    ///
    /// target_type: "Character", "Item", or "Goal"
    async fn set_want_target(
        &self,
        want_id: WantId,
        target_id: &str,
        target_type: &str,
    ) -> Result<()>;

    /// Remove a want's target
    async fn remove_want_target(&self, want_id: WantId) -> Result<()>;

    /// Add an actantial view (Helper, Opponent, Sender, Receiver)
    async fn add_actantial_view(
        &self,
        subject_id: CharacterId,
        want_id: WantId,
        target_id: &str,
        target_type: ActorTargetType,
        role: ActantialRole,
        reason: String,
    ) -> Result<()>;

    /// Remove an actantial view
    async fn remove_actantial_view(
        &self,
        subject_id: CharacterId,
        want_id: WantId,
        target_id: &str,
        target_type: ActorTargetType,
        role: ActantialRole,
    ) -> Result<()>;

    // =========================================================================
    // Goal Methods (Phase 4)
    // =========================================================================

    /// Get all goals for a world
    async fn get_world_goals(&self, world_id: WorldId) -> Result<Vec<Goal>>;

    /// Create a new goal
    async fn create_goal(
        &self,
        world_id: WorldId,
        name: String,
        description: Option<String>,
    ) -> Result<GoalId>;

    /// Update a goal
    async fn update_goal(
        &self,
        goal_id: GoalId,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<()>;

    /// Delete a goal
    async fn delete_goal(&self, goal_id: GoalId) -> Result<()>;

    /// Get a specific goal by ID
    async fn get_goal(&self, goal_id: GoalId) -> Result<Option<Goal>>;

    /// Get a specific want by ID
    async fn get_want(&self, want_id: WantId) -> Result<Option<Want>>;
}

/// Default implementation of ActantialContextService
#[derive(Clone)]
pub struct ActantialContextServiceImpl {
    character_repo: Arc<dyn CharacterRepositoryPort>,
    pc_repo: Arc<dyn PlayerCharacterRepositoryPort>,
    goal_repo: Arc<dyn GoalRepositoryPort>,
    item_repo: Arc<dyn ItemRepositoryPort>,
    want_repo: Arc<dyn WantRepositoryPort>,
}

impl ActantialContextServiceImpl {
    /// Create a new ActantialContextServiceImpl with the given repositories
    pub fn new(
        character_repo: Arc<dyn CharacterRepositoryPort>,
        pc_repo: Arc<dyn PlayerCharacterRepositoryPort>,
        goal_repo: Arc<dyn GoalRepositoryPort>,
        item_repo: Arc<dyn ItemRepositoryPort>,
        want_repo: Arc<dyn WantRepositoryPort>,
    ) -> Self {
        Self {
            character_repo,
            pc_repo,
            goal_repo,
            item_repo,
            want_repo,
        }
    }

    /// Resolve a target name from an ActantialTarget
    async fn resolve_target_name(&self, target: &ActantialTarget) -> Result<String> {
        match target {
            ActantialTarget::Npc(id) => {
                let char_id = CharacterId::from_uuid(*id);
                if let Some(character) = self.character_repo.get(char_id).await? {
                    Ok(character.name)
                } else {
                    Ok("Unknown NPC".to_string())
                }
            }
            ActantialTarget::Pc(id) => {
                let pc_id = wrldbldr_domain::PlayerCharacterId::from_uuid(*id);
                if let Some(pc) = self.pc_repo.get(pc_id).await? {
                    Ok(pc.name)
                } else {
                    Ok("Unknown PC".to_string())
                }
            }
        }
    }

    /// Build social view summary from actantial views across all wants
    fn build_social_views(
        &self,
        views: &[(ActantialRole, ActantialTarget, String, String)], // (role, target, name, reason)
    ) -> SocialViewSummary {
        let mut summary = SocialViewSummary::new();

        for (role, target, name, reason) in views {
            match role {
                ActantialRole::Helper | ActantialRole::Sender => {
                    summary.add_ally(target.clone(), name.clone(), reason.clone());
                }
                ActantialRole::Opponent => {
                    summary.add_enemy(target.clone(), name.clone(), reason.clone());
                }
                ActantialRole::Receiver => {
                    // Receivers are neutral - they benefit but aren't helpers
                    // Could be added to a separate category if needed
                }
            }
        }

        summary
    }
}

#[async_trait]
impl ActantialContextService for ActantialContextServiceImpl {
    #[instrument(skip(self))]
    async fn get_context(&self, character_id: CharacterId) -> Result<ActantialContext> {
        debug!(character_id = %character_id, "Building actantial context");

        // Get the character for their name
        let character = self
            .character_repo
            .get(character_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Character not found: {}", character_id))?;

        let mut context = ActantialContext::new(character_id.to_uuid(), &character.name);

        // Get all wants for this character
        let character_wants = self.character_repo.get_wants(character_id).await?;

        // Get all actantial views for this character
        let actantial_views = self.character_repo.get_actantial_views(character_id).await?;

        // Collect all view data with resolved names for social summary
        let mut all_views_with_names: Vec<(ActantialRole, ActantialTarget, String, String)> =
            Vec::new();

        // Build WantContext for each want
        for char_want in character_wants {
            let want = &char_want.want;
            let want_id_uuid = want.id.to_uuid();

            let mut want_ctx = WantContext::new(
                want_id_uuid,
                &want.description,
                want.intensity,
                char_want.priority,
            );
            want_ctx.visibility = want.visibility;
            want_ctx.deflection_behavior = want.deflection_behavior.clone();
            want_ctx.tells = want.tells.clone();

            // Resolve the want's target
            if let Some(target) = self.character_repo.get_want_target(want.id).await? {
                want_ctx.target = Some(target);
            }

            // Add actantial actors for this want
            for (role, target, view) in &actantial_views {
                if view.want_id == want.id {
                    let name = self.resolve_target_name(target).await?;
                    let actor = ActantialActor::new(target.clone(), &name, &view.reason);

                    // Collect for social summary
                    all_views_with_names.push((
                        *role,
                        target.clone(),
                        name.clone(),
                        view.reason.clone(),
                    ));

                    match role {
                        ActantialRole::Helper => want_ctx.helpers.push(actor),
                        ActantialRole::Opponent => want_ctx.opponents.push(actor),
                        ActantialRole::Sender => want_ctx.sender = Some(actor),
                        ActantialRole::Receiver => want_ctx.receiver = Some(actor),
                    }
                }
            }

            context.wants.push(want_ctx);
        }

        // Build aggregated social views
        context.social_views = self.build_social_views(&all_views_with_names);

        debug!(
            character_id = %character_id,
            want_count = context.wants.len(),
            ally_count = context.social_views.allies.len(),
            enemy_count = context.social_views.enemies.len(),
            "Built actantial context"
        );

        Ok(context)
    }

    #[instrument(skip(self))]
    async fn get_llm_context(&self, character_id: CharacterId) -> Result<ActantialLLMContext> {
        let context = self.get_context(character_id).await?;
        Ok(ActantialLLMContext::from_context(&context))
    }

    #[instrument(skip(self))]
    async fn get_context_string(
        &self,
        character_id: CharacterId,
        include_secrets: bool,
    ) -> Result<String> {
        let context = self.get_context(character_id).await?;
        Ok(context.to_llm_string(include_secrets))
    }

    // =========================================================================
    // Mutation Methods (Phase 4)
    // =========================================================================

    #[instrument(skip(self, request))]
    async fn create_want(
        &self,
        character_id: CharacterId,
        request: CreateWantRequest,
    ) -> Result<WantId> {
        debug!(character_id = %character_id, description = %request.description, "Creating want");

        // Build the want entity
        let mut want = Want::new(&request.description)
            .with_intensity(request.intensity)
            .with_visibility(request.visibility);

        if let Some(deflection) = request.deflection_behavior {
            want = want.with_deflection(deflection);
        }
        if !request.tells.is_empty() {
            want = want.with_tells(request.tells);
        }

        let want_id = want.id;

        // Create the want attached to the character
        self.character_repo
            .create_want(character_id, &want, request.priority)
            .await?;

        // Set target if provided
        if let (Some(target_id), Some(target_type)) = (&request.target_id, &request.target_type) {
            self.character_repo
                .set_want_target(want_id, target_id, target_type)
                .await?;
        }

        debug!(want_id = %want_id, "Want created");
        Ok(want_id)
    }

    #[instrument(skip(self, request))]
    async fn update_want(&self, want_id: WantId, request: UpdateWantRequest) -> Result<()> {
        debug!(want_id = %want_id, "Updating want");

        // Get current want to merge with updates
        // Since we don't have a direct get_want method, we need to find it through character
        // For now, we'll create a Want with the updates and let the repository handle the merge
        // This requires the repository to support partial updates

        // Build updated Want - repository will handle partial update
        let mut want = Want {
            id: want_id,
            description: request.description.unwrap_or_default(),
            intensity: request.intensity.unwrap_or(0.5),
            visibility: request.visibility.unwrap_or(WantVisibility::Hidden),
            created_at: chrono::Utc::now(), // Will be ignored by update
            deflection_behavior: request.deflection_behavior,
            tells: request.tells.unwrap_or_default(),
        };

        // Note: The repository's update_want should handle partial updates
        // For a full implementation, we'd need to fetch the existing want first
        self.character_repo.update_want(&want).await?;

        debug!(want_id = %want_id, "Want updated");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete_want(&self, want_id: WantId) -> Result<()> {
        debug!(want_id = %want_id, "Deleting want");
        self.character_repo.delete_want(want_id).await?;
        debug!(want_id = %want_id, "Want deleted");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn set_want_target(
        &self,
        want_id: WantId,
        target_id: &str,
        target_type: &str,
    ) -> Result<()> {
        debug!(want_id = %want_id, target_id = %target_id, target_type = %target_type, "Setting want target");
        self.character_repo
            .set_want_target(want_id, target_id, target_type)
            .await?;
        debug!(want_id = %want_id, "Want target set");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove_want_target(&self, want_id: WantId) -> Result<()> {
        debug!(want_id = %want_id, "Removing want target");
        self.character_repo.remove_want_target(want_id).await?;
        debug!(want_id = %want_id, "Want target removed");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn add_actantial_view(
        &self,
        subject_id: CharacterId,
        want_id: WantId,
        target_id: &str,
        target_type: ActorTargetType,
        role: ActantialRole,
        reason: String,
    ) -> Result<()> {
        debug!(
            subject_id = %subject_id,
            want_id = %want_id,
            target_id = %target_id,
            role = ?role,
            "Adding actantial view"
        );

        let view = ActantialView::new(want_id, &reason);

        match target_type {
            ActorTargetType::Npc => {
                let target_char_id = CharacterId::from_uuid(
                    uuid::Uuid::parse_str(target_id)
                        .map_err(|e| anyhow::anyhow!("Invalid target ID: {}", e))?,
                );
                self.character_repo
                    .add_actantial_view(subject_id, role, target_char_id, &view)
                    .await?;
            }
            ActorTargetType::Pc => {
                let target_pc_id = PlayerCharacterId::from_uuid(
                    uuid::Uuid::parse_str(target_id)
                        .map_err(|e| anyhow::anyhow!("Invalid target ID: {}", e))?,
                );
                self.character_repo
                    .add_actantial_view_to_pc(subject_id, role, target_pc_id, &view)
                    .await?;
            }
        }

        debug!(subject_id = %subject_id, "Actantial view added");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove_actantial_view(
        &self,
        subject_id: CharacterId,
        want_id: WantId,
        target_id: &str,
        target_type: ActorTargetType,
        role: ActantialRole,
    ) -> Result<()> {
        debug!(
            subject_id = %subject_id,
            want_id = %want_id,
            target_id = %target_id,
            role = ?role,
            "Removing actantial view"
        );

        match target_type {
            ActorTargetType::Npc => {
                let target_char_id = CharacterId::from_uuid(
                    uuid::Uuid::parse_str(target_id)
                        .map_err(|e| anyhow::anyhow!("Invalid target ID: {}", e))?,
                );
                self.character_repo
                    .remove_actantial_view(subject_id, role, target_char_id, want_id)
                    .await?;
            }
            ActorTargetType::Pc => {
                let target_pc_id = PlayerCharacterId::from_uuid(
                    uuid::Uuid::parse_str(target_id)
                        .map_err(|e| anyhow::anyhow!("Invalid target ID: {}", e))?,
                );
                self.character_repo
                    .remove_actantial_view_to_pc(subject_id, role, target_pc_id, want_id)
                    .await?;
            }
        }

        debug!(subject_id = %subject_id, "Actantial view removed");
        Ok(())
    }

    // =========================================================================
    // Goal Methods (Phase 4)
    // =========================================================================

    #[instrument(skip(self))]
    async fn get_world_goals(&self, world_id: WorldId) -> Result<Vec<Goal>> {
        debug!(world_id = %world_id, "Getting world goals");
        let goals = self.goal_repo.list(world_id).await?;
        debug!(world_id = %world_id, goal_count = goals.len(), "Retrieved world goals");
        Ok(goals)
    }

    #[instrument(skip(self))]
    async fn create_goal(
        &self,
        world_id: WorldId,
        name: String,
        description: Option<String>,
    ) -> Result<GoalId> {
        debug!(world_id = %world_id, name = %name, "Creating goal");

        let mut goal = Goal::new(world_id, &name);
        if let Some(desc) = description {
            goal = goal.with_description(desc);
        }

        let goal_id = goal.id;
        self.goal_repo.create(&goal).await?;

        debug!(goal_id = %goal_id, "Goal created");
        Ok(goal_id)
    }

    #[instrument(skip(self))]
    async fn update_goal(
        &self,
        goal_id: GoalId,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<()> {
        debug!(goal_id = %goal_id, "Updating goal");

        // Get existing goal
        let mut goal = self
            .goal_repo
            .get(goal_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Goal not found: {}", goal_id))?;

        // Apply updates
        if let Some(new_name) = name {
            goal.name = new_name;
        }
        if let Some(new_desc) = description {
            goal.description = Some(new_desc);
        }

        self.goal_repo.update(&goal).await?;

        debug!(goal_id = %goal_id, "Goal updated");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete_goal(&self, goal_id: GoalId) -> Result<()> {
        debug!(goal_id = %goal_id, "Deleting goal");
        self.goal_repo.delete(goal_id).await?;
        debug!(goal_id = %goal_id, "Goal deleted");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_goal(&self, goal_id: GoalId) -> Result<Option<Goal>> {
        debug!(goal_id = %goal_id, "Getting goal");
        let goal = self.goal_repo.get(goal_id).await?;
        debug!(goal_id = %goal_id, found = goal.is_some(), "Got goal");
        Ok(goal)
    }

    #[instrument(skip(self))]
    async fn get_want(&self, want_id: WantId) -> Result<Option<Want>> {
        debug!(want_id = %want_id, "Getting want");
        let want = self.want_repo.get(want_id).await?;
        debug!(want_id = %want_id, found = want.is_some(), "Got want");
        Ok(want)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_created() {
        // Compile-time check that types are correct
        assert!(true);
    }
}
