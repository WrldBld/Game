//! Application state and composition.

use std::sync::Arc;

use crate::entities;
use crate::infrastructure::{
    clock::{SystemClock, SystemRandom},
    neo4j::Neo4jRepositories,
    ports::{ClockPort, ImageGenPort, LlmPort, QueuePort, RandomPort, SettingsRepo},
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
    pub act: Arc<entities::Act>,
    pub skill: Arc<entities::Skill>,
    pub interaction: Arc<entities::Interaction>,
    pub challenge: Arc<entities::Challenge>,
    pub narrative: Arc<entities::Narrative>,
    pub staging: Arc<entities::Staging>,
    pub observation: Arc<entities::Observation>,
    pub inventory: Arc<entities::Inventory>,
    pub assets: Arc<entities::Assets>,
    pub world: Arc<entities::World>,
    pub flag: Arc<entities::Flag>,
    pub goal: Arc<entities::Goal>,
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
    pub settings: use_cases::SettingsUseCases,
    pub staging: use_cases::StagingUseCases,
    pub npc: use_cases::NpcUseCases,
    pub inventory: use_cases::InventoryUseCases,
    pub story_events: use_cases::StoryEventUseCases,
    pub lore: use_cases::LoreUseCases,
    pub location_events: use_cases::LocationEventUseCases,
}

impl App {
    /// Create a new App with all dependencies wired up.
    pub fn new(
        repos: Neo4jRepositories,
        llm: Arc<dyn LlmPort>,
        image_gen: Arc<dyn ImageGenPort>,
        queue: Arc<SqliteQueue>,
        settings_repo: Arc<dyn SettingsRepo>,
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
        let act = Arc::new(entities::Act::new(repos.act.clone()));
        let skill = Arc::new(entities::Skill::new(repos.skill.clone()));
        let interaction = Arc::new(entities::Interaction::new(repos.interaction.clone()));
        let challenge = Arc::new(entities::Challenge::new(repos.challenge.clone()));
        let narrative = Arc::new(entities::Narrative::new(
            repos.narrative.clone(),
            repos.location.clone(),
            repos.world.clone(),
            repos.player_character.clone(),
            repos.observation.clone(),
            repos.challenge.clone(),
            repos.flag.clone(),
            repos.scene.clone(),
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
        let world = Arc::new(entities::World::new(repos.world.clone(), clock.clone()));
        let flag = Arc::new(entities::Flag::new(repos.flag.clone()));
        let goal = Arc::new(entities::Goal::new(repos.goal.clone()));
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
            act: act.clone(),
            skill: skill.clone(),
            interaction: interaction.clone(),
            challenge: challenge.clone(),
            narrative: narrative.clone(),
            staging: staging.clone(),
            observation: observation.clone(),
            inventory: inventory.clone(),
            assets: assets.clone(),
            world: world.clone(),
            flag: flag.clone(),
            goal: goal.clone(),
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
            )),
        );

        let scene_change = use_cases::SceneChangeBuilder::new(location.clone(), inventory.clone());

