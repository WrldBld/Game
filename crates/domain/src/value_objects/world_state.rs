//! World state value objects for conversation history and pending approvals.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single entry in the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationEntry {
    /// When this conversation occurred
    pub timestamp: DateTime<Utc>,
    /// Who spoke
    pub speaker: Speaker,
    /// What was said
    pub message: String,
}

impl ConversationEntry {
    /// Create a new conversation entry with the current timestamp.
    pub fn new(speaker: Speaker, message: String) -> Self {
        Self {
            timestamp: Utc::now(),
            speaker,
            message,
        }
    }

    /// Create a player conversation entry.
    pub fn player(pc_id: String, pc_name: String, message: String) -> Self {
        Self::new(Speaker::Player { pc_id, pc_name }, message)
    }

    /// Create an NPC conversation entry.
    pub fn npc(npc_id: String, npc_name: String, message: String) -> Self {
        Self::new(Speaker::Npc { npc_id, npc_name }, message)
    }

    /// Create a system message entry.
    pub fn system(message: String) -> Self {
        Self::new(Speaker::System, message)
    }

    /// Create a DM message entry.
    pub fn dm(message: String) -> Self {
        Self::new(Speaker::Dm, message)
    }
}

/// Who is speaking in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
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
#[serde(rename_all = "camelCase")]
pub struct PendingApprovalItem {
    /// Unique identifier for this approval request
    pub approval_id: String,
    /// What type of approval is needed
    pub approval_type: ApprovalType,
    /// When this approval was requested
    pub created_at: DateTime<Utc>,
    /// Additional data specific to the approval type
    pub data: serde_json::Value,
}

impl PendingApprovalItem {
    /// Create a new pending approval item with the current timestamp.
    pub fn new(approval_id: String, approval_type: ApprovalType, data: serde_json::Value) -> Self {
        Self {
            approval_id,
            approval_type,
            created_at: Utc::now(),
            data,
        }
    }
}

/// Categories of items that can require DM approval.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
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

    #[test]
    fn test_conversation_entry_constructors() {
        let player = ConversationEntry::player("pc1".into(), "Hero".into(), "Hello!".into());
        assert!(matches!(player.speaker, Speaker::Player { .. }));

        let npc = ConversationEntry::npc("npc1".into(), "Merchant".into(), "Welcome!".into());
        assert!(matches!(npc.speaker, Speaker::Npc { .. }));

        let system = ConversationEntry::system("Game saved.".into());
        assert!(matches!(system.speaker, Speaker::System));

        let dm = ConversationEntry::dm("Roll for initiative.".into());
        assert!(matches!(dm.speaker, Speaker::Dm));
    }

    #[test]
    fn test_speaker_serialization() {
        let player = Speaker::Player {
            pc_id: "pc1".into(),
            pc_name: "Hero".into(),
        };
        let json = serde_json::to_string(&player).unwrap();
        assert!(json.contains("\"type\":\"player\""));

        let npc = Speaker::Npc {
            npc_id: "npc1".into(),
            npc_name: "Merchant".into(),
        };
        let json = serde_json::to_string(&npc).unwrap();
        assert!(json.contains("\"type\":\"npc\""));
    }

    #[test]
    fn test_approval_type_serialization() {
        let dialogue = ApprovalType::Dialogue;
        let json = serde_json::to_string(&dialogue).unwrap();
        assert_eq!(json, "\"dialogue\"");

        let challenge = ApprovalType::ChallengeOutcome;
        let json = serde_json::to_string(&challenge).unwrap();
        assert_eq!(json, "\"challenge_outcome\"");
    }
}
