//! Application state and composition.

use std::sync::Arc;

use crate::entities;
use crate::infrastructure::{
    clock::{SystemClock, SystemRandom},
    neo4j::Neo4jRepositories,
    ports::{ClockPort, ImageGenPort, LlmPort, QueuePort, RandomPort},
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
    pub queue: Arc<dyn QueuePort>,
    pub llm: Arc<dyn LlmPort>,
}

/// Container for all entity modules.
pub struct Entities {
    pub character: Arc<entities::Character>,
    pub player_character: Arc<entities::PlayerCharacter>,
    pub location: Arc<entities::Location>,
    pub scene: Arc<entities::Scene>,
    pub challenge: Arc<entities::Challenge>,
    pub narrative: Arc<entities::Narrative>,
    pub staging: Arc<entities::Staging>,
    pub observation: Arc<entities::Observation>,
    pub inventory: Arc<entities::Inventory>,
    pub assets: Arc<entities::Assets>,
    pub world: Arc<entities::World>,
    pub flag: Arc<entities::Flag>,
    pub lore: Arc<entities::Lore>,
    pub location_state: Arc<entities::LocationStateEntity>,
    pub region_state: Arc<entities::RegionStateEntity>,
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
    pub narrative: use_cases::NarrativeUseCases,
    pub time: use_cases::TimeUseCases,
    pub visual_state: use_cases::VisualStateUseCases,
}

