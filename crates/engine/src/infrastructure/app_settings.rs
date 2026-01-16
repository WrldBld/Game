//! Application settings and LLM context budget configuration
//!
//! These types are infrastructure concerns - they configure LLM token budgets,
//! circuit breakers, health checks, and other operational settings.
//!
//! # Architectural Note (ADR-002: Settings Serialization)
//!
//! AppSettings intentionally includes serde derives because settings are stored and
//! transmitted across infrastructure boundaries. UI metadata belongs in protocol.
//!
//! # Per-World Settings
//!
//! Settings can be global (applied to all worlds) or per-world (override global).
//! The `world_id` field determines scope:
//! - `None` = global settings (default)
//! - `Some(world_id)` = per-world override
//!
//! # Context Budget Overview
//!
//! Token budgets allow fine-grained control over how much context
//! is provided to the LLM in each category. When a category exceeds
//! its budget, it can be summarized to fit within the allocation.
//!
//! Default values are tuned for ~8K context models (Ollama defaults).
//! Larger models can increase these values accordingly.
//!
//! # Implementation Status
//!
//! - `ContextBudgetConfig`: Stored in world settings, exposed via Settings API
//! - `ContextCategory`: Defined, used for budget configuration
//! - `TokenCounter`: Defined, ready for use in prompt building
//! - **Budget enforcement**: NOT YET FULLY IMPLEMENTED
//!
//! The settings can be configured per-world, but actual token counting and
//! budget enforcement is not yet wired into `PromptContextService::build_prompt_from_action()`.

use serde::{Deserialize, Serialize};
use wrldbldr_domain::WorldId;

// ============================================================================
// Batch Queue Failure Policy
// ============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatchQueueFailurePolicy {
    /// Any failure while queueing prompts fails the entire batch.
    AllOrNothing,
    /// Continue queueing remaining prompts; fail only if none queued successfully.
    BestEffort,

    /// Forward-compatibility fallback for newer variants.
    #[serde(other)]
    Unknown,
}

fn default_batch_queue_failure_policy() -> BatchQueueFailurePolicy {
    BatchQueueFailurePolicy::AllOrNothing
}

impl std::fmt::Display for BatchQueueFailurePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchQueueFailurePolicy::AllOrNothing => write!(f, "all_or_nothing"),
            BatchQueueFailurePolicy::BestEffort => write!(f, "best_effort"),
            BatchQueueFailurePolicy::Unknown => write!(f, "unknown"),
        }
    }
}

impl std::str::FromStr for BatchQueueFailurePolicy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "all_or_nothing" | "allornothing" | "all" => Ok(BatchQueueFailurePolicy::AllOrNothing),
            "best_effort" | "besteffort" | "best" => Ok(BatchQueueFailurePolicy::BestEffort),
            _ => Err(()),
        }
    }
}

// ============================================================================
// Context Category
// ============================================================================

/// Categories of context that can be included in LLM prompts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextCategory {
    /// Scene description, location, time, atmosphere
    Scene,
    /// NPC details: personality, motivations, relationships
    Character,
    /// Recent conversation history with the player
    ConversationHistory,
    /// Active challenges that could be triggered
    Challenges,
    /// Active narrative events / story beats
    NarrativeEvents,
    /// GM's directorial notes, tone, pacing guidance
    DirectorialNotes,
    /// Location-specific details (connected locations, NPCs who frequent here)
    LocationContext,
    /// Player character details and stats
    PlayerContext,
}

impl ContextCategory {
    /// Returns all context categories in priority order
    /// Higher priority categories are less likely to be summarized aggressively
    pub fn all_by_priority() -> Vec<Self> {
        vec![
            Self::Character,           // Most important - who they're talking to
            Self::Scene,               // Current situation
            Self::DirectorialNotes,    // GM guidance
            Self::ConversationHistory, // Recent context
            Self::Challenges,          // What could happen
            Self::NarrativeEvents,     // Story beats
            Self::LocationContext,     // Environmental details
            Self::PlayerContext,       // Player info
        ]
    }

    /// Display name for this category
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Scene => "Scene Context",
            Self::Character => "Character Details",
            Self::ConversationHistory => "Conversation History",
            Self::Challenges => "Active Challenges",
            Self::NarrativeEvents => "Narrative Events",
            Self::DirectorialNotes => "Directorial Notes",
            Self::LocationContext => "Location Context",
            Self::PlayerContext => "Player Context",
        }
    }
}

// ============================================================================
// Context Budget Config
// ============================================================================

/// Token budget configuration for LLM context building
///
/// Each category has its own token budget. When the total context
/// would exceed these limits, lower-priority categories are summarized
/// or truncated to fit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextBudgetConfig {
    /// Total token budget for the entire system prompt
    /// Should leave room for user message and response
    total_budget_tokens: usize,

    /// Budget for scene context (location, time, atmosphere)
    scene_tokens: usize,

    /// Budget for character details (personality, wants, relationships)
    character_tokens: usize,

    /// Budget for conversation history
    conversation_history_tokens: usize,

    /// Budget for active challenges
    challenges_tokens: usize,

    /// Budget for narrative events
    narrative_events_tokens: usize,

    /// Budget for directorial notes
    directorial_notes_tokens: usize,

    /// Budget for location-specific context
    location_context_tokens: usize,

    /// Budget for player character context
    player_context_tokens: usize,

    /// Whether to enable automatic summarization when over budget
    enable_summarization: bool,

    /// Model to use for summarization (uses main model if None)
    summarization_model: Option<String>,
}

impl Default for ContextBudgetConfig {
    fn default() -> Self {
        // Defaults tuned for ~8K context models
        // Total: ~4000 tokens for system prompt, leaving ~4000 for conversation
        Self {
            total_budget_tokens: 4000,
            scene_tokens: 500,
            character_tokens: 800,
            conversation_history_tokens: 1000,
            challenges_tokens: 400,
            narrative_events_tokens: 400,
            directorial_notes_tokens: 300,
            location_context_tokens: 300,
            player_context_tokens: 300,
            enable_summarization: true,
            summarization_model: None,
        }
    }
}

