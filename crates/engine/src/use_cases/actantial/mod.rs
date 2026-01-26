// Actantial use cases - methods for future motivation features
#![allow(dead_code)]

//! Actantial (motivations) use cases.
//!
//! Handles goals, wants, and actantial context operations.

use std::sync::Arc;

use wrldbldr_domain::{
    ActantialContext, ActantialRole, ActantialTarget, CharacterId, DomainError, GoalId, GoalName,
    Want, WantId, WantTarget, WantVisibility, WorldId,
};

use crate::infrastructure::ports::{
    ActantialViewRecord, CharacterRepo, ClockPort, GoalDetails, GoalRepo, RepoError, WantDetails,
    WantTargetRef,
};
use crate::use_cases::validation::{require_non_empty, ValidationError};

/// Shared error type for actantial use cases.
#[derive(Debug, thiserror::Error)]
pub enum ActantialError {
    #[error("{entity_type} not found: {id}")]
    NotFound {
        entity_type: &'static str,
        id: String,
    },
    #[error("Unauthorized: {message}")]
    Unauthorized {
        message: String,
    },
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
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
    goal: Arc<dyn GoalRepo>,
}

impl GoalOps {
    pub fn new(goal: Arc<dyn GoalRepo>) -> Self {
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
        let goal_name = GoalName::new(&name).map_err(ActantialError::Domain)?;

        let mut goal = wrldbldr_domain::Goal::new(world_id, goal_name);
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
        world_id: WorldId,
        goal_id: GoalId,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<GoalDetails, ActantialError> {
        let mut details = self
            .goal
            .get(goal_id)
            .await?
            .ok_or(ActantialError::NotFound {
                entity_type: "Goal",
                id: goal_id.to_string(),
            })?;

        // Validate goal belongs to requested world
        if details.goal.world_id() != world_id {
            return Err(ActantialError::Unauthorized {
                message: "Goal not in current world".to_string(),
            });
        }

        // Rebuild the goal with updated values using from_storage
        let new_name = if let Some(name) = name {
            GoalName::new(&name).map_err(ActantialError::Domain)?
        } else {
            details.goal.name().clone()
        };

        let new_description = match description {
            Some(desc) if desc.trim().is_empty() => None,
            Some(desc) => Some(desc),
            None => details.goal.description().map(|s| s.to_string()),
        };

        details.goal = wrldbldr_domain::Goal::from_storage(
            details.goal.id(),
            details.goal.world_id(),
            new_name,
            new_description,
        );

        self.goal.save(&details.goal).await?;
        Ok(details)
    }

    pub async fn delete(
        &self,
        world_id: WorldId,
        goal_id: GoalId,
    ) -> Result<(), ActantialError> {
        let details = self
            .goal
            .get(goal_id)
            .await?
            .ok_or(ActantialError::NotFound {
                entity_type: "Goal",
                id: goal_id.to_string(),
            })?;

        // Validate goal belongs to requested world
        if details.goal.world_id() != world_id {
            return Err(ActantialError::Unauthorized {
                message: "Goal not in current world".to_string(),
            });
        }

        self.goal.delete(goal_id).await?;
        Ok(())
    }
}

// =============================================================================
// Want Operations
// =============================================================================

pub struct WantOps {
    character: Arc<dyn CharacterRepo>,
    clock: Arc<dyn ClockPort>,
}

impl WantOps {
    pub fn new(character: Arc<dyn CharacterRepo>, clock: Arc<dyn ClockPort>) -> Self {
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
        let mut details =
            self.character
                .get_want(want_id)
                .await?
                .ok_or(ActantialError::NotFound {
                    entity_type: "Want",
                    id: want_id.to_string(),
                })?;

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
    character: Arc<dyn CharacterRepo>,
}

impl ActantialContextOps {
    pub fn new(character: Arc<dyn CharacterRepo>) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::clock::FixedClock;
    use crate::infrastructure::ports::{ClockPort, MockCharacterRepo, MockGoalRepo};
    use chrono::TimeZone;
    use std::sync::Arc;

