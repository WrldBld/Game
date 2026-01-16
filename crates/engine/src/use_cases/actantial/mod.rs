//! Actantial (motivations) use cases.
//!
//! Handles goals, wants, and actantial context operations.

use std::sync::Arc;

use wrldbldr_domain::{
    ActantialContext, ActantialRole, ActantialTarget, CharacterId, GoalId, Want, WantId,
    WantTarget, WantVisibility, WorldId,
};

use crate::infrastructure::ports::{
    ActantialViewRecord, GoalDetails, RepoError, WantDetails, WantTargetRef,
};
use crate::repositories::character::Character;
use crate::repositories::{Clock, Goal};
use crate::use_cases::validation::{require_non_empty, ValidationError};

/// Shared error type for actantial use cases.
#[derive(Debug, thiserror::Error)]
pub enum ActantialError {
    #[error("Not found")]
    NotFound,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

impl From<ValidationError> for ActantialError {
    fn from(err: ValidationError) -> Self {
        ActantialError::InvalidInput(err.to_string())
    }
}

/// Container for actantial use cases.
pub struct ActantialUseCases {
    pub goals: GoalOps,
    pub wants: WantOps,
    pub context: ActantialContextOps,
}

impl ActantialUseCases {
    pub fn new(goals: GoalOps, wants: WantOps, context: ActantialContextOps) -> Self {
        Self {
            goals,
            wants,
            context,
        }
    }
}

// =============================================================================
// Goal Operations
// =============================================================================

pub struct GoalOps {
    goal: Arc<Goal>,
}

impl GoalOps {
    pub fn new(goal: Arc<Goal>) -> Self {
        Self { goal }
    }

    pub async fn list(&self, world_id: WorldId) -> Result<Vec<GoalDetails>, ActantialError> {
        Ok(self.goal.list_in_world(world_id).await?)
    }

    pub async fn get(&self, goal_id: GoalId) -> Result<Option<GoalDetails>, ActantialError> {
        Ok(self.goal.get(goal_id).await?)
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        name: String,
        description: Option<String>,
    ) -> Result<GoalDetails, ActantialError> {
        require_non_empty(&name, "Goal name")?;

        let mut goal = wrldbldr_domain::Goal::new(world_id, name);
        if let Some(description) = description {
            if !description.trim().is_empty() {
                goal = goal.with_description(description);
            }
        }

        self.goal.save(&goal).await?;

        Ok(GoalDetails {
            goal,
            usage_count: 0,
        })
    }

    pub async fn update(
        &self,
        goal_id: GoalId,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<GoalDetails, ActantialError> {
        let mut details = self
            .goal
            .get(goal_id)
            .await?
            .ok_or(ActantialError::NotFound)?;

        // Rebuild the goal with updated values using from_parts
        let new_name = if let Some(name) = name {
            require_non_empty(&name, "Goal name")?;
            name
        } else {
            details.goal.name().to_string()
        };

        let new_description = match description {
            Some(desc) if desc.trim().is_empty() => None,
            Some(desc) => Some(desc),
            None => details.goal.description().map(|s| s.to_string()),
        };

        details.goal = wrldbldr_domain::Goal::from_parts(
            details.goal.id(),
            details.goal.world_id(),
            new_name,
            new_description,
        );

        self.goal.save(&details.goal).await?;
        Ok(details)
    }

    pub async fn delete(&self, goal_id: GoalId) -> Result<(), ActantialError> {
        if self.goal.get(goal_id).await?.is_none() {
            return Err(ActantialError::NotFound);
        }
        self.goal.delete(goal_id).await?;
        Ok(())
    }
}

// =============================================================================
// Want Operations
// =============================================================================

pub struct WantOps {
    character: Arc<Character>,
    clock: Arc<Clock>,
}

impl WantOps {
    pub fn new(character: Arc<Character>, clock: Arc<Clock>) -> Self {
        Self { character, clock }
    }