impl ContextBudgetConfig {
    /// Create a configuration for larger context models (32K+)
    pub fn large_context() -> Self {
        Self {
            total_budget_tokens: 12000,
            scene_tokens: 1500,
            character_tokens: 2500,
            conversation_history_tokens: 3000,
            challenges_tokens: 1200,
            narrative_events_tokens: 1200,
            directorial_notes_tokens: 800,
            location_context_tokens: 900,
            player_context_tokens: 900,
            enable_summarization: true,
            summarization_model: None,
        }
    }

    /// Create a minimal configuration for very limited context models
    pub fn minimal() -> Self {
        Self {
            total_budget_tokens: 2000,
            scene_tokens: 250,
            character_tokens: 400,
            conversation_history_tokens: 500,
            challenges_tokens: 200,
            narrative_events_tokens: 200,
            directorial_notes_tokens: 150,
            location_context_tokens: 150,
            player_context_tokens: 150,
            enable_summarization: true,
            summarization_model: None,
        }
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Get the total budget tokens
    pub fn total_budget_tokens(&self) -> usize {
        self.total_budget_tokens
    }

    /// Get the scene tokens budget
    pub fn scene_tokens(&self) -> usize {
        self.scene_tokens
    }

    /// Get the character tokens budget
    pub fn character_tokens(&self) -> usize {
        self.character_tokens
    }

    /// Get the conversation history tokens budget
    pub fn conversation_history_tokens(&self) -> usize {
        self.conversation_history_tokens
    }

    /// Get the challenges tokens budget
    pub fn challenges_tokens(&self) -> usize {
        self.challenges_tokens
    }

    /// Get the narrative events tokens budget
    pub fn narrative_events_tokens(&self) -> usize {
        self.narrative_events_tokens
    }

    /// Get the directorial notes tokens budget
    pub fn directorial_notes_tokens(&self) -> usize {
        self.directorial_notes_tokens
    }

    /// Get the location context tokens budget
    pub fn location_context_tokens(&self) -> usize {
        self.location_context_tokens
    }

    /// Get the player context tokens budget
    pub fn player_context_tokens(&self) -> usize {
        self.player_context_tokens
    }

    /// Check if summarization is enabled
    pub fn enable_summarization(&self) -> bool {
        self.enable_summarization
    }

    /// Get the summarization model (if set)
    pub fn summarization_model(&self) -> Option<&str> {
        self.summarization_model.as_deref()
    }

    /// Get the budget for a specific category
    pub fn budget_for(&self, category: ContextCategory) -> usize {
        match category {
            ContextCategory::Scene => self.scene_tokens,
            ContextCategory::Character => self.character_tokens,
            ContextCategory::ConversationHistory => self.conversation_history_tokens,
            ContextCategory::Challenges => self.challenges_tokens,
            ContextCategory::NarrativeEvents => self.narrative_events_tokens,
            ContextCategory::DirectorialNotes => self.directorial_notes_tokens,
            ContextCategory::LocationContext => self.location_context_tokens,
            ContextCategory::PlayerContext => self.player_context_tokens,
        }
    }

    // -------------------------------------------------------------------------
    // Builder methods
    // -------------------------------------------------------------------------

    /// Set the budget for a specific category (builder pattern)
    pub fn with_budget_for(mut self, category: ContextCategory, tokens: usize) -> Self {
        match category {
            ContextCategory::Scene => self.scene_tokens = tokens,
            ContextCategory::Character => self.character_tokens = tokens,
            ContextCategory::ConversationHistory => self.conversation_history_tokens = tokens,
            ContextCategory::Challenges => self.challenges_tokens = tokens,
            ContextCategory::NarrativeEvents => self.narrative_events_tokens = tokens,
            ContextCategory::DirectorialNotes => self.directorial_notes_tokens = tokens,
            ContextCategory::LocationContext => self.location_context_tokens = tokens,
            ContextCategory::PlayerContext => self.player_context_tokens = tokens,
        }
        self
    }

    /// Set the total budget tokens
    pub fn with_total_budget_tokens(mut self, tokens: usize) -> Self {
        self.total_budget_tokens = tokens;
        self
    }

    /// Set the scene tokens budget
    pub fn with_scene_tokens(mut self, tokens: usize) -> Self {
        self.scene_tokens = tokens;
        self
    }

    /// Set the character tokens budget
    pub fn with_character_tokens(mut self, tokens: usize) -> Self {
        self.character_tokens = tokens;
        self
    }

    /// Set the conversation history tokens budget
    pub fn with_conversation_history_tokens(mut self, tokens: usize) -> Self {
        self.conversation_history_tokens = tokens;
        self
    }

    /// Set the challenges tokens budget
    pub fn with_challenges_tokens(mut self, tokens: usize) -> Self {
        self.challenges_tokens = tokens;
        self
    }

    /// Set the narrative events tokens budget
    pub fn with_narrative_events_tokens(mut self, tokens: usize) -> Self {
        self.narrative_events_tokens = tokens;
        self
    }

    /// Set the directorial notes tokens budget
    pub fn with_directorial_notes_tokens(mut self, tokens: usize) -> Self {
        self.directorial_notes_tokens = tokens;
        self
    }

    /// Set the location context tokens budget
    pub fn with_location_context_tokens(mut self, tokens: usize) -> Self {
        self.location_context_tokens = tokens;
        self
    }

    /// Set the player context tokens budget
    pub fn with_player_context_tokens(mut self, tokens: usize) -> Self {
        self.player_context_tokens = tokens;
        self
    }

    /// Set whether summarization is enabled
    pub fn with_enable_summarization(mut self, enable: bool) -> Self {
        self.enable_summarization = enable;
        self
    }

    /// Set the summarization model
    pub fn with_summarization_model(mut self, model: impl Into<String>) -> Self {
        self.summarization_model = Some(model.into());
        self
    }

    // -------------------------------------------------------------------------
    // Query methods
    // -------------------------------------------------------------------------

    /// Sum of all category budgets (may exceed total_budget_tokens if overlap is expected)
    pub fn sum_category_budgets(&self) -> usize {
        self.scene_tokens
            + self.character_tokens
            + self.conversation_history_tokens
            + self.challenges_tokens
            + self.narrative_events_tokens
            + self.directorial_notes_tokens
            + self.location_context_tokens
            + self.player_context_tokens
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.total_budget_tokens == 0 {
            return Err("Total budget must be greater than 0".to_string());
        }

        // Individual budgets can exceed total (summarization will handle it)
        // but warn if way over
        let sum = self.sum_category_budgets();
        if sum > self.total_budget_tokens * 3 {
            return Err(format!(
                "Sum of category budgets ({}) is more than 3x total budget ({})",
                sum, self.total_budget_tokens
            ));
        }

        Ok(())
    }
}

// ============================================================================
// Token Counting
// ============================================================================

/// Token counting configuration
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum TokenCountMethod {
    /// Simple character-based approximation: 1 token ≈ 4 characters
    /// Fast but less accurate, good for quick estimates
    CharacterApprox,
    /// Word-based approximation: 1 token ≈ 0.75 words (or ~1.33 tokens per word)
    /// Better for English text
    WordApprox,
    /// Hybrid: uses character count for short text, word count for longer text
    /// Provides best balance of speed and accuracy
    #[default]
    Hybrid,
}

/// Token counter for estimating LLM token usage
///
/// # Accuracy Notes
///
/// These are approximations tuned for typical LLM tokenizers (GPT-style BPE).
/// Actual token counts may vary by ±10-20% depending on:
/// - The specific model's tokenizer
/// - Language (non-English text often tokenizes differently)
/// - Special characters and formatting
/// - Code vs prose
///
/// For Ollama models (Llama, Mistral, etc.), these approximations are
/// generally conservative (slightly overestimate), which is desirable
/// for budget management.
#[derive(Debug, Clone)]
pub struct TokenCounter {
    method: TokenCountMethod,
    /// Characters per token for character-based counting
    chars_per_token: f64,
    /// Tokens per word for word-based counting
    tokens_per_word: f64,
    /// Threshold (in chars) for switching from char to word counting in Hybrid mode
    hybrid_threshold: usize,
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self {
            method: TokenCountMethod::Hybrid,
            chars_per_token: 4.0,  // ~4 characters per token
            tokens_per_word: 1.33, // ~1.33 tokens per word (GPT-style)
            hybrid_threshold: 100, // Use char counting for very short text
        }
    }
}

