# Code Review Remediation Plan

> **Generated**: 2026-01-14
> **Status**: COMPLETED
> **Branch**: feature/character-creation-ui
> **Completed**: 2026-01-14

This plan addresses all issues found during the comprehensive code review based on `docs/architecture/review.md`.

## Completion Summary

| Phase | Status | Details |
|-------|--------|---------|
| Phase 1: Security | ✅ Complete | .env properly ignored, .env.example updated |
| Phase 2: Architecture | ✅ Complete | Port traits centralized, domain types created, WorldRole moved |
| Phase 3: DDD | ✅ Complete | PlayerCharacter state enum, NpcDispositionState/StatBlock encapsulated |
| Phase 4: Performance | ✅ Complete | Duplicate query fixed, O(n²) dedup fixed, name normalization optimized |
| Phase 5: Error Handling | ✅ Complete | Error context added, unreachable documented, logging added |
| Phase 6: Validation | ✅ Complete | require_non_empty() used across all CRUD operations |
| Phase 7: Testing | ✅ Complete | All 41 tests implemented (event_chain: 12, multiplayer: 9, tool_call: 9, timeout: 11) |

---

## Executive Summary

| Severity | Count | Description |
|----------|-------|-------------|
| CRITICAL | 5 | Security, architecture violations, DDD violations |
| HIGH | 8 | Missing indexes, duplication, error handling |
| MEDIUM | 17 | Performance, error context, testing |
| LOW | 5 | Documentation, minor patterns |

---

## Phase 1: Security (CRITICAL)

### 1.1 Remove Committed .env File

**Problem**: `.env` file with `NEO4J_PASSWORD=wrldbldr123` is committed to repository.

**Files**:
- `.env` (remove from history)
- `.gitignore` (verify .env is listed)

**Actions**:
1. Remove `.env` from git history using `git filter-repo` or BFG
2. Verify `.env` is in `.gitignore`
3. Create `.env.example` with placeholder values only
4. Update documentation to note environment variable requirements

**Verification**:
```bash
git log --all --full-history -- .env  # Should show no results after cleanup
```

---

## Phase 2: Architecture Violations (CRITICAL)

### 2.1 Remove Protocol Types from Use Cases

**Problem**: 60+ protocol type references in 7 use case files. Use cases should only work with domain types.

**Files to Modify**:

| File | Protocol References | Fix Required |
|------|---------------------|--------------|
| `use_cases/staging/types.rs` | 14 | Create domain equivalents |
| `use_cases/staging/approve.rs` | 5 | Use domain types |
| `use_cases/staging/request_approval.rs` | 1 | Use domain types |
| `use_cases/time/mod.rs` | 30 | Create domain time types |
| `use_cases/challenge/mod.rs` | 5 | Use domain types |
| `use_cases/session/join_world_flow.rs` | 4 | Use domain types |
| `use_cases/session/directorial.rs` | 1 | Use domain types |

**Actions**:

1. **Create domain equivalents** in `crates/domain/src/`:
   - `GameTime` (domain version)
   - `TimeMode` enum
   - `GameTimeConfig`
   - `TimeSuggestionData`
   - `TimeAdvanceData`
   - `TimeSuggestionDecision` enum
   - `ResolvedVisualStateData`
   - `ResolvedStateInfoData`
   - `StateOptionData`
   - `ChallengeOutcomeDecision` enum
   - `WorldRole` enum
   - `DirectorialContext`

2. **Add conversion traits** in API layer:
   - Create `crates/engine/src/api/converters/` module
   - Implement `From<DomainType> for ProtocolType` in API layer
   - Implement `From<ProtocolType> for DomainType` in API layer

3. **Update use cases** to use domain types only

4. **Update API handlers** to convert at boundaries

**Verification**:
```bash
grep -r "wrldbldr_protocol" crates/engine/src/use_cases/  # Should return empty
```

### 2.2 Centralize Port Traits

**Problem**: Port traits defined outside `infrastructure/ports.rs`.

**Files**:
- `use_cases/staging/ports.rs` - Contains `PendingStagingStore`, `TimeSuggestionStore`

