//! Challenge state management using Dioxus signals
//!
//! Tracks active challenges, challenge results, and player skills.

use dioxus::prelude::*;

use crate::presentation::components::tactical::PlayerSkillData;

/// Roll submission status for challenge outcomes (P3.3/P3.4)
///
/// Tracks the state of a submitted roll as it goes through DM approval.
#[derive(Debug, Clone, PartialEq)]
pub enum RollSubmissionStatus {
    /// No roll has been submitted
    NotSubmitted,
    /// Roll submitted, waiting for DM approval
    AwaitingApproval {
        roll: i32,
        modifier: i32,
        total: i32,
        outcome_type: String,
    },
    /// Result received and ready to display
    ResultReady(ChallengeResultData),
    /// Result has been displayed, ready to close
    Dismissed,
}

impl Default for RollSubmissionStatus {
    fn default() -> Self {
        Self::NotSubmitted
    }
}

/// Challenge prompt data shown to player
#[derive(Debug, Clone, PartialEq)]
pub struct ChallengePromptData {
    /// Unique challenge ID
    pub challenge_id: String,
    /// Human-readable challenge name
    pub challenge_name: String,
    /// Associated skill name
    pub skill_name: String,
    /// Difficulty display (e.g., "DC 12", "Very Hard")
    pub difficulty_display: String,
    /// Challenge description/flavor text
    pub description: String,
    /// Character's skill modifier for this challenge
    pub character_modifier: i32,
    /// Suggested dice formula based on rule system (e.g., "1d20", "1d100", "2d6")
    pub suggested_dice: Option<String>,
    /// Human-readable hint about the rule system
    pub rule_system_hint: Option<String>,
}

/// Challenge result data for display
#[derive(Debug, Clone, PartialEq)]
pub struct ChallengeResultData {
    /// Challenge name
    pub challenge_name: String,
    /// Name of character who attempted the challenge
    pub character_name: String,
    /// The d20 roll result
    pub roll: i32,
    /// Applied modifier
    pub modifier: i32,
    /// Total result (roll + modifier)
    pub total: i32,
    /// Outcome type ("success", "failure", "critical_success", etc.)
    pub outcome: String,
    /// Descriptive outcome text
    pub outcome_description: String,
    /// Timestamp for ordering
    pub timestamp: u64,
    /// Roll breakdown string (e.g., "1d20(14) + 3 = 17" or "Manual: 18")
    pub roll_breakdown: Option<String>,
    /// Individual dice results if rolled with formula
    pub individual_rolls: Option<Vec<i32>>,
}

/// Challenge state for skill challenges
#[derive(Clone)]
pub struct ChallengeState {
    /// Active challenge prompt (if any)
    pub active_challenge: Signal<Option<ChallengePromptData>>,
    /// Recent challenge results for display
    pub challenge_results: Signal<Vec<ChallengeResultData>>,
    /// Player character skills with modifiers
    pub player_skills: Signal<Vec<PlayerSkillData>>,
    /// Roll submission status for the active challenge (P3.3/P3.4)
    pub roll_status: Signal<RollSubmissionStatus>,
}

impl ChallengeState {
    /// Create a new ChallengeState
    pub fn new() -> Self {
        Self {
            active_challenge: Signal::new(None),
            challenge_results: Signal::new(Vec::new()),
            player_skills: Signal::new(Vec::new()),
            roll_status: Signal::new(RollSubmissionStatus::default()),
        }
    }

    /// Set active challenge prompt
    pub fn set_active_challenge(&mut self, challenge: ChallengePromptData) {
        self.active_challenge.set(Some(challenge));
    }

    /// Clear active challenge
    pub fn clear_active_challenge(&mut self) {
        self.active_challenge.set(None);
    }

    /// Add a challenge result
    pub fn add_challenge_result(&mut self, result: ChallengeResultData) {
        self.challenge_results.write().push(result);
    }

    /// Set player skills
    pub fn set_player_skills(&mut self, skills: Vec<PlayerSkillData>) {
        self.player_skills.set(skills);
    }

    /// Add a player skill
    pub fn add_player_skill(&mut self, skill: PlayerSkillData) {
        self.player_skills.write().push(skill);
    }

    /// Clear all challenge state
    pub fn clear(&mut self) {
        self.active_challenge.set(None);
        self.challenge_results.set(Vec::new());
        self.player_skills.set(Vec::new());
        self.roll_status.set(RollSubmissionStatus::NotSubmitted);
    }

    /// Set roll as awaiting DM approval (P3.3/P3.4)
    pub fn set_awaiting_approval(&mut self, roll: i32, modifier: i32, total: i32, outcome_type: String) {
        self.roll_status.set(RollSubmissionStatus::AwaitingApproval {
            roll,
            modifier,
            total,
            outcome_type,
        });
    }

    /// Set result as ready to display (P3.3/P3.4)
    pub fn set_result_ready(&mut self, result: ChallengeResultData) {
        self.roll_status.set(RollSubmissionStatus::ResultReady(result));
    }

    /// Dismiss the result display (P3.3/P3.4)
    pub fn dismiss_result(&mut self) {
        self.roll_status.set(RollSubmissionStatus::Dismissed);
    }

    /// Clear the roll status (P3.3/P3.4)
    pub fn clear_roll_status(&mut self) {
        self.roll_status.set(RollSubmissionStatus::NotSubmitted);
    }
}

impl Default for ChallengeState {
    fn default() -> Self {
        Self::new()
    }
}
