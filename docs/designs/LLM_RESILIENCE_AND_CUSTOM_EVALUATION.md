# LLM Resilience and Custom Evaluation Design

**Created:** 2026-01-10
**Status:** Draft
**Related Issues:** #10, #22, #28

---

## Overview

This document outlines the design for three related LLM integration improvements:

1. **LLM Failure Handling** (#10) - Retry strategies and circuit breaker pattern
2. **Custom Scene Conditions** (#22) - LLM-evaluated conditions with context injection
3. **Custom Narrative Triggers** (#28) - LLM-evaluated triggers (same pattern as #22)

---

## 1. LLM Failure Handling (#10)

### Current State

The LLM integration (`OllamaClient`) has basic error handling that returns errors immediately on failure. There's no retry logic, no fallback models, and no protection against cascading failures.

### Proposed Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      ResilientLlmService                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐        │
│  │ Primary LLM  │ → │ Fallback LLM │ → │ Fallback LLM │        │
│  │ (Ollama:     │   │ (Ollama:     │   │ (Claude API) │        │
│  │  llama3.2)   │   │  mistral)    │   │              │        │
│  └──────────────┘   └──────────────┘   └──────────────┘        │
│         ↓                  ↓                  ↓                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │              Exponential Backoff Retry Layer                ││
│  │  - Max retries: 3 (configurable)                            ││
│  │  - Base delay: 1s                                           ││
│  │  - Max delay: 30s                                           ││
│  │  - Jitter: ±20%                                             ││
│  └─────────────────────────────────────────────────────────────┘│
│         ↓                                                       │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Circuit Breaker                          ││
│  │  States: Closed → Open → Half-Open → Closed                 ││
│  │  - Failure threshold: 5 consecutive failures                ││
│  │  - Open duration: 60s (configurable)                        ││
│  │  - Half-open probe: 1 request                               ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### Phase 1: Exponential Backoff (Issue #10 - Not yet implemented)

**Proposed implementation in `infrastructure/llm/resilient_client.rs`:**

```rust
pub struct RetryConfig {
    pub max_retries: u32,           // Default: 3
    pub base_delay_ms: u64,         // Default: 1000
    pub max_delay_ms: u64,          // Default: 30000
    pub jitter_factor: f64,         // Default: 0.2 (±20%)
}

pub struct ResilientLlmClient {
    inner: Arc<dyn LlmPort>,
    retry_config: RetryConfig,
}

impl ResilientLlmClient {
    pub async fn generate(&self, prompt: &str) -> Result<String, LlmError> {
        let mut attempt = 0;
        let mut last_error = None;

        while attempt <= self.retry_config.max_retries {
            match self.inner.generate(prompt).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    attempt += 1;

                    if attempt <= self.retry_config.max_retries {
                        let delay = self.calculate_delay(attempt);
                        tracing::warn!(
                            attempt = attempt,
                            delay_ms = delay,
                            "LLM request failed, retrying..."
                        );
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    fn calculate_delay(&self, attempt: u32) -> u64 {
        let base = self.retry_config.base_delay_ms;
        let exponential = base * 2u64.pow(attempt - 1);
        let capped = exponential.min(self.retry_config.max_delay_ms);

        // Add jitter
        let jitter_range = (capped as f64 * self.retry_config.jitter_factor) as u64;
        let jitter = rand::thread_rng().gen_range(0..=jitter_range * 2) as i64 - jitter_range as i64;

        (capped as i64 + jitter).max(0) as u64
    }
}
```

### Phase 2: Circuit Breaker (Future Issue)

**To be tracked in separate issue.** Design notes:

```rust
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,      // Failures before opening
    pub open_duration_secs: u64,     // How long to stay open
    pub half_open_max_requests: u32, // Requests allowed in half-open
}

pub enum CircuitState {
    Closed,                          // Normal operation
    Open { until: Instant },         // Rejecting all requests
    HalfOpen { allowed: u32 },       // Testing if service recovered
}

pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failure_count: AtomicU32,
    config: CircuitBreakerConfig,
}
```

**Fallback Chain:**
1. Primary model (e.g., `llama3.2`)
2. Fallback model 1 (e.g., `mistral`)
3. Fallback model 2 (e.g., external Claude API)
4. Circuit break → reject all LLM requests until service resumes

**Configuration (per-world settings):**
```rust
pub struct LlmResilienceSettings {
    pub primary_model: String,
    pub fallback_models: Vec<String>,
    pub retry_config: RetryConfig,
    pub circuit_breaker_config: CircuitBreakerConfig,
}
```

---

## 2. Custom Conditions & Triggers (#22, #28)

### Core Insight

Custom conditions and custom triggers share the same evaluation pattern:
- **Input**: A condition/trigger definition + game context
- **Output**: Boolean (condition met / trigger should fire)
- **Evaluator**: LLM with structured context

### Unified Design: LLM Condition Evaluator

```
┌─────────────────────────────────────────────────────────────────┐
│                    CustomConditionEvaluator                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                  Condition Definition                       ││
│  │  - Natural language description                             ││
│  │  - Context hints (which entities to include)                ││
│  │  - Confidence threshold (0.0-1.0)                           ││
│  └─────────────────────────────────────────────────────────────┘│
│                            ↓                                    │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                Context Assembly                              ││
│  │  - Explicit: DM-specified lore, characters, locations       ││
│  │  - Implicit: Current scene, time, recent events             ││
│  │  - Tool-fetched: LLM can request additional context         ││
│  └─────────────────────────────────────────────────────────────┘│
│                            ↓                                    │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                  LLM Evaluation                              ││
│  │  - System prompt with evaluation instructions               ││
│  │  - Assembled context                                        ││
│  │  - Tool calls for additional context                        ││
│  │  - Structured output: { result: bool, confidence, reason }  ││
│  └─────────────────────────────────────────────────────────────┘│
│                            ↓                                    │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                Result Processing                             ││
│  │  - Apply confidence threshold                               ││
│  │  - Log reasoning for DM review                              ││
│  │  - Cache result with TTL (optional)                         ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### Domain Model

```rust
/// A condition that can only be evaluated by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCondition {
    /// Unique identifier
    pub id: CustomConditionId,

    /// Human-readable name for DM reference
    pub name: String,

    /// Natural language description of when this condition is true
    /// Example: "The player has earned the trust of at least two
    ///          members of the Thieves Guild"
    pub description: String,

    /// Explicit context hints - what the LLM should consider
    pub context_hints: ConditionContextHints,

    /// Minimum confidence required (0.0-1.0, default 0.7)
    pub confidence_threshold: f32,

    /// Whether LLM can use tools to fetch additional context
    pub allow_tool_calls: bool,

    /// Optional: Cache evaluation result for this many game-minutes
    pub cache_ttl_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionContextHints {
    /// Specific lore entries to include in context
    pub lore_ids: Vec<LoreId>,

    /// Specific characters whose relationships/state matter
    pub character_ids: Vec<CharacterId>,

    /// Specific locations whose state matters
    pub location_ids: Vec<LocationId>,

    /// Specific flags to include
    pub flag_names: Vec<String>,

    /// Specific challenges (and their outcomes) to consider
    pub challenge_ids: Vec<ChallengeId>,

    /// Specific narrative events to consider
    pub event_ids: Vec<NarrativeEventId>,

    /// Free-form context the DM wants included
    pub additional_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionEvaluationResult {
    /// Whether the condition is met
    pub result: bool,

    /// LLM's confidence in the result (0.0-1.0)
    pub confidence: f32,

    /// LLM's reasoning (for DM review/debugging)
    pub reasoning: String,

    /// Context that was actually used
    pub context_used: Vec<String>,

    /// Whether any tool calls were made
    pub tool_calls_made: Vec<String>,
}
```

### LLM Tools for Context Fetching

When `allow_tool_calls` is true, the LLM can request additional context:

```rust
pub enum ConditionEvaluationTool {
    /// Get a character's current relationships
    GetCharacterRelationships { character_id: String },

    /// Get a character's inventory
    GetCharacterInventory { character_id: String },

    /// Get flags matching a pattern
    GetFlags { pattern: String },

    /// Get recent story events
    GetRecentEvents { count: u32 },

    /// Get lore by tag
    GetLoreByTag { tag: String },

    /// Get challenge outcomes for a character
    GetChallengeOutcomes { character_id: String, challenge_id: Option<String> },

    /// Get current scene state
    GetCurrentScene { pc_id: String },

    /// Get NPC disposition toward PC
    GetNpcDisposition { npc_id: String, pc_id: String },
}
```

### Evaluation Prompt Template

```
You are evaluating whether a game condition is currently met.

## Condition
Name: {condition.name}
Description: {condition.description}

## Current Game State
{assembled_context}

## Available Tools
{tool_definitions if allow_tool_calls}

## Instructions
1. Analyze the condition description carefully
2. Review the provided game state
3. {If tools available: Use tools to fetch any additional context you need}
4. Determine if the condition is TRUE or FALSE
5. Rate your confidence (0.0 to 1.0)
6. Explain your reasoning

## Response Format
```json
{
  "result": true/false,
  "confidence": 0.0-1.0,
  "reasoning": "Your explanation here"
}
```
```

### Integration Points

**Scene Entry Conditions:**
```rust
pub enum SceneCondition {
    CompletedScene(SceneId),
    HasItem(ItemId),
    KnowsCharacter(CharacterId),
    FlagSet { flag_name: String, value: Option<bool> },
    Custom(CustomConditionId),  // ← Evaluated via LLM
}
```

**Narrative Event Triggers:**
```rust
pub enum TriggerCondition {
    EnterLocation(LocationId),
    TalkToNpc(CharacterId),
    ChallengeComplete { challenge_id: ChallengeId, success_required: bool },
    // ... other hard-coded triggers ...
    Custom(CustomConditionId),  // ← Evaluated via LLM
}
```

> **Current State:** `SceneCondition::Custom(String)` and `NarrativeTriggerType::Custom`
> exist in the codebase but always return false/unmet. See `scene.rs:306-317` and
> `narrative_event.rs:656-677` for `KNOWN LIMITATION` comments. This design proposes
> migrating from `Custom(String)` to `Custom(CustomConditionId)` with full LLM evaluation.

### Migration Strategy

The current `Custom(String)` variant will be migrated to `Custom(CustomConditionId)`:

1. **Phase 1**: Add `CustomCondition` entity to domain with `CustomConditionId` type
2. **Phase 2**: Create migration that converts existing `Custom(String)` entries to
   `CustomCondition` entities, using the string as the `description` field
3. **Phase 3**: Update `SceneCondition` and `NarrativeTriggerType` enums to use
   `Custom(CustomConditionId)` instead of `Custom(String)`
4. **Phase 4**: Wire up `CustomConditionEvaluator` to replace the hardcoded `false` returns

### Caching Strategy

To avoid repeated LLM calls for the same condition:

```rust
pub struct ConditionCache {
    cache: RwLock<HashMap<(CustomConditionId, GameTime), CachedResult>>,
}

pub struct CachedResult {
    result: ConditionEvaluationResult,
    evaluated_at: GameTime,
    expires_at: GameTime,
}
```

Cache invalidation triggers:
- Explicit game state changes (flag set, item acquired, relationship changed)
- Time-based expiry (per-condition TTL)
- Scene/location change
- DM manual invalidation

### DM Workflow

1. **Create Custom Condition:**
   - Name: "Trusted by Thieves Guild"
   - Description: "The player has earned the trust of at least two members of the Thieves Guild through completed quests or positive relationships"
   - Context Hints:
     - Characters: [Marcus the Fence, Silvia the Pickpocket, Old Tom]
     - Lore: [Thieves Guild Hierarchy]
     - Flags: ["completed_fence_job", "saved_silvia"]
   - Confidence Threshold: 0.7
   - Allow Tool Calls: Yes

2. **Use in Scene:**
   - Scene entry condition: `Custom("Trusted by Thieves Guild")`
   - When player tries to enter scene → LLM evaluates
   - If confidence < threshold → condition fails
   - DM can review reasoning in logs

3. **Use in Narrative Event:**
   - Trigger: `Custom("Trusted by Thieves Guild")`
   - Same evaluation process

---

## 3. Implementation Phases

### Phase 1: Exponential Backoff (Issue #10 - Not yet implemented)
- [ ] Create `ResilientLlmClient` wrapper
- [ ] Implement retry logic with exponential backoff
- [ ] Add configuration to `AppSettings`
- [ ] Update `LlmPort` usage to use resilient wrapper
- [ ] Add metrics/logging for retry attempts

> **Note:** `AppSettings` already contains `circuit_breaker_failure_threshold` and
> `circuit_breaker_open_duration_secs` fields (with defaults 5 and 60), but
> these are not yet wired to any implementation.

### Phase 2: Circuit Breaker (New Issue)
- [ ] Implement `CircuitBreaker` with state machine
- [ ] Add fallback model chain support
- [ ] Add health check endpoint for LLM status
- [ ] Broadcast circuit state changes to DM
- [ ] Graceful degradation when circuit open

### Phase 3: Custom Condition Evaluator (Issues #22, #28)
- [ ] Add `CustomCondition` to domain
- [ ] Create `CustomConditionEvaluator` use case
- [ ] Implement context assembly from hints
- [ ] Create evaluation prompt template
- [ ] Wire into `SceneCondition::Custom`
- [ ] Wire into `TriggerCondition::Custom`

### Phase 4: LLM Tool Calls for Context (New Issue)
- [ ] Define `ConditionEvaluationTool` enum
- [ ] Implement tool handlers
- [ ] Add tool parsing to evaluation response
- [ ] Execute tool calls during evaluation
- [ ] Re-evaluate with augmented context

### Phase 5: Caching & Optimization
- [ ] Implement `ConditionCache`
- [ ] Add cache invalidation triggers
- [ ] Add cache hit/miss metrics
- [ ] DM UI for cache management

---

## 4. New Issues to Create

1. **Circuit Breaker for LLM Service**
   - Implement circuit breaker pattern
   - Fallback model chain
   - Health monitoring and graceful degradation

2. **LLM Context Fetching Tools**
   - Tool definitions for condition evaluation
   - Handlers for each tool type
   - Integration with evaluation flow

---

## 5. Questions for Discussion

1. **Caching granularity**: Should cache be per-PC or global?
2. **Confidence threshold UI**: Should DM be able to override per-evaluation?
3. **Tool call limits**: Max tools per evaluation to prevent runaway costs?
4. **Audit logging**: How much evaluation detail to persist for debugging?

---

## Appendix: Example Conditions

### Example 1: Reputation-Based
```yaml
name: "Known to the City Watch"
description: "The player has had at least one significant interaction with the City Watch, either positive (helped them) or negative (been arrested)"
context_hints:
  character_ids: [captain_harris, guard_jenkins]
  flag_names: ["arrested_before", "helped_city_watch"]
  challenge_ids: [bribe_guard_challenge]
confidence_threshold: 0.8
```

### Example 2: Story Progress
```yaml
name: "Uncovered the Conspiracy"
description: "The player has discovered evidence linking Baron Aldric to the smuggling operation through documents, NPC testimony, or investigation"
context_hints:
  lore_ids: [smuggling_operation_lore, baron_secrets_lore]
  character_ids: [baron_aldric, dock_worker_pete, merchant_elena]
  event_ids: [found_documents_event, witness_testimony_event]
confidence_threshold: 0.75
allow_tool_calls: true
```

### Example 3: Relationship-Based
```yaml
name: "Romantic Interest Established"
description: "A mutual romantic interest has developed between the player and the specified NPC based on their interactions and relationship trajectory"
context_hints:
  character_ids: [love_interest_npc]
  additional_context: "Consider dialogue tone, gifts given, and time spent together"
confidence_threshold: 0.85
```
