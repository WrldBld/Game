# Code Review Remediation Plan v5

**Date:** January 19, 2026
**Purpose:** Address findings from comprehensive code review: error handling, type safety, and cleanup
**Reviewer:** Claude Opus 4.5
**Priority:** Error handling (critical) > Type safety (high) > Cleanup (low)

---

## Executive Summary

A comprehensive code review identified **39 error handling issues**, **8 type safety issues**, and **4 cleanup tasks**. The most critical finding is that **challenge outcome triggers fail silently** - players can win challenges but not receive rewards.

| Phase | Priority | Issues | Est. Changes |
|-------|----------|--------|--------------|
| 1 | CRITICAL | 5 swallowed errors (challenge triggers) + 3 data corruption | ~80 lines |
| 2 | HIGH | 26 lost error context | ~100 lines |
| 3 | MEDIUM | 8 type safety issues | ~150 lines |
| 4 | LOW | 4 cleanup tasks | ~30 lines |

---

## Phase 1: Critical Error Handling (MUST FIX)

These issues cause **silent failures in game mechanics**.

### 1.1 Challenge Outcome Triggers Fail Silently

**File:** `crates/engine/src/use_cases/challenge/mod.rs`
**Lines:** 480-525
**Impact:** Players win challenges but don't receive rewards, scenes don't transition, challenges don't enable

#### Current Code (lines 480-486)

```rust
if let Err(e) = give_item
    .execute(target_pc_id, item_name.clone(), item_description.clone())
    .await
{
    tracing::warn!(error = %e, "Failed to give item to player");
}
Ok(())
```

#### Required Change

Change `if let Err` to propagate with `?`:

```rust
give_item
    .execute(target_pc_id, item_name.clone(), item_description.clone())
    .await
    .map_err(|e| {
        tracing::error!(
            error = %e,
            item = %item_name,
            pc_id = %target_pc_id,
            "Failed to give challenge reward item"
        );
        OutcomeDecisionError::TriggerExecutionFailed(format!(
            "Failed to give item '{}': {}", item_name, e
        ))
    })?;
Ok(())
```

#### Similar Fixes Required (same file)

| Lines | Trigger Type | Change |
|-------|--------------|--------|
| 496-499 | `set_current_scene` | Propagate with context |
| 509-512 | `enable_challenge` | Propagate with context |
| 522-525 | `disable_challenge` | Propagate with context |
| 536-539 | `modify_stat` | Propagate with context |

#### Update OutcomeDecisionError Enum

Add new variant to support trigger execution failures:

```rust
// In crates/engine/src/use_cases/challenge/mod.rs (around line 730)
#[derive(Debug, thiserror::Error)]
pub enum OutcomeDecisionError {
    // ... existing variants ...

    #[error("Trigger execution failed: {0}")]
    TriggerExecutionFailed(String),
}
```

---

### 1.2 Data Corruption Recovery Masks Issues

**File:** `crates/engine/src/infrastructure/neo4j/character_repo.rs`
**Line:** 73
**Impact:** Corrupted UUIDs generate new IDs, orphaning stat modifiers

#### Current Code

```rust
let id = Uuid::parse_str(&value.id).unwrap_or_else(|e| {
    let new_id = Uuid::new_v4();
    tracing::warn!(
        stored_id = %value.id,
        new_id = %new_id,
        error = %e,
        "Corrupted UUID in stored stat modifier, generating new ID. Modifier may become orphaned."
    );
    new_id
});
```

#### Required Change

Fail loudly on data corruption:

```rust
let id = Uuid::parse_str(&value.id).map_err(|e| {
    RepoError::database(
        "data_corruption",
        format!(
            "Corrupted UUID in stat modifier '{}': {}. Database integrity compromised.",
            value.id, e
        )
    )
})?;
```

---

### 1.3 Player Stats JSON Corruption Silently Resets

**File:** `crates/engine/src/infrastructure/neo4j/player_character_repo.rs`
**Line:** 386
**Impact:** Invalid stats JSON silently becomes empty map - player loses all stats

#### Current Code

```rust
let mut stats: std::collections::HashMap<String, i64> =
    serde_json::from_str(&stats_json).unwrap_or_default();
```

#### Required Change

```rust
let mut stats: std::collections::HashMap<String, i64> =
    serde_json::from_str(&stats_json).map_err(|e| {
        RepoError::database(
            "data_corruption",
            format!("Corrupted stats JSON for PlayerCharacter {}: {}", id, e)
        )
    })?;
```