    fn fixed_time() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    fn create_test_goal(world_id: WorldId) -> wrldbldr_domain::Goal {
        wrldbldr_domain::Goal::new(world_id, GoalName::new("Test Goal").unwrap())
            .with_description("A test goal for testing")
    }

    fn create_test_want() -> Want {
        Want::new("Test want description", fixed_time())
            .with_intensity(0.7)
            .with_visibility(WantVisibility::Known)
    }

    mod goal_ops {
        use super::*;

        #[tokio::test]
        async fn when_create_goal_succeeds() {
            let world_id = WorldId::new();

            let mut goal_repo = MockGoalRepo::new();
            goal_repo.expect_save().returning(|_| Ok(()));

            let ops = GoalOps::new(Arc::new(goal_repo) as Arc<dyn GoalRepo>);

            let result = ops
                .create(
                    world_id,
                    "Power".to_string(),
                    Some("The pursuit of power".to_string()),
                )
                .await;

            assert!(result.is_ok());
            let goal_details = result.unwrap();
            assert_eq!(goal_details.goal.name().as_str(), "Power");
            assert_eq!(
                goal_details.goal.description(),
                Some("The pursuit of power")
            );
            assert_eq!(goal_details.usage_count, 0);
        }

        #[tokio::test]
        async fn when_list_goals_succeeds() {
            let world_id = WorldId::new();
            let goal = create_test_goal(world_id);

            let mut goal_repo = MockGoalRepo::new();
            goal_repo
                .expect_list_in_world()
                .withf(move |w| *w == world_id)
                .returning(move |_| {
                    Ok(vec![GoalDetails {
                        goal: goal.clone(),
                        usage_count: 5,
                    }])
                });

            let ops = GoalOps::new(Arc::new(goal_repo) as Arc<dyn GoalRepo>);

            let result = ops.list(world_id).await;
            assert!(result.is_ok());
            let goals = result.unwrap();
            assert_eq!(goals.len(), 1);
            assert_eq!(goals[0].goal.name().as_str(), "Test Goal");
            assert_eq!(goals[0].usage_count, 5);
        }

        #[tokio::test]
        async fn when_get_goal_not_found_returns_none() {
            let goal_id = GoalId::new();

            let mut goal_repo = MockGoalRepo::new();
            goal_repo
                .expect_get()
                .withf(move |id| *id == goal_id)
                .returning(|_| Ok(None));

            let ops = GoalOps::new(Arc::new(goal_repo) as Arc<dyn GoalRepo>);

            let result = ops.get(goal_id).await;
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }
    }

    mod want_ops {
        use super::*;

        #[tokio::test]
        async fn when_create_want_succeeds() {
            let character_id = CharacterId::new();

            let mut character_repo = MockCharacterRepo::new();
            character_repo
                .expect_save_want()
                .returning(|_, _, _| Ok(()));

            let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(fixed_time()));

            let ops = WantOps::new(Arc::new(character_repo) as Arc<dyn CharacterRepo>, clock);

            let result = ops
                .create(
                    character_id,
                    "Find the treasure".to_string(),
                    0.8,
                    1,
                    WantVisibility::Known,
                    Some("Nervously change the subject".to_string()),
                    vec!["Fidgets when gold is mentioned".to_string()],
                )
                .await;

            assert!(result.is_ok());
            let want_details = result.unwrap();
            assert_eq!(want_details.want.description(), "Find the treasure");
            assert!((want_details.want.intensity() - 0.8).abs() < 0.001);
            assert_eq!(want_details.want.visibility(), WantVisibility::Known);
            assert_eq!(want_details.priority, 1);
        }

        #[tokio::test]
        async fn when_list_wants_succeeds() {
            let character_id = CharacterId::new();
            let want = create_test_want();

            let mut character_repo = MockCharacterRepo::new();
            character_repo
                .expect_get_wants()
                .withf(move |id| *id == character_id)
                .returning(move |cid| {
                    Ok(vec![WantDetails {
                        character_id: cid,
                        want: want.clone(),
                        priority: 1,
                        target: None,
                    }])
                });

            let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(fixed_time()));

