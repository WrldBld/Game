//! File-backed prompt template defaults store.
//!
//! Loads default prompt templates from files in the `./prompts/` directory.
//! Supports hot-reload in development via mtime-based cache invalidation.
//!
//! # YAML Front Matter Support
//!
//! Prompt files can include optional YAML front matter for metadata:
//!
//! ```markdown
//! ---
//! temperature: 0.4
//! ---
//! Your prompt content here...
//! ```
//!
//! The front matter is parsed and the temperature is extracted. The rest of the
//! file content (after the front matter) is returned as the prompt text.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::prompt_templates::ResolvedPromptTemplate;

/// Error type for defaults store operations.
#[derive(Debug, thiserror::Error)]
pub enum DefaultsError {
    #[error("Default file missing for key '{key}' at path '{path}'")]
    MissingFile { key: String, path: PathBuf },

    #[error("IO error reading default file for key '{key}': {source}")]
    Io {
        key: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid front matter YAML for key '{key}': {message}")]
    InvalidFrontMatter { key: String, message: String },
}

/// Cached entry for a prompt file.
#[derive(Debug, Clone)]
struct CachedPrompt {
    value: String,
    temperature: Option<f32>,
    mtime: std::time::SystemTime,
    last_checked: std::time::Instant,
}

/// Parsed front matter from a prompt template file.
#[derive(Debug, Clone, Default, serde::Deserialize)]
struct PromptFrontMatter {
    /// Optional temperature for LLM requests using this template
    temperature: Option<f32>,
}

/// Parse YAML front matter from prompt content.
///
/// Returns (front_matter, remaining_content) where front_matter is the parsed
/// metadata and remaining_content is the text after the front matter.
///
/// Front matter format:
/// ```markdown
/// ---
/// temperature: 0.4
/// ---
/// Content here...
/// ```
fn parse_front_matter(content: &str) -> (PromptFrontMatter, String) {
    // Check if content starts with ---
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (PromptFrontMatter::default(), content.to_string());
    }

    // Find the end of front matter (second ---)
    let after_first_delimiter = &trimmed[3..]; // Skip first ---
    let Some(end_pos) = after_first_delimiter.find("---") else {
        // No closing delimiter, treat entire content as prompt
        return (PromptFrontMatter::default(), content.to_string());
    };

    // Extract front matter YAML (between the --- delimiters)
    let yaml_content = &after_first_delimiter[..end_pos].trim();

    // Parse YAML
    let front_matter: PromptFrontMatter = match serde_yaml::from_str(yaml_content) {
        Ok(fm) => fm,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse front matter YAML, using defaults");
            PromptFrontMatter::default()
        }
    };

    // Extract remaining content (after second ---)
    let remaining = after_first_delimiter[end_pos + 3..].trim_start().to_string();

    // Validate temperature if present
    let validated_temp = front_matter.temperature.and_then(|t| {
        if t.is_finite() && (0.0..=2.0).contains(&t) {
            Some(t)
        } else {
            tracing::warn!(temperature = t, "Invalid temperature in front matter, ignoring");
            None
        }
    });

    (
        PromptFrontMatter {
            temperature: validated_temp,
        },
        remaining,
    )
}

/// Maps a template key to its file path relative to the prompts directory.
///
/// This function handles the mapping between key names (e.g., `suggestion.character_name`)
/// and the actual file layout in `prompts/` (e.g., `suggestions/character/name.md`).
///
/// The mapping handles:
/// - Pluralization differences (e.g., `suggestion` -> `suggestions`)
/// - Nested paths for character/location suggestions
/// - Actantial model prompts that use `suggestion.*` keys but live in `actantial/`
pub fn map_key_to_path(key: &str) -> String {
    match key {
        // Dialogue system
        "dialogue.response_format" => "dialogue/response_format.md",
        "dialogue.challenge_suggestion_format" => "dialogue/challenge_suggestion_format.md",
        "dialogue.narrative_event_format" => "dialogue/narrative_event_format.md",

        // Staging system
        "staging.system_prompt" => "staging/system_prompt.md",
        "staging.response_format" => "staging/response_format.md",
        "staging.role_instructions" => "staging/role_instructions.md",

        // Outcomes
        "outcome.system_prompt" => "outcomes/system_prompt.md",
        "outcome.branch_system_prompt" => "outcomes/branch_system_prompt.md",

        // Suggestions - character (plural directory, nested path)
        "suggestion.character_name" => "suggestions/character/name.md",
        "suggestion.character_description" => "suggestions/character/description.md",
        "suggestion.character_wants" => "suggestions/character/wants.md",
        "suggestion.character_fears" => "suggestions/character/fears.md",
        "suggestion.character_backstory" => "suggestions/character/backstory.md",

        // Suggestions - location (plural directory, nested path)
        "suggestion.location_name" => "suggestions/location/name.md",
        "suggestion.location_description" => "suggestions/location/description.md",
        "suggestion.location_atmosphere" => "suggestions/location/atmosphere.md",
        "suggestion.location_features" => "suggestions/location/features.md",
        "suggestion.location_secrets" => "suggestions/location/secrets.md",

        // Actantial model suggestions (use suggestion.* keys but live in actantial/)
        "suggestion.deflection_behavior" => "actantial/deflection_behavior.md",
        "suggestion.behavioral_tells" => "actantial/behavioral_tells.md",
        "suggestion.want_description" => "actantial/want_description.md",
        "suggestion.actantial_reason" => "actantial/actantial_reason.md",

        // System prompts
        "system.custom_condition_evaluator" => "system/custom_condition_evaluator.md",
        "system.staging_suggestions_system" => "system/staging_suggestions_system.md",
        "system.outcome_suggestions_system" => "system/outcome_suggestions_system.md",
        "system.generic_suggestion_system" => "system/generic_suggestion_system.md",

        // Fallback: direct mapping (should not be used for known keys)
        _ => {
            tracing::warn!(key = key, "Unknown prompt key, using fallback path mapping");
            return format!("{}.md", key.replace('.', "/"));
        }
    }
    .to_string()
}

