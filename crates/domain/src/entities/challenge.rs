//! Challenge entity - Skill checks, ability checks, and other game challenges
//!
//! Challenges can be attached to scenes and triggered either manually by the DM
//! or suggested by the LLM when trigger conditions are met.
//!
//! ## Graph-First Design (Phase 0.E)
//!
//! Challenges use Neo4j edges for relationships:
//! - `(Challenge)-[:REQUIRES_SKILL]->(Skill)` - Skill tested by this challenge
//! - `(Challenge)-[:TIED_TO_SCENE]->(Scene)` - Scene this challenge appears in
//! - `(Challenge)-[:REQUIRES_COMPLETION_OF {success_required}]->(Challenge)` - Prerequisites
//! - `(Challenge)-[:AVAILABLE_AT {always_available, time_restriction}]->(Location)` - Location availability
//! - `(Challenge)-[:ON_SUCCESS_UNLOCKS]->(Location)` - Location unlocked on success
//!
//! The embedded fields `scene_id`, `skill_id`, and `prerequisite_challenges` are
//! DEPRECATED and kept only for backward compatibility during migration.

use crate::error::DomainError;
use crate::{ChallengeId, LocationId, RegionId, SceneId, WorldId};
use serde::{Deserialize, Serialize};

// Re-export narrative resolution types from types module
pub use crate::types::{
    DifficultyDescriptor, DifficultyLadder, EffectLevel, NarrativeResolutionConfig,
    NarrativeResolutionStyle, NarrativeThresholds, Position,
};

/// A challenge that can be triggered during gameplay
///
/// ## Graph Relationships (via repository edge methods)
/// - `REQUIRES_SKILL` -> Skill: The skill tested by this challenge
/// - `TIED_TO_SCENE` -> Scene: Scene this challenge appears in (optional)
/// - `REQUIRES_COMPLETION_OF` -> Challenge: Prerequisite challenges
/// - `AVAILABLE_AT` -> Location: Locations where this challenge is available
/// - `ON_SUCCESS_UNLOCKS` -> Location: Locations unlocked on success
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Challenge {
    id: ChallengeId,
    world_id: WorldId,
    name: String,
    description: String,
    challenge_type: ChallengeType,
    difficulty: Difficulty,
    outcomes: ChallengeOutcomes,
    /// Conditions that trigger LLM to suggest this challenge (non-entity triggers stored as JSON)
    trigger_conditions: Vec<TriggerCondition>,
    /// Whether this challenge can currently be triggered
    active: bool,
    /// Display order in challenge library
    order: u32,
    /// Whether the DM favorited this challenge
    is_favorite: bool,
    /// Tags for filtering
    tags: Vec<String>,
    /// The stat to check for this challenge (e.g., "STR", "DEX", "ATHLETICS_MOD")
    /// If None, the modifier will be 0 unless provided by the client.
    #[serde(default)]
    check_stat: Option<String>,
}

impl Challenge {
    /// Create a new challenge.
    ///
    /// Note: The skill relationship should be set via `ChallengeRepositoryPort::set_required_skill()`
    /// after creating the challenge.
    pub fn new(world_id: WorldId, name: impl Into<String>, difficulty: Difficulty) -> Self {
        Self {
            id: ChallengeId::new(),
            world_id,
            name: name.into(),
            description: String::new(),
            challenge_type: ChallengeType::SkillCheck,
            difficulty,
            outcomes: ChallengeOutcomes::default(),
            trigger_conditions: Vec::new(),
            active: true,
            order: 0,
            is_favorite: false,
            tags: Vec::new(),
            check_stat: None,
        }
    }

    // === Accessors ===

    pub fn id(&self) -> ChallengeId {
        self.id
    }

    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn challenge_type(&self) -> ChallengeType {
        self.challenge_type
    }

    pub fn difficulty(&self) -> &Difficulty {
        &self.difficulty
    }

    pub fn outcomes(&self) -> &ChallengeOutcomes {
        &self.outcomes
    }

