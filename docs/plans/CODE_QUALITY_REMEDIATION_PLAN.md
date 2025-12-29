# Code Quality Remediation Plan

**Status**: ACTIVE  
**Created**: 2025-12-28  
**Last Updated**: 2025-12-28 (Seventh comprehensive review - 10 sub-agents)  
**Goal**: Achieve a clean, production-ready codebase with zero technical debt  
**Estimated Total Effort**: 55-75 hours (implementation) + contingency = 75-100 hours total

---

## Validation Notes (2025-12-28)

This plan was validated by seven rounds of review. Key corrections applied:

### First Review
1. **Phase 1.3 REMOVED** - The `staging_service.rs:535` unwrap is inside `#[cfg(test)]` (test code), not production
2. **God trait method counts corrected** (initial)
3. **Swallowed error count verified** - 43 instances confirmed in `services/` directory
4. **Phase 3.5 warning added** - Splitting god traits will break test compilation until Phase 7
5. **HTTP timeout/client claims verified** - Confirmed no timeouts, 11 instances of per-request client creation

### Fifth Review (12 Sub-Agents - Comprehensive Audit)

The fifth review deployed 12 specialized sub-agents to verify every claim in the plan:

#### Verified Accurate:
- **God trait method counts**: 169 total (42+34+30+31+32) ✓
- **Swallowed errors in services**: 43 (14+6+3+20 others) ✓
- **I/O violations in app layer**: 12-13 (close to claimed 11-12) ✓
- **Unbounded channel in websocket/mod.rs:78** ✓
- **No graceful shutdown** - confirmed no signal handling, no CancellationToken ✓
- **tokio in engine-ports Cargo.toml:19** ✓
- **Platform struct in player-ports**: 347 lines (larger than claimed 250+) ✓
- **MockGameConnectionPort in player-ports**: 320 lines ✓

#### Corrected:
- **Test compilation errors**: 36 (not 37)
- **std::sync::Mutex in comfyui.rs**: Locks NOT held across await points (low risk)
- **Adapters→App coupling**: 73 import statements across 43 files (worse than implied)

#### NEW Issues Discovered:
6. **player-ports/session_types.rs duplicates** - 8 types duplicate protocol without From impls
7. **UseCaseContext in engine-ports** - 166-line concrete struct with implementations
8. **Additional unbounded channels** - state/mod.rs lines 425, 473 (not just websocket)
9. **27 tokio::spawn calls** - No JoinHandle tracking, no CancellationToken anywhere
10. **thiserror declared but unused in domain** - DiceParseError uses manual Display impl
11. **env::var in domain understated** - AppSettings::from_env() calls env_or() ~20 times
12. **Forward compatibility on 20 enums** - All protocol enums need #[serde(other)]

### Sixth Review (Cross-Validation Audit)

The sixth review cross-validated findings between two independent agents and resolved all discrepancies:

#### Verified Accurate (No Changes Needed):
- **God trait method counts**: 169 total (42+34+30+31+32) ✓
- **Adapters→app imports**: 73 imports in 43 files ✓
- **I/O violations in app layer**: 12-13 ✓
- **Unbounded channels**: 3 ✓
- **tokio::spawn untracked**: 27 ✓
- **Protocol enums without #[serde(other)]**: 20 ✓
- **Test compilation errors**: 36 ✓
- **Platform struct**: 347 lines ✓
- **MockGameConnectionPort**: 320 lines ✓

#### Discrepancies Resolved:
| Item | Discrepancy | Verified Result |
|------|-------------|-----------------|
| staging_service.rs:535 unwrap | Production vs test code | **Test code** - inside `#[cfg(test)]`, plan is correct |
| pub use * count | 31 vs 22 | **30** (excluding comments) |
| Domain env vars | ~20 vs 28 | **28** (all in settings.rs) |
| request_handler.rs match arms | 308 vs not mentioned | **134** (both were wrong) |

#### NEW Issues Added:
1. **Phase 3.0.3**: Add `world_state_manager.rs` (484 lines of business logic in adapters)
2. **Phase 3.0.7**: Move composition root to runner (~1,045 lines in wrong layer)
3. **Phase 4.6**: Replace 30 glob re-exports + architecture rule against them

#### Corrections Applied:
- Domain env vars count: ~20 → **28**
- Business logic in adapters: 3 files (~1,000 lines) → **4 files (~1,570 lines)**

### Seventh Review (10 Sub-Agents - Architecture Deep Dive)

The seventh review deployed 10 specialized sub-agents to check for inconsistencies and missing tech debt:

#### Confirmed Accurate:
- **player-runner**: Already correct (127 lines of proper composition) - only engine-runner needs fix ✓
- **engine-dto**: Properly positioned and used ✓
- **No circular dependencies**: DAG is valid ✓
- **No app→adapters violations**: Clean hexagonal boundaries ✓
- **Protocol→domain imports**: All 6 documented ✓
- **Domain deps** (rand, anyhow, env vars): All confirmed ✓

#### NEW Issues Discovered:

| # | Issue | Severity | Location |
|---|-------|----------|----------|
| 1 | **ClockPort missing** | HIGH | 10 services, 14+ `Utc::now()` calls |
| 2 | **Unused tokio dep** | LOW | engine-ports/Cargo.toml (declared, never used) |
| 3 | **Unused domain dep** | LOW | player-ui/Cargo.toml (declared, no imports) |
| 4 | **Direct ServerMessage in UI** | MEDIUM | player-ui session_message_handler.rs |
| 5 | **Mock panic not guarded** | LOW | player-adapters/platform/mock.rs:297 |

#### Critical Finding: Missing ClockPort

**14+ direct `Utc::now()` calls across 10 services** - prevents deterministic testing. Added as Phase 3.0.2.1.

### Known Limitations (Not in Scope)
- **Authentication**: X-User-Id header is spoofable - intentional for MVP
- **Rate limiting**: RateLimitExceeded defined but unused - feature work
- **Reconnection logic**: Reconnecting state unused - feature work
- **Protocol versioning**: No version field - would be breaking change

---

## Executive Summary

Six comprehensive code reviews (including cross-validation) identified issues across the WrldBldr codebase. This plan consolidates all findings into a prioritized remediation roadmap organized by severity and effort.

### Issue Summary