impl TokenCounter {
    /// Create a new token counter with the specified method
    pub fn new(method: TokenCountMethod) -> Self {
        Self {
            method,
            ..Default::default()
        }
    }

    /// Create a token counter tuned for Llama-style models
    /// Llama tokenizers tend to be slightly more efficient
    pub fn llama_tuned() -> Self {
        Self {
            method: TokenCountMethod::Hybrid,
            chars_per_token: 3.8,
            tokens_per_word: 1.25,
            hybrid_threshold: 100,
        }
    }

    /// Count tokens in the given text
    pub fn count(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }

        match self.method {
            TokenCountMethod::CharacterApprox => self.count_by_chars(text),
            TokenCountMethod::WordApprox => self.count_by_words(text),
            TokenCountMethod::Hybrid => {
                if text.len() < self.hybrid_threshold {
                    self.count_by_chars(text)
                } else {
                    self.count_by_words(text)
                }
            }
        }
    }

    /// Count tokens using character-based approximation
    fn count_by_chars(&self, text: &str) -> usize {
        let chars = text.chars().count();
        ((chars as f64) / self.chars_per_token).ceil() as usize
    }

    /// Count tokens using word-based approximation
    fn count_by_words(&self, text: &str) -> usize {
        let words = text.split_whitespace().count();
        // Also account for punctuation and special tokens
        let special_chars = text
            .chars()
            .filter(|c| {
                matches!(
                    c,
                    '\n' | '\t' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}'
                )
            })
            .count();

        let base_tokens = (words as f64 * self.tokens_per_word).ceil() as usize;
        // Add ~0.5 tokens per special character group
        let special_tokens = special_chars / 2;

        base_tokens + special_tokens
    }

    /// Estimate how many characters fit within a token budget
    pub fn chars_for_budget(&self, tokens: usize) -> usize {
        ((tokens as f64) * self.chars_per_token) as usize
    }

    /// Estimate how many words fit within a token budget
    pub fn words_for_budget(&self, tokens: usize) -> usize {
        ((tokens as f64) / self.tokens_per_word) as usize
    }

    /// Check if text exceeds a token budget
    pub fn exceeds_budget(&self, text: &str, budget: usize) -> bool {
        self.count(text) > budget
    }

    /// Truncate text to fit within a token budget (by words)
    /// Returns the truncated text and whether truncation occurred
    pub fn truncate_to_budget(&self, text: &str, budget: usize) -> (String, bool) {
        let current_tokens = self.count(text);
        if current_tokens <= budget {
            return (text.to_string(), false);
        }

        // Estimate target word count
        let target_words = self.words_for_budget(budget);

        // Take words up to target, leaving room for ellipsis
        let words: Vec<&str> = text.split_whitespace().collect();
        let take_words = target_words.saturating_sub(1); // Leave room for "..."

        if take_words == 0 {
            return ("...".to_string(), true);
        }

        let truncated: String = words
            .iter()
            .take(take_words)
            .copied()
            .collect::<Vec<_>>()
            .join(" ");

        (format!("{}...", truncated), true)
    }
}

/// Convenience function to count tokens using default settings
pub fn count_tokens(text: &str) -> usize {
    TokenCounter::default().count(text)
}

