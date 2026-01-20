# Code Review Remediation Plan v6

## Overview

This plan addresses findings from the comprehensive code review conducted on 2026-01-20.

**Philosophy: Fail-Fast Error Handling**

WrldBldr uses a fail-fast approach where errors bubble up to the appropriate user:
- **Player-facing errors** → Display to player via WebSocket error message
- **DM-facing errors** → Display to DM via WebSocket error message
- **System errors** → Log with full context, return user-friendly message

Errors should NOT be silently swallowed. If an operation fails, the user who initiated it should know. This provides transparency and allows users to retry or report issues.

**Scope**: 3 parts, ~15 files, ~200 lines changed

---

## Part 1: Error Handling - Fail-Fast Compliance

### 1.1 CRITICAL: Silent Error Swallowing

**File**: `crates/engine/src/use_cases/narrative_operations.rs`
**Lines**: 423-442

**Current Code** (WRONG - silently swallows error):
```rust
if let Err(e) = self
    .narrative
    .record_dialogue_context(
        world_id,
        event_id,
        pc_id,
        npc_id,
        player_text,
        npc_text,
        topics_for_context,
        resolved_scene_id,
        resolved_location_id,
        resolved_region_id,
        world_game_time.clone(),
        timestamp,
    )
    .await
{
    tracing::error!(error = %e, "Failed to record dialogue conversation context");
}

Ok(event_id)  // Returns success even though recording failed!
```

**Expected Code** (Fail-fast - propagate error):
```rust
self.narrative
    .record_dialogue_context(
        world_id,
        event_id,
        pc_id,
        npc_id,
        player_text,
        npc_text,
        topics_for_context,
        resolved_scene_id,
        resolved_location_id,
        resolved_region_id,
        world_game_time.clone(),
        timestamp,
    )
    .await?;

Ok(event_id)
```

**Rationale**: Recording dialogue context is critical for the narrative system. If it fails, the user should know so they can retry. The error will bubble up to the WebSocket handler which will send an appropriate error message.

---

### 1.2 HIGH: Lost Error Context with `.ok()` - FAIL FAST

**File**: `crates/engine/src/use_cases/staging/approve.rs`
**Lines**: 241-284

**Current Code** (WRONG - loses parse error context and uses defaults):
```rust
let mood = npc_info
    .mood
    .as_deref()
    .and_then(|m| m.parse::<wrldbldr_domain::MoodState>().ok())  // .ok() discards error
    .unwrap_or(default_mood);

if let Some(sprite) = sprite_asset.and_then(|s| wrldbldr_domain::AssetPath::new(s).ok())
{
    staged_npc = staged_npc.with_sprite(sprite);
}
```

**Expected Code** (Fail-fast - propagate errors to DM):
```rust
let mood = match npc_info.mood.as_deref() {
    Some(mood_str) => mood_str
        .parse::<wrldbldr_domain::MoodState>()
        .map_err(|_| {
            StagingError::Validation(format!(
                "Invalid mood state '{}' for character {}",
                mood_str, npc_info.character_id
            ))
        })?,
    None => default_mood,
};

if let Some(sprite_path) = sprite_asset {
    let sprite = wrldbldr_domain::AssetPath::new(sprite_path.clone()).map_err(|e| {
        StagingError::Validation(format!(
            "Invalid sprite asset path '{}' for character {}: {}",
            sprite_path, npc_info.character_id, e
        ))
    })?;
    staged_npc = staged_npc.with_sprite(sprite);
}

if let Some(portrait_path) = portrait_asset {
    let portrait = wrldbldr_domain::AssetPath::new(portrait_path.clone()).map_err(|e| {
        StagingError::Validation(format!(
            "Invalid portrait asset path '{}' for character {}: {}",
            portrait_path, npc_info.character_id, e
        ))
    })?;
    staged_npc = staged_npc.with_portrait(portrait);
}
```