---

### 1.4 Unknown Tool Types Succeed Silently

**File:** `crates/engine/src/use_cases/approval/tool_executor.rs`
**Line:** 130
**Impact:** LLM hallucinations (unknown tools) appear to succeed

#### Current Code

```rust
_ => {
    tracing::warn!(tool_name = %tool.name, "Unknown tool type - skipping execution");
    Ok(ToolExecutionResult {
        tool_id: tool.id.clone(),
        description: format!("Unknown tool '{}' - no action taken", tool.name),
    })
}
```

#### Required Change

Add error type and fail:

```rust
// Add to ToolExecutionError enum (or create if doesn't exist)
#[error("Unknown tool type: {0}")]
UnknownTool(String),

// In match arm:
_ => {
    tracing::error!(tool_name = %tool.name, "Unknown tool type in LLM response");
    Err(ToolExecutionError::UnknownTool(tool.name.clone()))
}
```

---

## Phase 2: Error Context Preservation (HIGH Priority)

These issues discard original errors, making debugging difficult.

### Pattern to Apply

**BEFORE:**
```rust
something.parse().map_err(|_| {
    RepoError::database("parse", format!("Invalid Foo: '{}'", value))
})?
```

**AFTER:**
```rust
something.parse().map_err(|e| {
    RepoError::database("parse", format!("Invalid Foo '{}': {}", value, e))
})?
```

### 2.1 Neo4j Helpers (5 instances)

**File:** `crates/engine/src/infrastructure/neo4j/helpers.rs`

| Line | Context | Field to Include |
|------|---------|------------------|
| 159 | `get()` for required field (Node trait) | `e` (database error) |
| 172 | `get()` for required field (Node trait) | `e` (database error) |
| 176 | `get()` for datetime field (Node trait) | `e` (database error) |
| 281 | `get()` for required column (Row trait) | `e` (database error) |
| 296 | `get()` for required column (Row trait) | `e` (database error) |

**Note:** The JSON parse errors (lines 162-167, 284-292) already include the error `e`. Only the initial `get()` calls need fixing.

### 2.2 Character Repository (9 instances)

**File:** `crates/engine/src/infrastructure/neo4j/character_repo.rs`

| Line | Parse Target | Add to Message |
|------|--------------|----------------|
| 169 | `CampbellArchetype` | `e` (parse error) |
| 175 | `CharacterState` | `e` (parse error) |
| 229 | `CampbellArchetype` (base) | `e` (parse error) |
| 238 | `CampbellArchetype` (current) | `e` (parse error) |
| 271 | `MoodState` | `e` (parse error) |
| 298 | `Description::new()` | Change from `unwrap_or_else` to `map_err` |
| 619 | `WantPriority` | `e` (parse error) |
| 1033 | `RelationshipLevel` | `e` (parse error) |
| 1155 | `ActantialRole` | `e` (parse error) |

### 2.3 Narrative Repository (2 instances)

**File:** `crates/engine/src/infrastructure/neo4j/narrative_repo.rs`

| Line | Parse Target | Add to Message |
|------|--------------|----------------|
| 1520 | UUID in event chain | `e` (uuid parse error) |
| 1529 | UUID in event chain | `e` (uuid parse error) |

### 2.4 Staging Repository (1 instance)

**File:** `crates/engine/src/infrastructure/neo4j/staging_repo.rs`

| Line | Context | Add to Message |
|------|---------|----------------|
| 758 | NPC name field | `e` (database error) |

### 2.5 Content Repository (1 instance)

**File:** `crates/engine/src/infrastructure/neo4j/content_repo.rs`

| Line | Parse Target | Add to Message |
|------|--------------|----------------|
| 42 | `SkillCategory` | `e` (parse error) |

### 2.6 Lore Repository (2 instances)

**File:** `crates/engine/src/infrastructure/neo4j/lore_repo.rs`

| Line | Parse Target | Add to Message |
|------|--------------|----------------|
| 97 | `LoreCategory` | `e` (parse error) |
| 149 | `known_chunk_ids` column | `e` (database error) |

### 2.7 Scene Repository (1 instance)

**File:** `crates/engine/src/infrastructure/neo4j/scene_repo.rs`

| Line | Parse Target | Add to Message |
|------|--------------|----------------|
| 535 | `SceneCharacterRole` | `e` (parse error) |

### 2.8 Act Repository (1 instance)

