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
//! The fingerprint captures structural elements of the request:
//! - System prompt (first 200 chars for fuzzy matching)
//! - Message roles and content prefixes
//! - Tool names (if provided)
//! - Key parameters (temperature)
//!
//! This allows slight variations in message content while still
//! matching the "same" request semantically.

use sha2::{Digest, Sha256};

use crate::infrastructure::ports::{LlmRequest, ToolDefinition};

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
    /// Create a fingerprint from an LLM request without tools.
    pub fn from_request(request: &LlmRequest) -> Self {
        Self::from_request_with_tools(request, None)
    }

    /// Create a fingerprint from an LLM request with optional tools.
    ///
    /// Hashes structural elements to create a content-based identifier
    /// that survives minor content variations.
    pub fn from_request_with_tools(request: &LlmRequest, tools: Option<&[ToolDefinition]>) -> Self {
        let mut hasher = Sha256::new();

        // Hash system prompt prefix (first 200 chars for fuzzy matching)
        if let Some(system) = &request.system_prompt {
            hasher.update(b"system:");
            let prefix: String = system.chars().take(200).collect();
            hasher.update(prefix.as_bytes());
        }

        // Hash message structure (role + content prefix)
        for msg in &request.messages {
            hasher.update(b"msg:");
            hasher.update(format!("{:?}", msg.role).as_bytes());
            hasher.update(b":");
            // First 200 chars of content for fuzzy matching
            let content_prefix: String = msg.content.chars().take(200).collect();
            hasher.update(content_prefix.as_bytes());
        }

        // Hash tool names if present (order-independent by sorting)
        if let Some(tools) = tools {
            let mut tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
            tool_names.sort();
            hasher.update(b"tools:");
            for name in tool_names {
                hasher.update(name.as_bytes());
                hasher.update(b",");
            }
        }

        // Hash key parameters
        hasher.update(b"temp:");
        let temp = request.temperature.unwrap_or(0.7);
        hasher.update(&temp.to_le_bytes());

        let hash: [u8; 32] = hasher.finalize().into();
        let summary = Self::create_summary(request, tools);

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
    fn create_summary(request: &LlmRequest, tools: Option<&[ToolDefinition]>) -> String {
        let system_prefix: String = request
            .system_prompt
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(40)
            .collect();

        let last_msg_prefix: String = request
            .messages
            .last()
            .map(|m| m.content.chars().take(40).collect())
            .unwrap_or_default();

        let tool_count = tools.map(|t| t.len()).unwrap_or(0);

        if tool_count > 0 {
            format!(
                "sys:{:.30}... | msg:{:.30}... | tools:{}",
                system_prefix, last_msg_prefix, tool_count
            )
        } else {
            format!("sys:{:.30}... | msg:{:.30}...", system_prefix, last_msg_prefix)
        }
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
    fn test_same_request_same_fingerprint() {
        let request = LlmRequest::new(vec![ChatMessage::user("Hello, world!")])
            .with_system_prompt("You are a helpful assistant.");

        let fp1 = RequestFingerprint::from_request(&request);
        let fp2 = RequestFingerprint::from_request(&request);

        assert_eq!(fp1.to_hex(), fp2.to_hex());
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
    fn test_tools_affect_fingerprint() {
        let request = LlmRequest::new(vec![ChatMessage::user("Hello")]);

        let tools = vec![ToolDefinition {
            name: "get_weather".to_string(),
            description: "Get weather".to_string(),
            parameters: serde_json::json!({}),
        }];

        let fp1 = RequestFingerprint::from_request(&request);
        let fp2 = RequestFingerprint::from_request_with_tools(&request, Some(&tools));

        assert_ne!(fp1.to_hex(), fp2.to_hex());
    }

    #[test]
    fn test_tool_order_independent() {
        let request = LlmRequest::new(vec![ChatMessage::user("Hello")]);

        let tools1 = vec![
            ToolDefinition {
                name: "tool_a".to_string(),
                description: "A".to_string(),
                parameters: serde_json::json!({}),
            },
            ToolDefinition {
                name: "tool_b".to_string(),
                description: "B".to_string(),
                parameters: serde_json::json!({}),
            },
        ];

        let tools2 = vec![
            ToolDefinition {
                name: "tool_b".to_string(),
                description: "B".to_string(),
                parameters: serde_json::json!({}),
            },
            ToolDefinition {
                name: "tool_a".to_string(),
                description: "A".to_string(),
                parameters: serde_json::json!({}),
            },
        ];

        let fp1 = RequestFingerprint::from_request_with_tools(&request, Some(&tools1));
        let fp2 = RequestFingerprint::from_request_with_tools(&request, Some(&tools2));

        // Same tools in different order should produce same fingerprint
        assert_eq!(fp1.to_hex(), fp2.to_hex());
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
}