**Also update function signature**:
```rust
// BEFORE
async fn build_staged_npcs(&self, approved_npcs: &[ApprovedNpc]) -> Vec<StagedNpc>

// AFTER
async fn build_staged_npcs(&self, approved_npcs: &[ApprovedNpc]) -> Result<Vec<StagedNpc>, StagingError>
```

**Rationale**: Corrupted data should fail fast and inform the DM. Silent fallbacks hide data issues that need to be fixed. The DM can then fix the corrupted data and retry.

---

### 1.3 LOW: Discarded Domain Event Information

**File**: `crates/engine/src/use_cases/time/mod.rs`
**Line**: 224

**Context**: `World::advance_hours()` returns `TimeAdvanceResult` with:
- `previous_time: GameTime`
- `new_time: GameTime`
- `minutes_advanced: u32`
- `period_changed: bool`  ← This is lost!

**Current Code** (Loses `period_changed` info):
```rust
let previous_time = world.game_time().clone();
let _ = world.advance_hours(hours, self.clock.now());

self.world.save(&world).await?;

Ok(TimeAdvanceOutcome {
    previous_time,
    new_time: world.game_time().clone(),
    minutes_advanced: hours * 60,
})
```

**Expected Code** (Use domain result directly):
```rust
let result = world.advance_hours(hours, self.clock.now());

self.world.save(&world).await?;

Ok(TimeAdvanceOutcome {
    previous_time: result.previous_time,
    new_time: result.new_time,
    minutes_advanced: result.minutes_advanced,
    period_changed: result.period_changed,  // Propagate to caller
})
```

**Also update** `TimeAdvanceOutcome` at line 362:
```rust
pub struct TimeAdvanceOutcome {
    pub previous_time: GameTime,
    pub new_time: GameTime,
    pub minutes_advanced: u32,
    pub period_changed: bool,  // Add this field
}
```

**Rationale**: The `period_changed` field indicates dawn/dusk/etc. transitions that the UI may want to display. Currently this information is computed but discarded.

---

## Part 2: Type Safety - Replace Magic Strings with Enums

The domain layer already has proper enums defined. The protocol layer should use them instead of raw strings.

### 2.1 HIGH: Protocol Layer Uses String Instead of Enums

**File**: `crates/shared/src/requests/npc.rs`
**Lines**: 6-54

**Current Code** (WRONG - magic strings):
```rust
pub enum NpcRequest {
    SetNpcDisposition {
        npc_id: String,
        pc_id: String,
        disposition: String,  // Magic string
        #[serde(default)]
        reason: Option<String>,
    },
    SetNpcMood {
        npc_id: String,
        region_id: String,
        mood: String,  // Magic string
        #[serde(default)]
        reason: Option<String>,
    },
    // ...
}
```

**Expected Code** (Use enums from domain):
```rust
use wrldbldr_domain::{DispositionLevel, MoodState};

pub enum NpcRequest {
    SetNpcDisposition {
        npc_id: String,  // Keep as String for wire format, parse to CharacterId in handler
        pc_id: String,
        disposition: DispositionLevel,  // Type-safe enum
        #[serde(default)]
        reason: Option<String>,
    },
    SetNpcMood {
        npc_id: String,
        region_id: String,
        mood: MoodState,  // Type-safe enum
        #[serde(default)]
        reason: Option<String>,
    },
    // ...
}
```

**Note on ID types**: Keep IDs as `String` in wire format (they come from JSON). Parse to typed IDs (`CharacterId`, `RegionId`) in the WebSocket handler. The enums (`MoodState`, `DispositionLevel`) serialize/deserialize directly via serde.

**Existing enums to use**:
- `wrldbldr_domain::MoodState` (from `crates/domain/src/types/mood.rs`)
- `wrldbldr_domain::DispositionLevel` (from `crates/domain/src/types/disposition.rs`)

---

### 2.2 HIGH: Feat Entity Uses String Instead of Stat Enum

**File**: `crates/domain/src/entities/feat.rs`
**Lines**: 248-255 (GrantAbility variant in FeatBenefit enum)

