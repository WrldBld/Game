//! VCR-style LLM recording and playback for E2E tests.
//!
//! Records real LLM responses during test development, then replays them
//! for deterministic, fast test execution without requiring a live LLM.
//!
//! # Fingerprint-Based Matching
//!
//! Uses content-based fingerprinting to match requests to recordings.
//! This allows tests to add/remove LLM calls without breaking all
//! subsequent recordings (unlike sequential playback).
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

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::infrastructure::ollama::{OllamaClient, DEFAULT_OLLAMA_BASE_URL, DEFAULT_OLLAMA_MODEL};
use crate::infrastructure::ports::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, MessageRole, ToolCall, ToolDefinition,
};

use super::event_log::{ChatMessageLog, E2EEvent, E2EEventLog, TokenUsageLog, ToolCallLog};
use super::vcr_fingerprint::RequestFingerprint;

/// Current cassette format version.
///
/// Version history:
/// - 2.0: Fingerprint-indexed recordings for robust matching (current)
const CASSETTE_VERSION: &str = "2.0";

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
    /// Fingerprint hex for this recording.
    pub fingerprint: String,
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
    pub finish_reason: FinishReason,
}

/// Recorded tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Cassette file format containing recorded LLM interactions.
///
/// Version 2.0: Fingerprint-indexed recordings for robust matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCassette {
    version: String,
    recorded_at: String,
    llm_model: String,
    /// Recordings indexed by fingerprint hex.
    /// Each fingerprint can have multiple recordings (for repeated identical requests).
    recordings: HashMap<String, Vec<LlmRecording>>,
}

impl LlmCassette {
    /// Creates a new empty cassette for recording LLM interactions.
    ///
    /// The cassette is initialized with the current version and timestamp,
    /// ready to accumulate recordings during a test run.
    ///
    /// # Arguments
    ///
    /// * `llm_model` - The name of the LLM model being recorded (e.g., "llama3.2")
    pub fn new(llm_model: String) -> Self {
        Self {
            version: CASSETTE_VERSION.to_string(),
            recorded_at: chrono::Utc::now().to_rfc3339(),
            llm_model,
            recordings: HashMap::new(),
        }
    }

    /// Get the cassette version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Adds a recording indexed by its fingerprint.
    ///
    /// Multiple recordings can be stored under the same fingerprint to support
    /// tests that make identical requests multiple times. Recordings are stored
    /// in insertion order and retrieved FIFO.
    ///
    /// # Arguments
    ///
    /// * `fingerprint` - The hex-encoded SHA-256 fingerprint of the request
    /// * `recording` - The recorded LLM interaction to store
    pub fn add_recording(&mut self, fingerprint: String, recording: LlmRecording) {
        self.recordings.entry(fingerprint).or_default().push(recording);
    }

    /// Retrieves and removes the next recording for a fingerprint (FIFO order).
    ///
    /// When multiple recordings exist for the same fingerprint (from repeated
    /// identical requests), they are returned in the order they were recorded.
    /// Each call consumes one recording, so subsequent calls return the next one.
    ///
    /// Returns `None` if no recordings remain for the given fingerprint.
    pub fn get_recording(&mut self, fingerprint: &str) -> Option<LlmRecording> {
        self.recordings.get_mut(fingerprint).and_then(|v| {
            if v.is_empty() { None } else { Some(v.remove(0)) }
        })
    }

    /// Get the number of unique fingerprints.
    pub fn fingerprint_count(&self) -> usize {
        self.recordings.len()
    }

    /// Get the total number of recordings across all fingerprints.
    pub fn recording_count(&self) -> usize {
        self.recordings.values().map(|v| v.len()).sum()
    }