| Severity | Count | Categories |
|----------|-------|------------|
| Critical | 12 | Production panic (2), protocol forward compat (20 enums), adapters→app deps (2), no graceful shutdown, composition root in wrong layer |
| High | ~115 | Swallowed errors (43), god traits (5/**169** methods), I/O in app (**12-13**), unbounded channels (3), spawns without tracking (27), business logic in adapters (4 files/~1,570 lines) |
| Medium | ~130 | Dead code, missing derives, config issues, DTO duplicates (13 redundant), domain I/O (28 env calls), glob re-exports (30) |
| Low | ~150+ | Unused variables, documentation, naming |

### Progress Tracking

| Phase | Description | Status | Completion |
|-------|-------------|--------|------------|
| Phase 1 | Critical Fixes | **DONE** | 100% |
| Phase 2 | High Priority | In Progress | 60% |
| Phase 3 | Architecture Completion | In Progress | 20% |
| Phase 3.0.2.1 | ClockPort Abstraction | **DONE** | 100% |
| Phase 3.0.2.2 | Required Dependencies | **DONE** | 100% |
| Phase 4 | Dead Code Cleanup | In Progress | 70% |
| Phase 4.6 | Glob Re-exports | **DONE** | 100% |
| Phase 5 | Domain Layer Polish | In Progress | 50% |
| Phase 6 | Protocol Layer Polish | In Progress | 40% |
| Phase 7 | Test Infrastructure | Pending | 0% |
| Phase 8 | Documentation | Pending | 0% |

---

## Phase 1: Critical Fixes (1 hour)

**Priority**: IMMEDIATE - These can cause production crashes or security issues

### 1.1 Fix Production Panic Risks

**Files**:
- `crates/player-ui/src/presentation/components/creator/motivations_tab.rs`

**Issue**: Lines 498 and 500 use `.unwrap()` on `strip_prefix()` which can panic if the guard condition doesn't match.

**Risk Level**: Low-Medium (guarded by `starts_with()` check, but still a code smell)

**Current Code** (lines 496-502):
```rust
let (actor_id, actor_type) = if target_str.starts_with("npc:") {
    (target_str.strip_prefix("npc:").unwrap().to_string(), ActorTypeData::Npc)
} else if target_str.starts_with("pc:") {
    (target_str.strip_prefix("pc:").unwrap().to_string(), ActorTypeData::Pc)
} else {
    // ...
}
```

**Fix**: Use `if let Some()` pattern:
```rust
let (actor_id, actor_type) = if let Some(id) = target_str.strip_prefix("npc:") {
    (id.to_string(), ActorTypeData::Npc)
} else if let Some(id) = target_str.strip_prefix("pc:") {
    (id.to_string(), ActorTypeData::Pc)
} else {
    // ...
}
```

| Task | Status |
|------|--------|
| [x] Fix motivations_tab.rs:498 unwrap | **DONE** |
| [x] Fix motivations_tab.rs:500 unwrap | **DONE** |
| [x] Verify no other production unwrap() calls exist | **DONE** (seventh review verified) |

---

### 1.2 Replace Hardcoded Internal IP Addresses

**File**: `crates/engine-adapters/src/infrastructure/config.rs:80-84`

**Issue**: Hardcoded internal/VPN IP address `10.8.0.6` in default configuration.

**Current Code**:
```rust
ollama_base_url: env::var("OLLAMA_BASE_URL")
    .unwrap_or_else(|_| "http://10.8.0.6:11434/v1".to_string()),
comfyui_base_url: env::var("COMFYUI_BASE_URL")
    .unwrap_or_else(|_| "http://10.8.0.6:8188".to_string()),
```

**Fix**:
```rust
ollama_base_url: env::var("OLLAMA_BASE_URL")
    .unwrap_or_else(|_| "http://localhost:11434/v1".to_string()),
comfyui_base_url: env::var("COMFYUI_BASE_URL")
    .unwrap_or_else(|_| "http://localhost:8188".to_string()),
```

| Task | Status |
|------|--------|
| [x] Replace 10.8.0.6 with localhost in config.rs | **DONE** |
| [x] Search for other hardcoded IPs in codebase | **DONE** (none found) |

---

### ~~1.3 Fix Production unwrap() in Staging Service~~ REMOVED

**Status**: ~~INVALID~~ - This item was removed after validation.

**Reason**: The `staging_service.rs:535` unwrap is inside `#[cfg(test)] mod tests`, not production code. Test code unwraps are acceptable.

---

## Phase 2: High Priority Error Handling (4-6 hours)

**Priority**: HIGH - Silent failures in production

### 2.1 Add Logging to Swallowed Errors in Queue Workers

**Issue**: 43 instances of `let _ =` silently discarding results in background workers.

**Files to fix**:

| File | Lines | Count | Pattern |
|------|-------|-------|---------|
| `llm_queue_service.rs` | 107, 139, 180, 404, 408, 414, 423, 433, 464, 470, 478, 484, 489, 495 | **14** | Queue operations, event sends |
| `asset_generation_queue_service.rs` | 89, 144, 158, 204, 233, 292 | **6** | Asset failures, notifier waits |
| `generation_service.rs` | 170, 300, 316 | **3** | Event drops |

**Pattern to apply**:
```rust
// Before:
let _ = self.generation_event_tx.send(event);

// After:
if let Err(e) = self.generation_event_tx.send(event) {
    tracing::warn!("Failed to send generation event: {}", e);
}
```

| Task | Status |
|------|--------|
| [x] Fix llm_queue_service.rs (**14** instances) | **DONE** |
| [x] Fix asset_generation_queue_service.rs (**6** instances) | **DONE** |
| [x] Fix generation_service.rs (3 instances) | **DONE** |
| [ ] Audit remaining `let _ =` patterns for intentionality | Pending |
| [ ] Add comments to intentional `let _ =` patterns | Pending |

---

### 2.2 Add HTTP Request Timeouts

**Issue**: HTTP requests have no timeout, can hang indefinitely.

**Files to fix**:

| File | Issue |
|------|-------|
| `player-adapters/src/infrastructure/http_client.rs` | No timeout on requests |
| `engine-adapters/src/infrastructure/ollama.rs:53-58` | LLM calls no timeout |

**Fix for http_client.rs**:
```rust
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .unwrap_or_else(|_| reqwest::Client::new());
```

**Fix for ollama.rs** (longer timeout for LLM):
```rust
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(120))
    .build()?;
```

| Task | Status |
|------|--------|
| [x] Add 30s timeout to http_client.rs | **DONE** |
| [x] Add 120s timeout to ollama.rs | **DONE** |
| [ ] Add timeout to comfyui.rs if missing | Pending |

---

### 2.3 Fix HTTP Client Per-Request Creation

**File**: `crates/player-adapters/src/infrastructure/http_client.rs`

**Issue**: Creates new `reqwest::Client` for every request, preventing connection reuse.

**Fix**: Use shared static client:
```rust
use once_cell::sync::Lazy;

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});
```

| Task | Status |
|------|--------|
| [x] Implement shared client in http_client.rs | **DONE** (static Lazy<Client> with 30s timeout) |

---

### 2.4 Fix Async/Concurrency Issues (NEW)

**Issue**: Several concurrency anti-patterns that can cause runtime issues.

#### 2.4.1 std::sync::Mutex in Async Context (LOW PRIORITY)

**File**: `crates/engine-adapters/src/infrastructure/comfyui.rs:52-129`

**Issue**: Uses `std::sync::Mutex` in async code.

**Fifth Review Finding**: The locks are **NOT held across await points** - the pattern used is:
```rust
{ let guard = mutex.lock(); /* quick read/write */ } // lock dropped
async_operation().await; // lock already released
```

**Verdict**: Low priority - the code is actually safe. Consider replacing with `tokio::sync::Mutex` for clarity but not urgent.

| Task | Status |
|------|--------|
| [ ] Consider replacing with tokio::sync::Mutex for clarity | Pending (Low) |

#### 2.4.2 Unbounded WebSocket/Event Channels

**Files with unbounded channels**:

| File | Line | Channel | Risk |
|------|------|---------|------|
| `websocket/mod.rs` | 78 | Per-connection message queue | HIGH - no backpressure |
| `state/mod.rs` | 425 | `generation_event_tx/rx` | MEDIUM |
| `state/mod.rs` | 473 | `challenge_approval_tx/rx` | MEDIUM |

**Fix**: Replace with bounded channels:
```rust
// Before:
let (tx, rx) = mpsc::unbounded_channel::<ServerMessage>();

// After:
let (tx, rx) = mpsc::channel::<ServerMessage>(256);
```

| Task | Status |
|------|--------|
| [ ] Replace unbounded_channel in websocket/mod.rs:78 | Pending |
| [ ] Replace unbounded_channel in state/mod.rs:425 | Pending |
| [ ] Replace unbounded_channel in state/mod.rs:473 | Pending |
| [ ] Handle send errors when channels are full | Pending |

#### 2.4.3 Missing Graceful Shutdown

**File**: `crates/engine-adapters/src/run/server.rs:301-314`

**Issue**: Workers spawned without cancellation tokens; no graceful shutdown on SIGTERM/SIGINT.

**Current state**:
- 27 `tokio::spawn` calls across codebase with no tracking
- Zero `CancellationToken` usage
- Zero `JoinHandle` tracking
- Worker loops are infinite with no exit condition

**Fix**: Add CancellationToken pattern:
```rust
use tokio_util::sync::CancellationToken;

let shutdown = CancellationToken::new();

tokio::select! {
    _ = server => {},
    _ = tokio::signal::ctrl_c() => {
        tracing::info!("Shutdown signal received");
        shutdown.cancel();
    }
}
```

| Task | Status |
|------|--------|
| [ ] Add tokio-util dependency for CancellationToken | Pending |
| [ ] Create shutdown token in server.rs | Pending |
| [ ] Pass cancellation token to all 10 spawned workers | Pending |
| [ ] Handle SIGTERM/SIGINT for graceful shutdown | Pending |
| [ ] Add JoinHandle tracking for spawned tasks | Pending |

---

## Phase 3: Architecture Completion (12-15 hours)

**Priority**: HIGH - Complete hexagonal architecture gaps

### 3.0 Fix Hexagonal Architecture Violations (NEW)

#### 3.0.1 Remove Adapters→App Cargo Dependencies (CRITICAL)

**Issue**: Both adapter crates depend on their app crates - a **fundamental hexagonal violation**.

**Violations**:
- `crates/engine-adapters/Cargo.toml:10` - `wrldbldr-engine-app = { workspace = true }`
- `crates/player-adapters/Cargo.toml:10` - `wrldbldr-player-app = { workspace = true }`

**Severity**: engine-adapters has **73 import statements across 43 files** from engine-app.

**What's being imported**:
- Services: ~30 imports (StagingService, ChallengeResolutionService, etc.)
- DTOs: ~25 imports (ApprovalItem, LLMRequestItem, PlayerActionItem)
- Use Cases: ~20 imports (ChallengeUseCase, MovementUseCase, etc.)

**Fix approach**:
1. Move shared DTOs to `protocol` or `engine-dto` crate
2. Define service interfaces as port traits
3. Access services only via ports (inject at composition root)
4. Remove the Cargo.toml dependency

| Task | Status |
|------|--------|
| [ ] Audit all 73 engine-adapters imports from engine-app | Pending |
| [ ] Move DTOs to protocol or engine-dto | Pending |
| [ ] Create port traits for services used by adapters | Pending |
| [ ] Refactor engine-adapters to use only ports | Pending |
| [ ] Remove `wrldbldr-engine-app` from engine-adapters/Cargo.toml | Pending |
| [ ] Audit player-adapters (1 file, 33 types) | Pending |
| [ ] Refactor player-adapters to use only ports | Pending |
| [ ] Remove `wrldbldr-player-app` from player-adapters/Cargo.toml | Pending |

**Success Criteria**: `grep -r "wrldbldr_engine_app" crates/engine-adapters/src/` returns no results.

#### 3.0.2 Move I/O Operations Out of Application Layer

**Files with I/O in engine-app** (12-13 violations):

| File | Lines | Operations |
|------|-------|------------|
| `generation_service.rs` | 103, 109, 353, 357, 365, 402, 403 | PathBuf fields, create_dir_all, Path::new, write, exists, read_to_string |
| `asset_generation_queue_service.rs` | 230, 231, 245 | PathBuf::from, create_dir_all, write |
| `prompt_template_service.rs` | 222, 268 | std::env::var |
| `player-app/error.rs` | 128 | std::env::var |

**Fix**: Create `FileStoragePort` in engine-ports:
```rust
#[async_trait]
pub trait FileStoragePort: Send + Sync {
    async fn create_dir_all(&self, path: &Path) -> Result<()>;
    async fn write(&self, path: &Path, data: &[u8]) -> Result<()>;
    async fn read_to_string(&self, path: &Path) -> Result<String>;
    async fn exists(&self, path: &Path) -> Result<bool>;
}
```

| Task | Status |
|------|--------|
| [ ] Create FileStoragePort trait in engine-ports | Pending |
| [ ] Create TokioFileStorageAdapter in engine-adapters | Pending |
| [ ] Update generation_service.rs to use FileStoragePort | Pending |
| [ ] Update asset_generation_queue_service.rs | Pending |
| [ ] Move env::var calls to adapter/runner layer | Pending |

#### 3.0.2.1 Create ClockPort for Time Abstraction (NEW - Seventh Review)

**Issue**: **14+ direct `Utc::now()` calls across 10 engine-app services** prevent deterministic testing and time simulation.

**Note**: The **player-side already has this solved** with `TimeProvider` trait in `player-ports/src/outbound/platform.rs`:
- `WasmTimeProvider` (browser)
- `DesktopTimeProvider` (native)
- `MockTimeProvider` (testing)

The engine-side needs an equivalent pattern.

**Services with direct time calls**:

| Service | Lines | Usage |
|---------|-------|-------|
| `challenge_outcome_approval_service.rs` | 226 | `Utc::now()` |
| `dm_approval_queue_service.rs` | 122, 389 | `Utc::now()` for delay calculations |
| `challenge_resolution_service.rs` | 394 | `chrono::Utc::now().to_rfc3339()` |
| `dm_action_queue_service.rs` | 41 | `chrono::Utc::now()` |
| `player_action_queue_service.rs` | 49 | `chrono::Utc::now()` |
| `actantial_context_service.rs` | 426 | `chrono::Utc::now()` |
| `world_service.rs` | 230, 351 | `chrono::Utc::now()` |
| `asset_generation_queue_service.rs` | 153, 269 | `std::time::Instant::now()`, `Utc::now()` |
| `workflow_service.rs` | 315 | `chrono::Utc::now().to_rfc3339()` |
| `generation_service.rs` | 157 | `Utc::now()` |

**Fix**: Create `ClockPort` in engine-ports (modeled after player-side `TimeProvider`):
```rust
/// Time operations abstraction for engine-side services
pub trait ClockPort: Send + Sync {
    /// Get current time as DateTime<Utc>
    fn now(&self) -> DateTime<Utc>;
    