**Current Code** (WRONG - uses magic string):
```rust
/// Grant a special ability
GrantAbility {
    /// Name of the ability
    ability: String,  // e.g., "STR", "DEX" - should use Stat enum
    /// Description of what the ability does
    description: String,
    /// Uses per rest (if limited)
    uses: Option<AbilityUses>,
},
```

**Wait - Review this**: On closer inspection, `ability` here is the NAME of the ability being granted (e.g., "Darkvision", "Second Wind"), not a stat. This may be correct as-is.

**Check also lines 110-115** (Prerequisite::MinStat) - this DOES use `Stat` enum correctly:
```rust
MinStat {
    stat: Stat,  // Already using enum
    value: i32,
},
```

**And lines 214-219** (FeatBenefit::StatIncrease) - also correct:
```rust
StatIncrease {
    stat: Stat,  // Already using enum
    value: i32,
},
```

**Conclusion**: feat.rs is mostly correct. The `ability: String` in `GrantAbility` is for ability NAMES, not stats.

---

### 2.3 HIGH: CharacterSpells Uses String for Stat

**File**: `crates/domain/src/entities/character_content.rs`
**Line**: 28

**Current Code** (WRONG):
```rust
pub struct CharacterSpells {
    // ...
    /// Primary spellcasting ability (e.g., "INT", "WIS", "CHA")
    spellcasting_ability: Option<String>,
}
```

**Expected Code**:
```rust
use crate::value_objects::Stat;

pub struct CharacterSpells {
    // ...
    /// Primary spellcasting ability (e.g., Int, Wis, Cha)
    spellcasting_ability: Option<Stat>,
}
```

**Update accessor** (line 70):
```rust
// BEFORE
pub fn spellcasting_ability(&self) -> Option<&str> {
    self.spellcasting_ability.as_deref()
}

// AFTER
pub fn spellcasting_ability(&self) -> Option<Stat> {
    self.spellcasting_ability
}
```

**Update builder** (line 83):
```rust
// BEFORE
pub fn with_spellcasting_ability(mut self, ability: impl Into<String>) -> Self {
    self.spellcasting_ability = Some(ability.into());
    self
}

// AFTER
pub fn with_spellcasting_ability(mut self, ability: Stat) -> Self {
    self.spellcasting_ability = Some(ability);
    self
}
```

**Note**: Check callers of `with_spellcasting_ability()` - they will need to pass `Stat::Int`, `Stat::Wis`, etc. instead of string literals.

---

## Part 3: Documentation Updates

### 3.1 Add Fail-Fast Philosophy to AGENTS.md

**File**: `AGENTS.md`
**Location**: After "Error Handling" section (around line 634)

**Add new section**:
```markdown
### Fail-Fast Error Philosophy

WrldBldr uses fail-fast error handling where errors bubble up to the appropriate user:

| Error Type | Target | How |
|------------|--------|-----|
| Player action error | Player | WebSocket `Error` message |
| DM action error | DM | WebSocket `Error` message |
| System/infrastructure error | Both + logs | Generic message to user, full context to logs |

**DO**:
- Propagate errors with `?` operator
- Log context before converting to user-friendly errors
- Include entity IDs and operation names in error context

**DON'T**:
- Silently swallow errors with `if let Err(e) = ... { log }` returning `Ok`
- Use `.ok()` without logging what was lost
- Use `let _ =` on Results without documenting why

**When to use fallbacks (warn + default)**:
- Non-critical data enrichment (e.g., optional asset paths)
- Backward compatibility during migrations
- Always log a warning so issues are discoverable

**Pattern for fallback with logging**:
```rust
let value = match input.parse::<TargetType>() {
    Ok(v) => v,
    Err(e) => {
        tracing::warn!(
            input = %input,
            error = %e,
            "Failed to parse, using default"
        );
        TargetType::default()
    }
};
```
```

---

### 3.2 Update docs/architecture/review.md

