//! Request fingerprinting for VCR cassette matching.
//!
//! Provides content-based fingerprinting for LLM requests, enabling
//! robust matching that survives test reordering and minor changes.
//!
//! # Problem Solved
//!
//! Sequential VCR playback breaks when tests add/remove LLM calls,
//! because all subsequent recordings shift position. Fingerprint-based
//! matching allows recordings to be matched by request content instead
//! of call order.
//!
//! # Fingerprint Strategy
//!
//! The fingerprint uses SHA-256 hashing of normalized request content:
//!
//! - **System prompt**: Full content (None and empty string are normalized to equivalent)
//! - **Messages**: Full role and content for each message in order
//! - **Parameters**: Temperature (defaults to 0.7 if unset), max_tokens (defaults to 0 if unset)
//! - **UUID Normalization**: All UUIDs are replaced with a placeholder before hashing,
//!   ensuring fingerprints are stable across test runs even when entity IDs change.
//!
//! This creates stable fingerprints where semantically identical requests produce
//! the same fingerprint, enabling reliable cassette lookups.

use once_cell::sync::Lazy;
use regex::Regex;
use sha2::{Digest, Sha256};

use crate::infrastructure::ports::LlmRequest;

/// Regex pattern for matching UUIDs (both hyphenated and non-hyphenated forms).
///
/// Matches:
/// - Standard hyphenated: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
/// - Non-hyphenated: `xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`
static UUID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}|[0-9a-f]{32}")
        .expect("UUID regex should compile")
});

/// Placeholder used to replace UUIDs in fingerprint computation.
const UUID_PLACEHOLDER: &str = "<UUID>";

/// Default temperature when none specified in request.
const DEFAULT_TEMPERATURE: f32 = 0.7;

/// Characters to include in summary prefix for system prompt.
const SUMMARY_PREFIX_LEN: usize = 40;

/// Normalize a string by replacing all UUIDs with a placeholder.
///
/// This ensures fingerprints are stable across test runs even when
/// entity IDs (which are UUIDs) change between runs.
fn normalize_uuids(s: &str) -> String {
    UUID_REGEX.replace_all(s, UUID_PLACEHOLDER).into_owned()
}

/// Fingerprint for matching LLM requests.
///
/// Uses SHA-256 hash of structural request elements for content-based
/// cassette matching instead of sequential playback.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct RequestFingerprint {
    /// SHA-256 hash of request structural elements.
    hash: [u8; 32],
    /// Human-readable summary for debugging.
    summary: String,
}

impl RequestFingerprint {
    /// Create a fingerprint from an LLM request.
    ///
    /// Hashes normalized content of request elements to create a content-based
    /// identifier for cassette matching. UUIDs are replaced with placeholders
    /// to ensure stability across test runs.
    pub fn from_request(request: &LlmRequest) -> Self {
        let mut hasher = Sha256::new();

        // Hash normalized system prompt (treat None and empty as equivalent)
        let system = request.system_prompt.as_deref().unwrap_or("");
        if !system.is_empty() {
            hasher.update(b"system:");
            let normalized_system = normalize_uuids(system);
            hasher.update(normalized_system.as_bytes());
        }

        // Hash normalized message structure (role + content with UUIDs replaced)
        for msg in &request.messages {
            hasher.update(b"msg:");
            hasher.update(format!("{:?}", msg.role).as_bytes());
            hasher.update(b":");
            let normalized_content = normalize_uuids(&msg.content);
            hasher.update(normalized_content.as_bytes());
        }

        // Hash key parameters
        hasher.update(b"temp:");
        let temp = request.temperature.unwrap_or(DEFAULT_TEMPERATURE);
        hasher.update(&temp.to_le_bytes());

        hasher.update(b"max_tokens:");
        let max_tokens = request.max_tokens.unwrap_or(0);
        hasher.update(&max_tokens.to_le_bytes());

        let hash: [u8; 32] = hasher.finalize().into();
        let summary = Self::create_summary(request);

        Self { hash, summary }
    }

    /// Get the fingerprint as a hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.hash)
    }

    /// Get a short hex prefix for logging (first 8 chars).
    pub fn short_hex(&self) -> String {
        hex::encode(&self.hash[..4])
    }

    /// Get the human-readable summary.
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Create a human-readable summary of the request for debugging.
    fn create_summary(request: &LlmRequest) -> String {
        let system_prefix: String = request
            .system_prompt
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(SUMMARY_PREFIX_LEN)
            .collect();

        let last_msg_prefix: String = request
            .messages
            .last()
            .map(|m| m.content.chars().take(SUMMARY_PREFIX_LEN).collect())
            .unwrap_or_default();

        format!(
            "sys:{:.30}... | msg:{:.30}...",
            system_prefix, last_msg_prefix
        )
    }
}

