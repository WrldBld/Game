//! VCR-style LLM recording and playback for E2E tests.
//!
//! Records real LLM responses during test development, then replays them
//! for deterministic, fast test execution without requiring a live LLM.
//!
//! # Usage
//!
//! ```bash
//! # Record cassettes with real Ollama
//! E2E_LLM_MODE=record cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored
//!
//! # Playback from cassettes (fast, deterministic)
//! cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored
//!
//! # Live mode (always call LLM, no recording)
//! E2E_LLM_MODE=live cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored
//! ```

use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::infrastructure::ollama::OllamaClient;
use crate::infrastructure::ports::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, MessageRole, ToolCall, ToolDefinition,
};

use super::event_log::{ChatMessageLog, E2EEvent, E2EEventLog, TokenUsageLog, ToolCallLog};

/// Recording mode for VCR LLM.
#[derive(Debug, Clone)]
pub enum VcrMode {
    /// Call real LLM, record responses to cassette file.
    Record,
    /// Load responses from cassette, no LLM calls.
    Playback,
    /// Call real LLM without recording.
    Live,
}

/// A single recorded LLM interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRecording {
    /// Index in the recording sequence.
    pub index: usize,
    /// Summary of the request for debugging.
    pub request_summary: String,
    /// The LLM response.
    pub response: RecordedResponse,
    /// When this was recorded.
    pub recorded_at: String,
}

/// Recorded LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedResponse {
    pub content: String,
    pub tool_calls: Vec<RecordedToolCall>,
    pub finish_reason: String,
}

/// Recorded tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Cassette file format containing recorded LLM interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCassette {
    pub version: String,
    pub recorded_at: String,
    pub llm_model: String,
    pub recordings: Vec<LlmRecording>,
}

impl Default for LlmCassette {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            recorded_at: chrono::Utc::now().to_rfc3339(),
            llm_model: "unknown".to_string(),
            recordings: Vec::new(),
        }
    }
}

/// VCR-enabled LLM wrapper that can record and playback LLM responses.
pub struct VcrLlm {
    /// Real LLM client (None in playback mode).
    inner: Option<Arc<dyn LlmPort>>,
    /// Path to cassette file.
    cassette_path: PathBuf,
    /// Current mode.
    mode: VcrMode,
    /// Cassette data (recordings).
    cassette: Mutex<LlmCassette>,
    /// Playback queue for sequential playback.
    playback_queue: Mutex<VecDeque<LlmRecording>>,
    /// Model name for recording.
    model_name: String,
    /// Optional event log for comprehensive logging.
    event_log: Option<Arc<E2EEventLog>>,
}

impl VcrLlm {
    /// Create in recording mode with real Ollama client.
    pub fn recording(inner: Arc<dyn LlmPort>, cassette_path: PathBuf, model_name: &str) -> Self {
        Self {
            inner: Some(inner),
            cassette_path,
            mode: VcrMode::Record,
            cassette: Mutex::new(LlmCassette {
                version: "1.0".to_string(),
                recorded_at: chrono::Utc::now().to_rfc3339(),
                llm_model: model_name.to_string(),
                recordings: Vec::new(),
            }),
            playback_queue: Mutex::new(VecDeque::new()),
            model_name: model_name.to_string(),
            event_log: None,
        }
    }

    /// Create in playback mode from existing cassette file.
    pub fn playback(cassette_path: PathBuf) -> Result<Self, std::io::Error> {
        let content = fs::read_to_string(&cassette_path)?;
        let cassette: LlmCassette =
            serde_json::from_str(&content).map_err(|e| std::io::Error::other(e.to_string()))?;

        let playback_queue: VecDeque<_> = cassette.recordings.clone().into_iter().collect();

        Ok(Self {
            inner: None,
            cassette_path,
            mode: VcrMode::Playback,
            cassette: Mutex::new(cassette),
            playback_queue: Mutex::new(playback_queue),
            model_name: String::new(),
            event_log: None,
        })
    }

