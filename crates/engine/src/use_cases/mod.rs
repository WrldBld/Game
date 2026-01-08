//! Use cases - User story orchestration.
//!
//! Each module contains use cases for a specific domain area.
//! Use cases orchestrate across entity modules to fulfill user stories.

pub mod approval;
pub mod actantial;
pub mod assets;
pub mod challenge;
pub mod conversation;
pub mod inventory;
pub mod lore;
pub mod management;
pub mod movement;
pub mod narrative;
pub mod npc;
pub mod queues;
pub mod session;
pub mod staging;
pub mod story_events;
pub mod time;
pub mod visual_state;
pub mod world;

// Re-export main types
pub use approval::ApprovalUseCases;
pub use actantial::ActantialUseCases;
pub use assets::AssetUseCases;
pub use challenge::ChallengeUseCases;
pub use conversation::ConversationUseCases;
pub use inventory::InventoryUseCases;
pub use lore::LoreUseCases;
pub use management::ManagementUseCases;
pub use movement::MovementUseCases;
pub use narrative::NarrativeUseCases;
pub use npc::NpcUseCases;
pub use queues::QueueUseCases;
pub use session::SessionUseCases;
pub use staging::StagingUseCases;
pub use story_events::StoryEventUseCases;
pub use time::TimeUseCases;
pub use visual_state::VisualStateUseCases;
pub use world::WorldUseCases;
