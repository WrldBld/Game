//! Rule system configuration types
//!
//! Types for configuring the game's rule system (D20, D100, Narrative, etc.)
//! These determine how challenges are resolved, how stats work, etc.

use serde::{Deserialize, Serialize};

// Placeholder - will be populated in task 1.5
// For now, define minimal types so the crate compiles

/// Configuration for the rule system used in a world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSystemConfig {
    /// The type of rule system
    pub system_type: RuleSystemType,
    /// The specific variant/edition
    pub variant: RuleSystemVariant,
}

impl Default for RuleSystemConfig {
    fn default() -> Self {
        Self {
            system_type: RuleSystemType::D20,
            variant: RuleSystemVariant::DnD5e,
        }
    }
}

/// High-level rule system type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSystemType {
    /// D20-based systems (roll d20 + modifier vs DC)
    D20,
    /// D100/percentile systems (roll under skill)
    D100,
    /// Narrative/fiction-first systems
    Narrative,
    /// Custom rule system
    Custom,
}

/// Specific rule system variant
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSystemVariant {
    // D20 variants
    DnD5e,
    Pathfinder2e,
    
    // D100 variants
    CallOfCthulhu,
    RuneQuest,
    
    // Narrative variants
    FateCore,
    PbtA,
    
    // Custom
    Custom(String),
}
