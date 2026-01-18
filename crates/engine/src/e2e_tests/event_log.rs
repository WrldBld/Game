//! E2E Event Log for comprehensive test analysis.
//!
//! Captures all events, prompts, and responses during E2E tests for analysis,
//! debugging, and auditing decision flows.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wrldbldr_domain::WorldId;

use crate::infrastructure::ports::TokenUsage;

/// Maximum content length before truncation (10KB).
const MAX_CONTENT_LENGTH: usize = 10 * 1024;

// =============================================================================
// Event Types
// =============================================================================

/// All event types that can be logged during E2E tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum E2EEvent {
    // Session events
    SessionStarted {
        user_id: String,
        role: String,
    },
    SessionEnded {
        user_id: String,
    },

    // Queue operations
    ActionEnqueued {
        id: Uuid,
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
    },
    ActionProcessed {
        id: Uuid,
        duration_ms: u64,
    },
    LlmRequestEnqueued {
        id: Uuid,
        request_type: String,
        callback_id: String,
    },
    LlmRequestProcessed {
        id: Uuid,
        duration_ms: u64,
        tokens: Option<TokenUsageLog>,
    },
    ApprovalEnqueued {
        id: Uuid,
        decision_type: String,
        urgency: String,
    },
    ApprovalDecision {
        id: Uuid,
        decision: String,
        modified: bool,
        dm_feedback: Option<String>,
    },

    // LLM interactions (full capture)
    LlmPromptSent {
        request_id: Uuid,
        system_prompt: Option<String>,
        messages: Vec<ChatMessageLog>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        tools: Option<Vec<String>>,
    },
    LlmResponseReceived {
        request_id: Uuid,
        content: String,
        tool_calls: Vec<ToolCallLog>,
        finish_reason: String,
        tokens: Option<TokenUsageLog>,
        latency_ms: u64,
    },

    // Conversations
    ConversationStarted {
        id: Uuid,
        pc_id: String,
        npc_id: String,
        npc_name: String,
    },
    ConversationTurn {
        id: Uuid,
        speaker: String,
        content: String,
        turn_number: u32,
    },
    ConversationEnded {
        id: Uuid,
        total_turns: u32,
        summary: Option<String>,
    },

    // Challenges
    ChallengeTriggered {
        id: Option<Uuid>,
        name: String,
        target_pc: String,
        difficulty: i32,
    },
    ChallengeRoll {
        challenge_id: Uuid,
        roll: i32,
        modifier: i32,
        total: i32,
    },
    ChallengeOutcome {
        challenge_id: Uuid,
        success: bool,
        outcome_description: String,
    },

    // Staging
    StagingRequired {
        region_id: String,
        rule_based_npcs: Vec<String>,
        llm_suggested_npcs: Vec<String>,
    },
    StagingApproved {
        region_id: String,
        approved_npcs: Vec<String>,
        visual_state: Option<String>,
    },

    // Time
    TimeAdvanced {
        from: String,
        to: String,
        reason: String,
    },
    TimePaused {
        paused: bool,
    },

    // Narrative
    NarrativeEventTriggered {
        event_id: String,
        name: String,
        outcome: String,
    },

    // Errors
    Error {
        code: String,
        message: String,
        context: Option<serde_json::Value>,
    },
}

/// A chat message for logging (with truncation support).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageLog {
    pub role: String,
    pub content: String,
    pub truncated: bool,
}

impl ChatMessageLog {
    pub fn new(role: &str, content: &str) -> Self {
        let (content, truncated) = if content.len() > MAX_CONTENT_LENGTH {
            (
                format!("{}... [TRUNCATED]", &content[..MAX_CONTENT_LENGTH]),
                true,
            )
        } else {
            (content.to_string(), false)
        };

        Self {
            role: role.to_string(),
            content,
            truncated,
        }
    }
}

/// A tool call for logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallLog {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Token usage for logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageLog {
    pub prompt: u32,
    pub completion: u32,
    pub total: u32,
}

impl From<&TokenUsage> for TokenUsageLog {
    fn from(usage: &TokenUsage) -> Self {
        Self {
            prompt: usage.prompt_tokens,
            completion: usage.completion_tokens,
            total: usage.total_tokens,
        }
    }
}

// =============================================================================
// Event Log
// =============================================================================

/// A timestamped event in the log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampedEvent {
    pub timestamp: DateTime<Utc>,
    pub event: E2EEvent,
}

/// LLM metrics aggregated from events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmMetrics {
    pub calls: usize,
    pub total_prompt_tokens: u32,
    pub total_completion_tokens: u32,
    pub total_tokens: u32,
    pub total_latency_ms: u64,
}

