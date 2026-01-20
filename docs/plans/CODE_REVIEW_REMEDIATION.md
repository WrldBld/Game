# Code Review Remediation Plan

## Overview

Address findings from comprehensive code review conducted 2026-01-20.

**Status**: VALIDATED - All issues confirmed and expanded with implementation details

---

## Validation Summary

| Part | Issue | Validated | Confirmed | Priority |
|------|-------|-----------|-----------|----------|
| 1.1 | PC Ownership Security | ✅ | **CRITICAL** | P0 |
| 2.1 | Lost Error Context | ✅ | MEDIUM | P2 |
| 2.2 | Silent Error Swallowing | ✅ | MIXED | P2-P3 |
| 2.3 | Queue Cleanup Failures | ✅ | MEDIUM | P2 |
| 2.4 | Error Sanitization | ✅ | HIGH (Security) | P1 |
| 3.1 | Early Return Ok() | ✅ | MEDIUM | P3 |

---

## Part 1: Security Issues (P0 - CRITICAL)

### 1.1 PC Ownership Validation in Challenge Handlers

**Status**: CONFIRMED CRITICAL SECURITY ISSUE

**Location**: `crates/engine/src/api/websocket/ws_challenge.rs`

**Affected Handlers**:
1. `handle_challenge_roll` - Lines 169-200
2. `handle_challenge_roll_input` - Lines 365-406

**Current Vulnerable Code** (lines 192-200):
```rust
let pc_id = match conn_info.pc_id {
    Some(id) => id,
    None => {
        return Some(error_response(
            ErrorCode::BadRequest,
            "Must have a PC to roll challenges",
        ))
    }
};
// MISSING: No check that conn_info.pc_id matches the requesting connection's PC
```

**The Problem**:
- Handlers extract `pc_id` from `conn_info.pc_id`
- NO validation that the connection actually owns that PC
- An attacker could potentially roll challenges on behalf of other players

**Correct Pattern** (from `ws_inventory.rs:45-50`):
```rust
if !conn_info.is_dm() && conn_info.pc_id != Some(pc_uuid) {
    return Some(error_response(
        ErrorCode::Unauthorized,
        "Cannot control this PC",
    ));
}
```

**Fix for `handle_challenge_roll`** (insert after line 200):
```rust
// Verify PC ownership - security fix for challenge authorization
if !conn_info.is_dm() && conn_info.pc_id != Some(pc_id) {
    return Some(error_response(
        ErrorCode::Unauthorized,
        "Cannot roll challenges for another player's character",
    ));
}
```

**Fix for `handle_challenge_roll_input`** (insert after line 396):
```rust
// Verify PC ownership - security fix for challenge authorization
if !conn_info.is_dm() && conn_info.pc_id != Some(pc_id) {
    return Some(error_response(
        ErrorCode::Unauthorized,
        "Cannot roll challenges for another player's character",
    ));
}
```

**Test Cases to Add**:
```rust
#[tokio::test]
async fn test_challenge_roll_unauthorized_pc_rejected() {
    // Player A tries to roll as Player B's PC - should fail
}

#[tokio::test]
async fn test_challenge_roll_own_pc_allowed() {
    // Player rolls with their own PC - should succeed
}

#[tokio::test]
async fn test_dm_can_roll_any_challenge() {
    // DM should be able to roll on behalf of any PC
}
```

---

## Part 2: Error Handling (P1-P2)

### 2.1 Lost Error Context in Importers

**Status**: CONFIRMED

**Location**: `crates/engine/src/infrastructure/importers/fivetools.rs`
**Lines**: 2109, 2176

**Current Code** (discards error):
```rust
let rt = tokio::runtime::Handle::try_current()
    .map_err(|_| ContentError::LoadError("No tokio runtime available".to_string()))?;
```

**Problem**: Original `tokio::runtime::TryCurrentError` is discarded with `|_|`. This error implements `Display` and contains useful debugging info.

**Inconsistency**: The rest of the file (20+ locations) preserves errors:
```rust
.map_err(|e| ContentError::LoadError(e.to_string()))  // CORRECT pattern used elsewhere
```