/// Convenience function to check if text exceeds a token budget
pub fn exceeds_token_budget(text: &str, budget: usize) -> bool {
    TokenCounter::default().exceeds_budget(text, budget)
}

// ============================================================================
// Context Budget Enforcement
// ============================================================================

/// Result of enforcing budget on a context section
#[derive(Debug, Clone)]
pub struct EnforcementResult {
    /// The text after enforcement (may be truncated)
    text: String,
    /// Original token count before enforcement
    original_tokens: usize,
    /// Final token count after enforcement
    final_tokens: usize,
    /// Whether truncation occurred
    was_truncated: bool,
    /// The budget that was applied
    budget: usize,
}

impl EnforcementResult {
    /// Create a new enforcement result
    pub(crate) fn new(
        text: String,
        original_tokens: usize,
        final_tokens: usize,
        was_truncated: bool,
        budget: usize,
    ) -> Self {
        Self {
            text,
            original_tokens,
            final_tokens,
            was_truncated,
            budget,
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────

    /// Get the text after enforcement (may be truncated)
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the original token count before enforcement
    pub fn original_tokens(&self) -> usize {
        self.original_tokens
    }

    /// Get the final token count after enforcement
    pub fn final_tokens(&self) -> usize {
        self.final_tokens
    }

    /// Get whether truncation occurred
    pub fn was_truncated(&self) -> bool {
        self.was_truncated
    }

    /// Get the budget that was applied
    pub fn budget(&self) -> usize {
        self.budget
    }

    // ── Query Methods ────────────────────────────────────────────────────

    /// Returns true if the content exceeded its budget
    pub fn exceeded_budget(&self) -> bool {
        self.was_truncated
    }

    /// Returns the number of tokens saved by truncation
    pub fn tokens_saved(&self) -> usize {
        self.original_tokens.saturating_sub(self.final_tokens)
    }

    /// Consume self and return the text
    pub fn into_text(self) -> String {
        self.text
    }
}

/// Statistics about budget enforcement for a single prompt
#[derive(Debug, Clone, Default)]
pub struct EnforcementStats {
    /// Total tokens before enforcement
    total_original_tokens: usize,
    /// Total tokens after enforcement
    total_final_tokens: usize,
    /// Number of categories that were truncated
    categories_truncated: usize,
    /// Details per category
    category_results: Vec<(ContextCategory, EnforcementResult)>,
}

impl EnforcementStats {
    // ── Accessors ────────────────────────────────────────────────────────

    /// Get total tokens before enforcement
    pub fn total_original_tokens(&self) -> usize {
        self.total_original_tokens
    }

    /// Get total tokens after enforcement
    pub fn total_final_tokens(&self) -> usize {
        self.total_final_tokens
    }

    /// Get number of categories that were truncated
    pub fn categories_truncated(&self) -> usize {
        self.categories_truncated
    }

    /// Get details per category
    pub fn category_results(&self) -> &[(ContextCategory, EnforcementResult)] {
        &self.category_results
    }

    // ── Mutators (crate-internal) ────────────────────────────────────────

    /// Add tokens to the original total
    pub(crate) fn add_original_tokens(&mut self, tokens: usize) {
        self.total_original_tokens += tokens;
    }

    /// Add tokens to the final total
    pub(crate) fn add_final_tokens(&mut self, tokens: usize) {
        self.total_final_tokens += tokens;
    }

    /// Increment the truncated categories count
    pub(crate) fn increment_truncated(&mut self) {
        self.categories_truncated += 1;
    }

    /// Add a category result
    pub(crate) fn add_category_result(
        &mut self,
        category: ContextCategory,
        result: EnforcementResult,
    ) {
        self.category_results.push((category, result));
    }

    // ── Query Methods ────────────────────────────────────────────────────

    /// Returns true if any category was truncated
    pub fn any_truncated(&self) -> bool {
        self.categories_truncated > 0
    }

    /// Returns total tokens saved across all categories
    pub fn total_tokens_saved(&self) -> usize {
        self.total_original_tokens
            .saturating_sub(self.total_final_tokens)
    }
}

/// Enforces token budgets on context sections for LLM prompts
///
/// Uses `TokenCounter` to estimate token usage and truncates sections
/// that exceed their configured budget.
#[derive(Debug, Clone)]
pub struct ContextBudgetEnforcer {
    config: ContextBudgetConfig,
    counter: TokenCounter,
    /// Track enforcement stats for logging
    enforcement_stats: EnforcementStats,
}

impl ContextBudgetEnforcer {
    /// Create a new enforcer with the given configuration
    pub fn new(config: ContextBudgetConfig) -> Self {
        Self {
            config,
            counter: TokenCounter::default(),
            enforcement_stats: EnforcementStats::default(),
        }
    }

    /// Create an enforcer with a custom token counter
    pub fn with_counter(config: ContextBudgetConfig, counter: TokenCounter) -> Self {
        Self {
            config,
            counter,
            enforcement_stats: EnforcementStats::default(),
        }
    }

    /// Get the current enforcement statistics
    pub fn stats(&self) -> &EnforcementStats {
        &self.enforcement_stats
    }

    /// Reset enforcement statistics (call before building a new prompt)
    ///
    /// # Note on `&mut self`
    /// The enforcer is stateful, accumulating statistics across a prompt-building
    /// session. Call this method to reset statistics when starting a new prompt.
    pub fn reset_stats(&mut self) {
        self.enforcement_stats = EnforcementStats::default();
    }

    /// Enforce budget on a context section
    ///
    /// Returns the (possibly truncated) text and enforcement details.
    /// Also updates internal statistics.
    ///
    /// # Note on `&mut self`
    /// This method requires mutable access because the enforcer tracks cumulative
    /// statistics across multiple enforce calls. This is intentional: the enforcer
    /// is designed to be used across a prompt-building session, accumulating stats
    /// that can be logged or analyzed after all sections are processed.
    pub fn enforce(&mut self, category: ContextCategory, text: &str) -> EnforcementResult {
        let budget = self.config.budget_for(category);
        let original_tokens = self.counter.count(text);

        let (final_text, was_truncated) = if original_tokens > budget {
            self.counter.truncate_to_budget(text, budget)
        } else {
            (text.to_string(), false)
        };

        let final_tokens = self.counter.count(&final_text);

        let result = EnforcementResult::new(
            final_text,
            original_tokens,
            final_tokens,
            was_truncated,
            budget,
        );

        // Update stats
        self.enforcement_stats.add_original_tokens(original_tokens);
        self.enforcement_stats.add_final_tokens(final_tokens);
        if was_truncated {
            self.enforcement_stats.increment_truncated();
        }
        self.enforcement_stats
            .add_category_result(category, result.clone());

        result
    }

    /// Enforce budget and return just the text (convenience method)
    ///
    /// # Note on `&mut self`
    /// Delegates to `enforce()`, which updates internal statistics.
    pub fn enforce_text(&mut self, category: ContextCategory, text: &str) -> String {
        self.enforce(category, text).into_text()
    }

    /// Check if text exceeds budget without modifying it
    pub fn would_exceed(&self, category: ContextCategory, text: &str) -> bool {
        let budget = self.config.budget_for(category);
        self.counter.exceeds_budget(text, budget)
    }

    /// Get the budget for a category
    pub fn budget_for(&self, category: ContextCategory) -> usize {
        self.config.budget_for(category)
    }

    /// Get remaining budget after accounting for used tokens
    pub fn remaining_budget(&self, category: ContextCategory, used_tokens: usize) -> usize {
        self.config.budget_for(category).saturating_sub(used_tokens)
    }

    /// Count tokens in text
    pub fn count_tokens(&self, text: &str) -> usize {
        self.counter.count(text)
    }

    /// Check if total context exceeds total budget
    pub fn total_exceeds_budget(&self) -> bool {
        self.enforcement_stats.total_final_tokens() > self.config.total_budget_tokens()
    }

    /// Get the total budget
    pub fn total_budget(&self) -> usize {
        self.config.total_budget_tokens()
    }

    /// Finalize enforcement and return whether any truncation occurred
    pub fn finalize(&self) -> bool {
        self.enforcement_stats.any_truncated()
    }
}

/// Builder pattern for enforcing budgets on structured context
pub struct ContextBuilder {
    enforcer: ContextBudgetEnforcer,
    sections: Vec<(ContextCategory, String)>,
}

impl ContextBuilder {
    /// Create a new context builder with the given config
    pub fn new(config: ContextBudgetConfig) -> Self {
        Self {
            enforcer: ContextBudgetEnforcer::new(config),
            sections: Vec::new(),
        }
    }

    /// Add a section with budget enforcement
    pub fn add_section(mut self, category: ContextCategory, content: String) -> Self {
        let enforced = self.enforcer.enforce_text(category, &content);
        self.sections.push((category, enforced));
        self
    }

    /// Add a section only if content is non-empty
    pub fn add_section_if_present(
        self,
        category: ContextCategory,
        content: Option<String>,
    ) -> Self {
        match content {
            Some(c) if !c.trim().is_empty() => self.add_section(category, c),
            _ => self,
        }
    }

    /// Build the final context, joining all sections
    pub fn build(self) -> (String, EnforcementStats) {
        let stats = self.enforcer.enforcement_stats.clone();
        let result = self
            .sections
            .into_iter()
            .map(|(_, content)| content)
            .collect::<Vec<_>>()
            .join("\n\n");
        (result, stats)
    }

    /// Build with a custom separator
    pub fn build_with_separator(self, separator: &str) -> (String, EnforcementStats) {
        let stats = self.enforcer.enforcement_stats.clone();
        let result = self
            .sections
            .into_iter()
            .map(|(_, content)| content)
            .collect::<Vec<_>>()
            .join(separator);
        (result, stats)
    }

    /// Get current stats without consuming the builder
    pub fn current_stats(&self) -> &EnforcementStats {
        self.enforcer.stats()
    }
}

// ============================================================================
// App Settings
// ============================================================================

fn default_outcome_branch_count() -> usize {
    2
}
fn default_outcome_branch_min() -> usize {
    1
}
fn default_outcome_branch_max() -> usize {
    4
}
fn default_conversation_history_turns() -> usize {
    20
}
fn default_suggestion_tokens_per_branch() -> u32 {
    200
}
fn default_presence_cache_ttl_hours() -> i32 {
    3
}
fn default_use_llm_presence() -> bool {
    true
}

/// All configurable application settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    /// World ID for per-world settings. None = global defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    world_id: Option<WorldId>,

    // ============================================================================
    // Session & Conversation
    // ============================================================================
    /// Maximum conversation turns to store in session memory
    max_conversation_turns: usize,

    /// Number of conversation turns to include in LLM prompt context
    #[serde(default = "default_conversation_history_turns")]
    conversation_history_turns: usize,

    // ============================================================================
    // Circuit Breaker & Health
    // ============================================================================
    circuit_breaker_failure_threshold: u32,
    circuit_breaker_open_duration_secs: u64,
    health_check_cache_ttl_secs: u64,

    // ============================================================================
    // Validation Limits
    // ============================================================================
    /// Maximum length for entity names (characters, locations, etc.)
    max_name_length: usize,
    /// Maximum length for descriptions
    max_description_length: usize,

    // ============================================================================
    // Animation (synced to Player)
    // ============================================================================
    typewriter_sentence_delay_ms: u64,
    typewriter_pause_delay_ms: u64,
    typewriter_char_delay_ms: u64,

    // ============================================================================
    // Game Defaults
    // ============================================================================
    default_max_stat_value: i32,

    // ============================================================================
    // Staging System
    // ============================================================================
    /// Default NPC presence cache TTL in game hours for new locations
    #[serde(default = "default_presence_cache_ttl_hours")]
    default_presence_cache_ttl_hours: i32,

    /// Whether to use LLM for staging decisions by default
    #[serde(default = "default_use_llm_presence")]
    default_use_llm_presence: bool,

    // ============================================================================
    // Challenge System
    // ============================================================================
    /// Number of outcome branches to generate for each challenge result tier
    #[serde(default = "default_outcome_branch_count")]
    outcome_branch_count: usize,
    /// Minimum allowed branch count (for UI validation)
    #[serde(default = "default_outcome_branch_min")]
    outcome_branch_min: usize,
    /// Maximum allowed branch count (for UI validation)
    #[serde(default = "default_outcome_branch_max")]
    outcome_branch_max: usize,

    // ============================================================================
    // LLM Settings
    // ============================================================================
    /// Max tokens per outcome branch when generating suggestions
    #[serde(default = "default_suggestion_tokens_per_branch")]
    suggestion_tokens_per_branch: u32,

    /// Token budget configuration for LLM context building
    #[serde(default)]
    context_budget: ContextBudgetConfig,

    // ============================================================================
    // Asset Generation
    // ============================================================================
    /// Default style reference asset ID for image generation
    /// When set, new asset generations will use this asset's style by default
    #[serde(default, skip_serializing_if = "Option::is_none")]
    style_reference_asset_id: Option<String>,

    /// Policy for how to handle failures while queueing prompts for a batch.
    #[serde(default = "default_batch_queue_failure_policy")]
    batch_queue_failure_policy: BatchQueueFailurePolicy,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            world_id: None,
            max_conversation_turns: 30,
            conversation_history_turns: 20,
            circuit_breaker_failure_threshold: 5,
            circuit_breaker_open_duration_secs: 60,
            health_check_cache_ttl_secs: 30,
            max_name_length: 255,
            max_description_length: 10000,
            typewriter_sentence_delay_ms: 150,
            typewriter_pause_delay_ms: 80,
            typewriter_char_delay_ms: 30,
            default_max_stat_value: 20,
            default_presence_cache_ttl_hours: 3,
            default_use_llm_presence: true,
            outcome_branch_count: 2,
            outcome_branch_min: 1,
            outcome_branch_max: 4,
            suggestion_tokens_per_branch: 200,
            context_budget: ContextBudgetConfig::default(),
            style_reference_asset_id: None,
            batch_queue_failure_policy: default_batch_queue_failure_policy(),
        }
    }
}