            let ops = WantOps::new(Arc::new(character_repo) as Arc<dyn CharacterRepo>, clock);

            let result = ops.list(character_id).await;
            assert!(result.is_ok());
            let wants = result.unwrap();
            assert_eq!(wants.len(), 1);
            assert_eq!(wants[0].want.description(), "Test want description");
            assert_eq!(wants[0].priority, 1);
        }

        #[tokio::test]
        async fn when_create_want_empty_description_returns_error() {
            let character_id = CharacterId::new();

            let character_repo = MockCharacterRepo::new();
            let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(fixed_time()));

            let ops = WantOps::new(Arc::new(character_repo) as Arc<dyn CharacterRepo>, clock);

            let result = ops
                .create(
                    character_id,
                    "   ".to_string(), // Empty after trim
                    0.5,
                    1,
                    WantVisibility::Hidden,
                    None,
                    vec![],
                )
                .await;

            assert!(matches!(result, Err(ActantialError::Validation(_))));
        }
    }

    mod actantial_context_ops {
        use super::*;

        #[tokio::test]
        async fn when_get_context_succeeds() {
            let character_id = CharacterId::new();
            let context = ActantialContext::new(character_id, "Test Character");

            let mut character_repo = MockCharacterRepo::new();
            character_repo
                .expect_get_actantial_context()
                .withf(move |id| *id == character_id)
                .returning(move |_| Ok(Some(context.clone())));

            let ops = ActantialContextOps::new(Arc::new(character_repo) as Arc<dyn CharacterRepo>);

            let result = ops.get_context(character_id).await;
            assert!(result.is_ok());
            let ctx = result.unwrap();
            assert!(ctx.is_some());
            assert_eq!(ctx.unwrap().character_name(), "Test Character");
        }

        #[tokio::test]
        async fn when_get_context_not_found_returns_none() {
            let character_id = CharacterId::new();

            let mut character_repo = MockCharacterRepo::new();
            character_repo
                .expect_get_actantial_context()
                .withf(move |id| *id == character_id)
                .returning(|_| Ok(None));

            let ops = ActantialContextOps::new(Arc::new(character_repo) as Arc<dyn CharacterRepo>);

            let result = ops.get_context(character_id).await;
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }
    }

    mod error_handling {
        use super::*;

        #[test]
        fn test_domain_error_preserved() {
            let domain_err = DomainError::validation("Goal name cannot be empty");

            // Test that mapping DomainError via From trait preserves the error
            let use_case_err: ActantialError = ActantialError::Domain(domain_err);

            // Verify the source DomainError is accessible
            assert!(matches!(use_case_err, ActantialError::Domain(_)));

            let error_msg = use_case_err.to_string();
            assert!(error_msg.contains("Goal name cannot be empty"));
        }

        #[test]
        fn test_validation_error_preserves_chain() {
            let validation_err = ValidationError::Empty {
                field_name: "Goal name",
            };

            // Test that ValidationError converts to ActantialError via From trait
            let use_case_err: ActantialError = validation_err.into();

            // Verify it's a Validation variant and the source ValidationError is accessible
            assert!(matches!(use_case_err, ActantialError::Validation(_)));

            let error_msg = use_case_err.to_string();
            assert!(error_msg.contains("Goal name"));
            assert!(error_msg.contains("cannot be empty"));
        }

        #[test]
        fn test_validation_error_message_preserved() {
            let validation_err = ValidationError::TooLong {
                field_name: "Description",
                max: 500,
            };

            let use_case_err: ActantialError = validation_err.into();

            // Verify the original error message is accessible
            let error_msg = use_case_err.to_string();
            assert!(error_msg.contains("Description"));
            assert!(error_msg.contains("500"));
        }
    }
}
