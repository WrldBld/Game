# Codebase Improvement Plan (Refined)

## Overview

This plan addresses issues found in a comprehensive codebase review, refined by multiple expert reviews.

### Core Principles

1. **Hexagonal Architecture at System Boundaries ONLY** - Ports/adapters for external systems (Neo4j, LLMs, external APIs). NOT between internal features.
2. **Feature/Use-Case/Domain-Driven Internally** - Code organized by feature, use cases drive domain logic, no over-abstraction between features.
3. **Testing Strategy** - VCR support (already implemented), live testing, interaction logging for prompt analysis, E2E tests simulating real gameplay.
4. **Progressive Delivery** - Critical fixes first, then E2E WebSocket tests, then finalize the player experience.
5. **Pragmatism Over Purity** - Don't fix what isn't broken. Focus on real pain points.

### Current State Assessment

| Area | Status | Notes |
|------|--------|-------|
| Critical Bugs | 0% | Overflow, truncation, NaN not fixed |
| Memory Safety | 0% | HashMaps still unbounded |
| E2E Testing | 85% | VCR, event logging, 17 test modules exist |
| Duplication | 40% | GraphExt done, handlers not |
| Architecture | 60% | App grouped, Narrative still large but functional |

---

## Part 1: Critical Bug Fixes (IMMEDIATE)

### 1.1 Integer Overflow in Stat Rewards
**File:** `crates/engine/src/use_cases/narrative/execute_effects.rs:853`

**Problem:** Direct addition without overflow protection
```rust
let new_value = current_value + amount as i64;
```

**Fix:**
```rust
let new_value = current_value.saturating_add(amount as i64);
```

### 1.2 Unsafe i64 to i32 Truncation
**File:** `crates/domain/src/character_sheet.rs:786-810`

**Problem:** Silent truncation in `get_numeric_value()`

**Fix:**
```rust
if let Some(n) = v.as_i64() {
    return i32::try_from(n).ok();  // Returns None if out of range
}
```

### 1.3 NaN/Infinity Sentiment Values
**File:** `crates/engine/src/use_cases/narrative/execute_effects.rs:625`

**Fix:**
```rust
if !sentiment_change.is_finite() {
    return Err(EffectError::InvalidSentimentValue);
}
relationship.sentiment = (relationship.sentiment + sentiment_change).clamp(-1.0, 1.0);
```

### 1.4 Silent Error Swallowing
**File:** `crates/engine/src/use_cases/conversation/start.rs:125-131`

**Fix:**
```rust
let npc_disposition = match self.character.get_disposition(npc_id, pc_id).await {
    Ok(d) => d,
    Err(e) => {
        tracing::warn!(error = %e, "Failed to get NPC disposition");
        None
    }
};
```

---

## Part 2: Memory Safety - TTL Cache

### 2.1 Problem
**File:** `crates/engine/src/api/websocket/mod.rs:64-68`

`WsState` contains unbounded HashMaps that grow indefinitely:
- `pending_time_suggestions`
- `pending_staging_requests`
- `generation_read_state`

### 2.2 Solution: TTL-Based Cleanup

```rust
// infrastructure/cache/mod.rs
pub struct TtlCache<K, V> {
    entries: RwLock<HashMap<K, (V, Instant)>>,
    ttl: Duration,
}

impl<K: Eq + Hash + Clone, V: Clone> TtlCache<K, V> {
    pub fn new(ttl: Duration) -> Self { ... }
    pub async fn insert(&self, key: K, value: V) { ... }
    pub async fn get(&self, key: &K) -> Option<V> { ... }
    pub async fn cleanup_expired(&self) -> usize { ... }
}
```

**TTL Values:**
- `pending_time_suggestions`: 30 minutes
- `pending_staging_requests`: 1 hour
- `generation_read_state`: 5 minutes

**Cleanup:** Background task every 5 minutes.

---

## Part 3: Code Duplication Reduction

### 3.1 Neo4j Error Mapping - SKIP