    pub fn trigger_conditions(&self) -> &[TriggerCondition] {
        &self.trigger_conditions
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn order(&self) -> u32 {
        self.order
    }

    pub fn is_favorite(&self) -> bool {
        self.is_favorite
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn check_stat(&self) -> Option<&str> {
        self.check_stat.as_deref()
    }

    // === Builder Methods ===

    /// Set the challenge ID (used when loading from database).
    pub fn with_id(mut self, id: ChallengeId) -> Self {
        self.id = id;
        self
    }

    /// Set the stat to check for this challenge.
    pub fn with_check_stat(mut self, stat: impl Into<String>) -> Self {
        self.check_stat = Some(stat.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_challenge_type(mut self, challenge_type: ChallengeType) -> Self {
        self.challenge_type = challenge_type;
        self
    }

    pub fn with_outcomes(mut self, outcomes: ChallengeOutcomes) -> Self {
        self.outcomes = outcomes;
        self
    }

    pub fn with_trigger(mut self, condition: TriggerCondition) -> Self {
        self.trigger_conditions.push(condition);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }

    pub fn with_is_favorite(mut self, is_favorite: bool) -> Self {
        self.is_favorite = is_favorite;
        self
    }

    /// Validate all triggers in this challenge.
    ///
    /// Returns a list of validation errors, empty if all triggers are valid.
    pub fn validate_triggers(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // Validate trigger conditions
        for (i, condition) in self.trigger_conditions.iter().enumerate() {
            if let Err(e) = condition.validate() {
                errors.push(format!("Trigger condition {}: {}", i + 1, e));
            }
        }

        // Validate outcome triggers
        for (outcome_name, outcome) in self.all_outcomes_named() {
            for (i, trigger) in outcome.triggers.iter().enumerate() {
                if let Err(e) = trigger.validate() {
                    errors.push(format!("{} outcome trigger {}: {}", outcome_name, i + 1, e));
                }
            }
        }

        errors
    }

    /// Get all referenced challenge IDs from triggers.
    /// Used for validation to ensure referenced challenges exist.
    pub fn referenced_challenge_ids(&self) -> Vec<ChallengeId> {
        let mut ids = Vec::new();

        // From trigger conditions
        for condition in &self.trigger_conditions {
            ids.extend(condition.referenced_challenge_ids());
        }

        // From outcome triggers
        for (_, outcome) in self.all_outcomes_named() {
            for trigger in &outcome.triggers {
                ids.extend(trigger.referenced_challenge_ids());
            }
        }

        ids
    }

    /// Helper to iterate over all outcomes with their names
    fn all_outcomes_named(&self) -> Vec<(&str, &Outcome)> {
        let mut outcomes = vec![
            ("Success", &self.outcomes.success),
            ("Failure", &self.outcomes.failure),
        ];
        if let Some(ref partial) = self.outcomes.partial {
            outcomes.push(("Partial", partial));
        }
        if let Some(ref crit_success) = self.outcomes.critical_success {
            outcomes.push(("Critical Success", crit_success));
        }
        if let Some(ref crit_failure) = self.outcomes.critical_failure {
            outcomes.push(("Critical Failure", crit_failure));
        }
        outcomes
    }

    /// Check if a trigger condition matches some player action/context
    ///
    /// Logic:
    /// - All conditions with `required: true` must match (AND logic)
    /// - At least one condition overall must match (to avoid false positives)
    /// - If all conditions are optional, at least one must match (OR logic)
    /// - If all conditions are required, all must match (AND logic)
    pub fn matches_trigger(&self, action: &str, context: &str) -> bool {
        if self.trigger_conditions.is_empty() {
            return false;
        }

        // Check which conditions match
        let matched: Vec<bool> = self
            .trigger_conditions
            .iter()
            .map(|tc| tc.matches(action, context))
            .collect();

        // All required conditions must match
        let required_conditions_met = self
            .trigger_conditions
            .iter()
            .zip(matched.iter())
            .filter(|(tc, _)| tc.required)
            .all(|(_, &m)| m);

        // At least one condition must match overall
        let at_least_one_matches = matched.iter().any(|&m| m);

        required_conditions_met && at_least_one_matches
    }

    /// Evaluate a dice roll against this challenge's difficulty.
    ///
    /// Takes the raw roll (before modifiers) and the total modifier to apply.
    /// Returns the outcome type and a reference to the corresponding outcome.
    ///
    /// # Rule System Support
    /// - DC-based (D20 systems): Natural 20 = crit success, Natural 1 = crit failure,
    ///   total >= DC = success
    /// - Percentage-based (D100): Roll 1 = crit success, Roll 100 = crit failure,
    ///   roll <= target = success (lower is better)
    /// - Descriptor-based (Narrative): Uses configurable narrative resolution
    /// - Opposed: Always returns success (actual comparison done elsewhere)
    /// - Custom: Always returns success (DM adjudicates)
    ///
    /// # Backward Compatibility
    /// This method maintains the original signature for existing callers.
    /// For full narrative resolution support, use `evaluate_roll_narrative`.
    pub fn evaluate_roll(&self, roll: i32, modifier: i32) -> (OutcomeType, &Outcome) {
        self.evaluate_roll_narrative(roll, modifier, None, None, None, None)
    }

    /// Evaluate a dice roll with full narrative resolution support.
    ///
    /// # Arguments
    /// * `roll` - The raw roll value (sum of dice, or highest die for pools)
    /// * `modifier` - The modifier to apply (skill bonus, stat modifier, etc.)
    /// * `narrative_config` - Optional narrative resolution configuration
    /// * `position` - Optional position for Blades-style resolution
    /// * `effect` - Optional effect level for Blades-style resolution
    /// * `dice_results` - Optional individual dice results (for critical detection in pools)
    ///
    /// # Narrative Resolution Styles
    /// - **PbtA**: Fixed thresholds (configurable, default 10+/7-9/6-)
    /// - **Ladder**: Compare roll to descriptor's ladder value (Fate-style)
    /// - **Blades**: d6 pool with Position/Effect determining consequences
    pub fn evaluate_roll_narrative(
        &self,
        roll: i32,
        modifier: i32,
        narrative_config: Option<&NarrativeResolutionConfig>,
        position: Option<Position>,
        effect: Option<EffectLevel>,
        dice_results: Option<&[i32]>,
    ) -> (OutcomeType, &Outcome) {
        let outcome_type = match &self.difficulty {
            Difficulty::DC(dc) => {
                // D20 system: Natural 20 = crit success, Natural 1 = crit failure
                if roll == 20 && self.outcomes.critical_success.is_some() {
                    OutcomeType::CriticalSuccess
                } else if roll == 1 && self.outcomes.critical_failure.is_some() {
                    OutcomeType::CriticalFailure
                } else if (roll + modifier) >= *dc as i32 {
                    OutcomeType::Success
                } else {
                    OutcomeType::Failure
                }
            }
            Difficulty::Percentage(target) => {
                // D100 system: Roll 1 = crit success, Roll 100 = crit failure
                // Lower is better - roll must be <= target to succeed
                if roll == 1 && self.outcomes.critical_success.is_some() {
                    OutcomeType::CriticalSuccess
                } else if roll == 100 && self.outcomes.critical_failure.is_some() {
                    OutcomeType::CriticalFailure
                } else if roll <= *target as i32 {
                    OutcomeType::Success
                } else {
                    OutcomeType::Failure
                }
            }
            Difficulty::Descriptor(descriptor) => {
                // Use narrative config if provided, otherwise default PbtA
                let default_config = NarrativeResolutionConfig::default();
                let config = narrative_config.unwrap_or(&default_config);

                self.evaluate_narrative_roll(
                    roll,
                    modifier,
                    descriptor,
                    config,
                    position.unwrap_or_default(),
                    effect.unwrap_or_default(),
                    dice_results,
                )
            }
            Difficulty::Opposed | Difficulty::Custom(_) => {
                // Opposed/Custom: Always return success as placeholder
                // Actual resolution happens elsewhere (DM or opposed roll comparison)
                OutcomeType::Success
            }
        };

        // Return the appropriate outcome reference based on outcome type
        let outcome = self.outcome_for_type(outcome_type);
        (outcome_type, outcome)
    }

    /// Evaluate a narrative roll based on resolution style
    #[allow(clippy::too_many_arguments)]
    fn evaluate_narrative_roll(
        &self,
        roll: i32,
        modifier: i32,
        descriptor: &DifficultyDescriptor,
        config: &NarrativeResolutionConfig,
        _position: Position,
        effect: EffectLevel,
        dice_results: Option<&[i32]>,
    ) -> OutcomeType {
        match config.style {
            NarrativeResolutionStyle::PbtA
            | NarrativeResolutionStyle::Custom
            | NarrativeResolutionStyle::Unknown => {
                self.evaluate_pbta(roll, modifier, &config.thresholds)
            }
            NarrativeResolutionStyle::Ladder => {
                self.evaluate_ladder(roll, modifier, descriptor, &config.ladder)
            }
            NarrativeResolutionStyle::Blades => {
                self.evaluate_blades(dice_results, effect, &config.position_effect)
            }
        }
    }

    /// Evaluate using PbtA-style fixed thresholds
    fn evaluate_pbta(
        &self,
        roll: i32,
        modifier: i32,
        thresholds: &NarrativeThresholds,
    ) -> OutcomeType {
        let total = roll + modifier;

        // Check critical success first (if configured)
        if let Some(crit) = thresholds.critical_success {
            if total >= crit && self.outcomes.critical_success.is_some() {
                return OutcomeType::CriticalSuccess;
            }
        }

        // Check critical failure (if configured)
        if let Some(crit_fail) = thresholds.critical_failure {
            if total <= crit_fail && self.outcomes.critical_failure.is_some() {
                return OutcomeType::CriticalFailure;
            }
        }

        // Standard PbtA resolution
        if total >= thresholds.full_success {
            OutcomeType::Success
        } else if total >= thresholds.partial_success {
            OutcomeType::Partial
        } else {
            OutcomeType::Failure
        }
    }

    /// Evaluate using Fate-style ladder comparison
    fn evaluate_ladder(
        &self,
        roll: i32,
        modifier: i32,
        descriptor: &DifficultyDescriptor,
        ladder: &DifficultyLadder,
    ) -> OutcomeType {
        let total = roll + modifier;
        // Default to Fair (+2) if descriptor not in ladder
        let target = ladder.value_for(descriptor).unwrap_or(2);
        let shifts = total - target;

        if shifts >= ladder.style_threshold {
            // Succeed with style = critical success in our system
            OutcomeType::CriticalSuccess
        } else if shifts > ladder.tie_threshold {
            OutcomeType::Success
        } else if shifts == ladder.tie_threshold {
            // Tie = partial success (success at minor cost)
            OutcomeType::Partial
        } else {
            OutcomeType::Failure
        }
    }

    /// Evaluate using Blades-style d6 pool (highest die)
    fn evaluate_blades(
        &self,
        dice_results: Option<&[i32]>,
        effect: EffectLevel,
        config: &crate::types::PositionEffectConfig,
    ) -> OutcomeType {
        let dice = dice_results.unwrap_or(&[]);
        let highest = dice.iter().max().copied().unwrap_or(0);
        let thresholds = &config.pool_thresholds;

        // Check for critical (multiple max dice, typically 6s)
        let max_die_count = dice
            .iter()
            .filter(|&&d| d == thresholds.full_success)
            .count();
        let is_critical =
            config.enable_critical && max_die_count >= config.critical_dice_count as usize;

        if is_critical {
            // Critical = success with increased effect
            OutcomeType::CriticalSuccess
        } else if highest >= thresholds.full_success {
            OutcomeType::Success
        } else if highest >= thresholds.partial_success_min
            && highest <= thresholds.partial_success_max
        {
            OutcomeType::Partial
        } else {
            // For Blades, failure severity depends on Position (handled by caller)
            // We just return the base failure outcome
            // Effect level is already captured for clock tick calculation
            let _ = effect; // Acknowledge effect is used contextually
            OutcomeType::Failure
        }
    }

    /// Get the outcome reference for an outcome type
    fn outcome_for_type(&self, outcome_type: OutcomeType) -> &Outcome {
        match outcome_type {
            OutcomeType::CriticalSuccess => self
                .outcomes
                .critical_success
                .as_ref()
                .unwrap_or(&self.outcomes.success),
            OutcomeType::Success => &self.outcomes.success,
            OutcomeType::Partial => self
                .outcomes
                .partial
                .as_ref()
                .unwrap_or(&self.outcomes.success),
            OutcomeType::Failure => &self.outcomes.failure,
            OutcomeType::CriticalFailure => self
                .outcomes
                .critical_failure
                .as_ref()
                .unwrap_or(&self.outcomes.failure),
        }
    }
}

/// Types of challenges
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum ChallengeType {
    /// Standard skill check against difficulty
    #[default]
    SkillCheck,
    /// Raw attribute/ability check (no skill proficiency)
    AbilityCheck,
    /// Reactive defense check
    SavingThrow,
    /// Contest against another entity's skill
    OpposedCheck,
    /// Multi-roll challenge requiring accumulated successes
    ComplexChallenge,
    /// Unknown type for forward compatibility
    #[serde(other)]
    Unknown,
}

impl ChallengeType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SkillCheck => "Skill Check",
            Self::AbilityCheck => "Ability Check",
            Self::SavingThrow => "Saving Throw",
            Self::OpposedCheck => "Opposed Check",
            Self::ComplexChallenge => "Complex Challenge",
            Self::Unknown => "Unknown",
        }
    }
}

/// Challenge difficulty representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Difficulty {
    /// D20-style: roll + modifier >= DC
    DC(u32),
    /// D100-style: roll <= percentage target
    Percentage(u32),
    /// Narrative systems: descriptive difficulty
    Descriptor(DifficultyDescriptor),
    /// Opposed roll: compare to opponent's roll
    Opposed,
    /// Custom difficulty with notes
    Custom(String),
}