impl AppSettings {
    /// Create settings for a specific world from base settings
    ///
    /// # Note
    /// To load from environment variables, use `load_settings_from_env()`
    /// from `wrldbldr_engine_adapters::infrastructure::settings_loader`.
    pub fn for_world(base: AppSettings, world_id: WorldId) -> Self {
        let mut settings = base;
        settings.world_id = Some(world_id.into());
        settings
    }

    /// Merge per-world settings with global settings.
    /// Per-world values override global where present.
    pub fn merge_with_global(&self, _global: &AppSettings) -> AppSettings {
        // If this is global settings, just return a clone
        if self.world_id.is_none() {
            return self.clone();
        }

        // Start with global, then override with per-world values
        // For now, per-world settings completely override global
        // In the future, we could make this more granular with Option<T> fields
        self.clone()
    }

    // ============================================================================
    // Accessors
    // ============================================================================

    /// World ID for per-world settings. None = global defaults.
    pub fn world_id(&self) -> Option<WorldId> {
        self.world_id
    }

    /// Maximum conversation turns to store in session memory
    pub fn max_conversation_turns(&self) -> usize {
        self.max_conversation_turns
    }

    /// Number of conversation turns to include in LLM prompt context
    pub fn conversation_history_turns(&self) -> usize {
        self.conversation_history_turns
    }

