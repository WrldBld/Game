# Tech Debt Remediation Plan

**Created**: December 31, 2024  
**Last Updated**: December 31, 2024  
**Status**: VALIDATED (2 rounds) - READY FOR IMPLEMENTATION

---

## Executive Summary

This plan addresses technical debt identified during the comprehensive hexagonal architecture and code quality review. It covers four main areas:

1. **Shared Utilities Crate** - Create `wrldbldr-common` for duplicate code patterns
2. **ClockPort Injection** - Fix ~92 `Utc::now()` violations across adapters and app
3. **Large Repository Splitting** - Split 3 files totaling 5,891 lines
4. **Pattern Migration** - Consolidate 97 duplicate code patterns

**Total Estimated Effort**: 22-30 hours (with parallelization: 16-22 elapsed hours)

---

## Validation Summary

This plan has undergone **two rounds of validation** by multiple review agents.

### Round 1 Findings (Applied)
- Player-ports whitelist already complete
- Engine-ports whitelist already complete  
- Utc::now() count corrected (75→92)
- Added Neo4jRepository facade step
- Added RandomPort for workflow_helpers.rs

### Round 2 Findings (Applied)
| Finding | Action Taken |
|---------|--------------|
| Part 1 (whitelist) already implemented | Marked COMPLETE, removed from active work |
| Part 4 (RandomPort) is over-engineering | Marked DEFERRED - adapters can use rand |
| Utc::now() count refined to 92 | Updated in Part 2 |
| Repository splits can parallelize | Updated implementation order |
| story_event_repository has 0 Utc::now() | Can split before ClockPort injection |
| world_snapshot.rs Default impl missed | Added to Part 2 |
| Neo4jRepository location wrong | Corrected to engine-adapters |

---

## Part 1: Protocol Whitelist Updates

**Status**: COMPLETE (already implemented)  
**Effort**: 0 (no work needed)

### Background

Validation confirmed that both engine-ports and player-ports whitelists are **already implemented** in `crates/xtask/src/main.rs`:

**Engine-ports whitelist** (lines 515-517):
```rust
if file_name == "request_handler.rs"
    || file_name == "dm_approval_queue_service_port.rs"
    || file_name == "mod.rs"
```

**Player-ports whitelist** (lines 720-728):
```rust
let shared_kernel_files: HashSet<&str> = [
    "request_port.rs",
    "game_connection_port.rs",
    "mock_game_connection.rs",
    "player_events.rs",
    "session_types.rs",
]
```

### Verification

```bash
cargo xtask arch-check  # Should pass with no protocol violations
```

---

## Part 2: Shared Utilities Crate

**Status**: COMPLETE (commit 8d09c21)  
**Effort**: 2-3 hours  
**Priority**: HIGH (enables subsequent steps)

### Background

Validation confirmed:
- 51 occurrences of datetime parsing pattern (exact match)
- 46 occurrences of empty-string-to-option pattern (exact match)
- chrono is already a workspace dependency with WASM support
- No naming conflicts with existing code

### Crate Design

**Name**: `wrldbldr-common`  
**Location**: `crates/common/`  
**Layer**: Shared Kernel (same level as `protocol`)

**Cargo.toml**:
```toml
[package]
name = "wrldbldr-common"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
description = "WrldBldr Common - Shared utility functions"

[dependencies]
chrono = { workspace = true }

[lints]
workspace = true
```

### Module Structure

```
crates/common/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── datetime.rs
    └── string.rs
```

### Function Signatures

**datetime.rs**:
```rust
use chrono::{DateTime, Utc};

/// Parses RFC3339 timestamp, returning error on failure.
pub fn parse_datetime(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc))
}

/// Parses RFC3339 timestamp, falling back to provided default on error.
/// Use this when you have access to ClockPort: parse_datetime_or(&s, self.clock.now())
pub fn parse_datetime_or(s: &str, default: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(default)
}
```

**string.rs**:
```rust
/// Converts empty string to None, non-empty to Some.
pub fn none_if_empty(value: &str) -> Option<&str> {
    if value.is_empty() { None } else { Some(value) }
}

/// Owned version for String.
pub fn some_if_not_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

/// Extension trait for method chaining.
pub trait StringExt {
    fn into_option(self) -> Option<String>;
}

impl StringExt for String {
    fn into_option(self) -> Option<String> {
        some_if_not_empty(self)
    }
}
```

### Implementation Steps

