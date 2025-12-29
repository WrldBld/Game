//! Player port definitions and shared cross-layer types.

pub mod session_types;

// Re-export session types at crate root for convenience
pub use session_types::{
    ParticipantRole, DiceInput, ApprovalDecision, DirectorialContext,
    NpcMotivationData, ApprovedNpcInfo, AdHocOutcomes, ChallengeOutcomeDecision,
};

pub mod config {
    use std::str::FromStr;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ShellKind {
        Desktop,
        Mobile,
    }

    impl Default for ShellKind {
        fn default() -> Self {
            Self::Desktop
        }
    }

    impl FromStr for ShellKind {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s.trim().to_ascii_lowercase().as_str() {
                "desktop" => Ok(Self::Desktop),
                "mobile" => Ok(Self::Mobile),
                other => Err(format!("unknown shell kind: {other}")),
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct RunnerConfig {
        pub shell: ShellKind,
    }

    impl Default for RunnerConfig {
        fn default() -> Self {
            Self {
                shell: ShellKind::default(),
            }
        }
    }
}

pub mod outbound;