    /// Create in live mode (always calls LLM, no recording).
    pub fn live(inner: Arc<dyn LlmPort>) -> Self {
        Self {
            inner: Some(inner),
            cassette_path: PathBuf::new(),
            mode: VcrMode::Live,
            cassette: Mutex::new(LlmCassette::default()),
            playback_queue: Mutex::new(VecDeque::new()),
            model_name: String::new(),
            event_log: None,
        }
    }

    /// Attach an event log for comprehensive logging.
    pub fn with_event_log(mut self, event_log: Arc<E2EEventLog>) -> Self {
        self.event_log = Some(event_log);
        self
    }

    /// Create VcrLlm based on E2E_LLM_MODE environment variable.
    ///
    /// - `record`: Call real Ollama, save responses to cassette
    /// - `playback` or unset: Load from cassette (falls back to record if missing)
    /// - `live`: Call real Ollama without recording
    pub fn from_env(cassette_path: PathBuf) -> Self {
        let ollama_url =
            std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());

        match std::env::var("E2E_LLM_MODE").as_deref() {
            Ok("record") => {
                tracing::info!(cassette = ?cassette_path, "VcrLlm: Recording mode");
                let client = Arc::new(OllamaClient::new(&ollama_url, &model));
                Self::recording(client, cassette_path, &model)
            }
            Ok("live") => {
                tracing::info!("VcrLlm: Live mode (no recording)");
                let client = Arc::new(OllamaClient::new(&ollama_url, &model));
                Self::live(client)
            }
            _ => {
                // Default: try playback, fall back to recording if cassette doesn't exist
                match Self::playback(cassette_path.clone()) {
                    Ok(vcr) => {
                        tracing::info!(cassette = ?cassette_path, "VcrLlm: Playback mode");
                        vcr
                    }
                    Err(_) => {
                        tracing::info!(
                            cassette = ?cassette_path,
                            "VcrLlm: Cassette not found, falling back to recording mode"
                        );
                        let client = Arc::new(OllamaClient::new(&ollama_url, &model));
                        Self::recording(client, cassette_path, &model)
                    }
                }
            }
        }
    }

    /// Save cassette to disk. Call this at the end of recording.
    pub fn save_cassette(&self) -> Result<(), std::io::Error> {
        if !matches!(self.mode, VcrMode::Record) {
            return Ok(()); // Only save in record mode
        }

        let cassette = self.cassette.lock().unwrap();
        if cassette.recordings.is_empty() {
            tracing::warn!("VcrLlm: No recordings to save");
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = self.cassette_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&*cassette)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        fs::write(&self.cassette_path, json)?;

        tracing::info!(
            cassette = ?self.cassette_path,
            recordings = cassette.recordings.len(),
            "VcrLlm: Saved cassette"
        );
        Ok(())
    }

    /// Record a request/response pair.
    fn record(&self, request: &LlmRequest, response: &LlmResponse) {
        let mut cassette = self.cassette.lock().unwrap();
        let index = cassette.recordings.len();

        // Build request summary for debugging
        let request_summary = format!(
            "System: {}... | User: {}...",
            request
                .system_prompt
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(50)
                .collect::<String>(),
            request
                .messages
                .last()
                .map(|m| m.content.chars().take(50).collect::<String>())
                .unwrap_or_default()
        );

        let recording = LlmRecording {
            index,
            request_summary,
            response: RecordedResponse {
                content: response.content.clone(),
                tool_calls: response
                    .tool_calls
                    .iter()
                    .map(|tc| RecordedToolCall {
                        id: tc.id.clone(),
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                    })
                    .collect(),
                finish_reason: format!("{:?}", response.finish_reason),
            },
            recorded_at: chrono::Utc::now().to_rfc3339(),
        };

        cassette.recordings.push(recording);
    }

    /// Get next recorded response for playback.
    fn playback_next(&self) -> Result<LlmResponse, LlmError> {
        let mut queue = self.playback_queue.lock().unwrap();
        let recording = queue.pop_front().ok_or_else(|| {
            LlmError::RequestFailed("VcrLlm: No more recorded responses in cassette".to_string())
        })?;

        Ok(LlmResponse {
            content: recording.response.content,
            tool_calls: recording
                .response
                .tool_calls
                .into_iter()
                .map(|tc| ToolCall {
                    id: tc.id,
                    name: tc.name,
                    arguments: tc.arguments,
                })
                .collect(),
            finish_reason: match recording.response.finish_reason.as_str() {
                "Stop" => FinishReason::Stop,
                "Length" => FinishReason::Length,
                "ToolCalls" => FinishReason::ToolCalls,
                "ContentFilter" => FinishReason::ContentFilter,
                _ => FinishReason::Stop,
            },
            usage: None,
        })
    }

    /// Get the current mode.
    pub fn mode(&self) -> &VcrMode {
        &self.mode
    }

    /// Get the number of recordings.
    pub fn recording_count(&self) -> usize {
        self.cassette.lock().unwrap().recordings.len()
    }

    /// Get the number of remaining playback items.
    pub fn playback_remaining(&self) -> usize {
        self.playback_queue.lock().unwrap().len()
    }

    /// Log prompt sent event to the event log.
    fn log_prompt_sent(&self, request_id: Uuid, request: &LlmRequest, tools: Option<&[ToolDefinition]>) {
        if let Some(ref event_log) = self.event_log {
            let messages: Vec<ChatMessageLog> = request
                .messages
                .iter()
                .map(|m| {
                    let role = match m.role {
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::System => "system",
                        MessageRole::Unknown => "unknown",
                    };
                    ChatMessageLog::new(role, &m.content)
                })
                .collect();

            let tool_names = tools.map(|t| t.iter().map(|td| td.name.clone()).collect());

            event_log.log(E2EEvent::LlmPromptSent {
                request_id,
                system_prompt: request.system_prompt.clone(),
                messages,
                temperature: request.temperature,
                max_tokens: request.max_tokens,
                tools: tool_names,
            });
        }
    }

    /// Log response received event to the event log.
    fn log_response_received(&self, request_id: Uuid, response: &LlmResponse, latency_ms: u64) {
        if let Some(ref event_log) = self.event_log {
            let tool_calls: Vec<ToolCallLog> = response
                .tool_calls
                .iter()
                .map(|tc| ToolCallLog {
                    name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                })
                .collect();

            let tokens = response.usage.as_ref().map(TokenUsageLog::from);

            event_log.log(E2EEvent::LlmResponseReceived {
                request_id,
                content: response.content.clone(),
                tool_calls,
                finish_reason: format!("{:?}", response.finish_reason),
                tokens,
                latency_ms,
            });
        }
    }
}

