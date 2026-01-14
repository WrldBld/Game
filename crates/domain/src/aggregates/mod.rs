//! Aggregate roots - domain objects that own their related data
//!
//! Each aggregate:
//! - Has a unique identity
//! - Owns all its constituent parts (enforced by Rust ownership)
//! - Exposes behavior through methods, not public fields
//! - Returns domain events from mutations
//!
//! # Rustic DDD Principles
//!
//! Instead of porting Java/C# DDD patterns, we leverage Rust's strengths:
//!
//! | Java DDD Pattern | Rustic Equivalent |
//! |------------------|-------------------|
//! | Private fields + getters | Newtypes valid by construction |
//! | Aggregate root guards | Ownership (borrow checker enforces) |
//! | Value Object immutability | `#[derive(Clone)]` + no `&mut` methods |
//! | Factory pattern | `::new()` + builder pattern |
//! | Domain Events | Return enums from mutations |

pub mod character;
pub mod location;
pub mod narrative_event;
pub mod player_character;
pub mod scene;
pub mod world;

pub use character::{Character, StatBlock, StatModifier, StatValue};
pub use location::Location;
pub use narrative_event::NarrativeEvent;
pub use player_character::{PlayerCharacter, PlayerCharacterStateChange};
pub use scene::Scene;
pub use world::World;