impl Default for Difficulty {
    fn default() -> Self {
        Self::DC(10)
    }
}

impl Difficulty {
    /// Create a DC (Difficulty Class) difficulty for D20 systems.
    ///
    /// The DC must be at least 1. A roll + modifier >= DC results in success.
    ///
    /// # Arguments
    /// * `value` - The difficulty class value (must be >= 1)
    ///
    /// # Panics
    /// Panics if value is 0.
    ///
    /// # Example
    /// ```ignore
    /// let difficulty = Difficulty::dc(15); // DC 15 check
    /// ```
    pub fn dc(value: u32) -> Self {
        assert!(value >= 1, "DC must be at least 1");
        Difficulty::DC(value)
    }

    /// Create a percentage difficulty for D100/percentile systems.
    ///
    /// The percentage must be between 1 and 100 inclusive.
    /// In percentage systems, the roll must be <= the target to succeed
    /// (lower is better).
    ///
    /// # Arguments
    /// * `value` - The percentage target (1-100)
    ///
    /// # Panics
    /// Panics if value is not in the range 1-100.
    ///
    /// # Example
    /// ```ignore
    /// let difficulty = Difficulty::percentage(45); // 45% chance of success
    /// ```
    pub fn percentage(value: u32) -> Self {
        assert!((1..=100).contains(&value), "Percentage must be 1-100");
        Difficulty::Percentage(value)
    }