**File:** `crates/engine/src/infrastructure/neo4j/act_repo.rs`

| Line | Parse Target | Add to Message |
|------|--------------|----------------|
| 40 | `MonomythStage` | `e` (parse error) |

### 2.9 API/WebSocket (4 instances)

**File:** `crates/engine/src/api/websocket/mod.rs`

| Line | Context | Add to Message |
|------|---------|----------------|
| 865 | UUID parse for IDs | `e` (uuid parse error) |

**File:** `crates/engine/src/api/websocket/ws_npc.rs`

| Line | Context | Add to Message |
|------|---------|----------------|
| 101 | `RelationshipLevel` parse | `e` in error response |

**File:** `crates/engine/src/api/websocket/ws_creator.rs`

| Line | Context | Add to Message |
|------|---------|----------------|
| 592 | grid_layout columns parse | `e` (parse error) |
| 606 | grid_layout rows parse | `e` (parse error) |

---

## Phase 3: Type Safety (MEDIUM Priority)

### 3.1 Magic Strings for Stats → Stat Enum

The `Stat` enum already exists at `crates/domain/src/value_objects/stat.rs`. Use it consistently.

**Files to Update:**

#### 3.1.1 Feat System

**File:** `crates/domain/src/entities/feat.rs`

| Line | Current | Change To |
|------|---------|-----------|
| 112 | `stat: String` in `Prerequisite::MinStat` | `stat: Stat` |
| 219 | `stat: String` in `FeatBenefit::StatIncrease` | `stat: Stat` |
| 226 | `options: Vec<String>` in `FeatBenefit::StatChoice` | `options: Vec<Stat>` |
| 322 | `stat: String` in `UsesFormula::StatModifier` | `stat: Stat` |

**Update construction sites** (lines 424-451):
```rust
// BEFORE
Prerequisite::MinStat { stat: "STR".to_string(), min_value: 13 }

// AFTER
Prerequisite::MinStat { stat: Stat::Str, min_value: 13 }
```

#### 3.1.2 Skill System

**File:** `crates/domain/src/entities/skill.rs`

| Line | Current | Change To |
|------|---------|-----------|
| 28 | `base_attribute: Option<String>` | `base_attribute: Option<Stat>` |

**Update construction** (lines 189-1789):
```rust
// BEFORE
base_attribute: Some("STR".to_string()),

// AFTER
base_attribute: Some(Stat::Str),
```

#### 3.1.3 Challenge System

**File:** `crates/domain/src/entities/challenge.rs`

| Line | Current | Change To |
|------|---------|-----------|
| 781 | `ModifyCharacterStat { stat: String, ... }` | `stat: Stat` |

**Note:** The `Challenge.check_stat` field was already converted to `Option<Stat>` in the previous remediation.

### 3.2 Raw Uuid → QueueItemId

**File:** `crates/engine/src/infrastructure/ports/external.rs`

| Line | Current | Change To |
|------|---------|-----------|
| 211 | `-> Result<Uuid, ...>` | `-> Result<QueueItemId, ...>` |
| 215 | `-> Result<Uuid, ...>` | `-> Result<QueueItemId, ...>` |
| 219 | `-> Result<Uuid, ...>` | `-> Result<QueueItemId, ...>` |
| 230 | `id: Uuid` | `id: QueueItemId` |
| 231 | `id: Uuid` | `id: QueueItemId` |
| 244 | `id: Uuid` | `id: QueueItemId` |

**Update implementations:**
- `crates/engine/src/infrastructure/queue.rs`
- `crates/engine/src/e2e_tests/logging_queue.rs`
- `crates/engine/src/api/websocket/test_support.rs`

**Update use case result structs:**

**File:** `crates/engine/src/use_cases/conversation/continue_conversation.rs`
| Line | Current | Change To |
|------|---------|-----------|
| 28 | `action_queue_id: Uuid` | `action_queue_id: QueueItemId` |

**File:** `crates/engine/src/use_cases/conversation/start.rs`
| Line | Current | Change To |
|------|---------|-----------|
| 24 | `action_queue_id: Uuid` | `action_queue_id: QueueItemId` |

---

## Phase 4: Cleanup (LOW Priority)

### 4.1 Remove Legacy Type Aliases

**File:** `crates/engine/src/use_cases/assets/expression_sheet.rs`

Remove lines 34-35:
```rust
// Type aliases for old names to maintain compatibility
type CharacterRepository = dyn CharacterRepo;
```