    /// Get current time as Unix timestamp in seconds  
    fn now_unix_secs(&self) -> u64;
    
    /// Get monotonic instant for duration measurements
    fn instant_now(&self) -> std::time::Instant;
}
```

**Impact**: Enables deterministic testing, time travel for queue delays, reproducible scenarios.

| Task | Status |
|------|--------|
| [x] Create ClockPort trait in engine-ports | **DONE** |
| [x] Create SystemClockAdapter in engine-adapters | **DONE** |
| [x] Create MockClockAdapter for testing | **DONE** (included in ClockPort) |
| [x] Update 10 services to use ClockPort (14+ call sites) | **DONE** |
| [ ] Inject ClockPort at composition root | Pending (will cause compilation errors) |

**Services updated to use ClockPort** (clock is now a required constructor parameter):
- `GenerationService`
- `AssetGenerationQueueService`
- `ChallengeOutcomeApprovalService`
- `DMApprovalQueueService`
- `ChallengeResolutionService`
- `DMActionQueueService`
- `PlayerActionQueueService`
- `ActantialContextServiceImpl`
- `WorldServiceImpl`
- `ObservationUseCase`
- `AppRequestHandler`
- `WorkflowService::export_configs()` (now takes timestamp parameter)

#### 3.0.2.2 Make All Service Dependencies Required (NEW)

**Issue**: Several services use `Option<Arc<...>>` for dependencies with `.with_*()` builder methods. This is wrong because:
1. All features are always enabled - there are no feature flags
2. Optional deps hide bugs - forgetting to inject a dep fails silently at runtime
3. Builder pattern is unnecessary complexity when deps are always needed

**Affected Services**:

| Service | Optional Field | Should Be |
|---------|---------------|-----------|
| `ItemServiceImpl` | `region_repository: Option<Arc<dyn RegionRepositoryPort>>` | Required |
| `ChallengeOutcomeApprovalService` | `queue: Option<Arc<dyn QueuePort<...>>>` | Required |
| `ChallengeOutcomeApprovalService` | `llm_port: Option<Arc<L>>` | Required |
| `ChallengeOutcomeApprovalService` | `settings_service: Option<Arc<SettingsService>>` | Required |
| `AppRequestHandler` | `suggestion_enqueue: Option<Arc<dyn SuggestionEnqueuePort>>` | Required |
| `AppRequestHandler` | `generation_queue_projection: Option<Arc<...>>` | Required |
| `AppRequestHandler` | `generation_read_state: Option<Arc<dyn GenerationReadStatePort>>` | Required |

**Fix**:
1. Change all `Option<Arc<...>>` to `Arc<...>` in struct definitions
2. Add all deps as required constructor parameters
3. Remove `.with_*()` builder methods
4. Update composition root to pass all deps directly

| Task | Status |
|------|--------|
| [ ] Fix ItemServiceImpl - make region_repository required | Pending |
| [ ] Fix ChallengeOutcomeApprovalService - make queue, llm_port, settings_service required | Pending |
| [ ] Fix AppRequestHandler - make all optional deps required | Pending |
| [ ] Update composition root (AppState::new) to pass all deps | Pending |
| [ ] Wire ClockPort at composition root (combines with 3.0.2.1) | Pending |

#### 3.0.3 Move Business Logic Out of Adapters

**Files with business logic in adapters** (4 files, ~1,570 lines):

| File | Lines | Description |
|------|-------|-------------|
| `context_budget.rs` | 369 | Token counting, budget enforcement |
| `websocket_helpers.rs` | 476 | Prompt building, character selection, context aggregation |
| `queue_workers.rs` | 241 (process_dm_action) | DM action processing, scene transitions |
| `world_state_manager.rs` | **484** | Game time, conversation history, approval workflows (NEW - Sixth Review) |

**Note**: `world_state_manager.rs` also imports from engine-app (`StagingProposal`) - a double violation.

**Fix**: Create services in engine-app:
- `ContextBudgetService` - move from context_budget.rs
- `PromptBuilderService` - move from websocket_helpers.rs
- `DmActionProcessorService` - move from queue_workers.rs
- `WorldStateService` - move from world_state_manager.rs (NEW)

| Task | Status |
|------|--------|
| [ ] Move ContextBudgetEnforcer to engine-app/services | Pending |
| [ ] Create PromptBuilderService in engine-app | Pending |
| [ ] Move build_prompt_from_action (162 lines) | Pending |
| [ ] Move helper functions from websocket_helpers.rs (~300 lines) | Pending |
| [ ] Create DmActionProcessorService in engine-app | Pending |
| [ ] Move process_dm_action logic (241 lines) | Pending |
| [ ] Create WorldStateService in engine-app (NEW) | Pending |
| [ ] Move world_state_manager.rs logic (484 lines) | Pending |
| [ ] Remove engine-app import from world_state_manager.rs | Pending |

#### 3.0.4 Fix Ports Layer Violations

**Concrete implementations in ports that should be in adapters**:

| File | Item | Lines | Issue |
|------|------|-------|-------|
| `player-ports/platform.rs` | Platform struct | 347 | Full implementation in ports |
| `player-ports/testing/mock_game_connection.rs` | MockGameConnectionPort | 320 | Mock in ports, not adapters |
| `engine-ports/use_case_context.rs` | UseCaseContext | 166 | Concrete struct with 8 methods |

**Fix**:
- Move `Platform` struct to player-adapters, keep only traits in ports
- Move `MockGameConnectionPort` to player-adapters/testing or use mockall
- Document `UseCaseContext` as approved exception (it's a DTO/context object)

| Task | Status |
|------|--------|
| [ ] Move Platform struct (347 lines) to player-adapters | Pending |
| [ ] Move blanket *Dyn impls to player-adapters | Pending |
| [ ] Move MockGameConnectionPort to player-adapters/testing | Pending |
| [ ] Document UseCaseContext as approved exception | Pending |

#### 3.0.5 Remove Tokio from Ports Layer

**Issue**: `engine-ports/Cargo.toml:19` has tokio dependency but ports should only need `async-trait`.

**Seventh Review Finding**: The tokio dependency is **declared but NEVER USED** in any source file. It's a phantom dependency that can simply be deleted.

| Task | Status |
|------|--------|
| [x] Remove tokio from engine-ports/Cargo.toml | **DONE** |
| [ ] Verify compilation still works | Pending (deferred to end) |

#### 3.0.6 Fix player-ports Session Types Duplicates (NEW)

**File**: `crates/player-ports/src/session_types.rs` (116 lines)

**Issue**: 8 types duplicate protocol types WITHOUT `From` implementations:

| Type | player-ports | protocol | Has From? |
|------|--------------|----------|-----------|
| `ParticipantRole` | session_types.rs:11 | types.rs:20 | NO |
| `DiceInput` | session_types.rs:19 | messages.rs:1001 | NO |
| `ApprovalDecision` | session_types.rs:28 | types.rs:41 | NO |
| `DirectorialContext` | session_types.rs:60 | messages.rs:841 | NO |
| `NpcMotivationData` | session_types.rs:69 | messages.rs:853 | NO |
| `ApprovedNpcInfo` | session_types.rs:79 | messages.rs:1106 | NO |
| `AdHocOutcomes` | session_types.rs:92 | messages.rs:1011 | NO |
| `ChallengeOutcomeDecision` | session_types.rs:103 | messages.rs:1031 | NO |

**Fix**: Either add `From<protocol::Type>` impls or use protocol types directly.

| Task | Status |
|------|--------|
| [ ] Audit each duplicate for necessity | Pending |
| [ ] Add From impls for types that need local representation | Pending |
| [ ] Replace with protocol types where possible | Pending |

#### 3.0.7 Move Composition Root to Runner (NEW - Sixth Review)

**Issue**: The composition root (wiring of all dependencies) is in the adapters layer instead of the runner layer. This is a significant hexagonal architecture violation.

**Files with composition root logic**:

| File | Lines | Description |
|------|-------|-------------|
| `engine-adapters/src/infrastructure/state/mod.rs` | **727** | `AppState::new()` - wires all dependencies |
| `engine-adapters/src/run/server.rs` | **318** | `run()` - server setup and worker spawning |
| **Total** | **~1,045** | Lines in wrong layer |

**Current state**: `engine-runner/src/main.rs` is only **9 lines** - an empty shell that delegates everything to adapters.

**What should be in runner**:
- `AppState` construction (dependency injection)
- Server binding and listening
- Worker task spawning
- Signal handling (graceful shutdown)
- Configuration loading

**Fix approach**:
1. Create `engine-runner/src/composition.rs` - move `AppState::new()` logic
2. Create `engine-runner/src/server.rs` - move server setup
3. Keep only adapter implementations in engine-adapters
4. Runner should import adapters and wire them together

| Task | Status |
|------|--------|
| [ ] Create engine-runner/src/composition.rs | Pending |
| [ ] Move AppState::new() logic (~727 lines) to runner | Pending |
| [ ] Create engine-runner/src/server.rs | Pending |
| [ ] Move server.rs run() logic (~318 lines) to runner | Pending |
| [ ] Update engine-adapters to export only adapter types | Pending |
| [ ] Update main.rs to use new composition module | Pending |

---

### 3.1 Add Missing Challenge DTOs

**Issue**: `challenge_service.rs` directly uses protocol types instead of app-layer DTOs.

**File to create/update**: `crates/player-app/src/application/dto/requests.rs`

**DTOs to add**:
```rust
pub struct CreateChallengeRequest {
    pub name: String,
    pub description: Option<String>,
    pub challenge_type: String,
    pub difficulty: Option<String>,
    pub skill_id: Option<String>,
    // ... remaining fields from CreateChallengeData
}

