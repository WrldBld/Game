//! Management use cases for CRUD-style operations.
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

pub use act::ActCrud;
pub use character::CharacterCrud;
pub use interaction::InteractionCrud;
pub use location::LocationCrud;
pub use observation::{ObservationCrud, ObservationSummaryData};
pub use player_character::PlayerCharacterCrud;
pub use relationship::RelationshipCrud;
pub use scene::SceneCrud;
pub use skill::SkillCrud;
pub use world::WorldCrud;

use crate::infrastructure::ports::RepoError;
use wrldbldr_domain::DomainError;

/// Shared error type for management use cases.
#[derive(Debug, thiserror::Error)]
pub enum ManagementError {
    #[error("Not found")]
    NotFound,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Domain error: {0}")]
    Domain(String),
}

impl From<DomainError> for ManagementError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::Validation(msg) => ManagementError::InvalidInput(msg),
            DomainError::NotFound { entity_type, id } => {
                ManagementError::Domain(format!("{} with id {} not found", entity_type, id))
            }
            other => ManagementError::Domain(other.to_string()),
        }
    }
}

/// Container for management use cases.
pub struct ManagementUseCases {
    pub world: WorldCrud,
    pub character: CharacterCrud,
    pub location: LocationCrud,
    pub player_character: PlayerCharacterCrud,
    pub relationship: RelationshipCrud,
    pub observation: ObservationCrud,
    pub act: ActCrud,
    pub scene: SceneCrud,
    pub interaction: InteractionCrud,
    pub skill: SkillCrud,
}

impl ManagementUseCases {
    pub fn new(
        world: WorldCrud,
        character: CharacterCrud,
        location: LocationCrud,
        player_character: PlayerCharacterCrud,
        relationship: RelationshipCrud,
        observation: ObservationCrud,
        act: ActCrud,
        scene: SceneCrud,
        interaction: InteractionCrud,
        skill: SkillCrud,
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