        let conversation_start = Arc::new(use_cases::conversation::StartConversation::new(
            character.clone(),
            player_character.clone(),
            staging.clone(),
            scene.clone(),
            world.clone(),
            queue_port.clone(),
            clock.clone(),
        ));
        let conversation_continue = Arc::new(use_cases::conversation::ContinueConversation::new(
            character.clone(),
            player_character.clone(),
            staging.clone(),
            world.clone(),
            narrative.clone(),
            queue_port.clone(),
            clock.clone(),
        ));
        let conversation_end = Arc::new(use_cases::conversation::EndConversation::new(
            character.clone(),
            player_character.clone(),
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
                clock.clone(),
            ),
        ));

        let actantial = use_cases::ActantialUseCases::new(
            use_cases::actantial::GoalOps::new(goal.clone()),
            use_cases::actantial::WantOps::new(character.clone(), clock.clone()),
            use_cases::actantial::ActantialContextOps::new(character.clone()),
        );

        let ai = use_cases::AiUseCases::new(Arc::new(use_cases::ai::SuggestionOps::new(
            queue_port.clone(),
            world.clone(),
            character.clone(),
        )));

        let resolve_outcome = Arc::new(use_cases::challenge::ResolveOutcome::new(
            challenge.clone(),
            inventory.clone(),
            observation.clone(),
            scene.clone(),
            player_character.clone(),
        ));
        let outcome_decision = Arc::new(use_cases::challenge::OutcomeDecision::new(
            queue_port.clone(),
            resolve_outcome.clone(),
        ));

        let challenge_uc = use_cases::ChallengeUseCases::new(
            Arc::new(use_cases::challenge::RollChallenge::new(
                challenge.clone(),
                player_character.clone(),
                queue_port.clone(),
                random.clone(),
                clock.clone(),
            )),
            resolve_outcome,
            Arc::new(use_cases::challenge::TriggerChallengePrompt::new(
                challenge.clone(),
            )),
            outcome_decision,
            Arc::new(use_cases::challenge::ChallengeOps::new(
                challenge.clone(),
            )),
        );

        let approve_suggestion =
            Arc::new(use_cases::approval::ApproveSuggestion::new(queue_port.clone()));
        let approval = use_cases::ApprovalUseCases::new(
            Arc::new(use_cases::approval::ApproveStaging::new(staging.clone())),
            approve_suggestion.clone(),
            Arc::new(use_cases::approval::ApprovalDecisionFlow::new(
                approve_suggestion.clone(),
                narrative.clone(),
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
                scene.clone(),
                world.clone(),
                narrative.clone(),
            )),
            Arc::new(use_cases::queues::ProcessLlmRequest::new(
                queue_port.clone(),
                llm.clone(),
            )),
        );

        let execute_effects = Arc::new(use_cases::narrative::ExecuteEffects::new(
            inventory.clone(),
            challenge.clone(),
            narrative.clone(),
            character.clone(),
            observation.clone(),
            player_character.clone(),
            scene.clone(),
            flag.clone(),
            clock.clone(),
        ));
        let narrative_events = Arc::new(use_cases::narrative::NarrativeEventOps::new(
            narrative.clone(),
            execute_effects.clone(),
        ));
        let narrative_chains = Arc::new(use_cases::narrative::EventChainOps::new(narrative.clone()));
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

        let time_control = Arc::new(use_cases::time::TimeControl::new(world.clone()));
        let time_suggestions = Arc::new(use_cases::time::TimeSuggestions::new(time_control.clone()));
        let time_uc = use_cases::TimeUseCases::new(suggest_time, time_control, time_suggestions);

        let visual_state_uc = use_cases::VisualStateUseCases::new(Arc::new(
            use_cases::visual_state::ResolveVisualState::new(
                location_state.clone(),
                region_state.clone(),
                flag.clone(),
            ),
        ));

        let staging_uc = use_cases::StagingUseCases::new(
            Arc::new(use_cases::staging::RequestStagingApproval::new(
                character.clone(),
                staging.clone(),
                location.clone(),
                world.clone(),
                flag.clone(),
                visual_state_uc.resolve.clone(),
                settings_repo.clone(),
                llm.clone(),
            )),
            Arc::new(use_cases::staging::RegenerateStagingSuggestions::new(
                location.clone(),
                character.clone(),
                llm.clone(),
            )),
            Arc::new(use_cases::staging::ApproveStagingRequest::new(
                staging.clone(),
                world.clone(),
                character.clone(),
                location.clone(),
                location_state.clone(),
                region_state.clone(),
            )),
            Arc::new(use_cases::staging::AutoApproveStagingTimeout::new(
                character.clone(),
                staging.clone(),
                world.clone(),
                location.clone(),
                location_state.clone(),
                region_state.clone(),
                settings_repo.clone(),
            )),
        );

        // Create settings ops for SettingsUseCases
        let settings_ops = Arc::new(use_cases::settings::SettingsOps::new(settings_repo.clone()));

        let npc_uc = use_cases::NpcUseCases::new(
            Arc::new(use_cases::npc::NpcDisposition::new(
                character.clone(),
                clock.clone(),
            )),
            Arc::new(use_cases::npc::NpcMood::new(
                staging.clone(),
                character.clone(),
            )),
            Arc::new(use_cases::npc::NpcRegionRelationships::new(
                character.clone(),
            )),
            Arc::new(use_cases::npc::NpcLocationSharing::new(
                character.clone(),
                location.clone(),
                observation.clone(),
                clock.clone(),
            )),
            Arc::new(use_cases::npc::NpcApproachEvents::new(character.clone())),
        );

        let inventory_ops = Arc::new(use_cases::inventory::InventoryOps::new(inventory.clone()));
        let inventory_actions =
            Arc::new(use_cases::inventory::InventoryActions::new(inventory.clone()));
        let inventory_uc = use_cases::InventoryUseCases::new(inventory_ops, inventory_actions);

        let story_events_uc = use_cases::StoryEventUseCases::new(Arc::new(
            use_cases::story_events::StoryEventOps::new(narrative.clone()),
        ));

        let lore_uc =
            use_cases::LoreUseCases::new(Arc::new(use_cases::lore::LoreOps::new(lore.clone())));

        let location_events_uc = use_cases::LocationEventUseCases::new(Arc::new(
            use_cases::location_events::TriggerLocationEvent::new(location.clone()),
        ));

        let management = use_cases::ManagementUseCases::new(
            use_cases::management::WorldCrud::new(world.clone(), clock.clone()),
            use_cases::management::CharacterCrud::new(character.clone(), clock.clone()),
            use_cases::management::LocationCrud::new(location.clone()),
            use_cases::management::PlayerCharacterCrud::new(
                player_character.clone(),
                location.clone(),
                clock.clone(),
            ),
            use_cases::management::RelationshipCrud::new(character.clone(), clock.clone()),
            use_cases::management::ObservationCrud::new(
                observation.clone(),
                player_character.clone(),
                character.clone(),
                location.clone(),
                world.clone(),
                clock.clone(),
            ),
            use_cases::management::ActCrud::new(act.clone()),
            use_cases::management::SceneCrud::new(scene.clone()),
            use_cases::management::InteractionCrud::new(interaction.clone()),
            use_cases::management::SkillCrud::new(skill.clone()),
        );

        let settings = use_cases::SettingsUseCases::new(settings_ops);

        let join_world = Arc::new(use_cases::session::JoinWorld::new(
            world.clone(),
            location.clone(),
            character.clone(),
            scene.clone(),
            player_character.clone(),
        ));
        let join_world_flow =
            Arc::new(use_cases::session::JoinWorldFlow::new(join_world.clone()));
        let directorial_update = Arc::new(use_cases::session::DirectorialUpdate::new());
        let session =
            use_cases::SessionUseCases::new(join_world, join_world_flow, directorial_update);

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
            inventory: inventory_uc,
            story_events: story_events_uc,
            lore: lore_uc,
            location_events: location_events_uc,
        };

        Self {
            entities,
            use_cases,
            queue: queue,
            llm,
        }
    }
}