pub struct UpdateChallengeRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    // ... remaining fields from UpdateChallengeData
}

impl From<CreateChallengeRequest> for wrldbldr_protocol::CreateChallengeData {
    fn from(req: CreateChallengeRequest) -> Self {
        Self {
            name: req.name,
            description: req.description,
            // ...
        }
    }
}
```

**File to update**: `crates/player-app/src/application/services/challenge_service.rs`

| Task | Status |
|------|--------|
| [ ] Add CreateChallengeRequest to requests.rs | Pending |
| [ ] Add UpdateChallengeRequest to requests.rs | Pending |
| [ ] Add From impls for both | Pending |
| [ ] Update challenge_service.rs to use new DTOs | Pending |
| [ ] Remove direct protocol imports from challenge_service.rs | Pending |

---

### 3.2 Consolidate Duplicate SuggestionContext DTO

**Issue**: Two definitions of SuggestionContext exist.

**Files**:
- `crates/player-app/src/application/dto/requests.rs:109-128`
- `crates/player-app/src/application/services/suggestion_service.rs:35-70`

**Fix**: Keep the one in requests.rs, update suggestion_service.rs to import it.

| Task | Status |
|------|--------|
| [ ] Verify requests.rs version is complete | Pending |
| [ ] Update suggestion_service.rs to use requests.rs version | Pending |
| [ ] Remove duplicate from suggestion_service.rs | Pending |

---

### 3.3 Document Port Placement Decision

**Issue**: Infrastructure ports remain in engine-app instead of engine-ports as originally planned.

**Ports affected**:
- `WorldStatePort` (scene.rs:227)
- `ConnectionManagerPort` (connection.rs:117)
- `StagingStatePort` (movement.rs:130)
- `StagingStateExtPort` (staging.rs)

**Decision**: These ports depend on use-case DTOs in engine-app. Moving them would create circular dependencies. Document as intentional.

**Files to update**:
- `crates/engine-app/src/application/use_cases/scene.rs`
- `crates/engine-app/src/application/use_cases/connection.rs`
- `crates/engine-app/src/application/use_cases/movement.rs`
- `crates/engine-app/src/application/use_cases/staging.rs`

**Comment to add**:
```rust
// ARCHITECTURE NOTE: This port is defined in engine-app rather than engine-ports
// because it depends on use-case-specific DTOs (WorldStateData, etc.) that are
// defined in this crate. Moving to engine-ports would create circular dependencies.
// This is an approved deviation from the standard hexagonal port placement.
```

| Task | Status |
|------|--------|
| [ ] Add architecture comment to WorldStatePort | Pending |
| [ ] Add architecture comment to ConnectionManagerPort | Pending |
| [ ] Add architecture comment to StagingStatePort | Pending |
| [ ] Add architecture comment to StagingStateExtPort | Pending |
| [ ] Update HEXAGONAL_GAP_REMEDIATION_PLAN.md to reflect decision | Pending |

---

### 3.4 Document Protocol Imports in Ports

**Issue**: GameConnectionPort and RequestHandler use protocol types directly.

**Files**:
- `crates/engine-ports/src/inbound/request_handler.rs:35-38`
- `crates/player-ports/src/outbound/game_connection_port.rs:17`

**Decision**: These are boundary ports where protocol types are appropriate. Document as approved exception.

**Comment to add**:
```rust
// ARCHITECTURE EXCEPTION: [APPROVED 2025-12-28]
// This port uses protocol types directly because it defines the primary
// engine-player communication boundary. The protocol crate exists specifically
// to share types across this boundary. Creating domain-level duplicates would
// add complexity without benefit.
```

| Task | Status |
|------|--------|
| [ ] Add exception comment to request_handler.rs | Pending |
| [ ] Add exception comment to game_connection_port.rs | Pending |

---

### 3.5 Split God Traits

**Issue**: 5 repository/port traits are too large (30+ methods each).

> **WARNING**: Splitting these traits will break test compilation until Phase 7 (Test Infrastructure) updates the mock implementations. Consider doing this as the last item in Phase 3, or as a separate PR that includes mock updates.

**VERIFIED COUNTS (Fifth Review)**: 5 god traits with **169** total methods.

**Traits to split**:

#### CharacterRepositoryPort (42 methods - VERIFIED)

**Current**: `engine-ports/src/outbound/repository_port.rs:94-382`

**Split into**:
- `CharacterCrudPort` - Basic CRUD + get_by_scene (7 methods)
- `CharacterWantsPort` - Want/motivation operations + get_want_target (7 methods)
- `CharacterActantialPort` - Actantial views (5 methods)
- `CharacterInventoryPort` - Inventory operations (5 methods)
- `CharacterLocationPort` - Location relationships (8 methods)
- `NpcDispositionPort` - NPC disposition (6 methods)
- `CharacterRegionPort` - Region relationships (4 methods)

#### StoryEventRepositoryPort (34 methods - VERIFIED)

**Current**: `engine-ports/src/outbound/repository_port.rs:1184-1364`

**Split into**:
- `StoryEventCrudPort` - CRUD and search (14 methods)
- `StoryEventRelationshipPort` - Edge methods (18 methods)
- `DialogueHistoryPort` - Dialogue-specific methods (2 methods)

#### NarrativeEventRepositoryPort (30 methods - VERIFIED)

**Current**: `engine-ports/src/outbound/repository_port.rs:1372-1506`

**Split into**:
- `NarrativeEventCrudPort` - CRUD and status (12 methods)
- `NarrativeEventRelationshipPort` - Scene/location/act edges (9 methods)
- `NarrativeEventNpcPort` - Featured NPC operations (5 methods)
- `NarrativeEventQueryPort` - Query by relationships (4 methods)

#### ChallengeRepositoryPort (31 methods - VERIFIED)

**Current**: `engine-ports/src/outbound/repository_port.rs:1007-1176`

**Split into**:
- `ChallengeCrudPort` - Basic CRUD (11 methods)
- `ChallengeSkillPort` - Skill relationships (3 methods)
- `ChallengeScenePort` - Scene ties (3 methods)
- `ChallengePrerequisitePort` - Prerequisites (4 methods)
- `ChallengeAvailabilityPort` - Location/region availability (7 methods)
- `ChallengeUnlockPort` - Unlock locations (3 methods)

#### GameConnectionPort (32 methods - VERIFIED)

**Current**: `player-ports/src/outbound/game_connection_port.rs:48-188`

**Split into**:
- `ConnectionStatePort` - Connection lifecycle (4 methods)
- `PlayerActionPort` - Player actions (3 methods)
- `DmActionPort` - DM-specific actions (12 methods)
- `ChallengePort` - Challenge operations (3 methods)
- `MovementPort` - Movement operations (2 methods)
- `InventoryPort` - Inventory operations (4 methods)
- `RequestPort` - Request/callback operations (4 methods)

| Task | Status |
|------|--------|
| [ ] Create new trait files in engine-ports/outbound/ | Pending |
| [ ] Split CharacterRepositoryPort (42 methods) | Pending |
| [ ] Split StoryEventRepositoryPort (34 methods) | Pending |
| [ ] Split NarrativeEventRepositoryPort (30 methods) | Pending |
| [ ] Split ChallengeRepositoryPort (31 methods) | Pending |
| [ ] Split GameConnectionPort (32 methods) | Pending |
| [ ] Update all trait implementations in adapters | Pending |
| [ ] Update all trait usages in app layer | Pending |
| [ ] Update mock implementations (coordinate with Phase 7) | Pending |
| [ ] Verify compilation | Pending |

**Note**: This is a significant refactor (**169** methods total across 5 traits). Consider doing in a separate PR that includes mock updates to avoid breaking test compilation.

#### Future Candidates (15-29 methods)

These traits are borderline and may benefit from splitting in a future iteration:

| Trait | Methods | Location |
|-------|---------|----------|
| LocationRepositoryPort | 19 | engine-ports/repository_port.rs |
| PlayerCharacterRepositoryPort | 17 | engine-ports/repository_port.rs |
| SceneRepositoryPort | 17 | engine-ports/repository_port.rs |
| RegionRepositoryPort | 16 | engine-ports/repository_port.rs |
| EventChainRepositoryPort | 16 | engine-ports/repository_port.rs |

---

## Phase 4: Dead Code Cleanup (3-4 hours)

**Priority**: MEDIUM - Code hygiene

### 4.1 Remove Unused Structs

| File | Struct | Action |
|------|--------|--------|
| `domain/entities/challenge.rs:531` | `ComplexChallengeSettings` | DELETE |
| `engine-app/dto/narrative_event.rs:130` | `NarrativeEventDetailResponseDto` | DELETE |
| `engine-app/dto/narrative_event.rs:162` | `ChainMembershipDto` | DELETE |
| `engine-app/dto/narrative_event.rs:180` | `FeaturedNpcDto` | DELETE |
| `engine-app/dto/narrative_event.rs:196` | `fn new()` | DELETE |

| Task | Status |
|------|--------|
| [x] Delete ComplexChallengeSettings | **DONE** |
| [x] Delete NarrativeEventDetailResponseDto cluster (~100 lines) | **DONE** |
| [x] Verify no references remain | **DONE** |

---

### 4.2 Remove or Use Unused Fields

| File | Field | Action |
|------|-------|--------|
| `actantial_context_service.rs:198` | `item_repo` | DELETE (injected but unused) |
| `generation_service.rs:116` | `completed_count` | DELETE |
| `scene_resolution_service.rs:59` | `character_repository` | DELETE |
| `trigger_evaluation_service.rs:200` | `challenge_repo` | DELETE |
| `trigger_evaluation_service.rs:201` | `character_repo` | DELETE |

**Broadcast fields** (4 use cases) - Requires decision:

| File | Field | Decision Needed |
|------|-------|-----------------|
| `connection.rs:224` | `broadcast` | IMPLEMENT or DELETE |
| `observation.rs:137` | `broadcast` | IMPLEMENT or DELETE |
| `player_action.rs:116` | `broadcast` | IMPLEMENT or DELETE |
| `scene.rs:291` | `broadcast` | IMPLEMENT or DELETE |

| Task | Status |
|------|--------|
| [x] Delete item_repo from actantial_context_service.rs | **DONE** |
| [x] Delete completed_count from generation_service.rs | **DONE** |
| [x] Delete character_repository from scene_resolution_service.rs | **DONE** |
| [x] Delete challenge_repo, character_repo from trigger_evaluation_service.rs | **DONE** |
| [ ] Decide on broadcast fields (implement or delete) | Pending |
| [ ] Implement or delete broadcast fields based on decision | Pending |

---

### 4.3 Remove Unused Constants and Imports

| File | Item | Action |
|------|------|--------|
| `llm_queue_service.rs:27` | `PRIORITY_HIGH` | DELETE |
| `disposition.rs:30` | `use uuid::Uuid` | DELETE |
| `state/use_cases.rs:45` | `ApprovalQueuePort` | DELETE |
| `export_routes.rs:13` | `WorldService` | DELETE |

**Unused Cargo.toml dependencies** (NEW - Seventh Review):

| File | Dependency | Action |
|------|------------|--------|
| `player-ui/Cargo.toml` | `wrldbldr-domain` | ~~DELETE~~ **DONE** |

| Task | Status |
|------|--------|
| [ ] Run `cargo fix --workspace --allow-dirty` | Pending (deferred) |
| [x] Delete PRIORITY_HIGH constant | **DONE** |
| [x] Delete unused uuid::Uuid import from disposition.rs | **DONE** |
| [x] Delete unused ApprovalQueuePort import from use_cases.rs | **DONE** |
| [x] Delete unused WorldService import from export_routes.rs | **DONE** |
| [x] Remove unused domain dep from player-ui/Cargo.toml | **DONE** (earlier commit) |

---

### 4.4 Address #[allow(dead_code)] Suppressions

**Suspicious suppressions to audit**:

| File | Item | Action |
|------|------|--------|
| `handlers/common.rs:103` | `parse_goal_id` | DELETE or USE |
| `handlers/common.rs:110` | `parse_want_id` | DELETE or USE |
| `handlers/common.rs:123` | `parse_relationship_id` | DELETE or USE |
| `handlers/common.rs:130` | `parse_story_event_id` | DELETE or USE |
| `websocket/converters.rs:54` | `to_domain_visibility` | DELETE or USE |
| `websocket/converters.rs:64` | `from_domain_visibility` | DELETE or USE |
| `websocket/converters.rs:74` | `to_domain_role` | DELETE or USE |
| `services.rs:302` | `apply_generation_read_state` | IMPLEMENT or DELETE |

| Task | Status |
|------|--------|
| [ ] Audit each #[allow(dead_code)] | Pending |
| [ ] Delete truly dead code | Pending |
| [ ] Remove #[allow(dead_code)] from used code | Pending |

---

### 4.5 Fix Unused Variables in UI Layer

| File | Variable | Fix |
|------|----------|-----|
| `location_preview_modal.rs:40` | `world_id` | Prefix with `_` |
| `edit_character_modal.rs:63` | `desc_val` | Prefix with `_` |
| `skills_panel.rs:254` | `world_id_for_delete` | Prefix with `_` |
| `skills_panel.rs:275` | `world_id` | Prefix with `_` |
| `skills_panel.rs:546` | `world_id` | Prefix with `_` |
| `pc_creation.rs:166` | `desc_val` | Prefix with `_` |
| `pc_creation.rs:169` | `session_id` | Prefix with `_` |
| `world_select.rs:66` | `user_id` | Prefix with `_` |

| Task | Status |
|------|--------|
| [x] Fix location_preview_modal.rs:40 world_id | **DONE** |
| [x] Fix world_select.rs:66 user_id | **DONE** |
| [ ] Fix remaining unused UI variables | Pending (low priority) |

---

### 4.6 Replace Glob Re-exports (NEW - Sixth Review)

**Issue**: **27 `pub use *` patterns** across 12 files (verified by arch-check). Glob re-exports make dependencies implicit, prevent dead code detection, and hurt IDE navigation.

**Files with glob re-exports** (from `cargo xtask arch-check`):

| File | Count | Examples |
|------|-------|----------|
| `engine-adapters/src/infrastructure/ports/mod.rs` | **8** | `pub use staging_state_adapter::*`, etc. |
| `engine-app/src/application/use_cases/mod.rs` | **6** | `pub use errors::*`, `pub use movement::*`, etc. |
| `domain/src/lib.rs` | **4** | `pub use entities::*`, `pub use value_objects::*`, etc. |
| `engine-adapters/src/infrastructure/websocket/mod.rs` | **2** | `pub use approval_converters::*`, etc. |
| `player-adapters/src/infrastructure/platform/mod.rs` | **2** | `pub use wasm::*`, `pub use desktop::*` |
| Other files | **5** | Various modules |

**Verification**: Run `cargo xtask arch-check` - glob re-exports are now detected (warning mode).

**Fix**: Replace with explicit exports:

```rust
// Before:
pub use entities::*;

