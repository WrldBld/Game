//! Use cases - User story orchestration.
//!
//! Each module contains use cases for a specific domain area.
//! Use cases orchestrate across entity modules to fulfill user stories.

pub mod movement;
pub mod conversation;
pub mod challenge;
pub mod approval;
pub mod assets;
pub mod world;
pub mod queues;
pub mod narrative;
pub mod time;
pub mod visual_state;
pub mod management;
pub mod session;
pub mod staging;
pub mod npc;
pub mod inventory;
pub mod story_events;
pub mod lore;

// Re-export main types
pub use movement::MovementUseCases;
pub use conversation::ConversationUseCases;
pub use challenge::ChallengeUseCases;
pub use approval::ApprovalUseCases;
pub use assets::AssetUseCases;
pub use world::WorldUseCases;
pub use queues::QueueUseCases;
pub use narrative::NarrativeUseCases;
pub use time::TimeUseCases;
pub use visual_state::VisualStateUseCases;
pub use management::ManagementUseCases;
pub use session::SessionUseCases;
pub use staging::StagingUseCases;
pub use npc::NpcUseCases;
pub use inventory::InventoryUseCases;
pub use story_events::StoryEventUseCases;
pub use lore::LoreUseCases;