1. Create `crates/common/` directory structure
2. Add to workspace in root `Cargo.toml`:
   ```toml
   wrldbldr-common = { path = "crates/common", version = "0.1.0" }
   ```
3. Create source files with comprehensive tests
4. Verify: `cargo test -p wrldbldr-common`

---

## Part 3: ClockPort Injection

**Status**: COMPLETE (commits 6308334, 78a2f68)  
**Effort**: 6-8 hours  
**Priority**: HIGH (testability + architecture compliance)

### Background

Validation confirmed:
- **92 total occurrences** of `Utc::now()` in production code
  - 46 in persistence layer - **DONE** (commit 6308334)
  - 39 in other adapters (queues, HTTP, etc.) - **DONE** (commit 78a2f68)
  - Remaining 13 calls are acceptable (clock impls, test mocks, circuit breaker, error fallbacks)
- ClockPort trait has `now()` and `now_rfc3339()` methods
- Services already use constructor injection pattern

### Step 3.1: Update Neo4jRepository Facade

**File**: `crates/engine-adapters/src/infrastructure/persistence/mod.rs`

```rust
// Before
pub struct Neo4jRepository {
    connection: Neo4jConnection,
}

// After
pub struct Neo4jRepository {
    connection: Neo4jConnection,
    clock: Arc<dyn ClockPort>,
}

impl Neo4jRepository {
    pub async fn new(uri: &str, user: &str, password: &str, database: &str, clock: Arc<dyn ClockPort>) -> Result<Self> {
        let connection = Neo4jConnection::new(uri, user, password, database).await?;
        Ok(Self { connection, clock })
    }
    
    pub fn characters(&self) -> Neo4jCharacterRepository {
        Neo4jCharacterRepository::new(self.connection.clone(), self.clock.clone())
    }
    // ... update all 20+ accessor methods similarly
}
```

### Step 3.2: Update Repository Structs

Add `clock: Arc<dyn ClockPort>` to all repository structs:

```rust
pub struct Neo4jCharacterRepository {
    connection: Neo4jConnection,
    clock: Arc<dyn ClockPort>,
}

impl Neo4jCharacterRepository {
    pub fn new(connection: Neo4jConnection, clock: Arc<dyn ClockPort>) -> Self {
        Self { connection, clock }
    }
}
```

### Step 3.3: Replace Utc::now() Calls

```rust
// Before
.param("acquired_at", Utc::now().to_rfc3339());

// After (use ClockPort's helper)
.param("acquired_at", self.clock.now_rfc3339());

// For parse fallbacks
// Before
.unwrap_or_else(|_| Utc::now())

// After (with common crate)
parse_datetime_or(&timestamp_str, self.clock.now())
```

### Files to Modify (Validated Count: 92)

**Persistence Layer (46 occurrences)**:
| Repository | Count |
|------------|-------|
| `character_repository.rs` | 12 |
| `sqlite_queue.rs` | 9 |
| `memory_queue.rs` | 9 |
| `event_chain_repository.rs` | 7 |
| `narrative_event_repository.rs` | 6 |
| `observation_repository.rs` | 5 |
| `player_character_repository.rs` | 5 |
| `world_repository.rs` | 3 |

**Other Adapters (39 occurrences)**:
| File | Count |
|------|-------|
| `comfyui.rs` | 7 |
| `world_state_manager.rs` | 6 |
| `world_connection_manager.rs` | 4 |
| Other files | ~22 |

**Engine-App (3 occurrences)**:
| File | Lines | Fix |
|------|-------|-----|
| `dto/workflow.rs` | 109, 112 | Use `parse_datetime_or` with epoch fallback |
| `dto/world_snapshot.rs` | 74 | Change Default impl or use epoch |

### Step 3.4: Update Factory Wiring

**File**: `crates/engine-runner/src/composition/factories/infrastructure.rs`

Update `create_neo4j_repository()` to pass clock from `InfrastructureContext`.

### Verification

```bash
cargo check --workspace
cargo test --workspace
# Verify no remaining violations (excluding clock.rs)
rg "Utc::now\(\)" crates/engine-adapters/src --type rust | grep -v test | grep -v clock.rs
```

---

## Part 4: RandomPort Injection

**Status**: DEFERRED  
**Effort**: N/A  
**Priority**: LOW

### Background

Validation determined that injecting RandomPort into `workflow_helpers.rs` is **over-engineering**:

1. The adapters layer is **allowed** to use external crates like `rand` per architecture rules
2. There are **no existing tests** for this code that would benefit from deterministic seeds
3. The refactor would require adding `random()` to `AppStatePort` - significant API change
4. The `rand` usage is for ComfyUI seed randomization - non-determinism is actually desired

### Recommendation

Add a documentation comment instead:
```rust
// Note: Uses rand directly since this is adapter-layer code.
// If tests need deterministic seeds, inject RandomPort via prepare_workflow signature.
fn randomize_seeds(workflow: &mut serde_json::Value) {
    let mut rng = rand::thread_rng();
    // ...
}
```

This can be revisited when/if tests are added for workflow preparation.

---

## Part 5: Large Repository Splitting

**Status**: READY  
**Effort**: 10-12 hours  
**Priority**: MEDIUM (maintainability)

### Background

Validation confirmed:
- File sizes accurate (within 1-3 lines)
- Ports layer already uses identical directory structure
- All ISP sub-traits exist and are verified
- No cross-module private method calls needed
- Backward compatibility preserved through mod.rs re-exports
- **story_event_repository.rs has 0 Utc::now() calls** - can split independently

### Target Files

| File | Lines | ISP Traits | Split Into |
|------|-------|------------|------------|
| `story_event_repository.rs` | 1,813 | 4 | 7 files |
| `narrative_event_repository.rs` | 2,005 | 4 | 7 files |
| `character_repository.rs` | 2,073 | 6 | 8 files |

### File 1: story_event_repository.rs (1,813 lines)

**Can be done IMMEDIATELY** - no ClockPort dependency (0 Utc::now() calls)

**Split Structure**:
```
persistence/story_event_repository/
├── mod.rs              (~60 lines)   - Struct, re-exports
├── stored_types.rs     (~950 lines)  - StoredStoryEventType + variants
├── common.rs           (~60 lines)   - row_to_story_event
├── crud.rs             (~180 lines)  - CRUD + StoryEventCrudPort
├── query.rs            (~260 lines)  - Queries + StoryEventQueryPort
├── edge.rs             (~360 lines)  - Edge management + StoryEventEdgePort
└── dialogue.rs         (~100 lines)  - Dialogue ops + StoryEventDialoguePort
```

### File 2: narrative_event_repository.rs (2,005 lines)

**Split Structure**:
```
persistence/narrative_event_repository/
├── mod.rs              (~60 lines)
├── stored_types.rs     (~920 lines)
├── common.rs           (~130 lines)
├── crud.rs             (~450 lines)
├── tie.rs              (~220 lines)
├── npc.rs              (~130 lines)
└── query.rs            (~120 lines)
```

### File 3: character_repository.rs (2,073 lines)

**Split Structure**:
```
persistence/character_repository/
├── mod.rs                 (~60 lines)
├── common.rs              (~120 lines)
├── crud.rs                (~180 lines)
├── want.rs                (~230 lines)
├── actantial.rs           (~280 lines)
├── inventory.rs           (~200 lines)
├── location.rs            (~350 lines)
└── disposition.rs         (~260 lines)
```

### Implementation Pattern

**mod.rs Template**:
```rust
//! [Entity] repository implementation - split for maintainability

mod common;
mod crud;
// ... other modules

use super::connection::Neo4jConnection;
use wrldbldr_engine_ports::outbound::ClockPort;
use std::sync::Arc;

pub struct Neo4j[Entity]Repository {
    connection: Neo4jConnection,
    clock: Arc<dyn ClockPort>,
}

impl Neo4j[Entity]Repository {
    pub fn new(connection: Neo4jConnection, clock: Arc<dyn ClockPort>) -> Self {
        Self { connection, clock }
    }
    
    pub(crate) fn connection(&self) -> &Neo4jConnection {
        &self.connection
    }
    
    pub(crate) fn clock(&self) -> &Arc<dyn ClockPort> {
        &self.clock
    }
}
```

### Verification After Each Split

```bash
cargo check --workspace
cargo test --workspace
cargo xtask arch-check
```

---

## Part 6: Pattern Migration

**Status**: READY  
**Effort**: 2-3 hours  
**Priority**: LOW (cleanup after main work)

### Migrate Remaining Patterns to Common Crate

After Parts 2-3 are complete, migrate remaining duplicate patterns:

1. **engine-adapters** (43 datetime, 14 string patterns)
2. **player-ui** (5 datetime, 32 string patterns)
3. **engine-app** (remaining patterns)

---

## Part 7: Documentation Updates

**Status**: READY  
**Effort**: 1 hour  
**Priority**: LOW (after implementation)