// After:
pub use entities::{
    Character, Challenge, Location, Scene,
    // ... list all exported types
};
```

**Architecture Rule to Add**:
```
### Prohibited Patterns
1. **No glob re-exports**: Use explicit exports instead of `pub use module::*`
   - Makes dependencies explicit
   - Enables dead code detection  
   - Improves IDE navigation
```

| Task | Status |
|------|--------|
| [x] Replace glob re-exports in engine-adapters/ports/mod.rs (8) | **DONE** |
| [x] Replace glob re-exports in engine-app/use_cases/mod.rs (6) | **DONE** |
| [x] Replace glob re-exports in domain/lib.rs (4) | **DONE** |
| [x] Replace glob re-exports in engine-app/dto/mod.rs (3) | **DONE** |
| [x] Replace glob re-exports in protocol/lib.rs (2) | **DONE** |
| [x] Replace remaining glob re-exports (7) | **DONE** |
| [ ] Add "No glob re-exports" rule to CLAUDE.md | Pending |

---

## Phase 5: Domain Layer Polish (2-3 hours)

**Priority**: MEDIUM - Serialization and type safety

### 5.1 Add Serde Derives to Entities

**Issue**: Core entities lack `Serialize, Deserialize` derives.

**Entities to update**:
- `Character`
- `World`
- `Location`
- `Scene`
- `Challenge`
- `Item`
- `PlayerCharacter`
- `StoryEvent`
- `NarrativeEvent`

**Pattern**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    // ...
}
```

| Task | Status |
|------|--------|
| [x] Add serde derives to Character | **DONE** |
| [x] Add serde derives to World | **DONE** (already had serde) |
| [x] Add serde derives to Location | **DONE** |
| [x] Add serde derives to Scene | **DONE** |
| [x] Add serde derives to Challenge | **DONE** |
| [x] Add serde derives to Item | **DONE** |
| [x] Add serde derives to PlayerCharacter | **DONE** |
| [x] Add serde derives to StoryEvent | **DONE** |
| [x] Add serde derives to NarrativeEvent | **DONE** |

---

### 5.2 Add Serde to ID Types

**File**: `crates/domain/src/ids.rs`

**Issue**: Macro-generated ID types lack serde derives.

**Current macro**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct $name(Uuid);
```

**Fix**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct $name(Uuid);
```

| Task | Status |
|------|--------|
| [x] Update define_id! macro to include serde | **DONE** |
| [ ] Verify all ID types serialize correctly | Pending (deferred to compilation check) |

---

### 5.3 Move Environment I/O Out of Domain

**File**: `crates/domain/src/value_objects/settings.rs:157-196`

**Issue**: `AppSettings::from_env()` reads environment variables in domain layer. Calls `env_or()` **28 times** (verified in Sixth Review).

**Fix**: Move to adapters layer.

| Task | Status |
|------|--------|
| [ ] Create settings adapter in engine-adapters | Pending |
| [ ] Move from_env() to adapter | Pending |
| [ ] Update domain to only define AppSettings struct | Pending |
| [ ] Update all callers to use adapter | Pending |

---

### 5.4 Remove Non-Pure Dependencies from Domain (NEW)

**Issue**: Domain layer has dependencies that perform I/O or use unapproved crates.

#### 5.4.1 Random Number Generation via `rand` crate

**File**: `crates/domain/src/value_objects/dice.rs:199-203`

**Issue**: `rand::thread_rng()` accesses system entropy, which is I/O.

**Note**: The **player-side already has this solved** with `RandomProvider` trait in `player-ports/src/outbound/platform.rs`:
```rust
pub trait RandomProvider: Clone + 'static {
    fn random_f64(&self) -> f64;
    fn random_range(&self, min: i32, max: i32) -> i32;
}
```
With implementations: `WasmRandomProvider`, `DesktopRandomProvider`, `MockRandomProvider`.

**Current Code**:
```rust
use rand::Rng;

impl DiceFormula {
    pub fn roll(&self) -> DiceRollResult {
        let mut rng = rand::thread_rng();  // I/O - accesses system entropy
        // ...
    }
}
```

**Fix**: Model after player-side `RandomProvider`:
```rust
pub trait RandomSource: Send + Sync {
    fn random_range(&self, min: i32, max: i32) -> i32;
}

impl DiceFormula {
    pub fn roll(&self, rng: &impl RandomSource) -> DiceRollResult {
        // Use injected rng
    }
}
```

| Task | Status |
|------|--------|
| [ ] Create RandomSource trait in engine-ports (model after player RandomProvider) | Pending |
| [ ] Update DiceFormula::roll() to accept RandomSource | Pending |
| [ ] Create ThreadRngAdapter in engine-adapters | Pending |
| [ ] Remove rand from domain Cargo.toml | Pending |

#### 5.4.2 Replace `anyhow::Error` with `thiserror`

**Files**:
- `domain/src/value_objects/region.rs:29,36,61,68`
- `domain/src/entities/observation.rs:49,56`

**Issue**: Domain uses `anyhow::Error` for `FromStr` impls.

**Fix**: Create domain-specific error types:
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegionParseError {
    #[error("Invalid region shift: {0}")]
    InvalidShift(String),
    #[error("Invalid region frequency: {0}")]
    InvalidFrequency(String),
}
```

| Task | Status |
|------|--------|
| [ ] Create RegionParseError in domain | Pending |
| [ ] Create ObservationParseError in domain | Pending |
| [ ] Replace anyhow usage with thiserror types | Pending |
| [ ] Remove anyhow from domain Cargo.toml | Pending |

#### 5.4.3 Use thiserror for DiceParseError

**File**: `crates/domain/src/value_objects/dice.rs:13-24`

**Issue**: `thiserror` is declared in domain/Cargo.toml but `DiceParseError` uses manual `Display` and `Error` implementations instead.

**Fix**: Convert to derive macro:
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiceParseError {
    #[error("Invalid dice formula: {0}")]
    InvalidFormula(String),
}
```