    /// Get a human-readable description
    pub fn display(&self) -> String {
        match self {
            Self::DC(dc) => format!("DC {}", dc),
            Self::Percentage(p) => format!("{}%", p),
            Self::Descriptor(d) => d.display_name().to_string(),
            Self::Opposed => "Opposed".to_string(),
            Self::Custom(s) => s.clone(),
        }
    }

    /// Standard D20 difficulty presets
    pub fn d20_easy() -> Self {
        Self::DC(10)
    }
    pub fn d20_medium() -> Self {
        Self::DC(15)
    }
    pub fn d20_hard() -> Self {
        Self::DC(20)
    }
    pub fn d20_very_hard() -> Self {
        Self::DC(25)
    }

    /// D100 difficulty presets (based on typical skill values)
    pub fn d100_regular() -> Self {
        Self::Percentage(100)
    }
    pub fn d100_hard() -> Self {
        Self::Percentage(50)
    }
    pub fn d100_extreme() -> Self {
        Self::Percentage(20)
    }

    /// Get the suggested dice formula and hint text for this difficulty type.
    ///
    /// Returns (dice_formula, hint_text) tuple.
    pub fn dice_suggestion(&self) -> (&'static str, &'static str) {
        match self {
            Self::DC(_) => ("1d20", "Roll 1d20 and add your skill modifier"),
            Self::Percentage(_) => ("1d100", "Roll percentile dice (1d100), lower is better"),
            Self::Descriptor(_) => ("2d6", "Roll 2d6 and add your modifier"),
            Self::Opposed => ("1d20", "Opposed roll - both parties roll and compare"),
            Self::Custom(_) => ("1d20", "Custom difficulty - follow DM instructions"),
        }
    }

    /// Parse a difficulty string into a Difficulty variant.
    ///
    /// Supports formats:
    /// - "DC 15" or "DC15" -> Difficulty::DC(15)
    /// - "45%" -> Difficulty::Percentage(45)
    /// - Other strings -> Difficulty::Custom(string)
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if s.to_uppercase().starts_with("DC") {
            if let Ok(dc) = s[2..].trim().parse::<u32>() {
                return Self::DC(dc);
            }
        }
        if let Some(stripped) = s.strip_suffix('%') {
            if let Ok(pct) = stripped.trim().parse::<u32>() {
                return Self::Percentage(pct);
            }
        }
        Self::Custom(s.to_string())
    }
}

// DifficultyDescriptor is now imported from types module

/// Outcomes for a challenge
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeOutcomes {
    pub success: Outcome,
    pub failure: Outcome,
    /// For narrative systems or "meet DC exactly" results
    pub partial: Option<Outcome>,
    /// Natural 20 or roll of 01 on d100
    pub critical_success: Option<Outcome>,
    /// Natural 1 or fumble roll
    pub critical_failure: Option<Outcome>,
}

impl ChallengeOutcomes {
    pub fn simple(success: impl Into<String>, failure: impl Into<String>) -> Self {
        Self {
            success: Outcome::new(success),
            failure: Outcome::new(failure),
            partial: None,
            critical_success: None,
            critical_failure: None,
        }
    }

    pub fn with_partial(mut self, partial: impl Into<String>) -> Self {
        self.partial = Some(Outcome::new(partial));
        self
    }

    pub fn with_critical_success(mut self, critical: impl Into<String>) -> Self {
        self.critical_success = Some(Outcome::new(critical));
        self
    }

    pub fn with_critical_failure(mut self, critical: impl Into<String>) -> Self {
        self.critical_failure = Some(Outcome::new(critical));
        self
    }
}

/// A single outcome with narrative text and triggered effects
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    /// Narrative description shown to players
    pub description: String,
    /// Effects that trigger when this outcome occurs
    pub triggers: Vec<OutcomeTrigger>,
}

impl Outcome {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            triggers: Vec::new(),
        }
    }

    pub fn with_trigger(mut self, trigger: OutcomeTrigger) -> Self {
        self.triggers.push(trigger);
        self
    }
}

/// Effects triggered by challenge outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutcomeTrigger {
    /// Reveal hidden information to the player
    RevealInformation {
        info: String,
        /// Whether to add to journal/notes
        persist: bool,
    },
    /// Enable another challenge (unlock prerequisite)
    EnableChallenge { challenge_id: ChallengeId },
    /// Disable a challenge (remove from available)
    DisableChallenge { challenge_id: ChallengeId },
    /// Modify a character stat (HP, Sanity, etc.)
    ModifyCharacterStat { stat: String, modifier: i32 },
    /// Trigger a scene transition
    TriggerScene { scene_id: SceneId },
    /// Add an item to inventory
    GiveItem {
        item_name: String,
        item_description: Option<String>,
    },
    /// Custom trigger with free-text description
    Custom { description: String },
}

impl std::fmt::Display for OutcomeTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RevealInformation { info, persist } => {
                if *persist {
                    write!(f, "Reveal (persistent): {}", info)
                } else {
                    write!(f, "Reveal: {}", info)
                }
            }
            Self::EnableChallenge { challenge_id } => {
                write!(f, "Enable challenge: {}", challenge_id)
            }
            Self::DisableChallenge { challenge_id } => {
                write!(f, "Disable challenge: {}", challenge_id)
            }
            Self::ModifyCharacterStat { stat, modifier } => {
                if *modifier >= 0 {
                    write!(f, "Modify {}: +{}", stat, modifier)
                } else {
                    write!(f, "Modify {}: {}", stat, modifier)
                }
            }
            Self::TriggerScene { scene_id } => {
                write!(f, "Trigger scene: {}", scene_id)
            }
            Self::GiveItem {
                item_name,
                item_description,
            } => {
                if let Some(desc) = item_description {
                    write!(f, "Give item: {} ({})", item_name, desc)
                } else {
                    write!(f, "Give item: {}", item_name)
                }
            }
            Self::Custom { description } => {
                write!(f, "Custom: {}", description)
            }
        }
    }
}

