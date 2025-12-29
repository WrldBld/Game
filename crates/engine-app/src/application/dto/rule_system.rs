//! Rule system DTOs for HTTP API responses.
//!
//! Domain types (RuleSystemType, RuleSystemVariant, RuleSystemConfig) now have serde derives,
//! so they can be used directly in API responses. This module provides catalog/summary DTOs
//! for browsing rule systems and parsing helpers.

use serde::Serialize;

use wrldbldr_domain::value_objects::{RuleSystemConfig, RuleSystemType, RuleSystemVariant};

// ============================================================================
// Catalog/lookup DTOs (used by rule system routes)
// ============================================================================

/// Summary of a preset for browsing.
#[derive(Debug, Serialize)]
pub struct RuleSystemPresetSummaryDto {
    pub variant: RuleSystemVariant,
    pub name: String,
    pub description: String,
}

/// Summary of a rule system type for browsing.
#[derive(Debug, Serialize)]
pub struct RuleSystemSummaryDto {
    pub system_type: RuleSystemType,
    pub name: String,
    pub description: String,
    pub dice_notation: String,
    pub presets: Vec<RuleSystemPresetSummaryDto>,
}

/// Full preset details.
#[derive(Debug, Serialize)]
pub struct RuleSystemPresetDetailsDto {
    pub variant: RuleSystemVariant,
    pub config: RuleSystemConfig,
}

/// Details about a rule system type.
#[derive(Debug, Serialize)]
pub struct RuleSystemTypeDetailsDto {
    pub system_type: RuleSystemType,
    pub name: String,
    pub description: String,
    pub dice_notation: String,
    pub presets: Vec<RuleSystemPresetSummaryDto>,
}

// ============================================================================
// Parsing Helpers
// ============================================================================

pub fn parse_system_type(s: &str) -> Result<RuleSystemType, String> {
    match s.to_lowercase().as_str() {
        "d20" => Ok(RuleSystemType::D20),
        "d100" => Ok(RuleSystemType::D100),
        "narrative" => Ok(RuleSystemType::Narrative),
        "custom" => Ok(RuleSystemType::Custom),
        _ => Err(format!(
            "Unknown rule system type: {}. Valid types: d20, d100, narrative, custom",
            s
        )),
    }
}

pub fn parse_variant(s: &str) -> Result<RuleSystemVariant, String> {
    match s.to_lowercase().replace("-", "_").as_str() {
        "dnd5e" | "dnd_5e" => Ok(RuleSystemVariant::Dnd5e),
        "pathfinder2e" | "pathfinder_2e" => Ok(RuleSystemVariant::Pathfinder2e),
        "generic_d20" | "genericd20" => Ok(RuleSystemVariant::GenericD20),
        "coc7e" | "coc_7e" | "callofcthulhu7e" | "call_of_cthulhu_7e" => {
            Ok(RuleSystemVariant::CallOfCthulhu7e)
        }
        "runequest" | "rune_quest" => Ok(RuleSystemVariant::RuneQuest),
        "generic_d100" | "genericd100" => Ok(RuleSystemVariant::GenericD100),
        "kidsonbikes" | "kids_on_bikes" => Ok(RuleSystemVariant::KidsOnBikes),
        "fatecore" | "fate_core" | "fate" => Ok(RuleSystemVariant::FateCore),
        "pbta" | "poweredbyapocalypse" | "powered_by_apocalypse" => {
            Ok(RuleSystemVariant::PoweredByApocalypse)
        }
        _ => Err(format!(
            "Unknown variant: {}. Valid variants: dnd5e, pathfinder2e, generic_d20, coc7e, runequest, generic_d100, kidsonbikes, fatecore, pbta",
            s
        )),
    }
}
