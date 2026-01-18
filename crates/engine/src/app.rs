// App struct holds dependencies - some fields for future features
#![allow(dead_code)]

//! Application state and composition.

use std::sync::Arc;

use crate::infrastructure::{
    clock::{SystemClock, SystemRandom},
    neo4j::Neo4jRepositories,
    ports::{
        ActRepo, ChallengeRepo, CharacterRepo, ClockPort, ContentRepo, FlagRepo, GoalRepo,
        ImageGenPort, InteractionRepo, ItemRepo, LlmPort, LocationRepo, LocationStateRepo,
        LoreRepo, NarrativeRepo, ObservationRepo, PlayerCharacterRepo, QueuePort, RandomPort,
        RegionStateRepo, SceneRepo, SettingsRepo, StagingRepo, WorldRepo,
    },
};
use crate::repositories;
use crate::use_cases;
use crate::use_cases::content::{ContentService, ContentServiceConfig};

/// Main application state.
///
/// Holds all repository modules and use cases.
/// Passed to HTTP/WebSocket handlers via Axum state.
pub struct App {
    pub repositories: Repositories,
    pub use_cases: UseCases,
    pub queue: Arc<dyn QueuePort>,
    pub llm: Arc<dyn LlmPort>,
    pub content: Arc<ContentService>,
}

/// Container for all repository modules.
///
/// Per ADR-009, all fields are now `Arc<dyn PortTrait>` - port traits injected directly.
/// Only AssetsRepository remains as a wrapper because it coordinates 2 ports with real logic.
pub struct Repositories {
    // Port traits injected directly (ADR-009)
    pub character: Arc<dyn CharacterRepo>,
    pub player_character: Arc<dyn PlayerCharacterRepo>,
    pub location: Arc<dyn LocationRepo>,
    pub scene: Arc<dyn SceneRepo>,
    pub act: Arc<dyn ActRepo>,
    pub content: Arc<dyn ContentRepo>,
    pub interaction: Arc<dyn InteractionRepo>,
    pub challenge: Arc<dyn ChallengeRepo>,
    pub observation: Arc<dyn ObservationRepo>,
    pub item: Arc<dyn ItemRepo>,
    pub goal: Arc<dyn GoalRepo>,
    pub location_state: Arc<dyn LocationStateRepo>,
    pub region_state: Arc<dyn RegionStateRepo>,
    pub staging: Arc<dyn StagingRepo>,
    pub world: Arc<dyn WorldRepo>,
    pub flag: Arc<dyn FlagRepo>,
    pub lore: Arc<dyn LoreRepo>,
    pub narrative_repo: Arc<dyn NarrativeRepo>,

    // Wrapper types that add business logic beyond delegation
    pub narrative: Arc<use_cases::NarrativeOps>,
    pub assets: Arc<repositories::AssetsRepository>,
}

/// Container for all use cases.
pub struct UseCases {
    pub movement: use_cases::MovementUseCases,
    pub conversation: use_cases::ConversationUseCases,
    pub challenge: use_cases::ChallengeUseCases,
    pub approval: use_cases::ApprovalUseCases,
    pub actantial: use_cases::ActantialUseCases,
    pub ai: use_cases::AiUseCases,
    pub assets: use_cases::AssetUseCases,
    pub scene_change: use_cases::SceneChangeBuilder,
    pub world: use_cases::WorldUseCases,
    pub queues: use_cases::QueueUseCases,
    pub narrative: use_cases::NarrativeUseCases,
    pub player_action: use_cases::PlayerActionUseCases,
    pub time: use_cases::TimeUseCases,
    pub visual_state: use_cases::VisualStateUseCases,
    pub management: use_cases::ManagementUseCases,
    pub session: use_cases::SessionUseCases,
    pub settings: Arc<repositories::SettingsRepository>,
    pub staging: use_cases::StagingUseCases,
    pub npc: use_cases::NpcUseCases,
    pub story_events: use_cases::StoryEventUseCases,
    pub lore: use_cases::LoreUseCases,
    pub location_events: use_cases::LocationEventUseCases,
    pub custom_condition: Arc<use_cases::CustomConditionEvaluator>,
    pub inventory: use_cases::InventoryUseCases,
    pub character_sheet: use_cases::CharacterSheetUseCases,
}