Update usages at lines 139, 148 to use `dyn CharacterRepo` directly.

**File:** `crates/engine/src/use_cases/challenge/crud.rs`

Remove lines 10-11:
```rust
// Type alias for old name to maintain compatibility
type ChallengeRepository = dyn ChallengeRepo;
```

Update usages at lines 37, 41 to use `dyn ChallengeRepo` directly.

### 4.2 Challenge Entity Over-Encapsulation (Optional)

**File:** `crates/domain/src/entities/challenge.rs`

Per ADR-008, the `Challenge` entity has no invariants to protect. Consider making fields public:

**Current (lines 36-58):**
```rust
pub struct Challenge {
    id: ChallengeId,           // Private
    world_id: WorldId,         // Private
    name: ChallengeName,       // Private
    // ... all private
}
```

**Option A (Full public):**
```rust
pub struct Challenge {
    pub id: ChallengeId,
    pub world_id: WorldId,
    pub name: ChallengeName,
    // ... all public
}
```

**Option B (Keep as-is):** The current pattern is consistent with other entities, even if slightly over-encapsulated.

**Recommendation:** Keep as-is (Option B) for consistency. This is LOW priority.

---

## Verification Commands

After each phase, run:

```bash
# Compilation check
cargo check --workspace

# Clippy (no warnings)
cargo clippy --workspace -- -D warnings

# Tests
cargo test --workspace

# Specific test suites
cargo test -p wrldbldr-engine challenge  # Phase 1
cargo test -p wrldbldr-engine neo4j      # Phase 2
cargo test -p wrldbldr-domain            # Phase 3
```

---

## Implementation Order

### Phase 1 (Do First - Critical)
1. Fix `challenge/mod.rs` swallowed errors (5 triggers: GiveItem, TriggerScene, Enable/DisableChallenge, ModifyStat)
2. Fix `character_repo.rs:73` UUID corruption
3. Fix `player_character_repo.rs:386` stats JSON corruption
4. Fix `tool_executor.rs:130` unknown tool handling

### Phase 2 (Do Second - High)
5. Fix all 26 `.map_err(|_|` patterns in neo4j repos
6. Fix 4 API/WebSocket error context issues

### Phase 3 (Do Third - Medium)
7. Update `feat.rs` to use `Stat` enum
8. Update `skill.rs` to use `Stat` enum
9. Update `challenge.rs:781` to use `Stat` enum
10. Update `QueuePort` trait to use `QueueItemId`
11. Update all `QueuePort` implementations and callers

### Phase 4 (Do Last - Low)
12. Remove type aliases in `expression_sheet.rs` and `crud.rs`
13. (Optional) Review Challenge encapsulation

---

## Summary Table

| Phase | File | Line(s) | Change Type |
|-------|------|---------|-------------|
| 1.1 | challenge/mod.rs | 480-486, 496-499, 509-512, 522-525, 536-539 | Propagate errors |
| 1.2 | character_repo.rs | 73 | Fail on corruption |
| 1.3 | player_character_repo.rs | 386 | Fail on corruption |
| 1.4 | tool_executor.rs | 130 | Fail on unknown |
| 2.1 | helpers.rs | 159,172,176,281,296 | Add error context |
| 2.2 | character_repo.rs | 169,175,229,238,271,298,619,1033,1155 | Add error context |
| 2.3 | narrative_repo.rs | 1520,1529 | Add error context |
| 2.4 | staging_repo.rs | 758 | Add error context |
| 2.5 | content_repo.rs | 42 | Add error context |
| 2.6 | lore_repo.rs | 97,149 | Add error context |
| 2.7 | scene_repo.rs | 535 | Add error context |
| 2.8 | act_repo.rs | 40 | Add error context |
| 2.9 | ws_*.rs | various | Add error context |
| 3.1 | feat.rs | 112,219,226,322 | String → Stat |
| 3.1 | skill.rs | 28 | String → Stat |
| 3.1 | challenge.rs | 781 | String → Stat |
| 3.2 | ports/external.rs | 211,215,219,230,231,244 | Uuid → QueueItemId |
| 3.2 | conversation/*.rs | 24,28 | Uuid → QueueItemId |
| 4.1 | expression_sheet.rs | 34-35 | Remove alias |
| 4.1 | crud.rs | 10-11 | Remove alias |

**Total: ~50 files, ~330 lines changed**

---

## Revision History

| Date | Change |
|------|--------|
| 2026-01-19 | Initial plan from code review |
