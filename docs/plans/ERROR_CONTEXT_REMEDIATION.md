# Error Context Remediation Plan

**Date:** 2026-01-20
**Status:** Ready for Implementation
**Priority:** MEDIUM
**Estimated Changes:** ~25 lines

---

## Overview

This plan addresses the final remaining error context issues from the code review remediation. Most of the original plan has been completed:

| Part | Status | Description |
|------|--------|-------------|
| Part 1: Error Context | **Partial** | 4 issues remaining (1 critical, 3 minor) |
| Part 2: NpcPresence Enum | ✅ Complete | `NpcPresence` enum in `staging.rs` |
| Part 3: GameTimeConfig | ✅ Complete | Private fields with accessors |
| Part 4: Stat Enum | ✅ Complete | `Option<Stat>` in Challenge entity |

---

## Fail-Fast Philosophy

WrldBldr uses fail-fast error handling. This is documented in:
- `docs/CLAUDE.md` (lines 560-579)
- `docs/architecture/review.md` (lines 449-454)

**Key Principles:**
- Errors MUST bubble up to users (player or DM depending on who initiated)
- NO silent defaults for corrupted data
- Include full context in error messages (entity type, ID, operation, parse error)

**FromStr vs Serde distinction** (from `VisualStateSource::from_str` docstring):
- `FromStr` = internal/DB data → fail-fast on unknown values (data corruption)
- `#[serde(other)]` = external JSON → forward compatibility, map unknown → default

---

## Remaining Work

### Issue 1: CRITICAL - MoodState::from_str Violates Fail-Fast

**File:** `crates/domain/src/types/mood.rs`
**Line:** 215

**Current Code:**
```rust
impl FromStr for MoodState {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "happy" => Ok(MoodState::Happy),
            // ... other variants ...
            _ => Ok(MoodState::Unknown),  // PROBLEM: Never fails!
        }
    }
}
```

**Problem:** `MoodState::from_str` returns `Ok(MoodState::Unknown)` for unrecognized values instead of returning an error. This violates fail-fast because:
1. Corrupted database values silently become `Unknown`
2. The error handling in `approve.rs:244` is dead code (never triggers)
3. DM never learns about data corruption

**Expected Code:**
```rust
impl FromStr for MoodState {
    type Err = DomainError;

    /// Parses a string into a MoodState.
    ///
    /// Unlike serde deserialization (which falls back to `Unknown` for unknown values
    /// via `#[serde(other)]`), this returns an error for unrecognized inputs.
    ///
    /// **Rationale**: `FromStr` is typically used for internal/validated sources
    /// (e.g., database values) where unknown values indicate data corruption or a bug.
    /// Failing fast surfaces these issues immediately. Serde's fallback handles
    /// forward compatibility for external JSON payloads from updated clients.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "happy" => Ok(MoodState::Happy),
            "calm" => Ok(MoodState::Calm),
            "anxious" => Ok(MoodState::Anxious),
            "excited" => Ok(MoodState::Excited),
            "melancholic" => Ok(MoodState::Melancholic),
            "irritated" => Ok(MoodState::Irritated),
            "alert" => Ok(MoodState::Alert),
            "bored" => Ok(MoodState::Bored),
            "fearful" => Ok(MoodState::Fearful),
            "hopeful" => Ok(MoodState::Hopeful),
            "curious" => Ok(MoodState::Curious),
            "contemplative" => Ok(MoodState::Contemplative),
            "amused" => Ok(MoodState::Amused),
            "weary" => Ok(MoodState::Weary),
            "confident" => Ok(MoodState::Confident),
            "nervous" => Ok(MoodState::Nervous),
            "unknown" => Ok(MoodState::Unknown),  // Explicit "unknown" string is valid
            _ => Err(DomainError::parse(format!(
                "Unknown mood state: '{}'. Valid values: happy, calm, anxious, excited, \
                melancholic, irritated, alert, bored, fearful, hopeful, curious, \
                contemplative, amused, weary, confident, nervous, unknown",
                s
            ))),
        }
    }
}
```

**Changes:**
1. Return `Err(DomainError::parse(...))` for unknown values
2. Add docstring explaining fail-fast rationale
3. Allow explicit `"unknown"` string as valid input

**Note:** The `#[serde(other)]` attribute on `MoodState::Unknown` remains for JSON forward compatibility.

**Impact Analysis:** There are 3 places that parse `MoodState`:
1. `approve.rs:243` - Has `.map_err()` error handling ✅
2. `expression_config_editor.rs:62` - Uses `if let Ok(mood) = ...` pattern, silently ignores invalid values (acceptable for UI dropdown)
3. `mood.rs:231-234` - Unit tests (need updating)

---

### Issue 2: Include Parse Error in approve.rs (Now Functional)

**File:** `crates/engine/src/use_cases/staging/approve.rs`
**Line:** 244

After Issue 1 is fixed, this error handling will actually trigger. Update to include the parse error.

