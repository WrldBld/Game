//! Use cases - User story orchestration.
//!
//! Each module contains use cases for a specific domain area.
//! Use cases orchestrate across entity modules to fulfill user stories.

pub mod actantial;
pub mod ai;
pub mod approval;
pub mod assets;
pub mod challenge;
pub mod content;
pub mod conversation;
pub mod custom_condition;
pub mod inventory;
pub mod location_events;
pub mod lore;
pub mod management;
pub mod movement;
pub mod narrative;
pub mod narrative_operations;
pub mod npc;
pub mod observation;
pub mod player_action;
pub mod queues;
pub mod scene;
pub mod session;
pub mod settings;
pub mod staging;
pub mod story_events;
pub mod time;
pub mod validation;
pub mod visual_state;
pub mod world;

// Re-export main types
// Note: Location and Scene wrapper types were removed per ADR-009.
// Use the port traits directly: LocationRepo, SceneRepo from infrastructure::ports.
pub use actantial::ActantialUseCases;
pub use ai::AiUseCases;
pub use approval::ApprovalUseCases;
pub use assets::AssetUseCases;
pub use challenge::ChallengeUseCases;
pub use conversation::ConversationUseCases;
pub use custom_condition::CustomConditionEvaluator;
pub use location_events::LocationEventUseCases;
pub use lore::LoreUseCases;
pub use management::ManagementUseCases;
pub use movement::MovementUseCases;
pub use movement::SceneChangeBuilder;
pub use narrative::NarrativeUseCases;
pub use narrative_operations::Narrative;
pub use npc::NpcUseCases;
pub use player_action::PlayerActionUseCases;
pub use queues::QueueUseCases;

pub use session::SessionUseCases;
pub use staging::StagingUseCases;
pub use story_events::StoryEventUseCases;
pub use time::TimeUseCases;
pub use visual_state::VisualStateUseCases;
pub use world::WorldUseCases;