    /// Check if there are no recordings.
    pub fn is_empty(&self) -> bool {
        self.recordings.is_empty()
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
    /// Model name for recording.
    model_name: String,
}

/// Decorator that adds event logging to any LlmPort implementation.
/// This follows the decorator pattern to separate logging concerns from VCR logic.
pub struct LoggingLlmDecorator {
    inner: Arc<dyn LlmPort>,
    event_log: Arc<E2EEventLog>,
}

impl LoggingLlmDecorator {
    pub fn new(inner: Arc<dyn LlmPort>, event_log: Arc<E2EEventLog>) -> Self {
        Self { inner, event_log }
    }

    /// Log prompt sent event to the event log.
    fn log_prompt_sent(&self, request_id: Uuid, request: &LlmRequest, tools: Option<&[ToolDefinition]>) {
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

        self.event_log.log(E2EEvent::LlmPromptSent {
            request_id,
            system_prompt: request.system_prompt.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: tool_names,
        });
    }

    /// Log response received event to the event log.
    fn log_response_received(&self, request_id: Uuid, response: &LlmResponse, latency_ms: u64) {
        let tool_calls: Vec<ToolCallLog> = response
            .tool_calls
            .iter()
            .map(|tc| ToolCallLog {
                name: tc.name.clone(),
                arguments: tc.arguments.clone(),
            })
            .collect();

        let tokens = response.usage.as_ref().map(TokenUsageLog::from);

        self.event_log.log(E2EEvent::LlmResponseReceived {
            request_id,
            content: response.content.clone(),
            tool_calls,
            finish_reason: format!("{:?}", response.finish_reason),
            tokens,
            latency_ms,
        });
    }
}

#[async_trait]
impl LlmPort for LoggingLlmDecorator {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let request_id = Uuid::new_v4();
        self.log_prompt_sent(request_id, &request, None);
        let start = std::time::Instant::now();

        let response = self.inner.generate(request).await?;

        let latency_ms = start.elapsed().as_millis() as u64;
        self.log_response_received(request_id, &response, latency_ms);

        Ok(response)
    }

    async fn generate_with_tools(
        &self,
        request: LlmRequest,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, LlmError> {
        let request_id = Uuid::new_v4();
        self.log_prompt_sent(request_id, &request, Some(&tools));
        let start = std::time::Instant::now();

        let response = self.inner.generate_with_tools(request, tools).await?;

        let latency_ms = start.elapsed().as_millis() as u64;
        self.log_response_received(request_id, &response, latency_ms);

        Ok(response)
    }
}

impl VcrLlm {
    /// Creates a VcrLlm in recording mode.
    ///
    /// In this mode, all LLM requests are forwarded to the inner client, and
    /// both requests and responses are recorded to the cassette. Call
    /// [`save_cassette()`](Self::save_cassette) at the end of the test to persist recordings.
    ///
    /// # Arguments
    ///
    /// * `inner` - The real LLM client to forward requests to
    /// * `cassette_path` - Path where the cassette file will be saved
    /// * `model_name` - Name of the model for cassette metadata
    pub fn recording(inner: Arc<dyn LlmPort>, cassette_path: PathBuf, model_name: &str) -> Self {
        Self {
            inner: Some(inner),
            cassette_path,
            mode: VcrMode::Record,
            cassette: Mutex::new(LlmCassette::new(model_name.to_string())),
            model_name: model_name.to_string(),
        }
    }