#[async_trait]
impl LlmPort for VcrLlm {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let request_id = Uuid::new_v4();
        self.log_prompt_sent(request_id, &request, None);
        let start = Instant::now();

        let response = match &self.mode {
            VcrMode::Record => {
                let inner = self.inner.as_ref().ok_or_else(|| {
                    LlmError::RequestFailed("VcrLlm: No inner LLM in record mode".to_string())
                })?;
                let response = inner.generate(request.clone()).await?;
                self.record(&request, &response);
                response
            }
            VcrMode::Playback => self.playback_next()?,
            VcrMode::Live => {
                let inner = self.inner.as_ref().ok_or_else(|| {
                    LlmError::RequestFailed("VcrLlm: No inner LLM in live mode".to_string())
                })?;
                inner.generate(request).await?
            }
        };

        self.log_response_received(request_id, &response, start.elapsed().as_millis() as u64);
        Ok(response)
    }

    async fn generate_with_tools(
        &self,
        request: LlmRequest,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, LlmError> {
        let request_id = Uuid::new_v4();
        self.log_prompt_sent(request_id, &request, Some(&tools));
        let start = Instant::now();

        let response = match &self.mode {
            VcrMode::Record => {
                let inner = self.inner.as_ref().ok_or_else(|| {
                    LlmError::RequestFailed("VcrLlm: No inner LLM in record mode".to_string())
                })?;
                let response = inner.generate_with_tools(request.clone(), tools).await?;
                self.record(&request, &response);
                response
            }
            VcrMode::Playback => self.playback_next()?,
            VcrMode::Live => {
                let inner = self.inner.as_ref().ok_or_else(|| {
                    LlmError::RequestFailed("VcrLlm: No inner LLM in live mode".to_string())
                })?;
                inner.generate_with_tools(request, tools).await?
            }
        };

        self.log_response_received(request_id, &response, start.elapsed().as_millis() as u64);
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::ChatMessage;
    use tempfile::TempDir;

    #[test]
    fn test_cassette_serialization() {
        let cassette = LlmCassette {
            version: "1.0".to_string(),
            recorded_at: "2026-01-12T10:00:00Z".to_string(),
            llm_model: "test-model".to_string(),
            recordings: vec![LlmRecording {
                index: 0,
                request_summary: "Test request".to_string(),
                response: RecordedResponse {
                    content: "Test response".to_string(),
                    tool_calls: vec![],
                    finish_reason: "Stop".to_string(),
                },
                recorded_at: "2026-01-12T10:00:01Z".to_string(),
            }],
        };

        let json = serde_json::to_string_pretty(&cassette).unwrap();
        let parsed: LlmCassette = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.version, "1.0");
        assert_eq!(parsed.recordings.len(), 1);
        assert_eq!(parsed.recordings[0].response.content, "Test response");
    }

    #[tokio::test]
    async fn test_playback_mode() {
        let temp_dir = TempDir::new().unwrap();
        let cassette_path = temp_dir.path().join("test.json");

        // Create a cassette file
        let cassette = LlmCassette {
            version: "1.0".to_string(),
            recorded_at: "2026-01-12T10:00:00Z".to_string(),
            llm_model: "test".to_string(),
            recordings: vec![
                LlmRecording {
                    index: 0,
                    request_summary: "First".to_string(),
                    response: RecordedResponse {
                        content: "First response".to_string(),
                        tool_calls: vec![],
                        finish_reason: "Stop".to_string(),
                    },
                    recorded_at: "2026-01-12T10:00:01Z".to_string(),
                },
                LlmRecording {
                    index: 1,
                    request_summary: "Second".to_string(),
                    response: RecordedResponse {
                        content: "Second response".to_string(),
                        tool_calls: vec![],
                        finish_reason: "Stop".to_string(),
                    },
                    recorded_at: "2026-01-12T10:00:02Z".to_string(),
                },
            ],
        };

        fs::write(&cassette_path, serde_json::to_string(&cassette).unwrap()).unwrap();

        // Load in playback mode
        let vcr = VcrLlm::playback(cassette_path).unwrap();

        assert_eq!(vcr.playback_remaining(), 2);

        // First request
        let request = LlmRequest::new(vec![ChatMessage::user("Hello")]);
        let response = vcr.generate(request).await.unwrap();
        assert_eq!(response.content, "First response");
        assert_eq!(vcr.playback_remaining(), 1);

        // Second request
        let request = LlmRequest::new(vec![ChatMessage::user("World")]);
        let response = vcr.generate(request).await.unwrap();
        assert_eq!(response.content, "Second response");
        assert_eq!(vcr.playback_remaining(), 0);

        // Third request should fail
        let request = LlmRequest::new(vec![ChatMessage::user("Error")]);
        let result = vcr.generate(request).await;
        assert!(result.is_err());
    }
}
