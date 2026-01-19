//! Stat value object - represents character abilities/stats for skill checks.
//!
//! Provides type safety for stat references instead of using magic strings like "STR", "DEX".

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Character stats/abilities used in skill checks.
///
/// Primarily covers D&D 5e ability scores, but includes a `Custom` fallback
/// for other game systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Stat {
    /// Strength - physical power
    Str,
    /// Dexterity - agility and reflexes
    Dex,
    /// Constitution - endurance and health
    Con,
    /// Intelligence - reasoning and memory
    Int,
    /// Wisdom - perception and insight
    Wis,
    /// Charisma - force of personality
    Cha,
    /// Custom/unknown stat (for other game systems)
    #[serde(other)]
    Custom,
}

impl Stat {
    /// Returns the short uppercase string representation (e.g., "STR", "DEX").
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Str => "STR",
            Self::Dex => "DEX",
            Self::Con => "CON",
            Self::Int => "INT",
            Self::Wis => "WIS",
            Self::Cha => "CHA",
            Self::Custom => "CUSTOM",
        }
    }

    /// Returns the full name of the stat (e.g., "Strength", "Dexterity").
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Str => "Strength",
            Self::Dex => "Dexterity",
            Self::Con => "Constitution",
            Self::Int => "Intelligence",
            Self::Wis => "Wisdom",
            Self::Cha => "Charisma",
            Self::Custom => "Custom",
        }
    }

    /// Returns all standard D&D 5e stats.
    pub fn all_standard() -> [Stat; 6] {
        [
            Self::Str,
            Self::Dex,
            Self::Con,
            Self::Int,
            Self::Wis,
            Self::Cha,
        ]
    }
}

impl fmt::Display for Stat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Stat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "STR" | "STRENGTH" => Ok(Self::Str),
            "DEX" | "DEXTERITY" => Ok(Self::Dex),
            "CON" | "CONSTITUTION" => Ok(Self::Con),
            "INT" | "INTELLIGENCE" => Ok(Self::Int),
            "WIS" | "WISDOM" => Ok(Self::Wis),
            "CHA" | "CHARISMA" => Ok(Self::Cha),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_as_str() {
        assert_eq!(Stat::Str.as_str(), "STR");
        assert_eq!(Stat::Dex.as_str(), "DEX");
        assert_eq!(Stat::Con.as_str(), "CON");
        assert_eq!(Stat::Int.as_str(), "INT");
        assert_eq!(Stat::Wis.as_str(), "WIS");
        assert_eq!(Stat::Cha.as_str(), "CHA");
        assert_eq!(Stat::Custom.as_str(), "CUSTOM");
    }

    #[test]
    fn test_stat_from_str() {
        assert_eq!(Stat::from_str("STR"), Ok(Stat::Str));
        assert_eq!(Stat::from_str("str"), Ok(Stat::Str));
        assert_eq!(Stat::from_str("Strength"), Ok(Stat::Str));
        assert_eq!(Stat::from_str("DEX"), Ok(Stat::Dex));
        assert_eq!(Stat::from_str("dexterity"), Ok(Stat::Dex));
        assert_eq!(Stat::from_str("UNKNOWN"), Err(()));
    }

    #[test]
    fn test_stat_display() {
        assert_eq!(format!("{}", Stat::Str), "STR");
        assert_eq!(format!("{}", Stat::Cha), "CHA");
    }

    #[test]
    fn test_stat_serde_roundtrip() {
        let stat = Stat::Dex;
        let json = serde_json::to_string(&stat).unwrap();
        assert_eq!(json, "\"DEX\"");
        let parsed: Stat = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, stat);
    }

    #[test]
    fn test_stat_serde_unknown_becomes_custom() {
        let json = "\"UNKNOWN_STAT\"";
        let parsed: Stat = serde_json::from_str(json).unwrap();
        assert_eq!(parsed, Stat::Custom);
    }
}
