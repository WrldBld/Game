//! World state value objects for conversation history and pending approvals.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single entry in the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConversationEntry {
    /// When this conversation occurred
    pub timestamp: DateTime<Utc>,
    /// Who spoke
    pub speaker: Speaker,
    /// What was said
    pub message: String,
}

impl ConversationEntry {
    /// Create a new conversation entry with the specified timestamp.
    ///
    /// # Hexagonal Architecture Note
    /// Timestamp is injected rather than using direct time sources to keep domain pure.
    /// Call sites should use `clock_port.now()` to get the current time.
    pub fn new(speaker: Speaker, message: String, now: DateTime<Utc>) -> Self {
        Self {
            timestamp: now,
            speaker,
            message,
        }
    }

    // ── Factory Methods ──────────────────────────────────────────────────

    /// Create a player conversation entry.
    pub fn player(pc_id: String, pc_name: String, message: String, now: DateTime<Utc>) -> Self {
        Self::new(Speaker::Player { pc_id, pc_name }, message, now)
    }

    /// Create an NPC conversation entry.
    pub fn npc(npc_id: String, npc_name: String, message: String, now: DateTime<Utc>) -> Self {
        Self::new(Speaker::Npc { npc_id, npc_name }, message, now)
    }

    /// Create a system message entry.
    pub fn system(message: String, now: DateTime<Utc>) -> Self {
        Self::new(Speaker::System, message, now)
    }

    /// Create a DM message entry.
    pub fn dm(message: String, now: DateTime<Utc>) -> Self {
        Self::new(Speaker::Dm, message, now)
    }
}

/// Who is speaking in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Speaker {
    /// A player character speaking
    Player {
        /// The PC's unique identifier
        pc_id: String,
        /// The PC's display name
        pc_name: String,
    },
    /// An NPC speaking
    Npc {
        /// The NPC's unique identifier
        npc_id: String,
        /// The NPC's display name
        npc_name: String,
    },
    /// A system message
    System,
    /// The Dungeon Master
    Dm,
}

/// An item awaiting DM approval.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingApprovalItem {
    /// Unique identifier for this approval request
    pub approval_id: String,
    /// What type of approval is needed
    pub approval_type: ApprovalType,
    /// When this approval was requested
    pub created_at: DateTime<Utc>,
    /// Additional data specific to the approval type
    pub data: String,
}

impl PendingApprovalItem {
    /// Create a new pending approval item with the specified timestamp.
    ///
    /// # Hexagonal Architecture Note
    /// Timestamp is injected rather than using direct time sources to keep domain pure.
    /// Call sites should use `clock_port.now()` to get the current time.
    pub fn new(
        approval_id: String,
        approval_type: ApprovalType,
        data: String,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            approval_id,
            approval_type,
            created_at: now,
            data,
        }
    }
}

/// Categories of items that can require DM approval.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ApprovalType {
    /// Dialogue content needs approval
    Dialogue,
    /// A challenge attempt needs approval
    Challenge,
    /// A narrative event needs approval
    NarrativeEvent,
    /// The outcome of a challenge needs approval
    ChallengeOutcome,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_time() -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn test_conversation_entry_constructors() {
        let now = fixed_time();

        let player = ConversationEntry::player("pc1".into(), "Hero".into(), "Hello!".into(), now);
        assert!(matches!(player.speaker, Speaker::Player { .. }));

        let npc = ConversationEntry::npc("npc1".into(), "Merchant".into(), "Welcome!".into(), now);
        assert!(matches!(npc.speaker, Speaker::Npc { .. }));

        let system = ConversationEntry::system("Game saved.".into(), now);
        assert!(matches!(system.speaker, Speaker::System));

        let dm = ConversationEntry::dm("Roll for initiative.".into(), now);
        assert!(matches!(dm.speaker, Speaker::Dm));
    }

    #[test]
    fn test_speaker_variants() {
        let player = Speaker::Player {
            pc_id: "pc1".into(),
            pc_name: "Hero".into(),
        };
        assert!(matches!(player, Speaker::Player { .. }));

        let npc = Speaker::Npc {
            npc_id: "npc1".into(),
            npc_name: "Merchant".into(),
        };
        assert!(matches!(npc, Speaker::Npc { .. }));
    }

    #[test]
    fn test_approval_type_variants() {
        let dialogue = ApprovalType::Dialogue;
        assert!(matches!(dialogue, ApprovalType::Dialogue));

        let challenge = ApprovalType::ChallengeOutcome;
        assert!(matches!(challenge, ApprovalType::ChallengeOutcome));
    }
}