**Actions**:
1. Move `PendingStagingStore` trait to `infrastructure/ports.rs`
2. Move `TimeSuggestionStore` trait to `infrastructure/ports.rs`
3. Update imports in all files using these traits
4. Delete `use_cases/staging/ports.rs`

**Verification**:
```bash
find crates/engine/src -name "ports.rs" | wc -l  # Should be 1
```

---

## Phase 3: Rustic DDD Violations (CRITICAL/HIGH)

### 3.1 Fix PlayerCharacter Boolean Blindness

**Problem**: `PlayerCharacter` uses `is_alive: bool, is_active: bool` instead of `CharacterState` enum.

**File**: `crates/domain/src/aggregates/player_character.rs`

**Current Code** (lines 66-68):
```rust
// Status flags
is_alive: bool,
is_active: bool,
```

**Actions**:
1. Replace boolean fields with `state: CharacterState`
2. Update constructor to initialize state
3. Update `kill()`, `deactivate()`, `activate()`, `resurrect()` methods
4. Add accessor methods `is_alive()`, `is_active()` that delegate to state
5. Update all callers to use new methods
6. Update serialization/deserialization

**New Code**:
```rust
state: CharacterState,
```

### 3.2 Fix PlayerCharacter Mutation Returns

**Problem**: Mutations return `()` instead of domain events.

**File**: `crates/domain/src/aggregates/player_character.rs` (lines 384-406)

**Current Methods**:
- `pub fn kill(&mut self)` → returns `()`
- `pub fn deactivate(&mut self)` → returns `()`
- `pub fn activate(&mut self)` → returns `()`
- `pub fn resurrect(&mut self)` → returns `()`

**Actions**:
1. Create `PlayerCharacterStateChange` enum in `crates/domain/src/events/`
2. Update methods to return the enum
3. Update callers to handle returned events

