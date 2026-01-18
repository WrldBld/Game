//! Tactical gameplay components for skill checks and challenges.
//!
//! Includes challenge roll modals, skills display, and dice roll visualization.

pub mod challenge_roll;
pub mod skills_display;

pub use challenge_roll::ChallengeRollModal;
pub use skills_display::{PlayerSkillData, SkillsDisplay};