/// File-backed store for prompt template defaults.
///
/// Loads templates from disk using a deterministic key-to-path mapping.
/// The mapping is defined by `map_key_to_path` which aligns keys with the
/// actual `prompts/` directory layout.
///
/// For example:
/// - `dialogue.response_format` -> `./prompts/dialogue/response_format.md`
/// - `suggestion.character_name` -> `./prompts/suggestions/character/name.md`
#[derive(Debug, Clone)]
pub struct PromptDefaultsStore {
    base_dir: PathBuf,
    cache: Arc<RwLock<HashMap<String, CachedPrompt>>>,
    /// Cache TTL for mtime checks (to avoid excessive stat calls)
    cache_ttl: std::time::Duration,
}

impl PromptDefaultsStore {
    /// Create a new defaults store with the given base directory.
    ///
    /// The base directory should contain subdirectories matching template key categories.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: std::time::Duration::from_secs(2),
        }
    }

    /// Create a new defaults store with a custom cache TTL.
    ///
    /// Shorter TTLs enable faster hot-reload in development.
    pub fn with_cache_ttl(mut self, ttl: std::time::Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Get the default value for a template key.
    ///
    /// Returns `Ok(None)` if the key is not known (no file exists at the expected path).
    /// Returns `Err(DefaultsError)` if the file exists but cannot be read.
    ///
    /// This method strips any YAML front matter and returns only the prompt text.
    /// Use `get_with_metadata` to also retrieve the temperature from front matter.
    pub async fn get(&self, key: &str) -> Result<Option<String>, DefaultsError> {
        self.get_with_metadata(key)
            .await
            .map(|opt| opt.map(|resolved| resolved.text))
    }

    /// Get the resolved template with metadata (text + temperature from front matter).
    ///
    /// Returns `Ok(None)` if the key is not known (no file exists at the expected path).
    /// Returns `Err(DefaultsError)` if the file exists but cannot be read.
    ///
    /// This method parses YAML front matter for temperature metadata:
    /// ```markdown
    /// ---
    /// temperature: 0.4
    /// ---
    /// Your prompt content here...
    /// ```
    pub async fn get_with_metadata(
        &self,
        key: &str,
    ) -> Result<Option<ResolvedPromptTemplate>, DefaultsError> {
        let path = self.key_to_path(key);

        // Fast path: check cache
        let now = std::time::Instant::now();
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(key) {
                // If cache is fresh, return immediately
                if now.duration_since(cached.last_checked) < self.cache_ttl {
                    return Ok(Some(ResolvedPromptTemplate {
                        text: cached.value.clone(),
                        temperature: cached.temperature,
                    }));
                }
            }
        }

        // Check if file exists
        if !path.exists() {
            return Ok(None);
        }

        // Get file metadata for mtime
        let mtime = tokio::fs::metadata(&path)
            .await
            .map_err(|e| DefaultsError::Io {
                key: key.to_string(),
                path: path.clone(),
                source: e,
            })?
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        // Check if cache is still valid (mtime unchanged)
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(key) {
                if cached.mtime == mtime {
                    let value = cached.value.clone();
                    let temperature = cached.temperature;
                    // Update last_checked and return cached value
                    drop(cache);
                    let mut cache = self.cache.write().await;
                    if let Some(cached) = cache.get_mut(key) {
                        cached.last_checked = now;
                    }
                    return Ok(Some(ResolvedPromptTemplate { text: value, temperature }));
                }
            }
        }

        // Read file from disk
        let raw_content = tokio::fs::read_to_string(&path).await.map_err(|e| {
            DefaultsError::Io {
                key: key.to_string(),
                path: path.clone(),
                source: e,
            }
        })?;

        // Parse front matter and extract content
        let (front_matter, text) = parse_front_matter(&raw_content);

        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(
            key.to_string(),
            CachedPrompt {
                value: text.clone(),
                temperature: front_matter.temperature,
                mtime,
                last_checked: now,
            },
        );

        Ok(Some(ResolvedPromptTemplate {
            text,
            temperature: front_matter.temperature,
        }))
    }

    /// Convert a template key to its file path.
    fn key_to_path(&self, key: &str) -> PathBuf {
        let relative_path = map_key_to_path(key);
        self.base_dir.join(relative_path)
    }

    /// Get the file path for a template key (for debugging/monitoring).
    pub fn path_for_key(&self, key: &str) -> PathBuf {
        self.key_to_path(key)
    }

    /// Clear the cache (useful for testing or forced reload).
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Preload all known keys into the cache.
    ///
    /// This is useful at startup to avoid first-request latency.
    /// Returns a map of key -> result for any keys that failed to load.
    pub async fn preload(&self, keys: &[&str]) -> HashMap<String, DefaultsError> {
        let mut errors = HashMap::new();

        for key in keys {
            match self.get(key).await {
                Ok(Some(_)) => {}
                Ok(None) => {
                    // Key not found - this is expected for optional keys
                }
                Err(e) => {
                    errors.insert(key.to_string(), e);
                }
            }
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_store() -> (PromptDefaultsStore, TempDir) {
        let temp_dir = TempDir::new().expect("should create temp dir");
        let store = PromptDefaultsStore::new(temp_dir.path());
        (store, temp_dir)
    }

    fn write_test_file(dir: &TempDir, relative_path: &str, content: &str) -> PathBuf {
        let path = dir.path().join(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("should create parent dirs");
        }
        let mut file = std::fs::File::create(&path).expect("should create file");
        file.write_all(content.as_bytes()).expect("should write content");
        path
    }

    #[tokio::test]
    async fn test_get_existing_file() {
        let (store, temp_dir) = create_test_store();
        write_test_file(&temp_dir, "dialogue/response_format.md", "Test content");

        let result = store.get("dialogue.response_format").await;
        assert!(result.is_ok());
        assert_eq!(result.expect("should have result"), Some("Test content".to_string()));
    }

    #[tokio::test]
    async fn test_get_missing_file_returns_none() {
        let (store, _temp_dir) = create_test_store();

        let result = store.get("unknown.key").await;
        assert!(result.is_ok());
        assert_eq!(result.expect("should have result"), None);
    }

    #[tokio::test]
    async fn test_caching_returns_same_value() {
        let (store, temp_dir) = create_test_store();
        write_test_file(&temp_dir, "test/key.md", "Original");

        // First read
        let first = store.get("test.key").await.expect("should get first");
        assert_eq!(first, Some("Original".to_string()));

        // Second read should come from cache
        let second = store.get("test.key").await.expect("should get second");
        assert_eq!(second, Some("Original".to_string()));
    }

    #[tokio::test]
    async fn test_key_to_path_mapping() {
        let (store, temp_dir) = create_test_store();

        // Test cases covering all mapping patterns:
        // - Direct mapping: dialogue.*, staging.*
        // - Plural + nested: suggestions/character/*, suggestions/location/*
        // - Actantial remapping: suggestion.deflection_behavior -> actantial/deflection_behavior.md
        // - Outcomes plural: outcome.* -> outcomes/*
        let cases = vec![
            ("dialogue.response_format", "dialogue/response_format.md"),
            ("staging.system_prompt", "staging/system_prompt.md"),
            ("suggestion.character_name", "suggestions/character/name.md"),
            ("suggestion.character_description", "suggestions/character/description.md"),
            ("suggestion.location_name", "suggestions/location/name.md"),
            ("suggestion.deflection_behavior", "actantial/deflection_behavior.md"),
            ("outcome.system_prompt", "outcomes/system_prompt.md"),
            ("system.staging_suggestions_system", "system/staging_suggestions_system.md"),
        ];

        for (key, expected) in cases {
            let path = store.key_to_path(key);
            assert_eq!(
                path.strip_prefix(temp_dir.path()).expect("should strip prefix"),
                PathBuf::from(expected),
                "Key '{}' should map to '{}'",
                key,
                expected
            );
        }
    }

    #[tokio::test]
    async fn test_preload() {
        let (store, temp_dir) = create_test_store();
        write_test_file(&temp_dir, "a/b.md", "Content A");
        write_test_file(&temp_dir, "c/d.md", "Content B");

        let errors = store.preload(&["a.b", "c.d", "missing.key"]).await;
        assert!(errors.is_empty());

        // Verify cache was populated
        let cached_a = store.get("a.b").await.expect("should get cached a");
        assert_eq!(cached_a, Some("Content A".to_string()));
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let (store, temp_dir) = create_test_store();
        write_test_file(&temp_dir, "test/key.md", "Original");

        // Populate cache
        store.get("test.key").await.expect("should get test key");

        // Clear cache
        store.clear_cache().await;

        // Should still work after clearing (reads from disk again)
        let result = store.get("test.key").await.expect("should get result");
        assert_eq!(result, Some("Original".to_string()));
    }

    // =============================================================================
    // Front Matter Tests
    // =============================================================================

    #[tokio::test]
    async fn test_get_with_metadata_no_front_matter() {
        let (store, temp_dir) = create_test_store();
        write_test_file(&temp_dir, "test/key.md", "Just prompt content");

        let result = store.get_with_metadata("test.key").await.expect("should get result");
        assert!(result.is_some());
        let resolved = result.expect("should have resolved");
        assert_eq!(resolved.text, "Just prompt content");
        assert_eq!(resolved.temperature, None);
    }

    #[tokio::test]
    async fn test_get_with_metadata_with_temperature() {
        let (store, temp_dir) = create_test_store();
        let content = "---\ntemperature: 0.4\n---\nPrompt content here";
        write_test_file(&temp_dir, "test/key.md", content);

        let result = store.get_with_metadata("test.key").await.expect("should get result");
        assert!(result.is_some());
        let resolved = result.expect("should have resolved");
        assert_eq!(resolved.text, "Prompt content here");
        assert_eq!(resolved.temperature, Some(0.4));
    }

    #[tokio::test]
    async fn test_get_with_metadata_strips_front_matter() {
        let (store, temp_dir) = create_test_store();
        let content = "---\ntemperature: 0.7\n---\nFirst line\nSecond line";
        write_test_file(&temp_dir, "test/key.md", content);

        let result = store.get_with_metadata("test.key").await.expect("should get result");
        assert!(result.is_some());
        let resolved = result.expect("should have resolved");
        assert_eq!(resolved.text, "First line\nSecond line");
        assert_eq!(resolved.temperature, Some(0.7));
    }

    #[tokio::test]
    async fn test_get_strips_front_matter() {
        let (store, temp_dir) = create_test_store();
        let content = "---\ntemperature: 0.5\n---\nPrompt text";
        write_test_file(&temp_dir, "test/key.md", content);

        // get() should return only the text, no front matter
        let result = store.get("test.key").await.expect("should get result");
        assert_eq!(result, Some("Prompt text".to_string()));
    }

    #[tokio::test]
    async fn test_front_matter_invalid_temperature_ignored() {
        let (store, temp_dir) = create_test_store();
        // Temperature outside valid range should be ignored
        let content = "---\ntemperature: 3.0\n---\nPrompt";
        write_test_file(&temp_dir, "test/key.md", content);

        let result = store.get_with_metadata("test.key").await.expect("should get result");
        assert!(result.is_some());
        let resolved = result.expect("should have resolved");
        assert_eq!(resolved.temperature, None); // Invalid temperature ignored
        assert_eq!(resolved.text, "Prompt");
    }

    #[tokio::test]
    async fn test_front_matter_negative_temperature_ignored() {
        let (store, temp_dir) = create_test_store();
        let content = "---\ntemperature: -0.5\n---\nPrompt";
        write_test_file(&temp_dir, "test/key.md", content);

        let result = store.get_with_metadata("test.key").await.expect("should get result");
        assert!(result.is_some());
        let resolved = result.expect("should have resolved");
        assert_eq!(resolved.temperature, None); // Negative temperature ignored
    }

    #[tokio::test]
    async fn test_front_matter_caching() {
        let (store, temp_dir) = create_test_store();
        let content = "---\ntemperature: 0.6\n---\nCached content";
        write_test_file(&temp_dir, "test/key.md", content);

        // First read populates cache
        let first = store.get_with_metadata("test.key").await.expect("should get first");
        assert_eq!(first.expect("should have first").temperature, Some(0.6));

        // Second read should come from cache with same temperature
        let second = store.get_with_metadata("test.key").await.expect("should get second");
        assert_eq!(second.expect("should have second").temperature, Some(0.6));
    }
}
