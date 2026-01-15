//! Character-related domain events
//!
//! These enums communicate what happened when character state was modified,
//! allowing callers to react appropriately.

use crate::value_objects::{CampbellArchetype, CharacterName, CharacterState, Description};

/// Outcome of applying damage to a character
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DamageOutcome {
    /// Character was already dead, no effect
    AlreadyDead,
    /// Character took damage but survived
    Wounded { damage_dealt: i32, remaining_hp: i32 },
    /// Character was killed by this damage
    Killed { damage_dealt: i32 },
    /// No HP tracking on this character
    NoHpTracking,
}

/// Outcome of healing a character
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealOutcome {
    /// Character is dead, cannot heal
    Dead,
    /// Healing applied
    Healed { amount_healed: i32, new_hp: i32 },
    /// Already at max HP
    AlreadyFull,
    /// No HP tracking on this character
    NoHpTracking,
}

/// An archetype transformation that occurred
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchetypeShift {
    pub from: CampbellArchetype,
    pub to: CampbellArchetype,
    pub reason: String,
}

/// Outcome of updating character metadata fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterUpdate {
    NameChanged { from: CharacterName, to: CharacterName },
    DescriptionChanged { from: Description, to: Description },
    SpriteChanged {
        from: Option<String>,
        to: Option<String>,
    },
    PortraitChanged {
        from: Option<String>,
        to: Option<String>,
    },
}

/// Outcome of toggling character active/inactive state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterStateChange {
    StateChanged { from: CharacterState, to: CharacterState },
    Unchanged { state: CharacterState },
}

/// Outcome of attempting to resurrect a character
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResurrectOutcome {
    /// Character was not dead
    NotDead,
    /// Character was resurrected
    Resurrected { hp_restored_to: i32 },
}
