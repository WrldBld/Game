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
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
}

impl From<ValidationError> for ManagementError {
    fn from(err: ValidationError) -> Self {
        ManagementError::InvalidInput(err.to_string())
    }
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