impl App {
    /// Create a new App with all dependencies wired up.
    pub fn new(
        repos: Neo4jRepositories,
        llm: Arc<dyn LlmPort>,
        image_gen: Arc<dyn ImageGenPort>,
        queue: Arc<SqliteQueue>,
    ) -> Self {
        // Create infrastructure services
        let clock: Arc<dyn ClockPort> = Arc::new(SystemClock::new());
        let random: Arc<dyn RandomPort> = Arc::new(SystemRandom::new());
        let queue_port: Arc<dyn QueuePort> = queue.clone();

        // Create entity modules
        let character = Arc::new(entities::Character::new(repos.character.clone()));
        let player_character = Arc::new(entities::PlayerCharacter::new(
            repos.player_character.clone(),
        ));
        let location = Arc::new(entities::Location::new(repos.location.clone()));
        let scene = Arc::new(entities::Scene::new(repos.scene.clone()));
        let challenge = Arc::new(entities::Challenge::new(repos.challenge.clone()));
        let narrative = Arc::new(entities::Narrative::new(
            repos.narrative.clone(),
            repos.location.clone(),
            repos.player_character.clone(),
            repos.observation.clone(),
            repos.challenge.clone(),
            clock.clone(),
        ));
        let staging = Arc::new(entities::Staging::new(repos.staging.clone()));
        let observation = Arc::new(entities::Observation::new(
            repos.observation.clone(),
            repos.location.clone(),
            clock.clone(),
        ));
        let inventory = Arc::new(entities::Inventory::new(
            repos.item.clone(),
            repos.character.clone(),
            repos.player_character.clone(),
        ));
        let assets = Arc::new(entities::Assets::new(repos.asset.clone(), image_gen));
        let world = Arc::new(entities::World::new(repos.world.clone()));
        let flag = Arc::new(entities::Flag::new(repos.flag.clone()));
        let lore = Arc::new(entities::Lore::new(repos.lore.clone()));
        let location_state = Arc::new(entities::LocationStateEntity::new(
            repos.location_state.clone(),
        ));
        let region_state = Arc::new(entities::RegionStateEntity::new(repos.region_state.clone()));

        let entities = Entities {
            character: character.clone(),
            player_character: player_character.clone(),
            location: location.clone(),
            scene: scene.clone(),
            challenge: challenge.clone(),
            narrative: narrative.clone(),
            staging: staging.clone(),
            observation: observation.clone(),
            inventory: inventory.clone(),
            assets: assets.clone(),
            world: world.clone(),
            flag: flag.clone(),
            lore: lore.clone(),
            location_state: location_state.clone(),
            region_state: region_state.clone(),
        };

        // Create time use case first (needed by movement)
        let suggest_time = Arc::new(use_cases::time::SuggestTime::new(
            world.clone(),
            clock.clone(),
        ));

        // Create use cases
        let movement = use_cases::MovementUseCases::new(
            Arc::new(use_cases::movement::EnterRegion::new(
                player_character.clone(),
                location.clone(),
                staging.clone(),
                observation.clone(),
                narrative.clone(),
                scene.clone(),
                inventory.clone(),
                flag.clone(),
                world.clone(),
                suggest_time.clone(),
                clock.clone(),
            )),
            Arc::new(use_cases::movement::ExitLocation::new(
                player_character.clone(),
                location.clone(),
                staging.clone(),
                observation.clone(),
                narrative.clone(),
                scene.clone(),
                inventory.clone(),
                flag.clone(),
                world.clone(),
                suggest_time.clone(),
                clock.clone(),
            )),
        );

        let conversation = use_cases::ConversationUseCases::new(
            Arc::new(use_cases::conversation::StartConversation::new(
                character.clone(),
                player_character.clone(),
                staging.clone(),
                scene.clone(),
                queue_port.clone(),
                clock.clone(),
            )),
            Arc::new(use_cases::conversation::ContinueConversation::new(
                character.clone(),
                player_character.clone(),
                staging.clone(),
                queue_port.clone(),
                clock.clone(),
            )),
        );

        let challenge_uc = use_cases::ChallengeUseCases::new(
            Arc::new(use_cases::challenge::RollChallenge::new(
                challenge.clone(),
                player_character.clone(),
                queue_port.clone(),
                random.clone(),
                clock.clone(),
            )),
            Arc::new(use_cases::challenge::ResolveOutcome::new(
                challenge.clone(),
                inventory.clone(),
                observation.clone(),
                scene.clone(),
                player_character.clone(),
            )),
        );

        let approval = use_cases::ApprovalUseCases::new(
            Arc::new(use_cases::approval::ApproveStaging::new(staging.clone())),
            Arc::new(use_cases::approval::ApproveSuggestion::new(
                queue_port.clone(),
            )),
        );

        let generate_asset = Arc::new(use_cases::assets::GenerateAsset::new(
            assets.clone(),
            queue_port.clone(),
            clock.clone(),
        ));
        let expression_sheet = Arc::new(use_cases::assets::GenerateExpressionSheet::new(
            assets.clone(),
            character.clone(),
            queue_port.clone(),
            clock.clone(),
        ));
        let assets_uc = use_cases::AssetUseCases::new(generate_asset, expression_sheet);

        let world_uc = use_cases::WorldUseCases::new(
            Arc::new(use_cases::world::ExportWorld::new(
                world.clone(),
                location.clone(),
                character.clone(),
                inventory.clone(),
                narrative.clone(),
            )),
            Arc::new(use_cases::world::ImportWorld::new(
                world.clone(),
                location.clone(),
                character.clone(),
                inventory.clone(),
                narrative.clone(),
            )),
        );

        let queues = use_cases::QueueUseCases::new(
            Arc::new(use_cases::queues::ProcessPlayerAction::new(
                queue_port.clone(),
                character.clone(),
                player_character.clone(),
                staging.clone(),
            )),
            Arc::new(use_cases::queues::ProcessLlmRequest::new(
                queue_port,
                llm.clone(),
            )),
        );

        let narrative_uc =
            use_cases::NarrativeUseCases::new(Arc::new(use_cases::narrative::ExecuteEffects::new(
                inventory.clone(),
                challenge.clone(),
                narrative.clone(),
                character.clone(),
                observation.clone(),
                player_character.clone(),
                scene.clone(),
                flag.clone(),
                clock.clone(),
            )));

        let time_uc = use_cases::TimeUseCases::new(suggest_time);

        let visual_state_uc = use_cases::VisualStateUseCases::new(Arc::new(
            use_cases::visual_state::ResolveVisualState::new(
                location_state.clone(),
                region_state.clone(),
                flag.clone(),
            ),
        ));

        let use_cases = UseCases {
            movement,
            conversation,
            challenge: challenge_uc,
            approval,
            assets: assets_uc,
            world: world_uc,
            queues,
            narrative: narrative_uc,
            time: time_uc,
            visual_state: visual_state_uc,
        };

        Self {
            entities,
            use_cases,
            queue: queue,
            llm,
        }
    }
}