impl std::fmt::Display for RequestFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.short_hex(), self.summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::ChatMessage;

    #[test]
    fn test_uuid_normalization() {
        // Test hyphenated UUIDs
        let input = "Character 550e8400-e29b-41d4-a716-446655440000 said hello";
        let normalized = normalize_uuids(input);
        assert_eq!(normalized, "Character <UUID> said hello");

        // Test multiple UUIDs
        let input =
            "From 550e8400-e29b-41d4-a716-446655440000 to 6ba7b810-9dad-11d1-80b4-00c04fd430c8";
        let normalized = normalize_uuids(input);
        assert_eq!(normalized, "From <UUID> to <UUID>");

        // Test non-hyphenated UUIDs (32 hex chars)
        let input = "ID: 550e8400e29b41d4a716446655440000";
        let normalized = normalize_uuids(input);
        assert_eq!(normalized, "ID: <UUID>");

        // Test no UUIDs
        let input = "No UUIDs here";
        let normalized = normalize_uuids(input);
        assert_eq!(normalized, "No UUIDs here");

        // Test case insensitivity
        let input = "Upper: 550E8400-E29B-41D4-A716-446655440000";
        let normalized = normalize_uuids(input);
        assert_eq!(normalized, "Upper: <UUID>");
    }

    #[test]
    fn test_same_request_same_fingerprint() {
        let request = LlmRequest::new(vec![ChatMessage::user("Hello, world!")])
            .with_system_prompt("You are a helpful assistant.");

        let fp1 = RequestFingerprint::from_request(&request);
        let fp2 = RequestFingerprint::from_request(&request);

        assert_eq!(fp1.to_hex(), fp2.to_hex());
    }

    #[test]
    fn test_different_uuids_same_fingerprint() {
        // Two requests that differ only in UUIDs should have the same fingerprint
        let request1 = LlmRequest::new(vec![ChatMessage::user(
            "Talk to NPC 550e8400-e29b-41d4-a716-446655440000",
        )])
        .with_system_prompt("You are roleplaying as NPC 6ba7b810-9dad-11d1-80b4-00c04fd430c8");

        let request2 = LlmRequest::new(vec![ChatMessage::user(
            "Talk to NPC aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        )])
        .with_system_prompt("You are roleplaying as NPC 11111111-2222-3333-4444-555555555555");

        let fp1 = RequestFingerprint::from_request(&request1);
        let fp2 = RequestFingerprint::from_request(&request2);

        assert_eq!(
            fp1.to_hex(),
            fp2.to_hex(),
            "Requests differing only in UUIDs should have same fingerprint"
        );
    }

    #[test]
    fn test_different_content_different_fingerprint() {
        let req1 = LlmRequest::new(vec![ChatMessage::user("Hello")])
            .with_system_prompt("You are a helpful assistant.");

        let req2 = LlmRequest::new(vec![ChatMessage::user("Goodbye")])
            .with_system_prompt("You are a helpful assistant.");

        let fp1 = RequestFingerprint::from_request(&req1);
        let fp2 = RequestFingerprint::from_request(&req2);

        assert_ne!(fp1.to_hex(), fp2.to_hex());
    }

    #[test]
    fn test_different_system_prompt_different_fingerprint() {
        let req1 = LlmRequest::new(vec![ChatMessage::user("Hello")])
            .with_system_prompt("You are a helpful assistant.");

        let req2 = LlmRequest::new(vec![ChatMessage::user("Hello")])
            .with_system_prompt("You are a dangerous assistant.");

        let fp1 = RequestFingerprint::from_request(&req1);
        let fp2 = RequestFingerprint::from_request(&req2);

        assert_ne!(fp1.to_hex(), fp2.to_hex());
    }

    #[test]
    fn test_short_hex_length() {
        let request = LlmRequest::new(vec![ChatMessage::user("Test")]);
        let fp = RequestFingerprint::from_request(&request);

        assert_eq!(fp.short_hex().len(), 8); // 4 bytes = 8 hex chars
    }

    #[test]
    fn test_summary_format() {
        let request = LlmRequest::new(vec![ChatMessage::user("What is the weather?")])
            .with_system_prompt("You are a weather bot.");

        let fp = RequestFingerprint::from_request(&request);
        let summary = fp.summary();

        assert!(summary.contains("sys:"));
        assert!(summary.contains("msg:"));
    }

    #[test]
    fn test_display_includes_hash_and_summary() {
        let request = LlmRequest::new(vec![ChatMessage::user("Hello")]);
        let fp = RequestFingerprint::from_request(&request);

        let display = format!("{}", fp);
        assert!(display.starts_with("["));
        assert!(display.contains("]"));
    }

    #[test]
    fn test_empty_messages_fingerprint() {
        let request = LlmRequest::new(vec![]);
        let fp = RequestFingerprint::from_request(&request);
        assert_eq!(fp.to_hex().len(), 64);
    }

    #[test]
    fn test_none_vs_empty_system_prompt_same_fingerprint() {
        let req1 = LlmRequest::new(vec![ChatMessage::user("Hello")]);
        let req2 = LlmRequest::new(vec![ChatMessage::user("Hello")]).with_system_prompt("");

        let fp1 = RequestFingerprint::from_request(&req1);
        let fp2 = RequestFingerprint::from_request(&req2);

        assert_eq!(fp1.to_hex(), fp2.to_hex());
    }
}
