//! Generation batch status enumeration

use serde::{Deserialize, Serialize};

/// Status of a generation batch
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BatchStatus {
    /// Waiting in queue to be processed
    Queued,
    /// Currently being generated
    Generating {
        /// Progress 0-100
        progress: u8,
    },
    /// Generation complete, awaiting user selection
    ReadyForSelection,
    /// User has selected assets, batch is complete
    Completed,
    /// Generation failed
    Failed { error: String },
}

impl BatchStatus {
    /// Check if this is a terminal state (no further transitions expected)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed { .. })
    }

    /// Check if generation is actively running
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Generating { .. })
    }

    /// Check if waiting in queue
    pub fn is_queued(&self) -> bool {
        matches!(self, Self::Queued)
    }

    /// Check if ready for user selection
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::ReadyForSelection)
    }
}

impl std::fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "Queued"),
            Self::Generating { progress } => write!(f, "Generating ({}%)", progress),
            Self::ReadyForSelection => write!(f, "Ready"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed { error } => write!(f, "Failed: {}", error),
        }
    }
}