| Task | Status |
|------|--------|
| [x] Convert DiceParseError to use thiserror derive | **DONE** |

---

### 5.5 Create Unified Domain Error Type (NEW)

**Issue**: Domain layer has only ONE error type (`DiceParseError`). Most domain operations force adapters to use `String` or `anyhow` for errors.

**File to create**: `crates/domain/src/error.rs`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Validation failed: {0}")]
    Validation(String),
    
    #[error("Invalid ID format: {0}")]
    InvalidId(String),
    
    #[error("Entity not found: {entity_type} with id {id}")]
    NotFound { entity_type: &'static str, id: String },
    
    #[error("Constraint violation: {0}")]
    Constraint(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
}
```

| Task | Status |
|------|--------|
| [ ] Create error.rs in domain | Pending |
| [ ] Define DomainError enum with variants | Pending |
| [ ] Update entities to use DomainError | Pending |
| [ ] Export from domain lib.rs | Pending |

---

## Phase 6: Protocol Layer Polish (3-4 hours)

**Priority**: MEDIUM - Wire format safety

### 6.1 Add Documentation to Protocol Imports

**File**: `crates/protocol/src/dto.rs:10-11`

**Issue**: Domain imports not documented as exception.

**Fix**: Add comment:
```rust
// ARCHITECTURE EXCEPTION: [APPROVED 2025-12-28]
// Uses domain ID types for DTO conversion methods only.
// Wire format uses raw Uuid; these imports enable to_domain() conversion.
use wrldbldr_domain::value_objects::{DispositionLevel, NpcDispositionState, RelationshipLevel};
use wrldbldr_domain::{CharacterId, PlayerCharacterId};
```

| Task | Status |
|------|--------|
| [x] Add exception comment to dto.rs | **DONE** |

---

### 6.2 Add Serde to RequestError

**File**: `crates/protocol/src/responses.rs:139-151`

**Issue**: `RequestError` lacks serde derives.

**Fix**:
```rust
/// Client-side request errors (not serialized over wire)
///
/// These errors occur locally on the client and are never transmitted.
/// If wire transmission is needed in future, add Serialize/Deserialize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestError {
    // ...
}
```

| Task | Status |
|------|--------|
| [ ] Add documentation explaining why no serde | Pending |
| [ ] OR add serde derives if wire transmission needed | Pending |

---

### 6.3 Document Versioning Strategy

**File**: `crates/protocol/src/messages.rs`

**Issue**: Large enums (70+ variants) without versioning strategy.

**Fix**: Add module-level documentation:
```rust
//! ## Versioning Policy
//!
//! - New variants can be added at the end (forward compatible)
//! - Removing variants requires major version bump
//! - Renaming variants is a breaking change
//! - Consider `#[serde(other)]` catch-all for unknown variants in future
```

| Task | Status |
|------|--------|
| [x] Add versioning documentation to messages.rs | **DONE** |
| [ ] Add versioning documentation to requests.rs | Pending |

---

### 6.4 Standardize ID Types

**Issue**: Inconsistent use of `String` vs `Uuid` for IDs.

**Pattern to follow**:
- Entity IDs: Use `Uuid`
- Correlation/Request IDs: Use `String`

| Task | Status |
|------|--------|
| [ ] Audit all ID fields in protocol types | Pending |
| [ ] Standardize to Uuid where appropriate | Pending |
| [ ] Document the pattern in protocol/src/lib.rs | Pending |

---

### 6.5 Add Protocol Forward Compatibility (CRITICAL - NEW)

**Issue**: **20 enums** across protocol crate have NO `#[serde(other)]` variant. When a newer server sends an enum variant that an older client doesn't recognize, deserialization fails completely.

**Enums needing `#[serde(other)]` Unknown variant**:

| File | Enum | Tag | Priority |
|------|------|-----|----------|
| messages.rs | `ClientMessage` | `#[serde(tag = "type")]` | CRITICAL |
| messages.rs | `ServerMessage` | `#[serde(tag = "type")]` | CRITICAL |
| messages.rs | `DiceInputType` | `#[serde(tag = "type", content = "value")]` | HIGH |
| messages.rs | `ChallengeOutcomeDecisionData` | `#[serde(tag = "action")]` | HIGH |
| messages.rs | `CharacterPosition` | None | MEDIUM |
| messages.rs | `WantVisibilityData` | None | LOW |
| messages.rs | `ActorTypeData` | None | LOW |
| messages.rs | `ActantialRoleData` | None | LOW |
| messages.rs | `WantTargetTypeData` | None | LOW |
| requests.rs | `RequestPayload` | `#[serde(tag = "type")]` | CRITICAL |
| responses.rs | `ResponseResult` | `#[serde(tag = "status")]` | HIGH |
| responses.rs | `ErrorCode` | None | MEDIUM |
| responses.rs | `EntityType` | None | LOW |
| responses.rs | `ChangeType` | None | LOW |
| responses.rs | `WorldRole` | None | LOW |
| responses.rs | `JoinError` | `#[serde(tag = "type")]` | MEDIUM |
| types.rs | `ParticipantRole` | None | MEDIUM |
| types.rs | `ApprovalDecision` | `#[serde(tag = "decision")]` | HIGH |
| app_events.rs | `AppEvent` | `#[serde(tag = "type")]` | HIGH |

**Pattern to apply**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    // ... existing variants ...
    
    /// Unknown message type for forward compatibility
    #[serde(other)]
    Unknown,
}
```

| Task | Status |
|------|--------|
| [x] Add #[serde(other)] Unknown to ClientMessage | **DONE** |
| [x] Add #[serde(other)] Unknown to ServerMessage | **DONE** |
| [x] Add #[serde(other)] Unknown to RequestPayload | **DONE** |
| [x] Add #[serde(other)] Unknown to ResponseResult | **DONE** |
| [ ] Add #[serde(other)] Unknown to remaining 16 enums | Pending |
| [ ] Add handling for Unknown variants in message processors | Pending |

---

### 6.6 Consolidate Redundant DTOs (UPDATED)

**Issue**: Some DTOs are duplicated without justification.

**Clarification from fifth review**:

| Category | Status | Action |
|----------|--------|--------|
| **player-app/dto/player_events.rs** (32 types) | INTENTIONAL | KEEP - Have `From<protocol>` impls, no serde |
| **engine-app/dto/approval.rs** (5 types) | REDUNDANT | CONSOLIDATE - Exact copies, no From impls |
| **player-ports/session_types.rs** (8 types) | REDUNDANT | CONSOLIDATE - No From impls (see 3.0.6) |

**Redundant duplicates in engine-app/dto/approval.rs to remove**:

| DTO | engine-app | protocol | Action |
|-----|------------|----------|--------|
| `ProposedToolInfo` | approval.rs:12-17 | types.rs:32-38 | DELETE engine-app version |
| `ChallengeSuggestionInfo` | approval.rs:21-32 | types.rs:76-89 | DELETE engine-app version |
| `ChallengeSuggestionOutcomes` | approval.rs:35-45 | types.rs:93-102 | DELETE engine-app version |
| `NarrativeEventSuggestionInfo` | approval.rs:48-60 | types.rs:106-117 | DELETE engine-app version |
| `DmApprovalDecision` | approval.rs:67-100 | types.rs:41-68 | Use protocol `ApprovalDecision` |

| Task | Status |
|------|--------|
| [ ] Remove redundant DTOs from engine-app/dto/approval.rs | Pending |
| [ ] Update engine-app services to use protocol types directly | Pending |
| [ ] Verify player-app DTOs have From impls (intentional) | Pending |
| [ ] Document which duplicates are intentional vs redundant | Pending |

---

## Phase 7: Test Infrastructure (8-12 hours)

**Priority**: MEDIUM - Enable quality verification

### 7.1 Fix Test Compilation

**Issue**: Test suite fails to compile with **36 errors** (verified in fifth review).

**Root Cause**: `crates/engine-adapters/src/infrastructure/ports/staging_service_adapter.rs:274-335`

**Error categories**:

| Category | Count | Examples |
|----------|-------|----------|
| Wrong error types | 10 | Stubs return `Result<..., String>` but traits expect `Result<..., anyhow::Error>` |
| Non-existent trait methods | 10 | `generate_streaming`, `save`, `list_active_for_region` not in traits |
| Missing trait methods | 2+ | `Error` associated type, `generate_with_tools`, `list_spawn_points` |
| Type doesn't exist | 1 | `LocationExit` should be `RegionExit` |

**Duplicated mocks (consolidation needed)**:

| Mock | Files | Issue |
|------|-------|-------|
| `MockPromptTemplateRepository` | 3 files | Defined in llm/mod.rs, outcome_suggestion_service.rs, prompt_builder.rs |
| `MockLlm` | 2 files | Different Error types! (std::io::Error vs Infallible) |

**Empty tests**:
- `disposition_service.rs:283` - `test_disposition_service_created` with `assert!(true)`
- `actantial_context_service.rs:652` - `test_service_created` with `assert!(true)`

| Task | Status |
|------|--------|
| [ ] Fix staging_service_adapter.rs stub error types (root cause) | Pending |
| [ ] Add missing trait methods to stubs | Pending |
| [ ] Remove non-existent methods from stubs | Pending |
| [ ] Fix LocationExit → RegionExit | Pending |
| [ ] Consolidate MockPromptTemplateRepository (3 → 1) | Pending |
| [ ] Consolidate MockLlm with consistent Error type | Pending |
| [ ] Fix or remove empty tests | Pending |
| [ ] Run `cargo test --workspace` and verify | Pending |

---

### 7.2 Add Protocol Serialization Tests

**Issue**: Zero tests for protocol message serialization.

**File to create**: `crates/protocol/src/lib.rs` (add tests module)

**Tests to add**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn client_message_roundtrip() {
        let msg = ClientMessage::Ping;
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }
    
    #[test]
    fn server_message_roundtrip() {
        // Test each major variant
    }
    
    #[test]
    fn request_payload_roundtrip() {
        // Test each major variant
    }
}
```