impl App {
    /// Create a new App with all dependencies wired up.
    pub fn new(
        repos: Neo4jRepositories,
        llm: Arc<dyn LlmPort>,
        image_gen: Arc<dyn ImageGenPort>,
        queue: Arc<dyn QueuePort>,
        settings_repo: Arc<dyn SettingsRepo>,
        content_config: ContentServiceConfig,
    ) -> Self {
        // Create infrastructure services
        let clock_port: Arc<dyn ClockPort> = Arc::new(SystemClock::new());
        let random_port: Arc<dyn RandomPort> = Arc::new(SystemRandom::new());
        let queue_port: Arc<dyn QueuePort> = queue.clone();
        let llm_port: Arc<dyn LlmPort> = llm.clone();

        // Port traits from Neo4j - used directly per ADR-009
        let character_repo: Arc<dyn CharacterRepo> = repos.character.clone();
        let player_character_repo: Arc<dyn PlayerCharacterRepo> = repos.player_character.clone();
        let location_repo: Arc<dyn LocationRepo> = repos.location.clone();
        let scene_repo: Arc<dyn SceneRepo> = repos.scene.clone();
        let act_repo: Arc<dyn ActRepo> = repos.act.clone();
        let content_repo: Arc<dyn ContentRepo> = repos.content.clone();
        let interaction_repo: Arc<dyn InteractionRepo> = repos.interaction.clone();
        let challenge_repo: Arc<dyn ChallengeRepo> = repos.challenge.clone();
        let observation_repo: Arc<dyn ObservationRepo> = repos.observation.clone();
        let item_repo: Arc<dyn ItemRepo> = repos.item.clone();
        let goal_repo: Arc<dyn GoalRepo> = repos.goal.clone();
        let location_state_repo: Arc<dyn LocationStateRepo> = repos.location_state.clone();
        let region_state_repo: Arc<dyn RegionStateRepo> = repos.region_state.clone();

        // Port traits for remaining repositories (now injected directly per ADR-009)
        let staging_repo: Arc<dyn StagingRepo> = repos.staging.clone();
        let world_repo: Arc<dyn WorldRepo> = repos.world.clone();
        let flag_repo: Arc<dyn FlagRepo> = repos.flag.clone();
        let lore_repo: Arc<dyn LoreRepo> = repos.lore.clone();
        let narrative_repo: Arc<dyn NarrativeRepo> = repos.narrative.clone();

        // Wrapper types that add business logic beyond delegation
        let record_visit = Arc::new(use_cases::observation::RecordVisit::new(
            repos.observation.clone(),
            repos.location.clone(),
            clock_port.clone(),
        ));
        let narrative = Arc::new(use_cases::NarrativeOps::new(
            repos.narrative.clone(),
            repos.location.clone(),
            repos.world.clone(),
            repos.player_character.clone(),
            repos.character.clone(),
            repos.observation.clone(),
            repos.challenge.clone(),
            repos.flag.clone(),
            repos.scene.clone(),
            clock_port.clone(),
        ));
        let assets = Arc::new(repositories::AssetsRepository::new(
            repos.asset.clone(),
            image_gen,
        ));

        let repositories_container = Repositories {
            // Port traits injected directly
            character: character_repo.clone(),
            player_character: player_character_repo.clone(),
            location: location_repo.clone(),
            scene: scene_repo.clone(),
            act: act_repo.clone(),
            content: content_repo.clone(),
            interaction: interaction_repo.clone(),
            challenge: challenge_repo.clone(),
            observation: observation_repo.clone(),
            item: item_repo.clone(),
            goal: goal_repo.clone(),
            location_state: location_state_repo.clone(),
            region_state: region_state_repo.clone(),
            staging: staging_repo.clone(),
            world: world_repo.clone(),
            flag: flag_repo.clone(),
            lore: lore_repo.clone(),
            narrative_repo: narrative_repo.clone(),
            // Wrapper types
            narrative: narrative.clone(),
            assets: assets.clone(),
        };

        // Create time use case first (needed by movement)
        let suggest_time = Arc::new(use_cases::time::SuggestTime::new(
            repos.world.clone(),
            clock_port.clone(),
        ));

        // Create scene resolution use case (needed by movement)
        let resolve_scene = Arc::new(use_cases::scene::ResolveScene::new(repos.scene.clone()));

        // Create use cases
        // Movement use cases inject port traits directly (ADR-009)
        let movement = use_cases::MovementUseCases::new(
            Arc::new(use_cases::movement::EnterRegion::new(
                repos.player_character.clone(),
                repos.location.clone(),
                repos.staging.clone(),
                repos.observation.clone(),
                record_visit.clone(),
                narrative.clone(),
                resolve_scene.clone(),
                repos.scene.clone(),
                repos.flag.clone(),
                repos.world.clone(),
                suggest_time.clone(),
            )),
            Arc::new(use_cases::movement::ExitLocation::new(
                repos.player_character.clone(),
                repos.location.clone(),
                repos.staging.clone(),
                repos.observation.clone(),
                record_visit.clone(),
                narrative.clone(),
                resolve_scene.clone(),
                repos.scene.clone(),
                repos.flag.clone(),
                repos.world.clone(),
                suggest_time.clone(),
            )),
        );

        let scene_change =
            use_cases::SceneChangeBuilder::new(repos.location.clone(), repos.item.clone());

        let conversation_start = Arc::new(use_cases::conversation::StartConversation::new(
            repos.character.clone(),
            repos.player_character.clone(),
            repos.staging.clone(),
            repos.scene.clone(),
            repos.world.clone(),
            queue_port.clone(),
            clock_port.clone(),
        ));
        let conversation_continue = Arc::new(use_cases::conversation::ContinueConversation::new(
            repos.character.clone(),
            repos.player_character.clone(),
            repos.staging.clone(),
            repos.world.clone(),
            narrative.clone(),
            queue_port.clone(),
            clock_port.clone(),
        ));
        let conversation_end = Arc::new(use_cases::conversation::EndConversation::new(
            repos.character.clone(),
            repos.player_character.clone(),
            narrative.clone(),
        ));
        let conversation = use_cases::ConversationUseCases::new(
            conversation_start.clone(),
            conversation_continue,
            conversation_end,
        );

        let player_action = use_cases::PlayerActionUseCases::new(Arc::new(
            use_cases::player_action::HandlePlayerAction::new(
                conversation_start,
                queue_port.clone(),
                clock_port.clone(),
            ),
        ));

        let actantial = use_cases::ActantialUseCases::new(
            use_cases::actantial::GoalOps::new(repos.goal.clone()),
            use_cases::actantial::WantOps::new(repos.character.clone(), clock_port.clone()),
            use_cases::actantial::ActantialContextOps::new(repos.character.clone()),
        );

        let ai = use_cases::AiUseCases::new(Arc::new(use_cases::ai::SuggestionOps::new(
            queue_port.clone(),
            repos.world.clone(),
            repos.character.clone(),
        )));

        let resolve_outcome = Arc::new(use_cases::challenge::ResolveOutcome::new(
            repos.challenge.clone(),
            repos.item.clone(),
            repos.player_character.clone(),
            repos.observation.clone(),
            repos.scene.clone(),
        ));
        let outcome_decision = Arc::new(use_cases::challenge::OutcomeDecision::new(
            queue_port.clone(),
            resolve_outcome.clone(),
        ));

        let challenge_uc = use_cases::ChallengeUseCases::new(
            Arc::new(use_cases::challenge::RollChallenge::new(
                repos.challenge.clone(),
                repos.player_character.clone(),
                queue_port.clone(),
                random_port.clone(),
                clock_port.clone(),
            )),
            resolve_outcome,
            Arc::new(use_cases::challenge::TriggerChallengePrompt::new(
                repos.challenge.clone(),
            )),
            outcome_decision,
            Arc::new(use_cases::challenge::ChallengeOps::new(
                repos.challenge.clone(),
            )),
        );

        let approve_suggestion = Arc::new(use_cases::approval::ApproveSuggestion::new(
            queue_port.clone(),
        ));
        let tool_executor = Arc::new(use_cases::approval::tool_executor::ToolExecutor::new(
            repos.item.clone(),
            repos.player_character.clone(),
            repos.character.clone(),
        ));
        let approval = use_cases::ApprovalUseCases::new(
            Arc::new(use_cases::approval::ApproveStaging::new(
                repos.staging.clone(),
            )),
            approve_suggestion.clone(),
            Arc::new(use_cases::approval::ApprovalDecisionFlow::new(
                approve_suggestion.clone(),
                narrative.clone(),
                queue_port.clone(),
                tool_executor,
            )),
        );

        let generate_asset = Arc::new(use_cases::assets::GenerateAsset::new(
            assets.clone(),
            queue_port.clone(),
            clock_port.clone(),
        ));
        let expression_sheet = Arc::new(use_cases::assets::GenerateExpressionSheet::new(
            assets.clone(),
            repos.character.clone(),
            queue_port.clone(),
            clock_port.clone(),
        ));
        let assets_uc = use_cases::AssetUseCases::new(generate_asset, expression_sheet);

        let world_uc = use_cases::WorldUseCases::new(
            Arc::new(use_cases::world::ExportWorld::new(
                repos.world.clone(),
                repos.location.clone(),
                repos.character.clone(),
                repos.item.clone(),
                narrative.clone(),
            )),
            Arc::new(use_cases::world::ImportWorld::new(
                repos.world.clone(),
                repos.location.clone(),
                repos.character.clone(),
                repos.item.clone(),
                narrative.clone(),
            )),
        );

        let queues = use_cases::QueueUseCases::new(
            Arc::new(use_cases::queues::ProcessPlayerAction::new(
                queue_port.clone(),
                repos.character.clone(),
                repos.player_character.clone(),
                repos.staging.clone(),
                repos.scene.clone(),
                repos.world.clone(),
                narrative.clone(),
                repos.location.clone(),
                repos.challenge.clone(),
            )),
            Arc::new(use_cases::queues::ProcessLlmRequest::new(
                queue_port.clone(),
                llm_port.clone(),
                repos.challenge.clone(),
                repos.narrative.clone(),
            )),
        );

        let execute_effects = Arc::new(use_cases::narrative::ExecuteEffects::new(
            repos.item.clone(),
            repos.player_character.clone(),
            repos.challenge.clone(),
            narrative.clone(),
            repos.character.clone(),
            repos.observation.clone(),
            repos.scene.clone(),
            repos.flag.clone(),
            repos.world.clone(),
            clock_port.clone(),
        ));
        let narrative_events = Arc::new(use_cases::narrative::NarrativeEventOps::new(
            narrative.clone(),
            execute_effects.clone(),
            clock_port.clone(),
        ));
        let narrative_chains =
            Arc::new(use_cases::narrative::EventChainOps::new(narrative.clone()));
        let narrative_decision = Arc::new(use_cases::narrative::NarrativeDecisionFlow::new(
            approve_suggestion.clone(),
            queue_port.clone(),
            narrative.clone(),
            execute_effects.clone(),
        ));
        let narrative_uc = use_cases::NarrativeUseCases::new(
            execute_effects,
            narrative_events,
            narrative_chains,
            narrative_decision,
        );

        let time_control = Arc::new(use_cases::time::TimeControl::new(
            repos.world.clone(),
            clock_port.clone(),
        ));
        let time_suggestions =
            Arc::new(use_cases::time::TimeSuggestions::new(time_control.clone()));
        let time_uc = use_cases::TimeUseCases::new(suggest_time, time_control, time_suggestions);

        let visual_state_uc = use_cases::VisualStateUseCases::new(Arc::new(
            use_cases::visual_state::ResolveVisualState::new(
                repos.location_state.clone(),
                repos.region_state.clone(),
            ),
        ));

        let settings_entity =
            Arc::new(repositories::SettingsRepository::new(settings_repo.clone()));

        let staging_uc = use_cases::StagingUseCases::new(
            Arc::new(use_cases::staging::RequestStagingApproval::new(
                repos.character.clone(),
                repos.staging.clone(),
                repos.location.clone(),
                repos.world.clone(),
                repos.flag.clone(),
                visual_state_uc.resolve.clone(),
                settings_repo.clone(),
                llm.clone(),
                clock_port.clone(),
            )),
            Arc::new(use_cases::staging::RegenerateStagingSuggestions::new(
                repos.location.clone(),
                repos.character.clone(),
                llm.clone(),
            )),
            Arc::new(use_cases::staging::ApproveStagingRequest::new(
                repos.staging.clone(),
                repos.world.clone(),
                repos.character.clone(),
                repos.location.clone(),
                repos.location_state.clone(),
                repos.region_state.clone(),
                clock_port.clone(),
            )),
            Arc::new(use_cases::staging::AutoApproveStagingTimeout::new(
                repos.character.clone(),
                repos.staging.clone(),
                repos.world.clone(),
                repos.location.clone(),
                repos.location_state.clone(),
                repos.region_state.clone(),
                settings_repo.clone(),
                clock_port.clone(),
            )),
        );

        let npc_uc = use_cases::NpcUseCases::new(
            Arc::new(use_cases::npc::NpcDisposition::new(
                repos.character.clone(),
                clock_port.clone(),
            )),
            Arc::new(use_cases::npc::NpcMood::new(
                repos.staging.clone(),
                repos.character.clone(),
            )),
            Arc::new(use_cases::npc::NpcRegionRelationships::new(
                repos.character.clone(),
            )),
            Arc::new(use_cases::npc::NpcLocationSharing::new(
                repos.character.clone(),
                repos.location.clone(),
                repos.observation.clone(),
                clock_port.clone(),
            )),
            Arc::new(use_cases::npc::NpcApproachEvents::new(
                repos.character.clone(),
            )),
        );

        let story_events_uc = use_cases::StoryEventUseCases::new(Arc::new(
            use_cases::story_events::StoryEventOps::new(narrative.clone()),
        ));

        let lore_uc = use_cases::LoreUseCases::new(Arc::new(use_cases::lore::LoreOps::new(
            repos.lore.clone(),
            clock_port.clone(),
        )));

        let location_events_uc = use_cases::LocationEventUseCases::new(Arc::new(
            use_cases::location_events::TriggerLocationEvent::new(repos.location.clone()),
        ));

        // Create custom condition evaluator for LLM-based condition/trigger evaluation
        let custom_condition = Arc::new(use_cases::CustomConditionEvaluator::new(llm_port.clone()));

        let management = use_cases::ManagementUseCases::new(
            use_cases::management::WorldManagement::new(repos.world.clone(), clock_port.clone()),
            use_cases::management::CharacterManagement::new(
                repos.character.clone(),
                clock_port.clone(),
            ),
            use_cases::management::LocationManagement::new(repos.location.clone()),
            use_cases::management::PlayerCharacterManagement::new(
                repos.player_character.clone(),
                repos.location.clone(),
                clock_port.clone(),
            ),
            use_cases::management::RelationshipManagement::new(
                repos.character.clone(),
                clock_port.clone(),
            ),
            use_cases::management::ObservationManagement::new(
                repos.observation.clone(),
                repos.player_character.clone(),
                repos.character.clone(),
                repos.location.clone(),
                repos.world.clone(),
                clock_port.clone(),
            ),
            use_cases::management::ActManagement::new(repos.act.clone()),
            use_cases::management::SceneManagement::new(repos.scene.clone()),
            use_cases::management::InteractionManagement::new(repos.interaction.clone()),
            use_cases::management::SkillManagement::new(repos.content.clone()),
        );

        let settings = settings_entity;

        let join_world = Arc::new(use_cases::session::JoinWorld::new(
            repos.world.clone(),
            repos.location.clone(),
            repos.character.clone(),
            repos.scene.clone(),
            repos.player_character.clone(),
        ));
        let join_world_flow = Arc::new(use_cases::session::JoinWorldFlow::new(join_world.clone()));
        let directorial_update = Arc::new(use_cases::session::DirectorialUpdate::new());
        let session =
            use_cases::SessionUseCases::new(join_world, join_world_flow, directorial_update);

        let inventory =
            use_cases::InventoryUseCases::new(repos.item.clone(), repos.player_character.clone());

        let character_sheet =
            use_cases::CharacterSheetUseCases::new(repos.character.clone(), repos.world.clone());

        let use_cases = UseCases {
            movement,
            conversation,
            challenge: challenge_uc,
            approval,
            actantial,
            ai,
            assets: assets_uc,
            scene_change,
            world: world_uc,
            queues,
            narrative: narrative_uc,
            player_action,
            time: time_uc,
            visual_state: visual_state_uc,
            management,
            session,
            settings,
            staging: staging_uc,
            npc: npc_uc,
            story_events: story_events_uc,
            lore: lore_uc,
            location_events: location_events_uc,
            custom_condition,
            inventory,
            character_sheet,
        };

        // Create content service for game content (races, classes, spells, etc.)
        let content = Arc::new(ContentService::new(content_config));

        // Register D&D 5e provider if fivetools path is configured
        if let Some(ref path) = content.config().fivetools_path {
            content.register_dnd5e_provider(path);
        }

        Self {
            repositories: repositories_container,
            use_cases,
            queue,
            llm,
            content,
        }
    }
}
