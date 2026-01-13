# Codebase Improvement Plan (Revised v2)

## Overview

This plan addresses code quality as a **primary concern**, not deferred work. Technical debt compounds—fixing it now prevents exponential growth in complexity.

### Core Principles

1. **Hexagonal Architecture at ALL System Boundaries**
   - External systems: Neo4j, LLMs, image generation, file system
   - Platform boundaries: async runtime, serialization format, time libraries
   - Protocol boundaries: WebSocket messages stay in API layer

2. **Business Logic is Platform and Dependency Agnostic**
   - Domain layer: Pure Rust, no infrastructure concerns
   - Use case layer: Depends only on ports, not concrete implementations
   - No protocol types in use cases

3. **Code Quality Over Speed**
   - Fix compounding debt now, not later
   - Tests should be fast, deterministic, and isolated
   - Errors should have context and be actionable

4. **Testing Strategy**
   - VCR for LLM interactions (deterministic replay)
   - Live testing with real services (development)
   - Full interaction logging (prompt analysis)
   - True E2E tests through WebSocket layer

---

## Current State Assessment

### Domain Layer: 9.5/10 Pure ✓
- No infrastructure leaks
- serde_json::Value usage is justified (multi-system character sheets)
- Proper ID typing throughout
- Good invariant enforcement

### Use Case Layer: CRITICAL ISSUES
- **13 files import protocol types** (wrldbldr_protocol)
- **tokio::sync::RwLock exposed** to business logic
- **serde_json::json! macro** used for responses
- Direct chrono calls instead of ClockPort

### Platform Boundaries: MISSING ABSTRACTIONS
- No FileSystemPort (logs, cassettes)
- No EnvironmentPort (configuration)
- Protocol types leak to use cases
- Async primitives not abstracted

### Technical Debt: COMPOUNDING
- WsState growing unbounded (memory leak risk)
- Monolithic use cases (1000+ lines each)
- Repository layer lacks query abstraction
- Error types inconsistent across modules

---

## Part 1: Platform Boundary Abstractions

### 1.1 Remove Protocol Types from Use Cases (CRITICAL)

**Problem:** 13 use case files import `wrldbldr_protocol::*`

**Files Affected:**
- `use_cases/staging/mod.rs` - imports ServerMessage, ApprovedNpcInfo, etc.
- `use_cases/session/*.rs` - imports WorldRole, ConnectedUser
- `use_cases/lore/mod.rs` - creates JSON responses
- And 10 more...

**Solution:** Create domain result types, let API layer translate

```rust
// BEFORE (use_cases/staging/mod.rs)
use wrldbldr_protocol::{ServerMessage, ApprovedNpcInfo, StagedNpcInfo};

pub async fn execute(&self) -> Result<(), StagingError> {
    let msg = ServerMessage::ApprovalRequest { ... };  // PROTOCOL IN USE CASE
    context.dm_notifier.notify_dms(world_id, msg).await;
}

// AFTER
// Domain types in domain layer
pub struct StagingApprovalResult {
    pub world_id: WorldId,
    pub staging_id: Uuid,
    pub npcs: Vec<StagedNpcInfo>,  // Domain type, not protocol
    pub waiting_pcs: Vec<WaitingPcInfo>,
}

// Use case returns domain type
pub async fn execute(&self) -> Result<StagingApprovalResult, StagingError> {
    Ok(StagingApprovalResult { ... })
}

// API layer translates to protocol
// In ws_staging.rs
let result = staging_use_case.execute().await?;
let msg = ServerMessage::ApprovalRequest {
    staging_id: result.staging_id.to_string(),
    npcs: result.npcs.into_iter().map(to_protocol_npc).collect(),
};
connections.notify_dms(world_id, msg).await;
```

### 1.2 Abstract Async Primitives (HIGH)

**Problem:** `tokio::sync::RwLock` exposed to use cases

```rust
// CURRENT - Use case knows about tokio
pub struct StagingApprovalContext<'a> {
    pub pending_staging_requests: &'a tokio::sync::RwLock<HashMap<String, PendingStagingRequest>>,
}
```

**Solution:** Create a port for ephemeral state

```rust
// infrastructure/ports.rs
#[async_trait]
pub trait PendingRequestStore: Send + Sync {
    async fn store(&self, key: String, request: PendingStagingRequest);
    async fn get(&self, key: &str) -> Option<PendingStagingRequest>;
    async fn remove(&self, key: &str) -> Option<PendingStagingRequest>;
    async fn list_for_world(&self, world_id: WorldId) -> Vec<PendingStagingRequest>;
}

// Use case now depends on abstraction
pub struct StagingApprovalContext<'a> {
    pub dm_notifier: &'a dyn DmNotificationPort,
    pub pending_requests: &'a dyn PendingRequestStore,  // Abstract!
}
```