    /// Creates a VcrLlm in playback mode from an existing cassette file.
    ///
    /// In this mode, no real LLM calls are made. Instead, responses are looked
    /// up by request fingerprint and returned from the cassette. This enables
    /// fast, deterministic test execution.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The cassette file cannot be read
    /// - The cassette JSON is malformed
    /// - The cassette version doesn't match the current version (version validation)
    pub fn playback(cassette_path: PathBuf) -> Result<Self, std::io::Error> {
        let content = fs::read_to_string(&cassette_path)?;
        let cassette: LlmCassette =
            serde_json::from_str(&content).map_err(|e| std::io::Error::other(e.to_string()))?;

        if cassette.version() != CASSETTE_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unsupported cassette version: {} (expected {})",
                        cassette.version(), CASSETTE_VERSION)
            ));
        }

        Ok(Self {
            inner: None,
            cassette_path,
            mode: VcrMode::Playback,
            cassette: Mutex::new(cassette),
            model_name: String::new(),
        })
    }

    /// Creates a VcrLlm in live mode (pass-through without recording).
    ///
    /// In this mode, all requests are forwarded directly to the inner LLM client
    /// without any recording or playback. Useful for debugging or when you want
    /// real LLM responses without affecting cassette files.
    ///
    /// # Arguments
    ///
    /// * `inner` - The real LLM client to forward all requests to
    pub fn live(inner: Arc<dyn LlmPort>) -> Self {
        Self {
            inner: Some(inner),
            cassette_path: PathBuf::new(),
            mode: VcrMode::Live,
            cassette: Mutex::new(LlmCassette::new("live".to_string())),
            model_name: String::new(),
        }
    }

    /// Creates a VcrLlm based on the `E2E_LLM_MODE` environment variable.
    ///
    /// This is the primary factory method for E2E tests, automatically selecting
    /// the appropriate mode based on environment configuration.
    ///
    /// # Environment Variables
    ///
    /// * `E2E_LLM_MODE`:
    ///   - `record`: Call real Ollama, save responses to cassette
    ///   - `playback` or unset: Load from cassette (falls back to record if cassette missing)
    ///   - `live`: Call real Ollama without recording
    /// * `OLLAMA_BASE_URL`: Override the Ollama server URL (default: http://localhost:11434)
    /// * `OLLAMA_MODEL`: Override the model name (default: llama3.2)
    pub fn from_env(cassette_path: PathBuf) -> Self {
        let ollama_url =
            std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| DEFAULT_OLLAMA_BASE_URL.to_string());
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_string());

        let mode_str = std::env::var("E2E_LLM_MODE").unwrap_or_default();

        // Helper closure for playback logic (used by playback and unknown modes)
        let try_playback = |cassette_path: PathBuf| -> Self {
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
        };

        match mode_str.as_str() {
            "record" => {
                tracing::info!(cassette = ?cassette_path, "VcrLlm: Recording mode");
                let client = Arc::new(OllamaClient::new(&ollama_url, &model));
                Self::recording(client, cassette_path, &model)
            }
            "live" => {
                tracing::info!("VcrLlm: Live mode (no recording)");
                let client = Arc::new(OllamaClient::new(&ollama_url, &model));
                Self::live(client)
            }
            "playback" | "" => {
                // Default: try playback, fall back to recording if cassette doesn't exist
                try_playback(cassette_path)
            }
            unknown => {
                tracing::warn!(
                    "Unknown E2E_LLM_MODE '{}', valid values: record, playback, live. Defaulting to playback.",
                    unknown
                );
                try_playback(cassette_path)
            }
        }
    }

    /// Saves the cassette to disk (only in recording mode).
    ///
    /// Call this at the end of a test run when in recording mode to persist
    /// the captured LLM interactions. In playback or live modes, this is a no-op.
    ///
    /// The cassette file is written as pretty-printed JSON to the path specified
    /// during construction. Parent directories are created if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if file I/O fails (permissions, disk full, etc.).
    pub fn save_cassette(&self) -> Result<(), std::io::Error> {
        if !matches!(self.mode, VcrMode::Record) {
            return Ok(()); // Only save in record mode
        }

        let cassette = self.cassette.lock().expect("cassette mutex poisoned");
        if cassette.is_empty() {
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
            fingerprints = cassette.fingerprint_count(),
            total_recordings = cassette.recording_count(),
            "VcrLlm: Saved cassette"
        );
        Ok(())
    }

    /// Record a request/response pair indexed by fingerprint.
    fn record(&self, request: &LlmRequest, tools: Option<&[ToolDefinition]>, response: &LlmResponse) {
        let fingerprint = RequestFingerprint::from_request_with_tools(request, tools);
        let fingerprint_hex = fingerprint.to_hex();

        let recording = LlmRecording {
            fingerprint: fingerprint_hex.clone(),
            request_summary: fingerprint.summary().to_string(),
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
                finish_reason: response.finish_reason.clone(),
            },
            recorded_at: chrono::Utc::now().to_rfc3339(),
        };

        let mut cassette = self.cassette.lock().expect("cassette mutex poisoned");
        cassette.add_recording(fingerprint_hex, recording);
    }

    /// Get recorded response matching the request fingerprint.
    fn playback_for_request(&self, request: &LlmRequest, tools: Option<&[ToolDefinition]>) -> Result<LlmResponse, LlmError> {
        let fingerprint = RequestFingerprint::from_request_with_tools(request, tools);
        let fingerprint_hex = fingerprint.to_hex();

        let mut cassette = self.cassette.lock().expect("cassette mutex poisoned");

        // Look up and remove the first recording for this fingerprint
        if let Some(recording) = cassette.get_recording(&fingerprint_hex) {
            tracing::debug!(
                fingerprint = %fingerprint,
                "VcrLlm: Matched recording"
            );

            return Ok(LlmResponse {
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
                finish_reason: recording.response.finish_reason,
                usage: None,
            });
        }

        // No matching recording found
        tracing::warn!(
            fingerprint = %fingerprint,
            "VcrLlm: No matching recording found for request"
        );

        Err(LlmError::RequestFailed(format!(
            "VcrLlm: No recording found for fingerprint {} ({})",
            fingerprint.short_hex(),
            fingerprint.summary()
        )))
    }

    /// Get the current mode.
    pub fn mode(&self) -> &VcrMode {
        &self.mode
    }

    /// Get the number of unique fingerprints recorded.
    pub fn fingerprint_count(&self) -> usize {
        self.cassette.lock().expect("cassette mutex poisoned").fingerprint_count()
    }

    /// Get the total number of recordings.
    pub fn recording_count(&self) -> usize {
        self.cassette.lock().expect("cassette mutex poisoned").recording_count()
    }

    /// Get the number of remaining playback items (across all fingerprints).
    pub fn playback_remaining(&self) -> usize {
        self.recording_count()
    }

    /// Execute an LLM request, handling all VCR modes uniformly.
    ///
    /// This unified method handles record/playback/live modes for both
    /// `generate()` and `generate_with_tools()`, eliminating code duplication.
    async fn execute_request(
        &self,
        request: LlmRequest,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<LlmResponse, LlmError> {
        match &self.mode {
            VcrMode::Record => {
                let inner = self.inner.as_ref().ok_or_else(|| {
                    LlmError::RequestFailed("VcrLlm: No inner LLM in record mode".to_string())
                })?;

                let response = if let Some(ref tools) = tools {
                    inner.generate_with_tools(request.clone(), tools.clone()).await?
                } else {
                    inner.generate(request.clone()).await?
                };

                self.record(&request, tools.as_deref(), &response);
                Ok(response)
            }
            VcrMode::Playback => {
                self.playback_for_request(&request, tools.as_deref())
            }
            VcrMode::Live => {
                let inner = self.inner.as_ref().ok_or_else(|| {
                    LlmError::RequestFailed("VcrLlm: No inner LLM in live mode".to_string())
                })?;

                if let Some(ref tools) = tools {
                    inner.generate_with_tools(request, tools.clone()).await
                } else {
                    inner.generate(request).await
                }
            }
        }
    }
}