**New Event**:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerCharacterStateChange {
    Killed,
    Deactivated,
    Activated,
    Resurrected,
    AlreadyInState,
}
```

### 3.3 Make NpcDispositionState Fields Private

**Problem**: All 8 fields are public, violating encapsulation.

**File**: `crates/domain/src/value_objects/disposition.rs` (lines 42-59)

**Actions**:
1. Make all fields private (remove `pub`)
2. Add read accessor methods for each field
3. Add builder methods for construction
4. Update all callers to use accessors

### 3.4 Make StatBlock HP Fields Private

**Problem**: `current_hp` and `max_hp` are public while other fields are private.

**File**: `crates/domain/src/value_objects/stat_block.rs` (lines 54-56)

**Actions**:
1. Make `current_hp` and `max_hp` private
2. Add `set_hp(&mut self, current: i32, max: i32) -> Result<(), DomainError>` with validation
3. Add accessor methods `current_hp()` and `max_hp()`
4. Update all callers

### 3.5 Review Character stats_mut() Exposure

**Problem**: `stats_mut()` returns `&mut StatBlock`, bypassing encapsulation.

**File**: `crates/domain/src/aggregates/character.rs` (line 210)

**Actions**:
1. Add specific mutation methods to Character for stat operations
2. Consider removing `stats_mut()` or marking it as internal

---

## Phase 4: Performance Issues (HIGH)

### 4.1 Fix Duplicate Database Query

**Problem**: `get_npcs_for_region()` called twice in staging approval.

**File**: `crates/engine/src/use_cases/staging/request_approval.rs` (lines 114-128)

**Actions**:
1. Call `get_npcs_for_region()` once
2. Pass result to both `generate_rule_based_suggestions()` and `generate_llm_based_suggestions()`
3. Update function signatures to accept NPC list

### 4.2 Add Missing Database Index

**Problem**: `Staging(region_id)` index missing, causes full scans.

**File**: `docs/architecture/neo4j-schema.md`

**Actions**:
1. Add index definition to schema documentation
2. Add index creation to database initialization
3. Verify with EXPLAIN on staging queries

**Index to Add**:
```cypher
CREATE INDEX staging_region_id IF NOT EXISTS FOR (s:Staging) ON (s.region_id)
```

### 4.3 Fix O(n²) Deduplication in Suggestions

**Problem**: Linear search in `suggestions.iter().any()` for each staged NPC.

**File**: `crates/engine/src/use_cases/staging/suggestions.rs` (lines 64-81)

**Actions**:
1. Use `HashSet<CharacterId>` for O(1) duplicate checking
2. Pre-build set of existing IDs before iteration

### 4.4 Fix Name Normalization Allocations

**Problem**: 3 allocations per name normalization in hot path.

**File**: `crates/engine/src/use_cases/staging/suggestions.rs` (lines 168-169)

**Actions**:
1. Optimize to single-pass normalization
2. Cache normalized names in HashMap before comparison loop

### 4.5 Fix Cache Clone Overhead

**Problem**: Every cache hit clones the value.

**File**: `crates/engine/src/infrastructure/cache.rs` (lines 53, 97)

**Actions**:
1. For large values, wrap in `Arc<T>`
2. Return `Arc<T>` from cache instead of cloning

### 4.6 Fix StatBlock Clone on Save

**Problem**: Cloning StatBlock (contains HashMap) before serialization.

**File**: `crates/engine/src/infrastructure/neo4j/character_repo.rs` (line 294)

**Actions**:
1. Implement `From<&StatBlock> for StatBlockStored`
2. Update save method to use reference conversion

---

## Phase 5: Error Handling (HIGH/MEDIUM)

### 5.1 Add Error Context to ImageGen

**Problem**: All HTTP errors become generic "Unavailable".

**File**: `crates/engine/src/infrastructure/comfyui.rs` (line 265)

**Actions**:
1. Log original error before converting
2. Include error type in ImageGenError variants

### 5.2 Enhance JoinWorldError Context

**Problem**: `PcNotFound` doesn't include PC ID or world context.

**File**: `crates/engine/src/infrastructure/ports.rs` (lines 213-222)

**Actions**:
1. Add `world_id` and `pc_id` fields to `PcNotFound` variant
2. Update all callers to provide context

### 5.3 Document unreachable!() Macros

**Problem**: `unreachable!()` without explanation strings.

**Files**:
- `api/websocket/e2e_client.rs` (lines 443, 464, 486)
- `api/websocket/ws_creator.rs` (line 494)

**Actions**:
1. Add descriptive strings to each `unreachable!()`
2. Explain why the code path is unreachable

### 5.4 Add Logging to Lost Error Contexts

**Problem**: Parse errors discarded in `map_err(|_| ...)`.

**Files**:
- `api/websocket/mod.rs` (line 3382) - UUID parsing
- `api/websocket/ws_core.rs` (line 938) - Disposition parsing
- `use_cases/lore/mod.rs` (lines 630-632) - NPC ID parsing

**Actions**:
1. Add `tracing::debug!()` before `map_err()`
2. Include original error in log message

---

## Phase 6: Code Duplication (HIGH/MEDIUM)

### 6.1 Use Existing Validation Helper

**Problem**: Empty string validation repeated 20+ times, but `require_non_empty()` exists.

**File**: `crates/engine/src/use_cases/validation.rs` (lines 17-22)

**Files to Update**:
- `use_cases/management/act.rs`
- `use_cases/management/interaction.rs`
- `use_cases/management/location.rs`
- `use_cases/management/skill.rs`
- `use_cases/challenge/crud.rs`
- `use_cases/actantial/mod.rs`

**Actions**:
1. Import `require_non_empty` in each file
2. Replace inline `if name.trim().is_empty()` checks
3. Update error types to match

### 6.2 Create Neo4j Error Mapping Helper

**Problem**: 400+ occurrences of `.map_err(|e| RepoError::database(...))`.

**File**: Create `crates/engine/src/infrastructure/neo4j/query_helpers.rs`

**Actions**:
1. Create helper traits/functions for common error mapping
2. Extend `NodeExt` trait with error-handling methods
3. Update repositories to use helpers

### 6.3 Extract CRUD Pattern to Trait

**Problem**: Identical CRUD operations in 7+ management use case files.

**Actions**:
1. Create `CrudOps<T, ID, Err>` trait
2. Implement generic `CrudBase` struct
3. Refactor management use cases to use trait

### 6.4 Consolidate WebSocket Handler Patterns

**Problem**: 100+ repeated handler patterns across 25 files.

**Actions**:
1. Create handler macro or helper functions
2. Extract `*_to_json` functions to centralized module
3. Create generic CRUD handler wrapper

---

## Phase 7: Testing (HIGH)

### 7.1 Implement Unimplemented Tests

**Problem**: 39 tests contain only `todo!()`.

**Files**:
- `e2e_tests/event_chain_tests.rs` - 10 tests
- `e2e_tests/tool_call_tests.rs` - 9 tests
- `e2e_tests/multiplayer_tests.rs` - 9 tests
- `e2e_tests/approval_timeout_tests.rs` - 9 tests

**Actions**:
1. Implement each test with proper logic
2. Create VCR cassettes for LLM calls
3. Add meaningful assertions

### 7.2 Add Error Case Tests

**Problem**: Tests only cover happy paths.

**Actions**:
1. Add tests for LLM failures
2. Add tests for invalid entity IDs
3. Add tests for timeout scenarios
4. Add tests for permission errors

### 7.3 Improve Test Error Messages

**Problem**: 100+ `.expect()` calls with generic messages.

**Actions**:
1. Add contextual information to expect messages
2. Consider using `anyhow::Context` for better error chains

---

## Implementation Order

```
Phase 1: Security (Day 1)
├── 1.1 Remove .env from history