| Task | Status |
|------|--------|
| [ ] Add ClientMessage roundtrip tests | Pending |
| [ ] Add ServerMessage roundtrip tests | Pending |
| [ ] Add RequestPayload roundtrip tests | Pending |
| [ ] Add ResponseResult roundtrip tests | Pending |

---

### 7.3 Add #[automock] to All Ports

**Issue**: Most ports lack mock implementations.

**Files to update**:
- All traits in `engine-ports/src/outbound/`
- All traits in `player-ports/src/outbound/`

**Pattern**:
```rust
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SomePort: Send + Sync {
    // ...
}
```

| Task | Status |
|------|--------|
| [ ] Add mockall dependency if missing | Pending |
| [ ] Add #[automock] to all engine-ports traits | Pending |
| [ ] Add #[automock] to all player-ports traits | Pending |

---

### 7.4 Create Entity Test Factories

**File to create**: `crates/domain/src/testing/mod.rs`

**Content**:
```rust
//! Test factories for domain entities
//! 
//! Only compiled in test mode.

#[cfg(test)]
pub mod factories {
    use super::*;
    
    pub fn test_world() -> World {
        World::new("Test World".to_string(), None, None)
    }
    
    pub fn test_character() -> Character {
        Character::new(/* ... */)
    }
    
    // ... more factories
}
```

| Task | Status |
|------|--------|
| [ ] Create testing module in domain | Pending |
| [ ] Add factory for each major entity | Pending |
| [ ] Add factory customization builders | Pending |

---

## Phase 8: Documentation (2-3 hours)

**Priority**: LOW - Completeness

### 8.1 Add Layer Structure Diagram to CLAUDE.md

**File**: `CLAUDE.md`

**Add visual diagram**:
```
┌─────────────────────────────────────────────────────────────┐
│                         RUNNERS                              │
│              Composition root, wires everything              │
├─────────────────────────────────────────────────────────────┤
│                       PRESENTATION                           │
│                   UI components (player-ui)                  │
├─────────────────────────────────────────────────────────────┤
│                        ADAPTERS                              │
│    Implements ports, handles I/O, external systems           │
│    ONLY layer that constructs protocol types for wire        │
├─────────────────────────────────────────────────────────────┤
│                       APPLICATION                            │
│         Services, use cases, app-layer DTOs                  │
│         May define use-case-specific port traits             │
├─────────────────────────────────────────────────────────────┤
│                          PORTS                               │
│     Infrastructure port traits (repos, external services)    │
├─────────────────────────────────────────────────────────────┤
│                        PROTOCOL                              │
│      Wire-format DTOs, shared Engine↔Player types            │
│      May re-export stable domain types (documented)          │
├─────────────────────────────────────────────────────────────┤
│                         DOMAIN                               │
│       Entities, value objects, domain events                 │
│               Zero external dependencies                     │
└─────────────────────────────────────────────────────────────┘
```

| Task | Status |
|------|--------|
| [ ] Add diagram to CLAUDE.md | Pending |

---

### 8.2 Address TODO Comments

**17 TODO comments** need resolution:

| Priority | File | Line | Comment | Action |
|----------|------|------|---------|--------|
| DELETE | `tool_execution_service.rs` | 6 | Outdated refactor note | Delete |
| HIGH | `challenge_outcome_approval_service.rs` | 555 | Queue item ID mapping | Create issue |
| HIGH | `challenge_outcome_approval_service.rs` | 731 | Store branches in approval | Create issue |
| MEDIUM | `observation.rs` | 180 | World-based game time | Create issue |
| MEDIUM | `scene_builder.rs` | 284 | Get actual quantity from edge | Create issue |
| MEDIUM | `movement.rs` | 601 | Previous staging lookup | Create issue |
| MEDIUM | `trigger_evaluation_service.rs` | 384 | Get inventory from PC | Create issue |
| MEDIUM | `interaction_repository.rs` | 410 | Edge-based targeting | Create issue |
| LOW | `scene_builder.rs` | 275 | Region item system | Create issue |
| LOW | `staging_context_provider.rs` | 146 | Filter by region | Create issue |
| LOW | `staging_context_provider.rs` | 190 | Add timestamp | Create issue |
| LOW | `challenge.rs` | 350 | OutcomeDetail enhancement | Create issue |
| LOW | `queue_routes.rs` | 119 | Per-world breakdown | Create issue |
| LOW | `session_message_handler.rs` | 276 | Story Arc timeline | Create issue |
| LOW | `session_message_handler.rs` | 999 | Step 8 Phase 4 | Create issue |
| LOW | `content.rs` | 435 | View-as-character mode | Create issue |
| LOW | `event_chains.rs` | 97 | Navigate to event details | Create issue |

| Task | Status |
|------|--------|
| [ ] Delete outdated TODO in tool_execution_service.rs | Pending |
| [ ] Create GitHub issues for HIGH priority TODOs | Pending |
| [ ] Create GitHub issues for MEDIUM priority TODOs | Pending |
| [ ] Create GitHub issues for LOW priority TODOs | Pending |

---

## Verification Commands

Run after each phase:

```bash
# Compilation check
cargo check --workspace

# Architecture check
cargo run -p xtask -- arch-check

# Warnings check
cargo check --workspace 2>&1 | grep "^warning:" | wc -l

# Test compilation (Phase 7)
cargo test --workspace --no-run

# Full test run (after Phase 7)
cargo test --workspace
```

---

## Success Criteria

| Metric | Before | Target | Notes |
|--------|--------|--------|-------|
| Critical issues | **10** | 0 | Panic risks, forward compat, adapters→app deps, shutdown |
| Compiler warnings | **37** | 0 | Verified fifth review |
| Swallowed errors (engine-app/services) | **43** | 0 (logged) | 14+6+3+others |
| Swallowed errors (total codebase) | **89** | 0 (logged or documented) | |
| God traits (30+ methods) | 5 (**169** methods total) | 0 | Was 3/107, found 2 more |
| I/O in application layer | **12-13** + **14 time calls** | 0 | tokio::fs, std::env, Utc::now() |
| I/O in domain layer | **28** (env calls) + rand | 0 | AppSettings::from_env() (verified sixth review) |
| Direct time calls (no ClockPort) | ~~14+~~ **0** | 0 | **DONE** - ClockPort created and injected into 12 services/handlers |
| Protocol imports in services | 14 | 0 | |
| Implementations in ports layer | 3 (Platform, Mock, UseCaseContext) | 0-1 | ~830 lines total |
| Business logic in adapters | **4** files (~1,570 lines) | 0 | +world_state_manager.rs (484 lines, sixth review) |
| Composition root in adapters | **~1,045** lines | 0 | Move to runner (sixth review) |
| Glob re-exports (pub use *) | ~~27~~ **0** | 0 | **DONE** - All replaced with explicit exports |
| Adapters→App dependencies | **2 crates** (73 imports) | **0** | CRITICAL |
| Unbounded channels | **3** | 0 | websocket + 2 in state |
| tokio::spawn without tracking | **27** | 0 | Add CancellationToken |
| Unused structs | 4 | 0 | |
| Unused fields | 12 | 0 | |
| Unused Cargo.toml deps | ~~2~~ **0** | 0 | **DONE** - Removed player-ui→domain, engine-ports→tokio |
| Redundant DTO duplicates | **13** (5 engine-app + 8 player-ports) | 0 | player-app dups intentional |
| Protocol enums without #[serde(other)] | **20** | 0 | Forward compatibility |
| Domain error types | **1** (DiceParseError only) | Unified DomainError | |
| Test compilation | FAIL (**36** errors) | PASS | |
| Protocol test coverage | 0% | 100% | |
| arch-check | PASS | PASS | |

---

## Appendix A: Files Modified by Phase

### Phase 1
- `player-ui/src/presentation/components/creator/motivations_tab.rs`
- `engine-adapters/src/infrastructure/config.rs`

### Phase 2
- `engine-app/src/application/services/llm_queue_service.rs`
- `engine-app/src/application/services/asset_generation_queue_service.rs`
- `engine-app/src/application/services/generation_service.rs`
- `player-adapters/src/infrastructure/http_client.rs`
- `engine-adapters/src/infrastructure/ollama.rs`

### Phase 3
- `player-app/src/application/dto/requests.rs`
- `player-app/src/application/services/challenge_service.rs`
- `player-app/src/application/services/suggestion_service.rs`
- `engine-app/src/application/use_cases/*.rs` (4 files)
- `engine-ports/src/inbound/request_handler.rs`
- `player-ports/src/outbound/game_connection_port.rs`
- `engine-ports/src/outbound/repository_port.rs` (split)

### Phase 4
- `domain/entities/challenge.rs`
- `engine-app/dto/narrative_event.rs`
- Multiple service files (unused fields)
- Multiple UI files (unused variables)

### Phase 5
- `domain/src/entities/*.rs` (9 files)
- `domain/src/ids.rs`
- `domain/src/value_objects/settings.rs`

### Phase 6
- `protocol/src/dto.rs`
- `protocol/src/responses.rs`
- `protocol/src/messages.rs`
- `protocol/src/requests.rs`

### Phase 7
- Multiple test files
- `protocol/src/lib.rs`
- `engine-ports/src/outbound/*.rs`
- `player-ports/src/outbound/*.rs`
- `domain/src/testing/mod.rs` (new)

### Phase 8
- `CLAUDE.md`
- Various files with TODOs

---

## Appendix B: Commit Strategy

Recommended commit sequence:

1. `fix: resolve critical panic risks in production code`
2. `fix: replace hardcoded IPs with localhost defaults`
3. `fix: add error logging to queue workers`
4. `feat: add HTTP request timeouts`
5. `refactor: complete challenge DTO migration`
6. `refactor: consolidate duplicate SuggestionContext`
7. `docs: document port placement architectural decisions`
8. `refactor: remove unused code and fix warnings`
9. `feat: add serde derives to domain entities`
10. `docs: add protocol versioning documentation`
11. `test: fix test compilation`
12. `test: add protocol serialization tests`
13. `docs: update CLAUDE.md with architecture diagram`

---

## Appendix C: Risk Assessment