**Fix for Line 2109** (in `count_content`):
```rust
let rt = tokio::runtime::Handle::try_current()
    .map_err(|e| ContentError::LoadError(format!("Tokio runtime error: {}", e)))?;
```

**Fix for Line 2176** (in `search_content`):
```rust
let rt = tokio::runtime::Handle::try_current()
    .map_err(|e| ContentError::LoadError(format!("Tokio runtime error: {}", e)))?;
```

---

### 2.2 Silent Error Swallowing

**Status**: VALIDATED - Mixed findings (some intentional, some need fixing)

#### 2.2.1 ws_staging.rs:323 - NOT AN ISSUE
The code properly returns error responses. False positive.

#### 2.2.2 challenge/mod.rs:652-664, 691-699 - INTENTIONAL (Document Only)
Queue cleanup failures are logged but operation continues. Comments document this as intentional:
```rust
// Challenge is now resolved. Queue cleanup is housekeeping - log failure
// but return success since the important operation completed.
```
**Action**: No code change. Consider background cleanup job (see Part 2.3).

#### 2.2.3 connections.rs:228-287 - SHOULD REVIEW
Broadcast methods silently drop messages to disconnected clients.
- `broadcast_to_world()` - Lines 224-237
- `broadcast_to_dms()` - Lines 261-274
- `send_to_pc()` - Lines 277-290

