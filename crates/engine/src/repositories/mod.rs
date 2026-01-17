//! Repository modules - Data access wrappers around port traits.
//!
//! Each repository wraps a port trait and provides the interface
//! for use cases to access persisted aggregates.
//!
//! Naming convention:
//! - `*Repository` - Data access wrappers for domain entities
//! - `*Service` - Wrappers for infrastructure services (LLM, queue, clock, random)
//! - `*Store` - In-memory state storage (session, pending staging, etc.)

pub mod act;
pub mod assets;
pub mod challenge;
pub mod character;
pub mod clock;
pub mod content;
pub mod directorial;
pub mod flag;
pub mod goal;
pub mod interaction;
pub mod inventory;
pub mod llm;
pub mod location;
pub mod location_state;
pub mod lore;
pub mod narrative;
pub mod observation;
pub mod pending_staging;
pub mod player_character;
pub mod queue;
pub mod random;
pub mod region_state;
pub mod scene;
pub mod session;
pub mod settings;
pub mod staging;
pub mod time_suggestion;
pub mod world;

#[cfg(test)]
mod narrative_integration_tests;

// Repositories (entity data access)
pub use act::ActRepository;
pub use assets::AssetsRepository;
pub use challenge::ChallengeRepository;
pub use character::CharacterRepository;
pub use content::ContentRepository;
pub use flag::FlagRepository;
pub use goal::GoalRepository;
pub use interaction::InteractionRepository;
pub use inventory::InventoryActionResult;
pub use inventory::InventoryError;
pub use inventory::InventoryRepository;
pub use location::Location;
pub use location::RegionExit;
pub use location::RegionExitsResult;
pub use location::SkippedExit;
pub use location_state::LocationStateRepository;
pub use lore::LoreRepository;
pub use narrative::NarrativeRepository;
pub use observation::ObservationRepository;
pub use player_character::PlayerCharacterRepository;
pub use region_state::RegionStateRepository;
pub use scene::SceneConsideration;
pub use scene::SceneRepository;
pub use scene::SceneResolutionContext;
pub use scene::SceneResolutionResult;
pub use settings::SettingsError;
pub use settings::SettingsRepository;
pub use staging::StagingRepository;
pub use world::WorldError;
pub use world::WorldRepository;

// Services (infrastructure wrappers)
pub use clock::ClockService;
pub use llm::LlmService;
pub use queue::QueueService;
pub use random::RandomService;

// Stores (in-memory state)
pub use directorial::DirectorialContextStore;
pub use pending_staging::PendingStaging;
pub use session::WorldSession;
pub use time_suggestion::TimeSuggestionStore;