### Files to Update

| Document | Update Needed |
|----------|---------------|
| `AGENTS.md` | Add `wrldbldr-common` to crate responsibilities table |
| `docs/architecture/hexagonal-architecture.md` | Add common to dependency diagram |
| Root `Cargo.toml` | Already covered in Part 2 |

### Verification

- Ensure architecture diagrams include `common` crate
- Run `cargo xtask arch-check` to verify all whitelists are current

---

## Implementation Order (Optimized with Parallelization)

### Critical Path
```
Part 2 (common crate) → Part 3 (ClockPort) → Part 6 (migration) → Part 7 (docs)
```

### Parallel Stream (Can Start Immediately)
```
Part 5.1 (split story_event) → Part 5.2 (split narrative_event) → Part 5.3 (split character)
```

### Optimized Timeline

| Day | Stream A (Core) | Stream B (Splitting) |
|-----|-----------------|----------------------|
| 1 | Part 2: Common crate (2-3h) | Part 5.1: Split story_event (2-3h) |
| 1-2 | Part 3.1-3.2: Facade + repos (3h) | Part 5.2: Split narrative_event (3-4h) |
| 2 | Part 3.3-3.4: Replace calls (3-4h) | Part 5.3: Split character (4-5h) |
| 3 | Part 6: Pattern migration (2-3h) | - |
| 3 | Part 7: Documentation (1h) | - |

**With Parallelization**: 16-22 elapsed hours  
**Without Parallelization**: 22-30 hours

### Dependency Table

| Step | Task | Effort | Dependencies |
|------|------|--------|--------------|
| 2 | Create wrldbldr-common | 2-3 hrs | None |
| 3.1 | Update Neo4jRepository facade | 1.5 hrs | Part 2 |
| 3.2 | Update repository structs | 2 hrs | Part 3.1 |
| 3.3 | Replace Utc::now() calls | 3-4 hrs | Parts 2, 3.2 |
| 5.1 | Split story_event | 2-3 hrs | **None** |
| 5.2 | Split narrative_event | 3-4 hrs | Part 3 (for clock field) |
| 5.3 | Split character_repository | 4-5 hrs | Part 3 (for clock field) |
| 6 | Migrate remaining patterns | 2-3 hrs | Part 2 |
| 7 | Documentation updates | 1 hr | All above |

---

## Verification Checklist

After each phase:
```bash
cargo check --workspace
cargo test --workspace  
cargo xtask arch-check
cargo clippy --workspace --all-targets
```

### Final Targets

| Metric | Before | After |
|--------|--------|-------|
| `Utc::now()` in production | 92 | 0 |
| Largest repository file | 2,073 lines | <400 lines |
| Duplicate patterns | 97 | 0 |
| arch-check | Passes | Passes |

---

## Deferred Items (Documented)

| Item | Reason | Future Work |
|------|--------|-------------|
| RandomPort in workflow_helpers | Over-engineering; adapters can use rand | Revisit when tests added |
| GameConnectionPort ISP migration | Infrastructure in place | Migrate consumers gradually |
| Large domain/UI files | Different characteristics | Separate cleanup phase |
| TODO comments (21) | Feature-related | Address per roadmap |

---

## Risk Mitigation

### Part 3 (ClockPort) Risk

This step touches many files. Mitigation:
1. Implement in a feature branch
2. One commit per repository modified
3. Run tests incrementally
4. Part 5.1 can validate the pattern first

### Rollback Plan

Each part is atomic:
- Part 2: Remove crate from workspace
- Part 3: Git revert (one repo at a time)
- Part 5: Git revert directory changes

---

## Change Log

| Date | Changes |
|------|---------|
| Dec 31, 2024 | Initial plan created from architecture review |
| Dec 31, 2024 | Validation round 1: Corrected counts, added facade step, added RandomPort |
| Dec 31, 2024 | Validation round 2: Part 1 already complete, Part 4 deferred, parallelization enabled, story_event can split independently, corrected Neo4jRepository location, added world_snapshot.rs violation |
| Dec 31, 2024 | Part 2 COMPLETE: Created wrldbldr-common crate with datetime and string utilities |
| Dec 31, 2024 | Part 3 (persistence) COMPLETE: Injected ClockPort into 12 Neo4j repositories, replaced 46 Utc::now() calls |
| Dec 31, 2024 | Part 3 COMPLETE: Injected ClockPort into queues, state manager, HTTP routes; 79 of 92 Utc::now() calls replaced |