impl OutcomeTrigger {
    /// Returns the type name of this trigger for display purposes
    pub fn trigger_type_name(&self) -> &'static str {
        match self {
            Self::RevealInformation { .. } => "reveal_information",
            Self::EnableChallenge { .. } => "enable_challenge",
            Self::DisableChallenge { .. } => "disable_challenge",
            Self::ModifyCharacterStat { .. } => "modify_stat",
            Self::TriggerScene { .. } => "trigger_scene",
            Self::GiveItem { .. } => "give_item",
            Self::Custom { .. } => "custom",
        }
    }

    /// Validate this trigger, returning an error message if invalid.
    ///
    /// Validation rules:
    /// - RevealInformation: info must be non-empty
    /// - GiveItem: item_name must be non-empty
    /// - ModifyCharacterStat: stat must be non-empty
    /// - Custom: description must be non-empty
    pub fn validate(&self) -> Result<(), DomainError> {
        match self {
            Self::RevealInformation { info, .. } => {
                if info.trim().is_empty() {
                    return Err(DomainError::validation(
                        "RevealInformation trigger requires non-empty info",
                    ));
                }
            }
            Self::GiveItem { item_name, .. } => {
                if item_name.trim().is_empty() {
                    return Err(DomainError::validation(
                        "GiveItem trigger requires non-empty item_name",
                    ));
                }
            }
            Self::ModifyCharacterStat { stat, .. } => {
                if stat.trim().is_empty() {
                    return Err(DomainError::validation(
                        "ModifyCharacterStat trigger requires non-empty stat name",
                    ));
                }
            }
            Self::Custom { description } => {
                if description.trim().is_empty() {
                    return Err(DomainError::validation(
                        "Custom trigger requires non-empty description",
                    ));
                }
            }
            // EnableChallenge, DisableChallenge, and TriggerScene have typed IDs that are always valid
            Self::EnableChallenge { .. }
            | Self::DisableChallenge { .. }
            | Self::TriggerScene { .. } => {}
        }
        Ok(())
    }

    /// Get referenced challenge IDs that this trigger depends on.
    /// Used for validation to ensure referenced challenges exist.
    pub fn referenced_challenge_ids(&self) -> Vec<ChallengeId> {
        match self {
            Self::EnableChallenge { challenge_id } | Self::DisableChallenge { challenge_id } => {
                vec![*challenge_id]
            }
            _ => vec![],
        }
    }

    pub fn reveal(info: impl Into<String>) -> Self {
        Self::RevealInformation {
            info: info.into(),
            persist: false,
        }
    }

    pub fn reveal_persistent(info: impl Into<String>) -> Self {
        Self::RevealInformation {
            info: info.into(),
            persist: true,
        }
    }

    pub fn enable(challenge_id: ChallengeId) -> Self {
        Self::EnableChallenge { challenge_id }
    }

    pub fn disable(challenge_id: ChallengeId) -> Self {
        Self::DisableChallenge { challenge_id }
    }

    pub fn modify_stat(stat: impl Into<String>, modifier: i32) -> Self {
        Self::ModifyCharacterStat {
            stat: stat.into(),
            modifier,
        }
    }

    pub fn scene(scene_id: SceneId) -> Self {
        Self::TriggerScene { scene_id }
    }
}

/// Condition that triggers LLM to suggest a challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerCondition {
    pub condition_type: TriggerType,
    /// Human-readable description for DM reference
    pub description: String,
    /// Whether this condition alone is sufficient (AND vs OR logic)
    pub required: bool,
}

impl TriggerCondition {
    pub fn new(condition_type: TriggerType, description: impl Into<String>) -> Self {
        Self {
            condition_type,
            description: description.into(),
            required: false,
        }
    }

    /// Mark this condition as required
    pub fn required_condition(mut self) -> Self {
        self.required = true;
        self
    }

    /// Check if this condition matches the given action/context
    pub fn matches(&self, action: &str, context: &str) -> bool {
        self.condition_type.matches(action, context)
    }

    /// Validate this trigger condition, returning an error message if invalid.
    pub fn validate(&self) -> Result<(), DomainError> {
        self.condition_type.validate()
    }

    /// Get referenced challenge IDs that this condition depends on.
    pub fn referenced_challenge_ids(&self) -> Vec<ChallengeId> {
        self.condition_type.referenced_challenge_ids()
    }
}

/// Types of trigger conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriggerType {
    /// Player interacts with specific object
    ObjectInteraction {
        /// Object keywords to match
        keywords: Vec<String>,
    },
    /// Player enters specific area/location
    EnterArea { area_keywords: Vec<String> },
    /// Player discusses specific topic
    DialogueTopic { topic_keywords: Vec<String> },
    /// Another challenge completed (success or failure)
    ChallengeComplete {
        challenge_id: ChallengeId,
        /// None = either, Some(true) = success only, Some(false) = failure only
        requires_success: Option<bool>,
    },
    /// Time-based trigger (after N turns/exchanges)
    TimeBased { turns: u32 },
    /// NPC present in scene
    NpcPresent { npc_keywords: Vec<String> },
    /// Free-text condition for LLM interpretation
    Custom { description: String },
}