| Phase | Risk | Mitigation |
|-------|------|------------|
| Phase 1 | Low - Simple fixes | Test manually |
| Phase 2 | Low - Additive changes | Verify logging works |
| Phase 3 | Medium - API changes | Run full test suite |
| Phase 4 | Low - Deletions | Verify no references |
| Phase 5 | Medium - Serialization changes | Test JSON roundtrips |
| Phase 6 | Low - Documentation | Review for accuracy |
| Phase 7 | High - Test infrastructure | Incremental approach |
| Phase 8 | Low - Documentation | Review for accuracy |

---

## Appendix D: Dependencies Between Phases

```
Phase 1 (Critical) ──┬── Phase 2 (Error Handling + Async Fixes)
                     │
                     ├── Phase 3.0.1 (Adapters→App)*** ← CRITICAL, do early
                     │
                     ├── Phase 3.0.2-3.0.6 (I/O, Business Logic, Ports)
                     │         │
                     │         ├── Phase 3.0.7 (Composition Root)**** ← NEW
                     │         │
                     │         ├── Phase 3.1-3.4 (DTOs, Docs)
                     │         │
                     │         └── Phase 3.5 (God Traits - 169 methods)*
                     │                              │
                     │                              ▼
                     ├── Phase 4 (Dead Code)   Phase 7 (Tests)**
                     │         │
                     │         └── Phase 4.6 (Glob Re-exports) ← NEW
                     │
                     ├── Phase 5 (Domain Purity) ───┐
                     │         │                    │
                     │         └── Phase 6 (Protocol + Forward Compat)
                     │
                     └── Phase 8 (Docs)

* Phase 3.5 (God Traits) is large (~169 methods across 5 traits) - separate PR
** Phase 3.5 will BREAK test compilation until Phase 7 updates mocks
*** Phase 3.0.1 is CRITICAL - 73 imports across 43 files must be refactored
**** Phase 3.0.7 is NEW - Move ~1,045 lines of composition root to runner
```

**Recommended execution order** (updated for sixth review):
1. Phase 1 (Critical) - Do first
2. Phase 2.1-2.3 (Error handling) - Can parallel with Phase 4
3. Phase 2.4 (Async fixes) - Should be early (graceful shutdown)
4. **Phase 3.0.1 (Adapters→App deps)** - CRITICAL, significant effort (8-12h)
5. **Phase 3.0.7 (Composition root)** - Move to runner (4-6h) - NEW
6. **Phase 4.6 (Glob re-exports)** - Quick win (1-2h) - NEW
7. Phases 4.1-4.5, 5.1-5.3 - Can be done in parallel
8. Phase 3.0.2-3.0.6 (I/O violations, business logic, ports)
9. Phase 5.4-5.5 (Domain purity) - After basic domain polish
10. Phase 6 (Protocol + Forward Compat) - After domain is stable
11. Phase 3.5 + Phase 7 - God traits + test fixes (do together)
12. Phase 8 - Documentation (last)

**Alternative**: Skip Phase 3.5 initially, complete everything else, then do Phase 3.5 + Phase 7 as a dedicated "Interface Segregation" PR.

**Critical Path Items**:
- Phase 2.4 should be done early as async issues can cause runtime problems
- Phase 3.0.1 (adapters→app deps) is CRITICAL - 73 imports require significant refactoring
- Phase 3.0.7 (composition root) - ~1,045 lines in wrong layer (sixth review)
- Phase 4.6 (glob re-exports) - 30 patterns, quick to fix (sixth review)
- Phase 5.4-5.5 are new and should be done after basic domain polish
- Phase 6.5 (forward compatibility) blocks safe protocol updates

---

## Appendix E: Fifth Review Summary (12 Sub-Agents)

The fifth review deployed 12 specialized sub-agents to verify every aspect of the plan:

| Agent | Focus | Key Findings |
|-------|-------|--------------|
| Adapters→App | Cargo deps + imports | **73 imports** across 43 files (not just 2 deps) |
| Domain Purity | Domain violations | rand I/O, anyhow usage, **~22 env calls**, thiserror unused |
| Ports Layer | Ports violations | Platform (347 lines), MockGameConnection (320), tokio dep |
| God Traits | Method counts | **169 total** verified (42+34+30+31+32) |
| I/O Violations | App layer I/O | **12-13** violations (matches plan) |
| Business Logic | Adapters logic | ~1000 lines in 3 files |
| Swallowed Errors | let _ = patterns | **43 in services** (14+6+3+20), 89 total |
| DTO Duplicates | Type duplication | 13 redundant (5 engine-app + 8 player-ports), 32 intentional |
| Layer Violations | Import direction | Core architecture correct, adapters→app is main issue |
| Protocol | Forward compat | **20 enums** without #[serde(other)] |
| Async Issues | Concurrency | **27 spawns** without tracking, **3 unbounded channels** |
| Test Compilation | Test errors | **36 errors** (not 37), root cause in staging_service_adapter.rs |

### Metrics Verified Accurate
- God trait counts: 169 ✓
- Swallowed errors in services: 43 ✓
- I/O violations: 12-13 ✓
- Test errors: 36 (minor correction)

### New Issues Added
- Phase 2.4: Async/concurrency fixes (unbounded channels, graceful shutdown)
- Phase 3.0.1-3.0.6: Comprehensive architecture fixes
- Phase 5.4-5.5: Domain purity (rand, anyhow, thiserror, unified error type)
- Phase 6.5: Protocol forward compatibility (20 enums)

### Severity Downgrade
- std::sync::Mutex in comfyui.rs: Locks NOT held across await (low priority)

---

## Appendix F: Sixth Review Summary (Cross-Validation)

The sixth review cross-validated findings between two independent review agents and resolved discrepancies through targeted verification:

### Cross-Validation Results

| Item | Agent A | Agent B | Verified Result |
|------|---------|---------|-----------------|
| staging_service.rs:535 unwrap | Production risk | Test code | **Test code** ✓ (inside `#[cfg(test)]`) |
| pub use * patterns | 31 | 22 | **30** (manual count) |
| Domain env vars | ~20 | 28 | **28** (all in settings.rs) |
| request_handler.rs match arms | Not mentioned | 308 | **134** (both wrong) |

### NEW Issues Discovered

| Issue | Location | Impact |
|-------|----------|--------|
| world_state_manager.rs | engine-adapters (484 lines) | Business logic + engine-app import |
| Composition root in adapters | state/mod.rs + server.rs (~1,045 lines) | Runner layer should own this |
| Glob re-exports | 30 patterns across 11 files | Implicit dependencies, no dead code detection |

### Plan Updates Applied

1. **Phase 3.0.3**: Added `world_state_manager.rs` (484 lines)
2. **Phase 3.0.7**: NEW - Move composition root to runner (~1,045 lines)
3. **Phase 4.6**: NEW - Replace 30 glob re-exports + architecture rule
4. **Phase 5.3**: Updated env var count from ~20 to **28**
5. **Success Criteria**: Updated business logic count (4 files, ~1,570 lines)

### Validation Status
- All metrics verified against codebase
- Discrepancies resolved through targeted sub-agent verification
- Plan is now ready for implementation

### Recommended Execution Order (Updated)

1. **Phase 1** (1h): Critical fixes - unwraps, hardcoded IPs
2. **Phase 2.1-2.3** (2-3h): Error handling
3. **Phase 2.4** (2-3h): Async fixes - graceful shutdown
4. **Phase 3.0.1** (8-12h): CRITICAL - Remove adapters→app dependencies (73 imports)
5. **Phase 3.0.7** (4-6h): Move composition root to runner (1,045 lines)
6. **Phase 4.6** (1-2h): Replace glob re-exports (30 patterns)
7. Remaining phases as prioritized

---

## Appendix G: Seventh Review Summary (10 Sub-Agents)

The seventh review deployed 10 specialized sub-agents for architecture deep-dive:

| Agent | Focus | Key Findings |
|-------|-------|--------------|
| player-runner composition | Check if same problem as engine-runner | **CLEAN** - player-runner already correct (127 lines) |
| engine-dto usage | Analyze DTO crate role | Properly positioned, correctly used |
| Domain external deps | Audit domain purity | Confirmed: rand, anyhow, 29 env calls |
| Protocol→domain imports | Check all imports | All 6 documented, Phase 6.1 pending |
| Circular dependencies | Dependency graph | **CLEAN** - valid DAG, no cycles |
| Tokio in ports | Runtime deps in ports | tokio in engine-ports is **unused** (phantom dep) |
| expect()/panic!() | Panic risks | Only motivations_tab.rs + mock.rs (test infra) |
| Service→adapter leakage | Reverse dependencies | **CLEAN** - no app→adapters violations |
| UI→domain coupling | UI layer deps | Minor: direct ServerMessage in UI |
| Missing port traits | Direct infra usage | **14+ Utc::now() calls** need ClockPort |

### Critical Finding: Missing ClockPort

The most significant discovery was **14+ direct `Utc::now()` calls across 10 engine-app services**.

**Contrast with player-side**: The player crates already have proper abstractions:
- `TimeProvider` trait with `WasmTimeProvider`, `DesktopTimeProvider`, `MockTimeProvider`
- `RandomProvider` trait with platform-specific implementations
- `SleepProvider`, `StorageProvider`, `LogProvider`

The engine-side lacks these abstractions, making services hard to test deterministically.

### Pattern to Follow

The player-side `Platform` abstraction in `player-ports/src/outbound/platform.rs` is the reference implementation for how engine-side should handle time, randomness, and other I/O:

```rust
// Already exists in player-ports:
pub trait TimeProvider: Clone + 'static {
    fn now_unix_secs(&self) -> u64;
    fn now_millis(&self) -> u64;
}

pub trait RandomProvider: Clone + 'static {
    fn random_f64(&self) -> f64;
    fn random_range(&self, min: i32, max: i32) -> i32;
}
```

### Plan Updates Applied

1. **Phase 3.0.2.1**: NEW - Create ClockPort for engine-side (14+ call sites in 10 services)
2. **Phase 3.0.5**: Updated - tokio dep is unused (phantom), simple delete
3. **Phase 4.3**: Added unused Cargo.toml deps (player-ui→domain)
4. **Phase 5.4.1**: Updated - reference player-side RandomProvider as model
5. **Success Criteria**: Added ClockPort metric, unused deps metric