**Status:** 609 occurrences of `.map_err(|e| RepoError::Database(e.to_string()))`

**Decision:** SKIP this refactoring.
- `GraphExt::run_or_err()` already exists for non-streaming operations
- Remaining duplications are on `execute()` for result streaming
- This is not a maintenance burden - the pattern is simple and consistent
- Neo4j API constraints make wrapping impractical

### 3.2 WebSocket Handler Error Patterns - HIGH PRIORITY

**Problem:** 131 instances of repeated error handling pattern

**Solution:** Generic wrapper function
```rust
// In mod.rs helpers section
async fn handle_use_case_result<T: Serialize>(
    result: Result<T, impl ToString>,
) -> Result<ResponseResult, ServerMessage> {
    match result {
        Ok(data) => Ok(ResponseResult::success(json!(data))),
        Err(e) => Ok(ResponseResult::error(ErrorCode::InternalError, e.to_string())),
    }
}

// Usage
handle_use_case_result(state.app.use_cases.lore.list(world_id).await)
```

**Why not macros:** Functions are clearer, testable, and IDE-friendly.

### 3.3 Game System Registry - MEDIUM PRIORITY

**Problem:** Three nearly-identical functions mapping game systems (45 lines duplicated)
- `has_schema_for_system()`
- `get_schema_for_system()`
- `get_provider_for_system()`

**Solution:**
```rust
pub struct GameSystemRegistry {
    systems: HashMap<String, SystemInfo>,
}

impl GameSystemRegistry {
    pub fn has_schema(&self, system_id: &str) -> bool { ... }
    pub fn get_schema(&self, system_id: &str) -> Option<CharacterSheetSchema> { ... }
    pub fn get_provider(&self, system_id: &str) -> Option<Box<dyn CharacterSheetProvider>> { ... }
}
```

### 3.4 ID Parsing Functions - LOW PRIORITY

**Problem:** 14 wrapper functions for ID parsing

**Solution:** Delete wrappers, use generic directly:
```rust
let world_id = parse_id_for_request(&world_id, request_id, WorldId::from_uuid, "Invalid world ID")?;
```

Or add trait for cleaner syntax:
```rust
impl FromWebSocketId for WorldId { ... }
let world_id = WorldId::from_ws_string(&world_id, request_id)?;
```

---

## Part 4: Architectural Improvements

### 4.1 Narrative Entity - CHANGE APPROACH, DON'T SPLIT

**Current Problem:** 10 repository dependencies injected

**Root Cause:** Entity operations need cross-domain context, but injecting repos creates testing burden.

**Solution:** Pass repos as method parameters instead of injecting:

```rust
// Instead of:
pub struct Narrative {
    repo: Arc<dyn NarrativeRepo>,
    character_repo: Arc<dyn CharacterRepo>,
    // ... 8 more repos
}

// Do this:
pub struct Narrative {
    repo: Arc<dyn NarrativeRepo>,  // Only primary
}

impl Narrative {
    pub async fn check_triggers(
        &self,
        region_id: RegionId,
        character_repo: &dyn CharacterRepo,  // Pass what you need
        location_repo: &dyn LocationRepo,
    ) -> Result<...> { ... }
}
```

**Why this works:**
- Testability: Mock only what you use
- Clarity: Method signature shows dependencies
- No extra ceremony: Use cases pass what they have

**DO NOT split the Narrative entity itself** - it has focused responsibility. The use_cases/narrative/ is already well-split into events.rs, chains.rs, decision.rs, execute_effects.rs.

### 4.2 App Container - REDUCE CEREMONY

**Current:** 529 lines, 250-line constructor

**Solution:** Group into `create_X()` methods:

```rust
impl App {
    pub fn new(config: AppConfig) -> Self {
        let repos = Self::create_repositories(&config);
        let entities = Self::create_entities(&repos);
        let use_cases = Self::create_use_cases(&entities, &repos);
        Self { entities, use_cases }
    }

    fn create_entities(repos: &Repositories) -> Entities {
        // Character-related
        let character = Arc::new(Character::new(repos.character.clone()));
        let player_character = Arc::new(PlayerCharacter::new(repos.player_character.clone()));
        // ... grouped logically
    }

    fn create_use_cases(entities: &Entities, repos: &Repositories) -> UseCases {
        UseCases {
            movement: Self::create_movement_use_cases(entities),
            conversation: Self::create_conversation_use_cases(entities),
            // ... by domain
        }
    }
}
```

### 4.3 Neo4j QueryBuilder - SKIP

**Decision:** Current `.param()` chain approach is fine.
- Adding abstraction doesn't meaningfully improve code
- Current pattern is readable and explicit
- Neo4rs API is already reasonable

### 4.4 Avoid These Traps

**DO NOT:**
1. Create wrapper services for entities (current direct access is correct)
2. Over-abstract internal feature communication (concrete types are fine internally)
3. Create adapters between features (use cases take entities directly)
4. Extract patterns before they're truly needed (3+ instances = consider extraction)

---

## Part 5: Testing Infrastructure

### 5.1 VCR (Record/Replay) - ALREADY IMPLEMENTED

**Location:** `crates/engine/src/e2e_tests/vcr_llm.rs`

**Current Strengths:**
- Sequential playback via `VecDeque<LlmRecording>`
- Environment-based mode selection (`E2E_LLM_MODE=record|playback|live`)
- Cassette serialization with request summaries
- Integration with `E2EEventLog`

**Enhancement Needed: Request Fingerprinting**

For tests with variable prompts, add fuzzy matching:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequestFingerprint {
    system_prompt_hash: Option<u64>,
    message_structure: Vec<MessageRole>,
    last_message_prefix: String,  // First 100 chars
    tool_names: Vec<String>,      // Sorted
}

impl VcrLlm {
    fn fingerprint_request(request: &LlmRequest) -> LlmRequestFingerprint { ... }
    fn find_matching_recording(&self, request: &LlmRequest) -> Option<LlmRecording> { ... }
}
```

### 5.2 Mode Selection

| Test Type | Mode | Environment |
|-----------|------|-------------|
| CI/fast | `playback` | Default (unset) |
| New test development | `record` | `E2E_LLM_MODE=record` + Ollama |
| Prompt iteration | `live` | `E2E_LLM_MODE=live` + Ollama |

### 5.3 Interaction Logging - ALREADY IMPLEMENTED

**Location:** `crates/engine/src/e2e_tests/event_log.rs`

**Enhancement:** Add test summary for analytics:

```rust
pub struct TestSummary {
    test_name: String,
    total_llm_calls: usize,
    total_tokens: usize,
    cassette_file: String,
    event_log_file: String,
}

// Append to logs/SUMMARY.jsonl after each test
```

### 5.4 E2E Test Scenarios

**Tier 1: Foundation (Mostly Complete)**
- [x] Simple conversation (`test_innkeeper_greeting`)
- [x] Multi-turn dialogue (`test_conversation_continuation`)
- [x] Movement + staging (movement_tests)

**Tier 2: Integrated Systems (To Add)**
- [ ] Challenge flow (roll, succeed/fail, effects)
- [ ] Narrative event cascade
- [ ] DM approval flow

**Tier 3: Concurrent Scenarios (Future)**
- [ ] Two players in same region
- [ ] DM + player session

### 5.5 WebSocket E2E Client (Phase 4)

For true end-to-end testing through WebSocket layer:

```rust
pub struct E2EWebSocketClient {
    sender: mpsc::Sender<ClientMessage>,
    receiver: mpsc::Receiver<ServerMessage>,
}