    /// Circuit breaker failure threshold
    pub fn circuit_breaker_failure_threshold(&self) -> u32 {
        self.circuit_breaker_failure_threshold
    }

    /// Circuit breaker open duration in seconds
    pub fn circuit_breaker_open_duration_secs(&self) -> u64 {
        self.circuit_breaker_open_duration_secs
    }

    /// Health check cache TTL in seconds
    pub fn health_check_cache_ttl_secs(&self) -> u64 {
        self.health_check_cache_ttl_secs
    }

    /// Maximum length for entity names (characters, locations, etc.)
    pub fn max_name_length(&self) -> usize {
        self.max_name_length
    }

    /// Maximum length for descriptions
    pub fn max_description_length(&self) -> usize {
        self.max_description_length
    }

    /// Typewriter sentence delay in milliseconds
    pub fn typewriter_sentence_delay_ms(&self) -> u64 {
        self.typewriter_sentence_delay_ms
    }

    /// Typewriter pause delay in milliseconds
    pub fn typewriter_pause_delay_ms(&self) -> u64 {
        self.typewriter_pause_delay_ms
    }

    /// Typewriter character delay in milliseconds
    pub fn typewriter_char_delay_ms(&self) -> u64 {
        self.typewriter_char_delay_ms
    }

    /// Default maximum stat value
    pub fn default_max_stat_value(&self) -> i32 {
        self.default_max_stat_value
    }

    /// Default NPC presence cache TTL in game hours for new locations
    pub fn default_presence_cache_ttl_hours(&self) -> i32 {
        self.default_presence_cache_ttl_hours
    }

    /// Whether to use LLM for staging decisions by default
    pub fn default_use_llm_presence(&self) -> bool {
        self.default_use_llm_presence
    }

    /// Number of outcome branches to generate for each challenge result tier
    pub fn outcome_branch_count(&self) -> usize {
        self.outcome_branch_count
    }

    /// Minimum allowed branch count (for UI validation)
    pub fn outcome_branch_min(&self) -> usize {
        self.outcome_branch_min
    }

    /// Maximum allowed branch count (for UI validation)
    pub fn outcome_branch_max(&self) -> usize {
        self.outcome_branch_max
    }