### 1.3 FileSystemPort for Logs and Cassettes (MEDIUM)

**Problem:** Direct file I/O in test fixtures

```rust
// CURRENT - Direct file system access
std::fs::create_dir_all(&log_dir)?;
std::fs::write(&filepath, content)?;
```

**Solution:** Abstract file operations

```rust
// infrastructure/ports.rs
pub trait FileSystemPort: Send + Sync {
    fn write_file(&self, path: &Path, content: &[u8]) -> Result<(), IoError>;
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, IoError>;
    fn create_dir_all(&self, path: &Path) -> Result<(), IoError>;
    fn exists(&self, path: &Path) -> bool;
}

// Test implementation
pub struct InMemoryFileSystem {
    files: RwLock<HashMap<PathBuf, Vec<u8>>>,
}

// Production implementation
pub struct RealFileSystem;
```

### 1.4 Remove serde_json from Use Case Responses (HIGH)

**Problem:** Use cases construct JSON responses

```rust
// CURRENT
pub async fn create_lore(&self) -> Result<serde_json::Value, ...> {
    Ok(serde_json::json!({
        "loreId": lore_id,
        "name": name,
    }))
}
```

**Solution:** Return domain types, serialize in API layer

```rust
// Use case returns domain type
pub async fn create_lore(&self) -> Result<Lore, LoreError> {
    Ok(Lore { id, name, ... })
}

// API layer serializes
let lore = lore_use_case.create_lore(input).await?;
Ok(ResponseResult::success(json!(lore)))  // Serialization at edge
```

---

## Part 2: Critical Bug Fixes

### 2.1 Integer Overflow in Stat Rewards
**File:** `crates/engine/src/use_cases/narrative/execute_effects.rs:853`

```rust
// FIX
let new_value = current_value.saturating_add(amount as i64);
```

### 2.2 Unsafe i64 to i32 Truncation
**File:** `crates/domain/src/character_sheet.rs:786-810`

```rust
// FIX
if let Some(n) = v.as_i64() {
    return i32::try_from(n).ok();
}
```

### 2.3 NaN/Infinity Sentiment Values
**File:** `crates/engine/src/use_cases/narrative/execute_effects.rs:625`

```rust
// FIX
if !sentiment_change.is_finite() {
    return Err(EffectError::InvalidSentimentValue);
}
```

### 2.4 Silent Error Swallowing
**File:** `crates/engine/src/use_cases/conversation/start.rs:125-131`

```rust
// FIX
let npc_disposition = match self.character.get_disposition(npc_id, pc_id).await {
    Ok(d) => d,
    Err(e) => {
        tracing::warn!(error = %e, npc_id = %npc_id, "Failed to get NPC disposition");
        None
    }
};
```

---

## Part 3: Memory Safety

### 3.1 TTL Cache for WsState

**Problem:** HashMaps grow indefinitely

```rust
// CURRENT - Memory leak
pub pending_time_suggestions: RwLock<HashMap<Uuid, TimeSuggestion>>,
```

**Solution:** TTL-based cache with cleanup

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

// TTL values:
// - pending_time_suggestions: 30 minutes
// - pending_staging_requests: 1 hour
// - generation_read_state: 5 minutes
```

### 3.2 Periodic Cleanup Task

```rust
// In WebSocket server setup
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(300));
    loop {
        interval.tick().await;
        let cleaned = state.cleanup_expired().await;
        if cleaned > 0 {
            tracing::info!(entries_cleaned = cleaned, "TTL cache cleanup");
        }
    }
});
```

---

## Part 4: Structural Debt Reduction

### 4.1 Split WsState into Focused Services

**Problem:** WsState is becoming a God Object

```rust
// CURRENT - Everything in one struct
pub struct WsState {
    pub app: Arc<App>,
    pub connections: Arc<ConnectionManager>,
    pub pending_time_suggestions: RwLock<HashMap<...>>,
    pub pending_staging_requests: RwLock<HashMap<...>>,
    pub generation_read_state: RwLock<HashMap<...>>,
}
```

**Solution:** Extract into focused services

```rust
// Split by concern
pub struct WsState {
    pub app: Arc<App>,
    pub connections: Arc<ConnectionManager>,
    pub time_store: Arc<dyn TimeSuggestionStore>,
    pub staging_store: Arc<dyn PendingRequestStore>,
    pub generation_store: Arc<dyn GenerationStateStore>,
}