#[async_trait]
impl LlmPort for VcrLlm {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.execute_request(request, None).await
    }

    async fn generate_with_tools(
        &self,
        request: LlmRequest,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, LlmError> {
        self.execute_request(request, Some(tools)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::ChatMessage;
    use tempfile::TempDir;

    /// Mock LLM for testing VcrLlm behavior.
    struct MockLlm {
        response: LlmResponse,
        call_count: std::sync::atomic::AtomicUsize,
    }

    impl MockLlm {
        fn new(response: LlmResponse) -> Self {
            Self {
                response,
                call_count: std::sync::atomic::AtomicUsize::new(0),
            }
        }

        fn call_count(&self) -> usize {
            self.call_count.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl LlmPort for MockLlm {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(self.response.clone())
        }

        async fn generate_with_tools(
            &self,
            _request: LlmRequest,
            _tools: Vec<ToolDefinition>,
        ) -> Result<LlmResponse, LlmError> {
            self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(self.response.clone())
        }
    }

    #[test]
    fn test_cassette_v2_serialization() {
        let recording = LlmRecording {
            fingerprint: "abc123".to_string(),
            request_summary: "Test request".to_string(),
            response: RecordedResponse {
                content: "Test response".to_string(),
                tool_calls: vec![],
                finish_reason: FinishReason::Stop,
            },
            recorded_at: "2026-01-12T10:00:01Z".to_string(),
        };

        let mut cassette = LlmCassette::new("test-model".to_string());
        cassette.add_recording("abc123".to_string(), recording);

        let json = serde_json::to_string_pretty(&cassette).unwrap();
        let mut parsed: LlmCassette = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.version(), "2.0");
        assert_eq!(parsed.fingerprint_count(), 1);
        // Retrieve and verify the recording
        let retrieved = parsed.get_recording("abc123").expect("recording should exist");
        assert_eq!(retrieved.response.content, "Test response");
    }

    #[tokio::test]
    async fn test_fingerprint_playback_mode() {
        let temp_dir = TempDir::new().unwrap();
        let cassette_path = temp_dir.path().join("test.json");

        // Create fingerprints for test requests
        let req1 = LlmRequest::new(vec![ChatMessage::user("Hello")]);
        let req2 = LlmRequest::new(vec![ChatMessage::user("World")]);
        let fp1 = super::super::vcr_fingerprint::RequestFingerprint::from_request(&req1);
        let fp2 = super::super::vcr_fingerprint::RequestFingerprint::from_request(&req2);

        // Create a cassette file with fingerprint-indexed recordings
        let mut cassette = LlmCassette::new("test".to_string());
        cassette.add_recording(
            fp1.to_hex(),
            LlmRecording {
                fingerprint: fp1.to_hex(),
                request_summary: "Hello request".to_string(),
                response: RecordedResponse {
                    content: "Hello response".to_string(),
                    tool_calls: vec![],
                    finish_reason: FinishReason::Stop,
                },
                recorded_at: "2026-01-12T10:00:01Z".to_string(),
            },
        );
        cassette.add_recording(
            fp2.to_hex(),
            LlmRecording {
                fingerprint: fp2.to_hex(),
                request_summary: "World request".to_string(),
                response: RecordedResponse {
                    content: "World response".to_string(),
                    tool_calls: vec![],
                    finish_reason: FinishReason::Stop,
                },
                recorded_at: "2026-01-12T10:00:02Z".to_string(),
            },
        );

        fs::write(&cassette_path, serde_json::to_string(&cassette).unwrap()).unwrap();

        // Load in playback mode
        let vcr = VcrLlm::playback(cassette_path).unwrap();

        assert_eq!(vcr.playback_remaining(), 2);

        // Request in REVERSE order - should still work with fingerprinting
        let response = vcr.generate(req2.clone()).await.unwrap();
        assert_eq!(response.content, "World response");

        let response = vcr.generate(req1.clone()).await.unwrap();
        assert_eq!(response.content, "Hello response");

        assert_eq!(vcr.playback_remaining(), 0);

        // Same request again should fail (no more recordings for that fingerprint)
        let result = vcr.generate(req1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_repeated_identical_requests() {
        let temp_dir = TempDir::new().unwrap();
        let cassette_path = temp_dir.path().join("test.json");

        // Create fingerprint for test request
        let request = LlmRequest::new(vec![ChatMessage::user("Repeat me")]);
        let fp = super::super::vcr_fingerprint::RequestFingerprint::from_request(&request);

        // Create cassette with multiple recordings for same fingerprint
        let mut cassette = LlmCassette::new("test".to_string());
        cassette.add_recording(
            fp.to_hex(),
            LlmRecording {
                fingerprint: fp.to_hex(),
                request_summary: "First".to_string(),
                response: RecordedResponse {
                    content: "First response".to_string(),
                    tool_calls: vec![],
                    finish_reason: FinishReason::Stop,
                },
                recorded_at: "2026-01-12T10:00:01Z".to_string(),
            },
        );
        cassette.add_recording(
            fp.to_hex(),
            LlmRecording {
                fingerprint: fp.to_hex(),
                request_summary: "Second".to_string(),
                response: RecordedResponse {
                    content: "Second response".to_string(),
                    tool_calls: vec![],
                    finish_reason: FinishReason::Stop,
                },
                recorded_at: "2026-01-12T10:00:02Z".to_string(),
            },
        );

        fs::write(&cassette_path, serde_json::to_string(&cassette).unwrap()).unwrap();

        let vcr = VcrLlm::playback(cassette_path).unwrap();

        // First call returns first recording
        let response = vcr.generate(request.clone()).await.unwrap();
        assert_eq!(response.content, "First response");

        // Second call returns second recording (same fingerprint)
        let response = vcr.generate(request.clone()).await.unwrap();
        assert_eq!(response.content, "Second response");

        // Third call fails (no more recordings)
        let result = vcr.generate(request).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_new_cassette_is_v2() {
        let cassette = LlmCassette::new("test-model".to_string());
        assert_eq!(cassette.version(), "2.0");
        assert!(cassette.is_empty());
    }

    #[tokio::test]
    async fn test_record_mode_saves_responses() {
        let temp_dir = TempDir::new().unwrap();
        let cassette_path = temp_dir.path().join("test_record.json");

        let mock_response = LlmResponse {
            content: "Mock response".to_string(),
            tool_calls: vec![],
            finish_reason: FinishReason::Stop,
            usage: None,
        };
        let mock_llm = Arc::new(MockLlm::new(mock_response));

        let vcr = VcrLlm::recording(mock_llm.clone(), cassette_path.clone(), "test-model");
        let request = LlmRequest::new(vec![ChatMessage::user("Hello")]);
        let response = vcr.generate(request).await.unwrap();

        assert_eq!(response.content, "Mock response");
        assert_eq!(mock_llm.call_count(), 1);

        vcr.save_cassette().unwrap();
        assert!(cassette_path.exists());
    }

    #[tokio::test]
    async fn test_live_mode_no_recording() {
        let mock_response = LlmResponse {
            content: "Live response".to_string(),
            tool_calls: vec![],
            finish_reason: FinishReason::Stop,
            usage: None,
        };
        let mock_llm = Arc::new(MockLlm::new(mock_response));

        let vcr = VcrLlm::live(mock_llm.clone());
        let request = LlmRequest::new(vec![ChatMessage::user("Hello")]);
        let response = vcr.generate(request).await.unwrap();

        assert_eq!(response.content, "Live response");
        assert_eq!(mock_llm.call_count(), 1);
        assert_eq!(vcr.recording_count(), 0); // No recording in live mode
    }

    #[test]
    fn test_playback_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let cassette_path = temp_dir.path().join("invalid.json");
        std::fs::write(&cassette_path, "not valid json").unwrap();

        let result = VcrLlm::playback(cassette_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_playback_incompatible_version() {
        let temp_dir = TempDir::new().unwrap();
        let cassette_path = temp_dir.path().join("old_version.json");

        let old_cassette = r#"{"version":"1.0","recorded_at":"","llm_model":"","recordings":{}}"#;
        std::fs::write(&cassette_path, old_cassette).unwrap();

        let result = VcrLlm::playback(cassette_path);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("version"));
    }
}