    /// Max tokens per outcome branch when generating suggestions
    pub fn suggestion_tokens_per_branch(&self) -> u32 {
        self.suggestion_tokens_per_branch
    }

    /// Token budget configuration for LLM context building
    pub fn context_budget(&self) -> &ContextBudgetConfig {
        &self.context_budget
    }

    /// Default style reference asset ID for image generation
    pub fn style_reference_asset_id(&self) -> Option<&str> {
        self.style_reference_asset_id.as_deref()
    }

    /// Policy for how to handle failures while queueing prompts for a batch
    pub fn batch_queue_failure_policy(&self) -> BatchQueueFailurePolicy {
        self.batch_queue_failure_policy
    }

    // ============================================================================
    // Builder-style setters (consume self)
    // ============================================================================

    /// Set the world ID
    pub fn with_world_id(self, world_id: Option<WorldId>) -> Self {
        Self { world_id, ..self }
    }

    /// Set maximum conversation turns
    pub fn with_max_conversation_turns(self, max_conversation_turns: usize) -> Self {
        Self {
            max_conversation_turns,
            ..self
        }
    }

    /// Set conversation history turns
    pub fn with_conversation_history_turns(self, conversation_history_turns: usize) -> Self {
        Self {
            conversation_history_turns,
            ..self
        }
    }

    /// Set circuit breaker failure threshold
    pub fn with_circuit_breaker_failure_threshold(
        self,
        circuit_breaker_failure_threshold: u32,
    ) -> Self {
        Self {
            circuit_breaker_failure_threshold,
            ..self
        }
    }

    /// Set circuit breaker open duration in seconds
    pub fn with_circuit_breaker_open_duration_secs(
        self,
        circuit_breaker_open_duration_secs: u64,
    ) -> Self {
        Self {
            circuit_breaker_open_duration_secs,
            ..self
        }
    }

    /// Set health check cache TTL in seconds
    pub fn with_health_check_cache_ttl_secs(self, health_check_cache_ttl_secs: u64) -> Self {
        Self {
            health_check_cache_ttl_secs,
            ..self
        }
    }

    /// Set maximum name length
    pub fn with_max_name_length(self, max_name_length: usize) -> Self {
        Self {
            max_name_length,
            ..self
        }
    }

    /// Set maximum description length
    pub fn with_max_description_length(self, max_description_length: usize) -> Self {
        Self {
            max_description_length,
            ..self
        }
    }

    /// Set typewriter sentence delay in milliseconds
    pub fn with_typewriter_sentence_delay_ms(self, typewriter_sentence_delay_ms: u64) -> Self {
        Self {
            typewriter_sentence_delay_ms,
            ..self
        }
    }

    /// Set typewriter pause delay in milliseconds
    pub fn with_typewriter_pause_delay_ms(self, typewriter_pause_delay_ms: u64) -> Self {
        Self {
            typewriter_pause_delay_ms,
            ..self
        }
    }

    /// Set typewriter character delay in milliseconds
    pub fn with_typewriter_char_delay_ms(self, typewriter_char_delay_ms: u64) -> Self {
        Self {
            typewriter_char_delay_ms,
            ..self
        }
    }

    /// Set default maximum stat value
    pub fn with_default_max_stat_value(self, default_max_stat_value: i32) -> Self {
        Self {
            default_max_stat_value,
            ..self
        }
    }

    /// Set default presence cache TTL in hours
    pub fn with_default_presence_cache_ttl_hours(
        self,
        default_presence_cache_ttl_hours: i32,
    ) -> Self {
        Self {
            default_presence_cache_ttl_hours,
            ..self
        }
    }

    /// Set whether to use LLM for staging decisions by default
    pub fn with_default_use_llm_presence(self, default_use_llm_presence: bool) -> Self {
        Self {
            default_use_llm_presence,
            ..self
        }
    }

    /// Set outcome branch count
    pub fn with_outcome_branch_count(self, outcome_branch_count: usize) -> Self {
        Self {
            outcome_branch_count,
            ..self
        }
    }

    /// Set outcome branch minimum
    pub fn with_outcome_branch_min(self, outcome_branch_min: usize) -> Self {
        Self {
            outcome_branch_min,
            ..self
        }
    }

    /// Set outcome branch maximum
    pub fn with_outcome_branch_max(self, outcome_branch_max: usize) -> Self {
        Self {
            outcome_branch_max,
            ..self
        }
    }

    /// Set suggestion tokens per branch
    pub fn with_suggestion_tokens_per_branch(self, suggestion_tokens_per_branch: u32) -> Self {
        Self {
            suggestion_tokens_per_branch,
            ..self
        }
    }

    /// Set context budget configuration
    pub fn with_context_budget(self, context_budget: ContextBudgetConfig) -> Self {
        Self {
            context_budget,
            ..self
        }
    }

    /// Set style reference asset ID
    pub fn with_style_reference_asset_id(self, style_reference_asset_id: Option<String>) -> Self {
        Self {
            style_reference_asset_id,
            ..self
        }
    }