impl TriggerType {
    /// Validate this trigger type, returning an error message if invalid.
    ///
    /// Validation rules:
    /// - ObjectInteraction: keywords must be non-empty and contain non-empty strings
    /// - EnterArea: area_keywords must be non-empty and contain non-empty strings
    /// - DialogueTopic: topic_keywords must be non-empty and contain non-empty strings
    /// - NpcPresent: npc_keywords must be non-empty and contain non-empty strings
    /// - Custom: description must be non-empty
    /// - ChallengeComplete and TimeBased: always valid (IDs are typed)
    pub fn validate(&self) -> Result<(), DomainError> {
        match self {
            Self::ObjectInteraction { keywords } => {
                if keywords.is_empty() {
                    return Err(DomainError::validation(
                        "ObjectInteraction trigger requires at least one keyword",
                    ));
                }
                if keywords.iter().all(|k| k.trim().is_empty()) {
                    return Err(DomainError::validation(
                        "ObjectInteraction trigger keywords cannot all be empty",
                    ));
                }
            }
            Self::EnterArea { area_keywords } => {
                if area_keywords.is_empty() {
                    return Err(DomainError::validation(
                        "EnterArea trigger requires at least one keyword",
                    ));
                }
                if area_keywords.iter().all(|k| k.trim().is_empty()) {
                    return Err(DomainError::validation(
                        "EnterArea trigger keywords cannot all be empty",
                    ));
                }
            }
            Self::DialogueTopic { topic_keywords } => {
                if topic_keywords.is_empty() {
                    return Err(DomainError::validation(
                        "DialogueTopic trigger requires at least one keyword",
                    ));
                }
                if topic_keywords.iter().all(|k| k.trim().is_empty()) {
                    return Err(DomainError::validation(
                        "DialogueTopic trigger keywords cannot all be empty",
                    ));
                }
            }
            Self::NpcPresent { npc_keywords } => {
                if npc_keywords.is_empty() {
                    return Err(DomainError::validation(
                        "NpcPresent trigger requires at least one keyword",
                    ));
                }
                if npc_keywords.iter().all(|k| k.trim().is_empty()) {
                    return Err(DomainError::validation(
                        "NpcPresent trigger keywords cannot all be empty",
                    ));
                }
            }
            Self::Custom { description } => {
                if description.trim().is_empty() {
                    return Err(DomainError::validation(
                        "Custom trigger requires non-empty description",
                    ));
                }
            }
            // ChallengeComplete and TimeBased have typed values that are always valid
            Self::ChallengeComplete { .. } | Self::TimeBased { .. } => {}
        }
        Ok(())
    }

    /// Get referenced challenge IDs that this trigger depends on.
    /// Used for validation to ensure referenced challenges exist.
    pub fn referenced_challenge_ids(&self) -> Vec<ChallengeId> {
        match self {
            Self::ChallengeComplete { challenge_id, .. } => vec![*challenge_id],
            _ => vec![],
        }
    }

    /// Check if this trigger type matches the given action/context
    pub fn matches(&self, action: &str, context: &str) -> bool {
        let action_lower = action.to_lowercase();
        let context_lower = context.to_lowercase();

        match self {
            Self::ObjectInteraction { keywords } => keywords.iter().any(|k| {
                let k_lower = k.to_lowercase();
                action_lower.contains(&k_lower) || context_lower.contains(&k_lower)
            }),
            Self::EnterArea { area_keywords } => area_keywords.iter().any(|k| {
                let k_lower = k.to_lowercase();
                action_lower.contains(&k_lower) || context_lower.contains(&k_lower)
            }),
            Self::DialogueTopic { topic_keywords } => topic_keywords.iter().any(|k| {
                let k_lower = k.to_lowercase();
                action_lower.contains(&k_lower) || context_lower.contains(&k_lower)
            }),
            Self::Custom { description } => {
                // Custom triggers rely on LLM interpretation
                // This basic implementation checks for keyword overlap
                let desc_lower = description.to_lowercase();
                let desc_words: Vec<&str> = desc_lower.split_whitespace().collect();
                desc_words
                    .iter()
                    .filter(|w| w.len() > 3)
                    .any(|w| action_lower.contains(*w) || context_lower.contains(*w))
            }
            // These require external state to evaluate
            Self::ChallengeComplete { .. } | Self::TimeBased { .. } | Self::NpcPresent { .. } => {
                false
            }
        }
    }

    pub fn object(keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::ObjectInteraction {
            keywords: keywords.into_iter().map(|k| k.into()).collect(),
        }
    }

    pub fn area(keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::EnterArea {
            area_keywords: keywords.into_iter().map(|k| k.into()).collect(),
        }
    }

    pub fn topic(keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::DialogueTopic {
            topic_keywords: keywords.into_iter().map(|k| k.into()).collect(),
        }
    }

    pub fn after_challenge(challenge_id: ChallengeId) -> Self {
        Self::ChallengeComplete {
            challenge_id,
            requires_success: None,
        }
    }

    pub fn after_challenge_success(challenge_id: ChallengeId) -> Self {
        Self::ChallengeComplete {
            challenge_id,
            requires_success: Some(true),
        }
    }

    pub fn custom(description: impl Into<String>) -> Self {
        Self::Custom {
            description: description.into(),
        }
    }
}

/// Type of outcome achieved in a challenge resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutcomeType {
    CriticalSuccess,
    Success,
    Partial,
    Failure,
    CriticalFailure,
}

impl OutcomeType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CriticalSuccess => "Critical Success!",
            Self::Success => "Success",
            Self::Partial => "Partial Success",
            Self::Failure => "Failure",
            Self::CriticalFailure => "Critical Failure!",
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::CriticalSuccess | Self::Success | Self::Partial)
    }
}

impl std::fmt::Display for OutcomeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Use snake_case format for serialization/protocol consistency
        let name = match self {
            Self::CriticalSuccess => "critical_success",
            Self::Success => "success",
            Self::Partial => "partial",
            Self::Failure => "failure",
            Self::CriticalFailure => "critical_failure",
        };
        write!(f, "{}", name)
    }
}

// =============================================================================
// Edge Support Structs (Graph-First Design)
// =============================================================================

/// Data for REQUIRES_COMPLETION_OF edge between challenges
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengePrerequisite {
    /// The prerequisite challenge ID
    pub challenge_id: ChallengeId,
    /// Whether success is required (true = must succeed, false = just attempted)
    pub success_required: bool,
}

impl ChallengePrerequisite {
    pub fn new(challenge_id: ChallengeId) -> Self {
        Self {
            challenge_id,
            success_required: false,
        }
    }

    pub fn requiring_success(challenge_id: ChallengeId) -> Self {
        Self {
            challenge_id,
            success_required: true,
        }
    }
}

/// Data for AVAILABLE_AT edge between Challenge and Location
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeLocationAvailability {
    /// The location where this challenge is available
    pub location_id: LocationId,
    /// Whether the challenge is always available at this location
    pub always_available: bool,
    /// Time restriction (if any): "Morning", "Afternoon", "Evening", "Night"
    pub time_restriction: Option<String>,
}

impl ChallengeLocationAvailability {
    pub fn new(location_id: LocationId) -> Self {
        Self {
            location_id,
            always_available: true,
            time_restriction: None,
        }
    }

    pub fn with_time_restriction(mut self, time: impl Into<String>) -> Self {
        self.time_restriction = Some(time.into());
        self.always_available = false;
        self
    }
}