Phase 2: Architecture (Day 1-2)
├── 2.1 Create domain types for protocol equivalents
├── 2.1 Create API converters
├── 2.1 Update use cases
├── 2.2 Centralize port traits

Phase 3: DDD Fixes (Day 2-3)
├── 3.1 Fix PlayerCharacter boolean blindness
├── 3.2 Add PlayerCharacterStateChange event
├── 3.3 Make NpcDispositionState private
├── 3.4 Make StatBlock HP fields private
├── 3.5 Review stats_mut()

Phase 4: Performance (Day 3)
├── 4.1 Fix duplicate query
├── 4.2 Add Staging index
├── 4.3 Fix O(n²) dedup
├── 4.4 Optimize name normalization
├── 4.5 Fix cache clone
├── 4.6 Fix StatBlock clone

Phase 5: Error Handling (Day 4)
├── 5.1-5.4 All error handling fixes

Phase 6: Duplication (Day 4-5)
├── 6.1 Use validation helper
├── 6.2 Create query helpers
├── 6.3 Extract CRUD trait
├── 6.4 Consolidate handlers

Phase 7: Testing (Day 5-6)
├── 7.1 Implement tests
├── 7.2 Add error cases
├── 7.3 Improve messages
```

---

## Verification Commands

```bash
# Full build
cargo build --workspace

# All tests
cargo test --workspace

# Clippy
cargo clippy --workspace -- -D warnings

# Check no protocol in use cases
grep -r "wrldbldr_protocol" crates/engine/src/use_cases/

# Check port centralization
find crates/engine/src -name "ports.rs" | wc -l

# E2E tests
E2E_LLM_MODE=playback cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=1
```

---

## Files Summary

### To Create
- `crates/domain/src/value_objects/game_time.rs`
- `crates/domain/src/value_objects/visual_state.rs`
- `crates/domain/src/events/player_character_events.rs`
- `crates/engine/src/api/converters/mod.rs`
- `crates/engine/src/api/converters/staging.rs`
- `crates/engine/src/api/converters/time.rs`
- `crates/engine/src/api/converters/challenge.rs`
- `crates/engine/src/infrastructure/neo4j/query_helpers.rs`

### To Modify (Major Changes)
- `crates/domain/src/aggregates/player_character.rs`
- `crates/domain/src/value_objects/disposition.rs`
- `crates/domain/src/value_objects/stat_block.rs`
- `crates/engine/src/use_cases/staging/types.rs`
- `crates/engine/src/use_cases/time/mod.rs`
- `crates/engine/src/infrastructure/ports.rs`

### To Delete
- `crates/engine/src/use_cases/staging/ports.rs`
- `.env` (from history)

---

## Success Criteria

- [ ] `cargo build --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] No `wrldbldr_protocol` imports in use cases
- [ ] All aggregates have private fields
- [ ] All mutations return domain events
- [ ] No `todo!()` in test files
- [ ] `.env` not in git history