impl LlmMetrics {
    pub fn avg_latency_ms(&self) -> f64 {
        if self.calls == 0 {
            0.0
        } else {
            self.total_latency_ms as f64 / self.calls as f64
        }
    }
}

/// Summary of the E2E test run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2ESummary {
    pub event_counts: HashMap<String, usize>,
    pub llm_calls: usize,
    pub total_tokens: TokenUsageLog,
    pub avg_llm_latency_ms: f64,
    pub conversations_count: usize,
    pub challenges_count: usize,
    pub errors_count: usize,
}

/// Test outcome.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestOutcome {
    Pass,
    Fail,
}

/// The complete E2E event log for a test run.
#[derive(Debug, Serialize, Deserialize)]
pub struct E2EEventLogData {
    pub version: String,
    pub test_name: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub world_id: Option<String>,
    pub cassette_path: Option<String>,
    pub outcome: Option<TestOutcome>,
    pub events: Vec<TimestampedEvent>,
    pub summary: Option<E2ESummary>,
}

/// Thread-safe E2E event log.
pub struct E2EEventLog {
    data: Mutex<E2EEventLogData>,
    start_instant: Instant,
}

impl E2EEventLog {
    /// Create a new event log for a test.
    pub fn new(test_name: &str) -> Self {
        Self {
            data: Mutex::new(E2EEventLogData {
                version: "1.0".to_string(),
                test_name: test_name.to_string(),
                started_at: Utc::now(),
                ended_at: None,
                world_id: None,
                cassette_path: None,
                outcome: None,
                events: Vec::new(),
                summary: None,
            }),
            start_instant: Instant::now(),
        }
    }

    /// Set the world ID for this test.
    pub fn set_world_id(&self, world_id: WorldId) {
        let mut data = self.data.lock().unwrap();
        data.world_id = Some(world_id.to_string());
    }

    /// Set the cassette path for this test.
    pub fn set_cassette_path(&self, path: &str) {
        let mut data = self.data.lock().unwrap();
        data.cassette_path = Some(path.to_string());
    }

    /// Log an event.
    pub fn log(&self, event: E2EEvent) {
        let mut data = self.data.lock().unwrap();
        data.events.push(TimestampedEvent {
            timestamp: Utc::now(),
            event,
        });
    }

    /// Finalize the log with outcome and generate summary.
    pub fn finalize(&self, outcome: TestOutcome) {
        let mut data = self.data.lock().unwrap();
        data.ended_at = Some(Utc::now());
        data.outcome = Some(outcome);
        data.summary = Some(Self::generate_summary(&data.events));
    }

    /// Generate a summary from events.
    fn generate_summary(events: &[TimestampedEvent]) -> E2ESummary {
        let mut event_counts: HashMap<String, usize> = HashMap::new();
        let mut llm_metrics = LlmMetrics::default();
        let mut conversations_count = 0;
        let mut challenges_count = 0;
        let mut errors_count = 0;

        for timestamped in events {
            // Count by event type
            let type_name = match &timestamped.event {
                E2EEvent::SessionStarted { .. } => "SessionStarted",
                E2EEvent::SessionEnded { .. } => "SessionEnded",
                E2EEvent::ActionEnqueued { .. } => "ActionEnqueued",
                E2EEvent::ActionProcessed { .. } => "ActionProcessed",
                E2EEvent::LlmRequestEnqueued { .. } => "LlmRequestEnqueued",
                E2EEvent::LlmRequestProcessed { .. } => "LlmRequestProcessed",
                E2EEvent::ApprovalEnqueued { .. } => "ApprovalEnqueued",
                E2EEvent::ApprovalDecision { .. } => "ApprovalDecision",
                E2EEvent::LlmPromptSent { .. } => "LlmPromptSent",
                E2EEvent::LlmResponseReceived { .. } => "LlmResponseReceived",
                E2EEvent::ConversationStarted { .. } => "ConversationStarted",
                E2EEvent::ConversationTurn { .. } => "ConversationTurn",
                E2EEvent::ConversationEnded { .. } => "ConversationEnded",
                E2EEvent::ChallengeTriggered { .. } => "ChallengeTriggered",
                E2EEvent::ChallengeRoll { .. } => "ChallengeRoll",
                E2EEvent::ChallengeOutcome { .. } => "ChallengeOutcome",
                E2EEvent::StagingRequired { .. } => "StagingRequired",
                E2EEvent::StagingApproved { .. } => "StagingApproved",
                E2EEvent::TimeAdvanced { .. } => "TimeAdvanced",
                E2EEvent::TimePaused { .. } => "TimePaused",
                E2EEvent::NarrativeEventTriggered { .. } => "NarrativeEventTriggered",
                E2EEvent::Error { .. } => "Error",
            };
            *event_counts.entry(type_name.to_string()).or_insert(0) += 1;

            // Aggregate metrics
            match &timestamped.event {
                E2EEvent::LlmResponseReceived {
                    tokens, latency_ms, ..
                } => {
                    llm_metrics.calls += 1;
                    llm_metrics.total_latency_ms += latency_ms;
                    if let Some(t) = tokens {
                        llm_metrics.total_prompt_tokens += t.prompt;
                        llm_metrics.total_completion_tokens += t.completion;
                        llm_metrics.total_tokens += t.total;
                    }
                }
                E2EEvent::ConversationStarted { .. } => {
                    conversations_count += 1;
                }
                E2EEvent::ChallengeTriggered { .. } => {
                    challenges_count += 1;
                }
                E2EEvent::Error { .. } => {
                    errors_count += 1;
                }
                _ => {}
            }
        }

        E2ESummary {
            event_counts,
            llm_calls: llm_metrics.calls,
            total_tokens: TokenUsageLog {
                prompt: llm_metrics.total_prompt_tokens,
                completion: llm_metrics.total_completion_tokens,
                total: llm_metrics.total_tokens,
            },
            avg_llm_latency_ms: llm_metrics.avg_latency_ms(),
            conversations_count,
            challenges_count,
            errors_count,
        }
    }