**File**: `docs/architecture/review.md`
**Location**: Section "3. Error Handling" (around line 439)

**Update the section to include**:
```markdown
### 3. Error Handling

| Issue | How to Detect | Severity |
|-------|--------------|----------|
| Silent error swallowing | `if let Err(e) = ... { log }` then returns `Ok` | CRITICAL |
| Lost error context | `.map_err(|_| SomeError::Generic)` or `.ok()` | HIGH |
| Discarded Result | `let _ =` on a Result without `// INTENTIONAL` comment | MEDIUM |
| Silent unwrap | `.unwrap()` on Result without justification | HIGH |
| Missing `?` propagation | Manual match on Result when `?` would work | LOW |

**Fail-Fast Requirement**:
- Errors MUST bubble up to users (player or DM depending on who initiated)
- Use `?` operator for propagation
- Only use fallbacks for non-critical operations, and ALWAYS log a warning

**Anti-pattern** (WRONG):
```rust
if let Err(e) = some_operation().await {
    tracing::error!(error = %e, "Operation failed");
}
Ok(result)  // User thinks it succeeded!
```

**Correct pattern**:
```rust
some_operation().await?;  // Error propagates to user
Ok(result)
```
```

---

## Execution Order

| Order | Part | Files | Priority | Est. Changes |
|-------|------|-------|----------|--------------|
| 1 | Error propagation (1.1) | narrative_operations.rs | CRITICAL | ~15 lines |
| 2 | Error logging (1.2) | staging/approve.rs | HIGH | ~30 lines |
| 3 | Protocol enums (2.1) | shared/requests/npc.rs | HIGH | ~20 lines |
| 4 | Stat enum (2.3) | character_content.rs | HIGH | ~15 lines |
| 5 | Return value handling (1.3) | time/mod.rs + types | LOW | ~10 lines |
| 6 | Documentation (3.1, 3.2) | AGENTS.md, review.md | MEDIUM | ~60 lines |

---

## Verification

After each part, run:
```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace
```

Specific tests:
```bash
# Part 1: Error handling
cargo test -p wrldbldr-engine narrative
cargo test -p wrldbldr-engine staging
cargo test -p wrldbldr-engine time

# Part 2: Type safety
cargo test -p wrldbldr-shared
cargo test -p wrldbldr-domain character_content
```

---

## Summary

| Part | Issue | Files | Priority |
|------|-------|-------|----------|
| 1.1 | Silent error swallowing | narrative_operations.rs | CRITICAL |
| 1.2 | Lost error context `.ok()` | staging/approve.rs | HIGH |
| 2.1 | String enums in protocol | shared/requests/npc.rs | HIGH |
| 2.3 | String stat in domain | character_content.rs | HIGH |
| 1.3 | Discarded domain event info | time/mod.rs | LOW |
| 3 | Documentation (fail-fast) | AGENTS.md, review.md | MEDIUM |
| 4 | Compilation fixes (tech debt) | 15 files | CRITICAL |

**Total**: ~200 lines of code changes + ~60 lines of documentation + ~150 lines of tech debt cleanup

---

## Pre-Implementation Checklist

**Verified** (as of 2026-01-20):

- [x] `wrldbldr_domain::MoodState` - Exported via `pub use value_objects::{ ..., MoodState, ... }` (lib.rs:296)
- [x] `wrldbldr_domain::DispositionLevel` - Exported via `pub use value_objects::{ ..., DispositionLevel, ... }` (lib.rs:276)
- [x] `Stat` enum - Located at `crates/domain/src/value_objects/stat.rs`, already exported
- [x] `with_spellcasting_ability()` - Only defined in character_content.rs:83, no callers found (safe to change signature)

**Import paths for Part 2.1**:
```rust
// In shared/src/requests/npc.rs, add:
use wrldbldr_domain::{DispositionLevel, MoodState};
```

**Import paths for Part 2.3**:
```rust
// In domain/src/entities/character_content.rs, add:
use crate::value_objects::Stat;
```