**Assessment**:
- For broadcasts: Acceptable (best-effort to many clients)
- For `send_to_pc()`: Concerning (single player updates shouldn't silently fail)
- For `broadcast_critical_to_world()`: Lines 323-340 - Critical messages should track delivery

**Recommendation**: Add return type to `send_to_pc()` indicating delivery success:
```rust
pub async fn send_to_pc(&self, pc_id: PlayerCharacterId, message: ServerMessage) -> bool {
    // Return whether message was delivered
}
```

#### 2.2.4 challenge/mod.rs:455-461 - SHOULD FIX
```rust
if let Err(e) = self.observation.save_deduced_info(target_pc_id, info.clone()).await {
    tracing::warn!(error = %e, "Failed to persist revealed information");
}
```
**Problem**: Player learns info but it's not persisted to observations.
**Fix**: Propagate error or return partial success indicator.

---

### 2.3 Queue Cleanup Failures

**Status**: CONFIRMED - Intentional pattern but creates cleanup debt

**Location**: `crates/engine/src/use_cases/challenge/mod.rs`
**Lines**: 652-664 (Accept), 691-703 (Edit)

**Current Code**:
```rust
if let Err(e) = self.queue.mark_complete(QueueItemId::from(approval_id)).await {
    tracing::error!(
        approval_id = %approval_id,
        error = %e,
        "Failed to mark approval as complete. Queue item may remain and require manual cleanup."
    );
}
Ok(OutcomeDecisionResult::Resolved(...))  // Returns success anyway
```

**Consequences**:
- Queue item stays in "pending" state
- DM sees completed challenges in approval queue
- Potential double-processing if queue is re-queried

**Comparison**: Other queue operations propagate errors:
- `use_cases/queues/mod.rs:178` - Uses `?` to propagate
- `use_cases/approval/mod.rs:182-186` - Uses `?` to propagate

**Recommended Fix - Two Options**:

**Option A (Immediate)**: Propagate error
```rust
self.queue.mark_complete(QueueItemId::from(approval_id)).await
    .map_err(|e| OutcomeDecisionError::QueueCleanup(e))?;
```

**Option B (Long-term)**: Add background cleanup job
```rust
// New background task to clean up stale queue items
pub struct QueueCleanupWorker { ... }
// Runs every 5 minutes, completes items stuck in Processing > 5 min
```

---

### 2.4 Inconsistent Error Sanitization (SECURITY)

**Status**: CONFIRMED - Security issue

**Location**: `crates/engine/src/api/websocket/ws_time.rs`
**Line**: 489

**Current Code** (EXPOSES INTERNAL ERRORS):
```rust
Err(e) => Ok(ResponseResult::error(
    ErrorCode::InternalError,
    e.to_string(),  // ← UNSANITIZED - could expose DB errors, connection strings
)),
```

**Risk**: `TimeControlError::Repo(RepoError)` can contain:
- Database connection errors (host, port)
- Neo4j driver errors
- Schema/query information
- Stack traces

**Inconsistency**: Same file has 14 correct uses of `sanitize_repo_error()`:
- Lines 52, 127, 192, 250, 273, 330, 353, 455, 523, 588, 654, 734, 802, 839

**Fix for Line 489**:
```rust
Err(e) => Ok(ResponseResult::error(
    ErrorCode::InternalError,
    &sanitize_repo_error(&e, "getting game time"),
)),
```

**Reference**: `sanitize_repo_error()` at `api/websocket/error_sanitizer.rs:26`
- Logs full error server-side
- Returns generic message to client

---

## Part 3: Code Quality (P3)

### 3.1 Early Return Ok() on Missing Data

**Status**: CONFIRMED - Both locations validated

#### 3.1.1 RecordVisit (HIGH priority)

**Location**: `crates/engine/src/use_cases/observation/record_visit.rs`
**Line**: 55

**Current Code**:
```rust
let location_id = match region {
    Some(r) => r.location_id(),
    None => {
        tracing::warn!(region_id = %region_id, pc_id = %pc_id, "Cannot record visit: region not found");
        return Ok(());  // Caller thinks success!
    }
};
```

**Problem**:
- Caller (`EnterRegion::execute()`) can't distinguish success from skip
- A missing region during movement indicates data corruption
- The PC just moved to this region, so it MUST exist

**Fix**: Return error instead
```rust
let region = self.location_repo.get_region(region_id).await?
    .ok_or_else(|| RepoError::not_found("Region", region_id.to_string()))?;
let location_id = region.location_id();
```

**Test Update**: `handles_missing_region_gracefully()` (lines 241-271) should expect error, not Ok.

#### 3.1.2 AssetRepo::save (MEDIUM priority)

**Location**: `crates/engine/src/infrastructure/neo4j/asset_repo.rs`
**Line**: 114

**Current Code**:
```rust
let relationship_query = match asset.entity_type() {
    EntityType::Character => query(...),
    EntityType::Location => query(...),
    EntityType::Item => query(...),
    _ => return Ok(()),  // Silently skips relationship creation
};
```

**Problem**:
- Asset is saved but relationship is NOT created
- Creates orphaned asset node in graph
- Caller thinks asset is fully saved

**Fix**: Either remove catch-all or return explicit error
```rust
_ => return Err(RepoError::constraint(
    "asset",
    format!("Entity type {:?} does not support assets", asset.entity_type())
)),
```

---

## Execution Order

| Order | Part | Priority | Effort | Risk |
|-------|------|----------|--------|------|
| 1 | 1.1 PC Ownership | P0 CRITICAL | 30 min | Security |
| 2 | 2.4 Error Sanitization | P1 HIGH | 5 min | Security |
| 3 | 2.1 Lost Error Context | P2 | 10 min | Low |
| 4 | 2.3 Queue Cleanup (Option A) | P2 | 30 min | Medium |
| 5 | 2.2.4 Observation Save | P2 | 15 min | Low |
| 6 | 3.1.1 RecordVisit Error | P3 | 20 min | Low |
| 7 | 3.1.2 AssetRepo Error | P3 | 15 min | Low |

---

## Verification

After each fix:
```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

Specific tests:
```bash
# After Part 1.1
cargo test -p wrldbldr-engine ws_challenge

# After Part 2.4
cargo test -p wrldbldr-engine ws_time

# After Part 3.1.1
cargo test -p wrldbldr-engine observation
```

---

## Summary

| Category | Issues | Critical | High | Medium | Low |
|----------|--------|----------|------|--------|-----|
| Security | 2 | 1 | 1 | 0 | 0 |
| Error Handling | 4 | 0 | 0 | 3 | 1 |
| Code Quality | 2 | 0 | 0 | 1 | 1 |
| **Total** | **8** | **1** | **1** | **4** | **2** |

**Estimated Total Effort**: ~2-3 hours for all fixes
