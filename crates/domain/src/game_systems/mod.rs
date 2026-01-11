//! Game system implementations for various TTRPGs.
//!
//! This module provides system-specific calculation engines and mechanics
//! for different tabletop roleplaying games. Each system implements the
//! core traits defined in `traits.rs`.
//!
//! # Supported Systems
//!
//! - D&D 5th Edition (`dnd5e`)
//! - Pathfinder 2e (`pf2e`)
//! - Call of Cthulhu 7e (`coc7e`)
//! - FATE Core (`fate_core`)
//! - Blades in the Dark (`blades`)
//! - Powered by the Apocalypse (`pbta`, `pbta_aw`, `pbta_dw`, `pbta_motw`)

mod blades;
mod coc7e;
mod dnd5e;
mod fate_core;
mod pbta;
mod pf2e;
mod traits;

// D&D 5e exports
pub use dnd5e::{skill_ability as dnd5e_skill_ability, Dnd5eSystem};

// Pathfinder 2e exports
pub use pf2e::{
    determine_success as pf2e_determine_success, multiple_attack_penalty, skill_ability as pf2e_skill_ability,
    DegreeOfSuccess, Pf2eProficiencyRank, Pf2eSystem,
};

// Call of Cthulhu 7e exports
pub use coc7e::{
    check_success as coc_check_success, get_skill_base as coc_skill_base, is_critical, is_fumble,
    sanity_check, Coc7eSystem, Lifestyle, SanityCheckResult, SuccessLevel,
};

// FATE Core exports
pub use fate_core::{
    roll_4df, ConsequenceSeverity, FateAction, FateCoreSystem, FateOutcome, InvokeType,
    LadderRating,
};

// Blades in the Dark exports
pub use blades::{
    BladesOutcome, BladesSystem, CrewType, EffectLevel, HarmLevel, LoadLevel, Playbook, Position,
    ProgressClock, TraumaCondition,
};

// Powered by the Apocalypse exports
pub use pbta::{
    HarmSystem as PbtaHarmSystem, ModifierType, MoveHold, PbtaModifier, PbtaMove, PbtaOutcome,
    PbtaStatSet, PbtaSystem, PbtaVariant,
};

// Core traits
pub use traits::{
    CalculationEngine, CasterType, CharacterSheetProvider, CompendiumProvider, ContentError,
    FilterField, FilterFieldType, FilterSchema, GameSystem, ProficiencyLevel, RestType,
    SpellcastingSystem,
};


use std::sync::Arc;

/// Registry of available game systems.
pub struct GameSystemRegistry {
    systems: Vec<Arc<dyn GameSystem>>,
}

impl Default for GameSystemRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GameSystemRegistry {
    /// Create a new registry with all built-in game systems.
    pub fn new() -> Self {
        let mut registry = Self {
            systems: Vec::new(),
        };
        // Register built-in systems
        registry.register(Arc::new(Dnd5eSystem::new()));
        registry.register(Arc::new(Pf2eSystem::new()));
        registry.register(Arc::new(Coc7eSystem::new()));
        registry.register(Arc::new(FateCoreSystem::new()));
        registry.register(Arc::new(BladesSystem::new()));
        registry.register(Arc::new(PbtaSystem::generic()));
        registry.register(Arc::new(PbtaSystem::apocalypse_world()));
        registry.register(Arc::new(PbtaSystem::dungeon_world()));
        registry.register(Arc::new(PbtaSystem::monster_of_the_week()));
        registry
    }

    /// Create an empty registry without built-in systems.
    pub fn empty() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    /// Register a game system.
    pub fn register(&mut self, system: Arc<dyn GameSystem>) {
        self.systems.push(system);
    }

    /// Get a game system by its ID.
    pub fn get(&self, system_id: &str) -> Option<Arc<dyn GameSystem>> {
        self.systems
            .iter()
            .find(|s| s.system_id() == system_id)
            .cloned()
    }

    /// List all registered system IDs.
    pub fn list_systems(&self) -> Vec<&str> {
        self.systems.iter().map(|s| s.system_id()).collect()
    }

    /// List all registered systems with their display names.
    pub fn list_systems_with_names(&self) -> Vec<(&str, &str)> {
        self.systems
            .iter()
            .map(|s| (s.system_id(), s.display_name()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_includes_dnd5e() {
        let registry = GameSystemRegistry::new();
        assert!(registry.list_systems().contains(&"dnd5e"));
        assert!(registry.get("dnd5e").is_some());
    }

    #[test]
    fn registry_includes_all_systems() {
        let registry = GameSystemRegistry::new();
        let systems = registry.list_systems();

        // Check all systems are registered
        assert!(systems.contains(&"dnd5e"));
        assert!(systems.contains(&"pf2e"));
        assert!(systems.contains(&"coc7e"));
        assert!(systems.contains(&"fate_core"));
        assert!(systems.contains(&"blades"));
        assert!(systems.contains(&"pbta"));
        assert!(systems.contains(&"pbta_aw"));
        assert!(systems.contains(&"pbta_dw"));
        assert!(systems.contains(&"pbta_motw"));

        // Total should be 9 systems
        assert_eq!(systems.len(), 9);
    }

    #[test]
    fn empty_registry_has_no_systems() {
        let registry = GameSystemRegistry::empty();
        assert!(registry.list_systems().is_empty());
    }

    #[test]
    fn registry_list_with_names() {
        let registry = GameSystemRegistry::new();
        let systems = registry.list_systems_with_names();
        assert!(systems.iter().any(|(id, name)| *id == "dnd5e" && *name == "D&D 5th Edition"));
        assert!(systems.iter().any(|(id, name)| *id == "pf2e" && *name == "Pathfinder 2nd Edition"));
        assert!(systems.iter().any(|(id, name)| *id == "coc7e" && *name == "Call of Cthulhu 7th Edition"));
        assert!(systems.iter().any(|(id, name)| *id == "fate_core" && *name == "FATE Core"));
        assert!(systems.iter().any(|(id, name)| *id == "blades" && *name == "Blades in the Dark"));
    }

    #[test]
    fn can_get_each_system_by_id() {
        let registry = GameSystemRegistry::new();

        let pf2e = registry.get("pf2e").expect("PF2e should be registered");
        assert_eq!(pf2e.display_name(), "Pathfinder 2nd Edition");

        let coc = registry.get("coc7e").expect("CoC should be registered");
        assert_eq!(coc.display_name(), "Call of Cthulhu 7th Edition");

        let fate = registry.get("fate_core").expect("FATE should be registered");
        assert_eq!(fate.display_name(), "FATE Core");

        let blades = registry.get("blades").expect("Blades should be registered");
        assert_eq!(blades.display_name(), "Blades in the Dark");
    }
}