impl E2EWebSocketClient {
    pub async fn connect(url: &str, world_id: WorldId) -> Result<Self> { ... }
    pub async fn send_and_expect<P>(&mut self, msg: ClientMessage, predicate: P) -> Result<ServerMessage> { ... }
    pub async fn join_as_player(&mut self, user_id: &str, pc_id: PlayerCharacterId) { ... }
    pub async fn say(&mut self, message: &str) -> Result<DialogueResponse> { ... }
    pub async fn move_to(&mut self, region_id: RegionId) -> Result<MovementResponse> { ... }
}
```

### 5.6 Ensuring Determinism

- **VCR cassettes** - Record once, replay deterministically
- **Fixed clock** - Use `FixedClock` for reproducible time-based logic
- **Deterministic test data** - Seeded world state

---

## Part 6: Implementation Phases

### Phase 1A: Critical Bug Fixes (2-3 days)
**Priority: CRITICAL** | **Blocks: Production readiness**

- [ ] Fix integer overflow: `execute_effects.rs:853` → `saturating_add`
- [ ] Fix i64→i32 truncation: `character_sheet.rs:786-810` → `try_from`
- [ ] Add NaN/Infinity validation: `execute_effects.rs:625`
- [ ] Add error logging: `conversation/start.rs:125-131`

### Phase 1B: Memory Safety (3-4 days)
**Priority: CRITICAL** | **Blocks: Long-running server reliability**

- [ ] Create `TtlCache<K,V>` utility in `infrastructure/cache/`
- [ ] Replace WsState HashMaps with TtlCache instances
- [ ] Add periodic cleanup task (interval: 5 min)

### Phase 2: E2E Testing Polish (1-2 days)
**Priority: HIGH** | **Status: 85% complete**

- [x] VCR LLM recording/playback
- [x] Event logging system
- [ ] Document cassette workflow in README
- [ ] Add Tier 2 scenarios (challenge, narrative events, DM approval)
- [ ] Add request fingerprinting for variable prompts (optional)

### Phase 3: Duplication Reduction (4-5 days)
**Priority: MEDIUM** | **Can be deferred**

- [ ] Create `handle_use_case_result()` function
- [ ] Refactor top 10 handlers to use it
- [ ] Create GameSystemRegistry
- [ ] Consolidate ID parser functions (optional)

### Phase 4: Architecture Polish (DEFER)
**Priority: LOW** | **Decision: Defer until after MVP**

- [ ] Refactor Narrative entity to pass repos as parameters
- [ ] Group App constructor into `create_X()` methods
- [ ] Add WebSocket E2E client for Tier 3 scenarios

### Phase 5: Consistency Standards (Ongoing)
**Priority: LOW** | **Applied to: New code only**

- [ ] Document error handling standard
- [ ] Document constructor pattern (config structs for 4+ deps)
- [ ] Enforce in PR reviews, don't retrofit

---

## Dependency Graph

```
Phase 1A (Critical Bugs) ──┐
                          ├──→ Phase 2 (E2E Polish) ──→ "Finalize Player" MVP
Phase 1B (Memory Safety) ──┘                              │
                                                          ↓
                                           Phase 3 (Duplication) ──→ Phase 4 (Architecture)
```

**Critical Path to MVP:** Phase 1A → Phase 1B → Phase 2 → Ship

---

## Minimum Viable Set for "Finalizing the Player"

**Must do:**
1. Phase 1A - Critical bugs (non-negotiable)
2. Phase 1B - Memory safety (non-negotiable for production)
3. Phase 2 - E2E tests (85% done, polish existing)

**Skip for now:**
- Phase 4 architecture refactoring (doesn't add gameplay value)
- Fancy builder patterns (current code works)

**Do later:**
- Phase 3 duplication (nice but doesn't block features)
- Phase 5 consistency (enforce in new code, don't retrofit)

---

## Summary

| Area | Effort | Impact | Priority |
|------|--------|--------|----------|
| Critical bug fixes | 2-3 days | HIGH | 1 |
| Memory safety (TTL) | 3-4 days | HIGH | 1 |
| E2E test polish | 1-2 days | HIGH | 2 |
| Handler error patterns | 3-4 hours | MEDIUM | 3 |
| Game system registry | 2-3 hours | MEDIUM | 3 |
| Narrative refactor | 2 days | LOW | 4 |
| App container grouping | 1 day | LOW | 4 |
| Neo4j QueryBuilder | Skip | N/A | - |