    pub async fn list(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<WantDetails>, ActantialError> {
        Ok(self.character.get_wants(character_id).await?)
    }

    pub async fn get(&self, want_id: WantId) -> Result<Option<WantDetails>, ActantialError> {
        Ok(self.character.get_want(want_id).await?)
    }

    pub async fn create(
        &self,
        character_id: CharacterId,
        description: String,
        intensity: f32,
        priority: u32,
        visibility: WantVisibility,
        deflection_behavior: Option<String>,
        tells: Vec<String>,
    ) -> Result<WantDetails, ActantialError> {
        require_non_empty(&description, "Want description")?;

        let now = self.clock.now();
        let mut want = Want::new(description, now)
            .with_intensity(intensity)
            .with_visibility(visibility);

        if let Some(deflection) = deflection_behavior {
            if !deflection.trim().is_empty() {
                want = want.with_deflection(deflection);
            }
        }

        want = want.with_tells(tells);

        self.character
            .save_want(character_id, &want, priority)
            .await?;

        Ok(WantDetails {
            character_id,
            want,
            priority: priority.max(1),
            target: None,
        })
    }

    pub async fn update(
        &self,
        want_id: WantId,
        description: Option<String>,
        intensity: Option<f32>,
        priority: Option<u32>,
        visibility: Option<WantVisibility>,
        deflection_behavior: Option<String>,
        tells: Option<Vec<String>>,
    ) -> Result<WantDetails, ActantialError> {
        let mut details = self
            .character
            .get_want(want_id)
            .await?
            .ok_or(ActantialError::NotFound)?;

        // Rebuild the want with updated values
        let new_description = if let Some(description) = description {
            require_non_empty(&description, "Want description")?;
            description
        } else {
            details.want.description().to_string()
        };

        let new_intensity = intensity.unwrap_or_else(|| details.want.intensity());
        let new_visibility = visibility.unwrap_or_else(|| details.want.visibility());
        let new_tells = tells.unwrap_or_else(|| details.want.tells().to_vec());

        // Rebuild the want using builder pattern
        let mut updated_want = Want::new(new_description, details.want.created_at())
            .with_id(details.want.id())
            .with_intensity(new_intensity)
            .with_visibility(new_visibility)
            .with_tells(new_tells);

        // Handle deflection behavior - if explicitly provided, use it; otherwise keep existing
        if let Some(deflection) = deflection_behavior {
            if !deflection.trim().is_empty() {
                updated_want = updated_want.with_deflection(deflection);
            }
            // If empty string provided, leave deflection as None (cleared)
        } else if let Some(existing_deflection) = details.want.deflection_behavior() {
            updated_want = updated_want.with_deflection(existing_deflection);
        }

        details.want = updated_want;

        if let Some(priority) = priority {
            details.priority = priority.max(1);
        }

        self.character
            .save_want(details.character_id, &details.want, details.priority)
            .await?;

        Ok(details)
    }

    pub async fn delete(&self, want_id: WantId) -> Result<(), ActantialError> {
        self.character.delete_want(want_id).await?;
        Ok(())
    }

    pub async fn set_target(
        &self,
        want_id: WantId,
        target: WantTargetRef,
    ) -> Result<WantTarget, ActantialError> {
        Ok(self.character.set_want_target(want_id, target).await?)
    }

    pub async fn remove_target(&self, want_id: WantId) -> Result<(), ActantialError> {
        self.character.remove_want_target(want_id).await?;
        Ok(())
    }
}

// =============================================================================
// Actantial Context Operations
// =============================================================================

pub struct ActantialContextOps {
    character: Arc<Character>,
}

impl ActantialContextOps {
    pub fn new(character: Arc<Character>) -> Self {
        Self { character }
    }

    pub async fn get_context(
        &self,
        character_id: CharacterId,
    ) -> Result<Option<ActantialContext>, ActantialError> {
        Ok(self.character.get_actantial_context(character_id).await?)
    }

    pub async fn add_view(
        &self,
        character_id: CharacterId,
        want_id: WantId,
        target: ActantialTarget,
        role: ActantialRole,
        reason: String,
    ) -> Result<ActantialViewRecord, ActantialError> {
        Ok(self
            .character
            .add_actantial_view(character_id, want_id, target, role, reason)
            .await?)
    }

    pub async fn remove_view(
        &self,
        character_id: CharacterId,
        want_id: WantId,
        target: ActantialTarget,
        role: ActantialRole,
    ) -> Result<(), ActantialError> {
        self.character
            .remove_actantial_view(character_id, want_id, target, role)
            .await?;
        Ok(())
    }
}