/// Data for AVAILABLE_AT_REGION edge between Challenge and Region
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeRegionAvailability {
    /// The region where this challenge is available
    pub region_id: RegionId,
    /// Whether the challenge is always available at this region
    pub always_available: bool,
    /// Time restriction (if any): "Morning", "Afternoon", "Evening", "Night"
    pub time_restriction: Option<String>,
}

impl ChallengeRegionAvailability {
    pub fn new(region_id: RegionId) -> Self {
        Self {
            region_id,
            always_available: true,
            time_restriction: None,
        }
    }

    pub fn with_time_restriction(mut self, time: impl Into<String>) -> Self {
        self.time_restriction = Some(time.into());
        self.always_available = false;
        self
    }
}

/// Data for ON_SUCCESS_UNLOCKS edge between Challenge and Location
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeUnlock {
    /// The location that gets unlocked on successful completion
    pub location_id: LocationId,
}

impl ChallengeUnlock {
    pub fn new(location_id: LocationId) -> Self {
        Self { location_id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_creation() {
        let world_id = WorldId::new();

        let challenge =
            Challenge::new(world_id, "Investigate the Statue", Difficulty::d20_medium())
                .with_description("Examine the ancient statue for hidden compartments")
                .with_outcomes(ChallengeOutcomes::simple(
                    "You find a hidden mechanism in the statue's base",
                    "The statue appears to be solid stone",
                ));

        assert_eq!(challenge.name(), "Investigate the Statue");
        assert!(challenge.active());
        assert_eq!(
            challenge.outcomes().success.description,
            "You find a hidden mechanism in the statue's base"
        );
    }

    #[test]
    fn test_trigger_condition_matching() {
        let trigger = TriggerCondition::new(
            TriggerType::object(["statue", "ancient", "stone"]),
            "When player examines the statue",
        );

        assert!(trigger.matches("I want to examine the statue", ""));
        assert!(trigger.matches("look at", "there is an ancient monument here"));
        assert!(!trigger.matches("I walk away", "there is a door"));
    }

    #[test]
    fn test_difficulty_display() {
        assert_eq!(Difficulty::DC(15).display(), "DC 15");
        assert_eq!(Difficulty::Percentage(45).display(), "45%");
        assert_eq!(
            Difficulty::Descriptor(DifficultyDescriptor::Hard).display(),
            "Hard"
        );
    }

    #[test]
    fn test_outcome_triggers() {
        let outcome = Outcome::new("You discover a secret passage!")
            .with_trigger(OutcomeTrigger::reveal_persistent("Map of the catacombs"))
            .with_trigger(OutcomeTrigger::enable(ChallengeId::new()));

        assert_eq!(outcome.triggers.len(), 2);
    }

    #[test]
    fn test_evaluate_roll_dc_success() {
        let world_id = WorldId::new();
        let challenge = Challenge::new(world_id, "Test", Difficulty::DC(15))
            .with_outcomes(ChallengeOutcomes::simple("Success!", "Failure!"));

        // Roll 10 + modifier 5 = 15, meets DC 15
        let (outcome_type, outcome) = challenge.evaluate_roll(10, 5);
        assert_eq!(outcome_type, OutcomeType::Success);
        assert_eq!(outcome.description, "Success!");

        // Roll 10 + modifier 3 = 13, below DC 15
        let (outcome_type, outcome) = challenge.evaluate_roll(10, 3);
        assert_eq!(outcome_type, OutcomeType::Failure);
        assert_eq!(outcome.description, "Failure!");
    }

    #[test]
    fn test_evaluate_roll_dc_critical() {
        let world_id = WorldId::new();
        let challenge = Challenge::new(world_id, "Test", Difficulty::DC(15)).with_outcomes(
            ChallengeOutcomes::simple("Success!", "Failure!")
                .with_critical_success("Critical!")
                .with_critical_failure("Fumble!"),
        );

        // Natural 20 = critical success
        let (outcome_type, outcome) = challenge.evaluate_roll(20, 0);
        assert_eq!(outcome_type, OutcomeType::CriticalSuccess);
        assert_eq!(outcome.description, "Critical!");

        // Natural 1 = critical failure
        let (outcome_type, outcome) = challenge.evaluate_roll(1, 10);
        assert_eq!(outcome_type, OutcomeType::CriticalFailure);
        assert_eq!(outcome.description, "Fumble!");
    }

    #[test]
    fn test_evaluate_roll_percentage() {
        let world_id = WorldId::new();
        let challenge = Challenge::new(world_id, "Test", Difficulty::Percentage(45))
            .with_outcomes(ChallengeOutcomes::simple("Success!", "Failure!"));

        // Roll 30 <= 45 = success (lower is better)
        let (outcome_type, _) = challenge.evaluate_roll(30, 0);
        assert_eq!(outcome_type, OutcomeType::Success);

        // Roll 50 > 45 = failure
        let (outcome_type, _) = challenge.evaluate_roll(50, 0);
        assert_eq!(outcome_type, OutcomeType::Failure);
    }

    #[test]
    fn test_evaluate_roll_descriptor() {
        let world_id = WorldId::new();
        let challenge = Challenge::new(
            world_id,
            "Test",
            Difficulty::Descriptor(DifficultyDescriptor::Moderate),
        )
        .with_outcomes(ChallengeOutcomes::simple("Success!", "Failure!").with_partial("Partial!"));

        // Roll 8 + modifier 3 = 11, >= 10 = full success
        let (outcome_type, _) = challenge.evaluate_roll(8, 3);
        assert_eq!(outcome_type, OutcomeType::Success);

        // Roll 5 + modifier 5 = 10, >= 10 = full success
        let (outcome_type, _) = challenge.evaluate_roll(5, 5);
        assert_eq!(outcome_type, OutcomeType::Success);

        // Roll 5 + modifier 3 = 8, 7-9 = partial success
        let (outcome_type, outcome) = challenge.evaluate_roll(5, 3);
        assert_eq!(outcome_type, OutcomeType::Partial);
        assert_eq!(outcome.description, "Partial!");

        // Roll 4 + modifier 3 = 7, 7-9 = partial success
        let (outcome_type, _) = challenge.evaluate_roll(4, 3);
        assert_eq!(outcome_type, OutcomeType::Partial);

        // Roll 3 + modifier 3 = 6, 6- = failure
        let (outcome_type, _) = challenge.evaluate_roll(3, 3);
        assert_eq!(outcome_type, OutcomeType::Failure);

        // Roll 2 + modifier 1 = 3, 6- = failure
        let (outcome_type, _) = challenge.evaluate_roll(2, 1);
        assert_eq!(outcome_type, OutcomeType::Failure);
    }

    #[test]
    fn test_difficulty_dice_suggestion() {
        assert_eq!(Difficulty::DC(15).dice_suggestion().0, "1d20");
        assert_eq!(Difficulty::Percentage(50).dice_suggestion().0, "1d100");
        assert_eq!(
            Difficulty::Descriptor(DifficultyDescriptor::Hard)
                .dice_suggestion()
                .0,
            "2d6"
        );
        assert_eq!(Difficulty::Opposed.dice_suggestion().0, "1d20");
    }

    #[test]
    fn test_difficulty_parse() {
        assert_eq!(Difficulty::parse("DC 15"), Difficulty::DC(15));
        assert_eq!(Difficulty::parse("DC15"), Difficulty::DC(15));
        assert_eq!(Difficulty::parse("dc 20"), Difficulty::DC(20));
        assert_eq!(Difficulty::parse("45%"), Difficulty::Percentage(45));
        assert_eq!(Difficulty::parse(" 80% "), Difficulty::Percentage(80));
        assert_eq!(
            Difficulty::parse("Very Hard"),
            Difficulty::Custom("Very Hard".to_string())
        );
    }

    #[test]
    fn test_evaluate_roll_narrative_pbta_custom_thresholds() {
        let world_id = WorldId::new();
        let challenge = Challenge::new(
            world_id,
            "Test",
            Difficulty::Descriptor(DifficultyDescriptor::Moderate),
        )
        .with_outcomes(ChallengeOutcomes::simple("Success!", "Failure!").with_partial("Partial!"));

        // Custom thresholds: 12+ success, 8+ partial
        let config = NarrativeResolutionConfig {
            style: NarrativeResolutionStyle::PbtA,
            thresholds: NarrativeThresholds {
                critical_success: None,
                full_success: 12,
                partial_success: 8,
                critical_failure: None,
            },
            ..Default::default()
        };

        // Roll 7 + 4 = 11, below 12 but >= 8 = partial with custom thresholds
        let (outcome_type, _) =
            challenge.evaluate_roll_narrative(7, 4, Some(&config), None, None, None);
        assert_eq!(outcome_type, OutcomeType::Partial);

        // Roll 8 + 4 = 12, >= 12 = success with custom thresholds
        let (outcome_type, _) =
            challenge.evaluate_roll_narrative(8, 4, Some(&config), None, None, None);
        assert_eq!(outcome_type, OutcomeType::Success);
    }

    #[test]
    fn test_evaluate_roll_narrative_ladder() {
        let world_id = WorldId::new();
        let challenge = Challenge::new(
            world_id,
            "Test",
            Difficulty::Descriptor(DifficultyDescriptor::Hard), // Hard = +4 on ladder
        )
        .with_outcomes(ChallengeOutcomes::simple("Success!", "Failure!").with_partial("Tie!"));

        let config = NarrativeResolutionConfig {
            style: NarrativeResolutionStyle::Ladder,
            ..Default::default()
        };

        // Roll 0 (4dF with all blanks) + modifier 4 = 4, equals Hard (+4) = tie = partial
        let (outcome_type, _) =
            challenge.evaluate_roll_narrative(0, 4, Some(&config), None, None, None);
        assert_eq!(outcome_type, OutcomeType::Partial);

        // Roll 0 + modifier 5 = 5, beats Hard (+4) by 1 = success
        let (outcome_type, _) =
            challenge.evaluate_roll_narrative(0, 5, Some(&config), None, None, None);
        assert_eq!(outcome_type, OutcomeType::Success);

        // Roll 0 + modifier 3 = 3, below Hard (+4) = failure
        let (outcome_type, _) =
            challenge.evaluate_roll_narrative(0, 3, Some(&config), None, None, None);
        assert_eq!(outcome_type, OutcomeType::Failure);
    }

    #[test]
    fn test_evaluate_roll_narrative_blades() {
        let world_id = WorldId::new();
        let challenge = Challenge::new(
            world_id,
            "Test",
            Difficulty::Descriptor(DifficultyDescriptor::Moderate),
        )
        .with_outcomes(ChallengeOutcomes::simple("Success!", "Failure!").with_partial("Partial!"));

        let config = NarrativeResolutionConfig {
            style: NarrativeResolutionStyle::Blades,
            ..Default::default()
        };

        // Single 6 = success
        let (outcome_type, _) = challenge.evaluate_roll_narrative(
            6,
            0,
            Some(&config),
            Some(Position::Risky),
            Some(EffectLevel::Standard),
            Some(&[6]),
        );
        assert_eq!(outcome_type, OutcomeType::Success);

        // 4 or 5 = partial
        let (outcome_type, _) = challenge.evaluate_roll_narrative(
            5,
            0,
            Some(&config),
            Some(Position::Risky),
            Some(EffectLevel::Standard),
            Some(&[5]),
        );
        assert_eq!(outcome_type, OutcomeType::Partial);

        // 1-3 = failure
        let (outcome_type, _) = challenge.evaluate_roll_narrative(
            3,
            0,
            Some(&config),
            Some(Position::Risky),
            Some(EffectLevel::Standard),
            Some(&[3]),
        );
        assert_eq!(outcome_type, OutcomeType::Failure);

        // Two 6s = critical
        let (outcome_type, _) = challenge.evaluate_roll_narrative(
            6,
            0,
            Some(&config),
            Some(Position::Risky),
            Some(EffectLevel::Standard),
            Some(&[6, 6]),
        );
        assert_eq!(outcome_type, OutcomeType::CriticalSuccess);
    }

    #[test]
    fn test_challenge_accessors() {
        let world_id = WorldId::new();
        let challenge = Challenge::new(world_id, "Test Challenge", Difficulty::DC(15))
            .with_description("A test")
            .with_tag("combat")
            .with_check_stat("STR")
            .with_active(false)
            .with_order(5)
            .with_is_favorite(true);

        assert_eq!(challenge.name(), "Test Challenge");
        assert_eq!(challenge.description(), "A test");
        assert_eq!(challenge.tags(), &["combat"]);
        assert_eq!(challenge.check_stat(), Some("STR"));
        assert!(!challenge.active());
        assert_eq!(challenge.order(), 5);
        assert!(challenge.is_favorite());
    }
}
