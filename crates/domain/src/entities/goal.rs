//! Goal entity - Abstract want targets
//!
//! # Graph-First Design (Phase 0.C)
//!
//! A Goal is an abstract desire target for Wants that don't target a specific
//! Character or Item. Examples: "power", "revenge", "peace", "recognition".
//!
//! ```cypher
//! (world:World)-[:CONTAINS_GOAL]->(goal:Goal)
//! (want:Want)-[:TARGETS]->(goal:Goal)
//! ```

use wrldbldr_domain::{GoalId, WorldId};

/// Abstract desire target (for Wants that don't target a Character or Item)
#[derive(Debug, Clone)]
pub struct Goal {
    pub id: GoalId,
    pub world_id: WorldId,
    /// Name of the goal (e.g., "Power", "Revenge", "Peace")
    pub name: String,
    /// Optional description of what this goal means
    pub description: Option<String>,
}

impl Goal {
    pub fn new(world_id: WorldId, name: impl Into<String>) -> Self {
        Self {
            id: GoalId::new(),
            world_id,
            name: name.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Common goals that can be used as templates when initializing a world.
/// These represent universal motivations across most narrative contexts.
pub mod common_goals {
    /// Common goal definition with name and description
    pub struct CommonGoalDef {
        pub name: &'static str,
        pub description: &'static str,
    }

    /// Default set of common goals for narrative-driven worlds
    pub const COMMON_GOALS: &[CommonGoalDef] = &[
        CommonGoalDef {
            name: "Power",
            description: "The desire to control, influence, or dominate others",
        },
        CommonGoalDef {
            name: "Wealth",
            description: "The pursuit of material riches and financial security",
        },
        CommonGoalDef {
            name: "Knowledge",
            description: "The quest for understanding, secrets, and hidden truths",
        },
        CommonGoalDef {
            name: "Revenge",
            description: "The need to settle old scores and right past wrongs",
        },
        CommonGoalDef {
            name: "Justice",
            description: "The pursuit of fairness, law, and moral order",
        },
        CommonGoalDef {
            name: "Love",
            description: "The desire for connection, romance, or familial bonds",
        },
        CommonGoalDef {
            name: "Freedom",
            description: "The yearning to escape constraints and live unbound",
        },
        CommonGoalDef {
            name: "Honor",
            description: "The pursuit of reputation, glory, and respect",
        },
        CommonGoalDef {
            name: "Survival",
            description: "The basic drive to stay alive and protect oneself",
        },
        CommonGoalDef {
            name: "Peace",
            description: "The desire for harmony, stability, and an end to conflict",
        },
        CommonGoalDef {
            name: "Recognition",
            description: "The need to be seen, acknowledged, and valued by others",
        },
        CommonGoalDef {
            name: "Redemption",
            description: "The hope to atone for past sins and find forgiveness",
        },
    ];

    /// Get common goal names as a list of strings
    pub fn common_goal_names() -> Vec<&'static str> {
        COMMON_GOALS.iter().map(|g| g.name).collect()
    }
}
