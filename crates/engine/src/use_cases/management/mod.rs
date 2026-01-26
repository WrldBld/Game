//! Management use cases for entity lifecycle operations.
//!
//! These use cases keep WebSocket handlers thin while coordinating entity modules.

mod act;
mod character;
mod interaction;
mod location;
mod observation;
mod player_character;
mod relationship;
mod scene;
mod skill;
mod world;

pub use act::ActManagement;
pub use character::CharacterManagement;
pub use interaction::InteractionManagement;
pub use location::LocationManagement;
pub use observation::ObservationManagement;
pub use player_character::PlayerCharacterManagement;
pub use relationship::RelationshipManagement;
pub use scene::SceneManagement;
pub use skill::SkillManagement;
pub use world::WorldManagement;

use super::validation::ValidationError;
use crate::infrastructure::ports::RepoError;
use wrldbldr_domain::DomainError;

/// Shared error type for management use cases.
#[derive(Debug, thiserror::Error)]
pub enum ManagementError {
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
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
}

/// Container for management use cases.
pub struct ManagementUseCases {
    pub world: WorldManagement,
    pub character: CharacterManagement,
    pub location: LocationManagement,
    pub player_character: PlayerCharacterManagement,
    pub relationship: RelationshipManagement,
    pub observation: ObservationManagement,
    pub act: ActManagement,
    pub scene: SceneManagement,
    pub interaction: InteractionManagement,
    pub skill: SkillManagement,
}

impl ManagementUseCases {
    pub fn new(
        world: WorldManagement,
        character: CharacterManagement,
        location: LocationManagement,
        player_character: PlayerCharacterManagement,
        relationship: RelationshipManagement,
        observation: ObservationManagement,
        act: ActManagement,
        scene: SceneManagement,
        interaction: InteractionManagement,
        skill: SkillManagement,
    ) -> Self {
        Self {
            world,
            character,
            location,
            player_character,
            relationship,
            observation,
            act,
            scene,
            interaction,
            skill,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod error_handling {
        use super::*;

        #[test]
        fn test_domain_error_preserved() {
            let domain_err = DomainError::validation("Character name cannot be empty");

            // Test that mapping DomainError via From trait preserves the error
            let use_case_err: ManagementError = ManagementError::Domain(domain_err);

            // Verify the source DomainError is accessible
            assert!(matches!(use_case_err, ManagementError::Domain(_)));

            let error_msg = use_case_err.to_string();
            assert!(error_msg.contains("Character name cannot be empty"));
        }

        #[test]
        fn test_validation_error_preserves_chain() {
            let validation_err = ValidationError::Empty {
                field_name: "Character name",
            };

            // Test that ValidationError converts to ManagementError via From trait
            let use_case_err: ManagementError = validation_err.into();

            // Verify it's a Validation variant and the source ValidationError is accessible
            assert!(matches!(use_case_err, ManagementError::Validation(_)));

            let error_msg = use_case_err.to_string();
            assert!(error_msg.contains("Character name"));
            assert!(error_msg.contains("cannot be empty"));
        }

        #[test]
        fn test_validation_error_message_preserved() {
            let validation_err = ValidationError::TooLong {
                field_name: "Description",
                max: 1000,
            };

            let use_case_err: ManagementError = validation_err.into();

            // Verify the original error message is accessible
            let error_msg = use_case_err.to_string();
            assert!(error_msg.contains("Description"));
            assert!(error_msg.contains("1000"));
        }

        #[test]
        fn test_repo_error_preserved_via_from() {
            let repo_err = RepoError::NotFound {
                entity_type: "Character",
                id: "123e4567-e89b-12d3-a456-426614174000".to_string(),
            };

            // Test that RepoError converts via From trait
            let use_case_err: ManagementError = repo_err.into();

            // Verify the source RepoError is accessible
            assert!(matches!(use_case_err, ManagementError::Repo(_)));

            let error_msg = use_case_err.to_string();
            assert!(error_msg.contains("Character"));
            assert!(error_msg.contains("123e4567"));
        }
    }
}