    /// Save the event log to a file.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let data = self.data.lock().unwrap();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&*data)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        fs::write(path, json)?;

        tracing::info!(
            path = ?path,
            events = data.events.len(),
            "E2E event log saved"
        );

        Ok(())
    }

    /// Get a copy of the summary (generates if not finalized).
    pub fn summary(&self) -> E2ESummary {
        let data = self.data.lock().unwrap();
        data.summary
            .clone()
            .unwrap_or_else(|| Self::generate_summary(&data.events))
    }

    /// Get elapsed time since test start in milliseconds.
    pub fn elapsed_ms(&self) -> u64 {
        self.start_instant.elapsed().as_millis() as u64
    }

    /// Get the event count.
    pub fn event_count(&self) -> usize {
        self.data.lock().unwrap().events.len()
    }
}

/// Shared reference to an event log.
pub type SharedEventLog = Arc<Mutex<E2EEventLog>>;

/// Create a new shared event log.
pub fn create_shared_log(test_name: &str) -> Arc<E2EEventLog> {
    Arc::new(E2EEventLog::new(test_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_log_creation() {
        let log = E2EEventLog::new("test_example");

        log.log(E2EEvent::SessionStarted {
            user_id: "user-123".to_string(),
            role: "player".to_string(),
        });

        log.log(E2EEvent::ConversationStarted {
            id: Uuid::new_v4(),
            pc_id: "pc-1".to_string(),
            npc_id: "npc-1".to_string(),
            npc_name: "Marta".to_string(),
        });

        assert_eq!(log.event_count(), 2);
    }

    #[test]
    fn test_summary_generation() {
        let log = E2EEventLog::new("test_summary");

        // Add some events
        log.log(E2EEvent::ConversationStarted {
            id: Uuid::new_v4(),
            pc_id: "pc-1".to_string(),
            npc_id: "npc-1".to_string(),
            npc_name: "Marta".to_string(),
        });

        log.log(E2EEvent::LlmResponseReceived {
            request_id: Uuid::new_v4(),
            content: "Hello!".to_string(),
            tool_calls: vec![],
            finish_reason: "stop".to_string(),
            tokens: Some(TokenUsageLog {
                prompt: 100,
                completion: 50,
                total: 150,
            }),
            latency_ms: 500,
        });

        log.log(E2EEvent::Error {
            code: "TEST_ERROR".to_string(),
            message: "Test error".to_string(),
            context: None,
        });

        log.finalize(TestOutcome::Pass);

        let summary = log.summary();
        assert_eq!(summary.conversations_count, 1);
        assert_eq!(summary.llm_calls, 1);
        assert_eq!(summary.total_tokens.total, 150);
        assert_eq!(summary.errors_count, 1);
    }

    #[test]
    fn test_chat_message_truncation() {
        let long_content = "x".repeat(20000);
        let msg = ChatMessageLog::new("user", &long_content);

        assert!(msg.truncated);
        assert!(msg.content.len() < long_content.len());
        assert!(msg.content.ends_with("[TRUNCATED]"));
    }

    #[test]
    fn test_event_serialization() {
        let event = E2EEvent::LlmPromptSent {
            request_id: Uuid::nil(),
            system_prompt: Some("You are an NPC".to_string()),
            messages: vec![ChatMessageLog::new("user", "Hello")],
            temperature: Some(0.7),
            max_tokens: Some(500),
            tools: Some(vec!["trigger_challenge".to_string()]),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("LlmPromptSent"));
        assert!(json.contains("You are an NPC"));
    }
}