    /// Set batch queue failure policy
    pub fn with_batch_queue_failure_policy(
        self,
        batch_queue_failure_policy: BatchQueueFailurePolicy,
    ) -> Self {
        Self {
            batch_queue_failure_policy,
            ..self
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Context Budget Config Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_default_config() {
        let config = ContextBudgetConfig::default();
        assert_eq!(config.total_budget_tokens, 4000);
        assert!(config.enable_summarization);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_large_context_config() {
        let config = ContextBudgetConfig::large_context();
        assert_eq!(config.total_budget_tokens, 12000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_category_budget_access() {
        let config = ContextBudgetConfig::default();
        assert_eq!(config.budget_for(ContextCategory::Character), 800);

        let config = config.with_budget_for(ContextCategory::Character, 1000);
        assert_eq!(config.budget_for(ContextCategory::Character), 1000);
    }

    #[test]
    fn test_validation_zero_budget() {
        let config = ContextBudgetConfig::default().with_total_budget_tokens(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_category_priority_order() {
        let priorities = ContextCategory::all_by_priority();
        assert_eq!(priorities[0], ContextCategory::Character);
        assert_eq!(priorities[1], ContextCategory::Scene);
    }

    // -------------------------------------------------------------------------
    // Token Counter Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_token_counter_empty_string() {
        let counter = TokenCounter::default();
        assert_eq!(counter.count(""), 0);
    }

    #[test]
    fn test_token_counter_short_text() {
        let counter = TokenCounter::default();
        // "Hello" = 5 chars, ~1.25 tokens -> ceil = 2
        let tokens = counter.count("Hello");
        assert!(tokens >= 1 && tokens <= 3);
    }

    #[test]
    fn test_token_counter_longer_text() {
        let counter = TokenCounter::default();
        // Longer text uses word-based counting in hybrid mode
        let text = "The quick brown fox jumps over the lazy dog. This is a sample sentence to test the token counting functionality.";
        let tokens = counter.count(text);
        // ~20 words * 1.33 = ~27 tokens
        assert!(tokens >= 20 && tokens <= 35);
    }

    #[test]
    fn test_token_counter_char_method() {
        let counter = TokenCounter::new(TokenCountMethod::CharacterApprox);
        // 20 chars / 4 = 5 tokens
        let tokens = counter.count("12345678901234567890");
        assert_eq!(tokens, 5);
    }

    #[test]
    fn test_token_counter_word_method() {
        let counter = TokenCounter::new(TokenCountMethod::WordApprox);
        // 5 words * 1.33 = 6.65 -> ceil = 7 tokens
        let tokens = counter.count("one two three four five");
        assert_eq!(tokens, 7);
    }

    #[test]
    fn test_token_counter_exceeds_budget() {
        let counter = TokenCounter::default();
        let text = "This is a test sentence that should exceed a small budget.";
        assert!(counter.exceeds_budget(text, 5));
        assert!(!counter.exceeds_budget(text, 100));
    }

    #[test]
    fn test_token_counter_truncate() {
        let counter = TokenCounter::default();
        let text = "This is a fairly long sentence that we want to truncate to fit within a smaller token budget.";

        let (truncated, was_truncated) = counter.truncate_to_budget(text, 10);
        assert!(was_truncated);
        assert!(truncated.ends_with("..."));
        assert!(counter.count(&truncated) <= 15); // Allow some margin
    }

    #[test]
    fn test_token_counter_no_truncate_needed() {
        let counter = TokenCounter::default();
        let text = "Short text";

        let (result, was_truncated) = counter.truncate_to_budget(text, 100);
        assert!(!was_truncated);
        assert_eq!(result, text);
    }

    #[test]
    fn test_convenience_functions() {
        assert_eq!(count_tokens(""), 0);
        assert!(count_tokens("Hello world") > 0);
        assert!(exceeds_token_budget("Hello world", 1));
        assert!(!exceeds_token_budget("Hi", 10));
    }

    #[test]
    fn test_llama_tuned_counter() {
        let counter = TokenCounter::llama_tuned();
        // Llama tuned should give slightly different results
        let text = "This is a test sentence.";
        let default_tokens = TokenCounter::default().count(text);
        let llama_tokens = counter.count(text);
        // Should be in similar range
        assert!(llama_tokens > 0);
        assert!((default_tokens as i32 - llama_tokens as i32).abs() <= 3);
    }

    // -------------------------------------------------------------------------
    // Context Budget Enforcement Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_enforce_within_budget() {
        let config = ContextBudgetConfig::default();
        let mut enforcer = ContextBudgetEnforcer::new(config);

        let text = "Short text";
        let result = enforcer.enforce(ContextCategory::Scene, text);

        assert!(!result.was_truncated());
        assert_eq!(result.text(), text);
        assert_eq!(result.original_tokens(), result.final_tokens());
    }

    #[test]
    fn test_enforce_exceeds_budget() {
        let config = ContextBudgetConfig::default().with_scene_tokens(5); // Very small budget

        let mut enforcer = ContextBudgetEnforcer::new(config);

        let text = "This is a much longer text that should definitely exceed our tiny token budget and require truncation to fit.";
        let result = enforcer.enforce(ContextCategory::Scene, text);

        assert!(result.was_truncated());
        assert!(result.final_tokens() <= 10); // Allow some margin
        assert!(result.text().ends_with("..."));
    }

    #[test]
    fn test_stats_tracking() {
        let config = ContextBudgetConfig::default()
            .with_scene_tokens(5)
            .with_character_tokens(1000);

        let mut enforcer = ContextBudgetEnforcer::new(config);

        // This will be truncated
        enforcer.enforce(
            ContextCategory::Scene,
            "This is a long scene description that exceeds budget",
        );
        // This won't be truncated
        enforcer.enforce(ContextCategory::Character, "Short");

        let stats = enforcer.stats();
        assert_eq!(stats.categories_truncated(), 1);
        assert_eq!(stats.category_results().len(), 2);
    }

    #[test]
    fn test_context_builder() {
        let config = ContextBudgetConfig::default();
        let builder = ContextBuilder::new(config)
            .add_section(ContextCategory::Scene, "Scene content".to_string())
            .add_section(ContextCategory::Character, "Character content".to_string());

        let (result, stats) = builder.build();

        assert!(result.contains("Scene content"));
        assert!(result.contains("Character content"));
        assert_eq!(stats.category_results().len(), 2);
    }

    #[test]
    fn test_would_exceed() {
        let config = ContextBudgetConfig::default().with_scene_tokens(5);

        let enforcer = ContextBudgetEnforcer::new(config);

        assert!(enforcer.would_exceed(ContextCategory::Scene, "This is a longer text"));
        assert!(!enforcer.would_exceed(ContextCategory::Scene, "Hi"));
    }
}