// Each store is a port with TTL implementation
```

### 4.2 Split Monolithic Use Cases

**Problem:** Management module is 1,499 lines with 10 CRUD operations

**Solution:** Each entity gets its own module

```
use_cases/
├── world/
│   ├── mod.rs          # WorldUseCases container
│   ├── create.rs       # CreateWorld
│   ├── update.rs       # UpdateWorld
│   └── query.rs        # ListWorlds, GetWorld
├── character/
│   ├── mod.rs
│   ├── create.rs
│   └── ...
├── staging/
│   ├── mod.rs          # StagingUseCases container
│   ├── request.rs      # RequestStagingApproval (~300 lines)
│   ├── regenerate.rs   # RegenerateSuggestion (~200 lines)
│   ├── approve.rs      # ApproveStagingRequest (~200 lines)
│   └── timeout.rs      # AutoApproveTimeout (~150 lines)
```

### 4.3 Narrative Entity - Pass Repos as Parameters

**Problem:** Narrative entity has 10 repository dependencies

```rust
// CURRENT - All repos injected
pub struct Narrative {
    repo: Arc<dyn NarrativeRepo>,
    location_repo: Arc<dyn LocationRepo>,
    // ... 8 more
}
```

**Solution:** Pass what you need

```rust
// AFTER - Only primary repo
pub struct Narrative {
    repo: Arc<dyn NarrativeRepo>,
}

impl Narrative {
    pub async fn check_triggers(
        &self,
        region_id: RegionId,
        ctx: &TriggerContext,  // Contains what this method needs
    ) -> Result<Vec<TriggeredEvent>, NarrativeError> { ... }
}

pub struct TriggerContext<'a> {
    pub character_repo: &'a dyn CharacterRepo,
    pub location_repo: &'a dyn LocationRepo,
    pub flag_repo: &'a dyn FlagRepo,
}
```

---

## Part 5: Error Handling Unification

### 5.1 Unified Error Strategy

**Problem:** Inconsistent error types (RepoError, ManagementError, SessionError, etc.)

**Solution:** Layered errors with context

```rust
// Domain errors - business rule violations
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Validation failed: {message}")]
    Validation { message: String, field: Option<String> },

    #[error("{entity_type} not found: {id}")]
    NotFound { entity_type: &'static str, id: String },

    #[error("Invariant violated: {0}")]
    InvariantViolation(String),
}

