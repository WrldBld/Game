//! Application state and composition.

use std::sync::Arc;

use crate::entities;
use crate::infrastructure::{
    neo4j::Neo4jRepositories,
    ports::{ImageGenPort, LlmPort},
    queue::SqliteQueue,
};
use crate::use_cases;

/// Main application state.
///
/// Holds all entity modules and use cases.
/// Passed to HTTP/WebSocket handlers via Axum state.
pub struct App {
    pub entities: Entities,
    pub use_cases: UseCases,
}

/// Container for all entity modules.
pub struct Entities {
    pub character: Arc<entities::Character>,
    pub location: Arc<entities::Location>,
    pub scene: Arc<entities::Scene>,
    pub challenge: Arc<entities::Challenge>,
    pub narrative: Arc<entities::Narrative>,
    pub staging: Arc<entities::Staging>,
    pub observation: Arc<entities::Observation>,
    pub inventory: Arc<entities::Inventory>,
    pub assets: Arc<entities::Assets>,
    pub world: Arc<entities::World>,
}

/// Container for all use cases.
pub struct UseCases {
    pub movement: use_cases::MovementUseCases,
    pub conversation: use_cases::ConversationUseCases,
    pub challenge: use_cases::ChallengeUseCases,
    pub approval: use_cases::ApprovalUseCases,
    pub assets: use_cases::AssetUseCases,
    pub world: use_cases::WorldUseCases,
    pub queues: use_cases::QueueUseCases,
}

impl App {
    /// Create a new App with all dependencies wired up.
    pub fn new(
        repos: Neo4jRepositories,
        llm: Arc<dyn LlmPort>,
        image_gen: Arc<dyn ImageGenPort>,
        queue: Arc<SqliteQueue>,
    ) -> Self {
        // Create entity modules
        let character = Arc::new(entities::Character::new(repos.character.clone()));
        let location = Arc::new(entities::Location::new(repos.location.clone()));
        let scene = Arc::new(entities::Scene::new(repos.scene.clone()));
        let challenge = Arc::new(entities::Challenge::new(repos.challenge.clone()));
        let narrative = Arc::new(entities::Narrative::new(repos.narrative.clone()));
        let staging = Arc::new(entities::Staging::new(repos.staging.clone()));
        let observation = Arc::new(entities::Observation::new(repos.observation.clone()));
        let inventory = Arc::new(entities::Inventory::new(repos.item.clone()));
        let assets = Arc::new(entities::Assets::new(repos.asset.clone(), image_gen));
        let world = Arc::new(entities::World::new(repos.world.clone()));

        let entities = Entities {
            character: character.clone(),
            location: location.clone(),
            scene: scene.clone(),
            challenge: challenge.clone(),
            narrative: narrative.clone(),
            staging: staging.clone(),
            observation: observation.clone(),
            inventory: inventory.clone(),
            assets: assets.clone(),
            world: world.clone(),
        };

        // Create use cases
        let movement = use_cases::MovementUseCases::new(
            Arc::new(use_cases::movement::EnterRegion::new(
                character.clone(),
                location.clone(),
                staging.clone(),
                observation.clone(),
                narrative.clone(),
            )),
            Arc::new(use_cases::movement::ExitLocation::new(
                location.clone(),
                staging.clone(),
                observation.clone(),
                narrative.clone(),
            )),
        );

        let conversation = use_cases::ConversationUseCases::new(
            Arc::new(use_cases::conversation::StartConversation::new(
                character.clone(),
                llm.clone(),
            )),
            Arc::new(use_cases::conversation::ContinueConversation::new(
                character.clone(),
                llm.clone(),
            )),
        );

        let challenge_uc = use_cases::ChallengeUseCases::new(
            Arc::new(use_cases::challenge::RollChallenge::new(challenge.clone())),
            Arc::new(use_cases::challenge::ResolveOutcome::new(challenge.clone())),
        );

        let approval = use_cases::ApprovalUseCases::new(
            Arc::new(use_cases::approval::ApproveStaging::new(staging.clone())),
            Arc::new(use_cases::approval::ApproveSuggestion::new()),
        );

        let assets_uc = use_cases::AssetUseCases::new(Arc::new(
            use_cases::assets::GenerateAsset::new(assets.clone()),
        ));

        let world_uc = use_cases::WorldUseCases::new(
            Arc::new(use_cases::world::ExportWorld::new(world.clone())),
            Arc::new(use_cases::world::ImportWorld::new(world.clone())),
        );

        let queues = use_cases::QueueUseCases::new(
            Arc::new(use_cases::queues::ProcessPlayerAction::new(queue.clone())),
            Arc::new(use_cases::queues::ProcessLlmRequest::new(queue)),
        );

        let use_cases = UseCases {
            movement,
            conversation,
            challenge: challenge_uc,
            approval,
            assets: assets_uc,
            world: world_uc,
            queues,
        };

        Self {
            entities,
            use_cases,
        }
    }
}