**Current Code:**
```rust
.map_err(|_| {
    StagingError::Validation(format!(
        "Invalid mood state '{}' for character {}",
        mood_str, npc_info.character_id
    ))
})?
```

**Expected Code:**
```rust
.map_err(|e| {
    StagingError::Validation(format!(
        "Invalid mood state '{}' for character {}: {}",
        mood_str, npc_info.character_id, e
    ))
})?
```

**Change:** Replace `|_|` with `|e|` and append `: {}` with `e`.

---

### Issue 3: NPC ID Parse Error Context (e2e_scenarios.rs)

**File:** `crates/engine/src/api/websocket/e2e_scenarios.rs`
**Lines:** 89, 116

These are in E2E test support code, but should still include error context for better debugging.

**Current Code (line 89):**
```rust
let npc_uuid: uuid::Uuid = conversation
    .npc_id
    .parse()
    .map_err(|_| E2EError::RequestFailed("Invalid NPC ID in conversation".to_string()))?;
```

**Current Code (line 116):**
```rust
let npc_uuid: uuid::Uuid = conversation
    .npc_id
    .parse()
    .map_err(|_| E2EError::RequestFailed("Invalid NPC ID in conversation".to_string()))?;
```

**Expected Code (both locations):**
```rust
let npc_uuid: uuid::Uuid = conversation
    .npc_id
    .parse()
    .map_err(|e| E2EError::RequestFailed(format!(
        "Invalid NPC ID '{}' in conversation: {}",
        conversation.npc_id, e
    )))?;
```

**Change:** Include the actual NPC ID value and the parse error in the message.

---

### Non-Issues (Acceptable As-Is)

**File:** `crates/engine/src/infrastructure/importers/fivetools.rs`
**Lines:** 2109, 2176

These use `tokio::runtime::Handle::try_current().map_err(|_| ...)` to check for a tokio runtime. The error type `TryCurrentError` doesn't provide meaningful additional context beyond "no runtime available", so these are acceptable as-is.

---

## Implementation Steps

1. **Edit `mood.rs` (Issue 1 - CRITICAL)**
   - Replace the `_ => Ok(MoodState::Unknown)` catch-all with:
     - `"unknown" => Ok(MoodState::Unknown)` for explicit unknown string
     - `_ => Err(DomainError::parse(...))` for truly unknown values
   - Add docstring explaining fail-fast rationale

2. **Edit `approve.rs:244` (Issue 2)**
   - Change `.map_err(|_|` to `.map_err(|e|`
   - Append `: {}` with `e` to the format string

3. **Edit `e2e_scenarios.rs:89` (Issue 3)**
   - Change `.map_err(|_|` to `.map_err(|e|`
   - Change error message to include `conversation.npc_id` and `e`

4. **Edit `e2e_scenarios.rs:116` (Issue 3)**
   - Same changes as line 89

5. **Update test in `mood.rs`**
   - The test `test_mood_parse` expects `"unknown_value".parse()` to return `Ok(MoodState::Unknown)`
   - Update to expect an error instead

---

## Verification

```bash
# Build check
cargo check --workspace

# Run tests
cargo test --workspace

# Run specific tests for mood parsing
cargo test -p wrldbldr-domain mood

# Verify no remaining |_| patterns in these files
grep -n "map_err(|_|" crates/engine/src/use_cases/staging/approve.rs
grep -n "map_err(|_|" crates/engine/src/api/websocket/e2e_scenarios.rs
# Both should return no matches after changes
```

---

## Summary

| Issue | Location | Change | Priority | Risk |
|-------|----------|--------|----------|------|
| 1 | `mood.rs:215` | Fail-fast on unknown mood values | CRITICAL | Medium |
| 2 | `approve.rs:244` | Include parse error in validation message | Low | Low |
| 3a | `e2e_scenarios.rs:89` | Include NPC ID and parse error | Low | Low |
| 3b | `e2e_scenarios.rs:116` | Include NPC ID and parse error | Low | Low |

**Total:** 4 changes across 3 files, ~30 lines modified

---

## Test Updates Required

The following test in `crates/domain/src/types/mood.rs` needs updating:

**Current Test:**
```rust
#[test]
fn test_mood_parse() {
    assert_eq!("happy".parse::<MoodState>().unwrap(), MoodState::Happy);
    assert_eq!("ANXIOUS".parse::<MoodState>().unwrap(), MoodState::Anxious);
    assert_eq!(
        "unknown_value".parse::<MoodState>().unwrap(),
        MoodState::Unknown
    );
}
```

**Expected Test:**
```rust
#[test]
fn test_mood_parse() {
    assert_eq!("happy".parse::<MoodState>().unwrap(), MoodState::Happy);
    assert_eq!("ANXIOUS".parse::<MoodState>().unwrap(), MoodState::Anxious);
    assert_eq!("unknown".parse::<MoodState>().unwrap(), MoodState::Unknown);
    assert!("unknown_value".parse::<MoodState>().is_err());
}
```