// Infrastructure errors - external system failures
#[derive(Debug, Error)]
pub enum InfraError {
    #[error("Database error: {message}")]
    Database { message: String, query: Option<String> },

    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Use case errors - wrap domain + infra with context
#[derive(Debug, Error)]
pub enum StagingError {
    #[error("Staging failed for region {region_id}: {source}")]
    Failed { region_id: RegionId, #[source] source: Box<dyn std::error::Error + Send + Sync> },

    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error(transparent)]
    Infra(#[from] InfraError),
}
```

### 5.2 Add Context to RepoError

```rust
// CURRENT - No context
pub enum RepoError {
    #[error("Not found")]
    NotFound,
}

// AFTER - Actionable errors
pub enum RepoError {
    #[error("{entity_type} not found: {id}")]
    NotFound { entity_type: &'static str, id: String },

    #[error("Database error in {operation}: {message}")]
    Database { operation: &'static str, message: String },
}
```

---

## Part 6: Testing Infrastructure

### 6.1 VCR for LLM Interactions (Already 85% Done)

**Enhancements needed:**
- Request fingerprinting for fuzzy matching
- Cassette versioning
- Document workflow

### 6.2 Separate Test Types

```rust
// Unit tests - mocked dependencies, fast
#[cfg(test)]
mod tests {
    #[test]
    fn test_staging_decision_logic() {
        // No LLM, no database, pure logic
    }
}

// Integration tests - real ports, mocked externals
#[cfg(test)]
mod integration_tests {
    #[tokio::test]
    async fn test_staging_with_mock_llm() {
        // Uses MockLlmPort, tests integration
    }
}

// E2E tests - full stack with VCR
#[cfg(test)]
mod e2e_tests {
    #[tokio::test]
    async fn test_full_conversation_flow() {
        // VCR cassettes, full system
    }
}

// Live tests - real services, marked ignore
#[tokio::test]
#[ignore = "requires ollama"]
async fn test_llm_prompt_quality() {
    // Real LLM, for development only
}
```

### 6.3 WebSocket E2E Client

```rust
pub struct E2EWebSocketClient {
    sender: mpsc::Sender<ClientMessage>,
    receiver: mpsc::Receiver<ServerMessage>,
}

impl E2EWebSocketClient {
    pub async fn connect(url: &str) -> Result<Self> { ... }
    pub async fn join_world(&mut self, world_id: Uuid, role: WorldRole) -> Result<WorldSnapshot> { ... }
    pub async fn say(&mut self, message: &str) -> Result<NarrativeResponse> { ... }
    pub async fn move_to(&mut self, region_id: Uuid) -> Result<MovementResult> { ... }
    pub async fn expect_message<P>(&mut self, predicate: P, timeout: Duration) -> Result<ServerMessage> { ... }
}
```

---

## Part 7: Implementation Phases

### Phase 1: Critical Fixes + Platform Boundaries (Week 1) ✅ COMPLETE
**Priority: CRITICAL** | **No deferral**

**Bug Fixes:**
- [x] Fix integer overflow: `execute_effects.rs:853` (saturating_add)
- [x] Fix i64→i32 truncation: `character_sheet.rs:786-810` (try_from)
- [x] Add NaN validation: `execute_effects.rs:625` (is_finite check)
- [x] Add error logging: `conversation/start.rs:125-131`

**Memory Safety:**
- [x] Create TtlCache utility (`infrastructure/cache.rs`)
- [x] Replace WsState HashMaps with TtlCache
- [x] Add cleanup task in main.rs

**Platform Boundaries:**
- [x] Create PendingRequestStore port (`staging/ports.rs`)
- [x] Create TimeSuggestionStore port (`staging/ports.rs`)
- [x] Move tokio::sync::RwLock behind ports

### Phase 2: Protocol Decoupling (Week 1-2) ✅ COMPLETE
**Priority: HIGH** | **Prevents further coupling**

- [x] Create domain result types for staging
- [x] Create domain result types for session
- [x] Remove wrldbldr_protocol imports from use cases
- [x] Move serde_json::json! to API layer (domain result types)
- [x] Update affected use case files (lore, narrative, story_events)

### Phase 3: Structural Improvements (Week 2-3) ✅ COMPLETE
**Priority: HIGH** | **Reduces compound risk**

- [x] Split WsState into focused services (TtlCache-based stores)
- [x] Split ManagementUseCases into entity modules (10 modules)
- [x] Split StagingUseCases into focused modules (7 modules)
- [ ] Refactor Narrative entity (deferred - works as-is, 10 deps)

### Phase 4: Error Handling (Week 3) - DEFERRED
**Priority: MEDIUM** | **Improves debugging**
**Status:** Current error logging with tracing context is adequate for MVP.

- [ ] Create layered error types (would touch 23+ files)
- [ ] Add context to RepoError (breaking change)
- [ ] Update use case errors (already have good patterns)
- [ ] Add error logging middleware (existing logging sufficient)

### Phase 5: Testing Polish (Week 3-4) - DEFERRED
**Priority: MEDIUM** | **Enables confident changes**
**Status:** VCR system already documented in vcr_llm.rs. E2E tests functional.

- [x] Document VCR workflow (inline docs in vcr_llm.rs)
- [ ] Add request fingerprinting (nice-to-have)
- [ ] Create WebSocket E2E client (nice-to-have)
- [ ] Add Tier 2 test scenarios (nice-to-have)
- [ ] Separate unit/integration/e2e tests (already organized)

### Phase 6: Ongoing Standards
**Priority: LOW** | **Applied to new code**

- [ ] Document architecture decisions (ADRs)
- [ ] Code review checklist
- [ ] New module template

---

## Dependency Graph

```
Phase 1 (Critical + Boundaries) ─────────────────────────────────┐
         │                                                        │
         ▼                                                        │
Phase 2 (Protocol Decoupling) ──→ Phase 3 (Structure) ──→ MVP ◀──┘
         │                              │
         ▼                              ▼
Phase 4 (Errors) ◀────────────── Phase 5 (Testing)
```

**Critical Path:** Phase 1 → Phase 2 → Phase 3 → MVP

---

## What We're NOT Deferring

Unlike the previous plan, these items are **mandatory before MVP**:

| Item | Why Not Defer |
|------|---------------|
| Protocol decoupling | Coupling compounds - more use cases will import protocol types |
| WsState split | Memory leak risk in production |
| Async primitive abstraction | Testing becomes impossible without this |
| Error context | Debugging production issues requires actionable errors |
| Monolithic use case split | 1500-line files will only grow larger |

---

## What We ARE Deferring (Safe to Defer)

| Item | Why Safe |
|------|----------|
| Repository query builder | Current pattern works, just verbose |
| Handler consolidation | 24 files is manageable, not growing fast |
| Full ADR documentation | Can document incrementally |
| N+1 query optimization | No evidence of performance issues yet |

---

## Summary

| Phase | Focus | Effort | Impact |
|-------|-------|--------|--------|
| 1 | Critical fixes + platform boundaries | 5-6 days | Prevents memory leaks, enables testing |
| 2 | Protocol decoupling | 4-5 days | Stops coupling from spreading |
| 3 | Structural improvements | 5-7 days | Reduces complexity |
| 4 | Error handling | 2-3 days | Improves debugging |
| 5 | Testing polish | 3-4 days | Enables confident changes |

**Total: ~20-25 days of focused work**

This is NOT tech debt to fix later—it's foundation work that prevents exponential complexity growth.
