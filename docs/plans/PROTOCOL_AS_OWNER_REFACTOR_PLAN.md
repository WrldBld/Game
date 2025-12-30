# Protocol-as-Owner Refactor Plan

## Overview

**Goal**: Eliminate duplicate DTO type definitions by making `protocol` the single source of truth for wire-format types.

**Types Being Consolidated**:
- `ProposedToolInfo`
- `ChallengeSuggestionInfo`
- `ChallengeSuggestionOutcomes`
- `NarrativeEventSuggestionInfo`

**Estimated Effort**: 2-3 hours

**Risk Level**: Low - types are structurally identical, only import paths change

---

## Current State Analysis

### Type Duplication Locations

| Crate | File | Lines | Derives |
|-------|------|-------|---------|
| **protocol** (canonical) | `src/types.rs` | 32-38, 78-120 | Debug, Clone, PartialEq, Serialize, Deserialize |
| engine-dto | `src/queue.rs` | 367-416 | Debug, Clone, Serialize, Deserialize |
| engine-ports | `src/outbound/dm_approval_queue_service_port.rs` | 94-142 | Debug, Clone, Serialize, Deserialize |
| player-ports | `src/inbound/player_events.rs` | 145-187 | Debug, Clone, PartialEq (NO serde) |

### Critical Constraint: Orphan Rule

After this refactor, the `From` impls in `engine-dto` will become orphan rule violations because:
- Domain types are from `wrldbldr_domain` (external)
- Protocol types will be from `wrldbldr_protocol` (external)
- `engine-dto` owns neither type

**Solution**: Convert `From` impls to standalone conversion functions (same pattern used in `engine-adapters/websocket/approval_converters.rs`).

### Dependency Graph (Verified Safe)

```
engine-dto    → protocol ✓ (exists)
engine-dto    → domain   ✓ (exists)
engine-dto    → engine-adapters ✗ (would create cycle - NOT ALLOWED)
engine-ports  → protocol ✓ (exists)
player-ports  → protocol ✓ (exists)
```

### Test Coverage

**No tests exist** for the `From` implementations being modified. The `engine-dto` crate has zero test coverage (documented technical debt in AGENTS.md).

---

## Detailed Implementation Plan

### Phase 1: Engine-DTO Refactor

**File**: `crates/engine-dto/src/queue.rs`

#### Step 1.1: Add Protocol Imports

Add after line 11:
```rust
// Re-export wire-format types from protocol (single source of truth)
pub use wrldbldr_protocol::{
    ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, NarrativeEventSuggestionInfo,
    ProposedToolInfo,
};
```

#### Step 1.2: Delete Duplicate Struct Definitions

Delete lines 366-416 (~51 lines):
```rust
// DELETE: Lines 366-416
/// Proposed tool call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedToolInfo { ... }

/// Challenge suggestion information for DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeSuggestionInfo { ... }

/// Challenge suggestion outcomes
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChallengeSuggestionOutcomes { ... }

/// Narrative event suggestion information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEventSuggestionInfo { ... }
```

#### Step 1.3: Convert From Impls to Standalone Functions

Delete the `From` impl blocks at lines 897-1014 and replace with standalone functions.

**Delete these impl blocks**:
- `impl From<ProposedTool> for ProposedToolInfo` (lines 897-906)
- `impl From<ProposedToolInfo> for ProposedTool` (lines 908-917)
- `impl From<ChallengeSuggestion> for ChallengeSuggestionInfo` (lines 923-936)
- `impl From<ChallengeSuggestionInfo> for ChallengeSuggestion` (lines 938-954)
- `impl From<DomainChallengeSuggestionOutcomes> for ChallengeSuggestionOutcomes` (lines 960-969)
- `impl From<ChallengeSuggestionOutcomes> for DomainChallengeSuggestionOutcomes` (lines 971-980)
- `impl From<NarrativeEventSuggestion> for NarrativeEventSuggestionInfo` (lines 986-998)
- `impl From<NarrativeEventSuggestionInfo> for NarrativeEventSuggestion` (lines 1001-1014)

**Add these standalone functions** (in the same location):

```rust
// ----------------------------------------------------------------------------
// Standalone conversion functions (required due to orphan rule)
// These convert between domain types and protocol types
// ----------------------------------------------------------------------------

/// Convert domain ProposedTool to protocol ProposedToolInfo
pub fn proposed_tool_to_info(tool: ProposedTool) -> ProposedToolInfo {
    ProposedToolInfo {
        id: tool.id,
        name: tool.name,
        description: tool.description,
        arguments: tool.arguments,
    }
}

/// Convert protocol ProposedToolInfo to domain ProposedTool
pub fn info_to_proposed_tool(info: ProposedToolInfo) -> ProposedTool {
    ProposedTool {
        id: info.id,
        name: info.name,
        description: info.description,
        arguments: info.arguments,
    }
}

/// Convert domain ChallengeSuggestion to protocol ChallengeSuggestionInfo
pub fn challenge_suggestion_to_info(suggestion: ChallengeSuggestion) -> ChallengeSuggestionInfo {
    ChallengeSuggestionInfo {
        challenge_id: suggestion.challenge_id,
        challenge_name: suggestion.challenge_name,
        skill_name: suggestion.skill_name,
        difficulty_display: suggestion.difficulty_display,
        confidence: suggestion.confidence,
        reasoning: suggestion.reasoning,
        target_pc_id: suggestion.target_pc_id.map(|id| id.to_string()),
        outcomes: suggestion.outcomes.map(outcomes_to_info),
    }
}

/// Convert protocol ChallengeSuggestionInfo to domain ChallengeSuggestion
pub fn info_to_challenge_suggestion(info: ChallengeSuggestionInfo) -> ChallengeSuggestion {
    ChallengeSuggestion {
        challenge_id: info.challenge_id,
        challenge_name: info.challenge_name,
        skill_name: info.skill_name,
        difficulty_display: info.difficulty_display,
        confidence: info.confidence,
        reasoning: info.reasoning,
        target_pc_id: info
            .target_pc_id
            .and_then(|s| Uuid::parse_str(&s).ok())
            .map(PlayerCharacterId::from),
        outcomes: info.outcomes.map(info_to_outcomes),
    }
}

/// Convert domain ChallengeSuggestionOutcomes to protocol ChallengeSuggestionOutcomes
pub fn outcomes_to_info(outcomes: DomainChallengeSuggestionOutcomes) -> ChallengeSuggestionOutcomes {
    ChallengeSuggestionOutcomes {
        success: outcomes.success,
        failure: outcomes.failure,
        critical_success: outcomes.critical_success,
        critical_failure: outcomes.critical_failure,
    }
}

/// Convert protocol ChallengeSuggestionOutcomes to domain ChallengeSuggestionOutcomes
pub fn info_to_outcomes(info: ChallengeSuggestionOutcomes) -> DomainChallengeSuggestionOutcomes {
    DomainChallengeSuggestionOutcomes {
        success: info.success,
        failure: info.failure,
        critical_success: info.critical_success,
        critical_failure: info.critical_failure,
    }
}

/// Convert domain NarrativeEventSuggestion to protocol NarrativeEventSuggestionInfo
pub fn narrative_event_suggestion_to_info(
    suggestion: NarrativeEventSuggestion,
) -> NarrativeEventSuggestionInfo {
    NarrativeEventSuggestionInfo {
        event_id: suggestion.event_id,
        event_name: suggestion.event_name,
        description: suggestion.description,
        scene_direction: suggestion.scene_direction,
        confidence: suggestion.confidence,
        reasoning: suggestion.reasoning,
        matched_triggers: suggestion.matched_triggers,
        suggested_outcome: suggestion.suggested_outcome,
    }
}

/// Convert protocol NarrativeEventSuggestionInfo to domain NarrativeEventSuggestion
pub fn info_to_narrative_event_suggestion(
    info: NarrativeEventSuggestionInfo,
) -> NarrativeEventSuggestion {
    NarrativeEventSuggestion {
        event_id: info.event_id,
        event_name: info.event_name,
        description: info.description,
        scene_direction: info.scene_direction,
        confidence: info.confidence,
        reasoning: info.reasoning,
        matched_triggers: info.matched_triggers,
        suggested_outcome: info.suggested_outcome,
    }
}
```

#### Step 1.4: Update .into() Call Sites

Update these 10 locations to use the new conversion functions:

| Line | Current Code | New Code |
|------|--------------|----------|
| ~794 | `data.proposed_tools.into_iter().map(Into::into).collect()` | `data.proposed_tools.into_iter().map(proposed_tool_to_info).collect()` |
| ~796 | `data.challenge_suggestion.map(Into::into)` | `data.challenge_suggestion.map(challenge_suggestion_to_info)` |
| ~797 | `data.narrative_event_suggestion.map(Into::into)` | `data.narrative_event_suggestion.map(narrative_event_suggestion_to_info)` |
| ~822 | `dto.proposed_tools.into_iter().map(Into::into).collect()` | `dto.proposed_tools.into_iter().map(info_to_proposed_tool).collect()` |
| ~824 | `dto.challenge_suggestion.map(Into::into)` | `dto.challenge_suggestion.map(info_to_challenge_suggestion)` |
| ~825 | `dto.narrative_event_suggestion.map(Into::into)` | `dto.narrative_event_suggestion.map(info_to_narrative_event_suggestion)` |
| ~933 | `suggestion.outcomes.map(Into::into)` | `suggestion.outcomes.map(outcomes_to_info)` |
| ~951 | `dto.outcomes.map(Into::into)` | `dto.outcomes.map(info_to_outcomes)` |
| ~1036 | `data.outcome_triggers.into_iter().map(Into::into).collect()` | `data.outcome_triggers.into_iter().map(proposed_tool_to_info).collect()` |
| ~1064 | `dto.outcome_triggers.into_iter().map(Into::into).collect()` | `dto.outcome_triggers.into_iter().map(info_to_proposed_tool).collect()` |

**Note**: Line numbers are approximate and will shift after deletions.

---

### Phase 2: Engine-Ports Refactor

**File**: `crates/engine-ports/src/outbound/dm_approval_queue_service_port.rs`

#### Step 2.1: Add Protocol Import

Add near top of file (after existing imports):
```rust
use wrldbldr_protocol::{
    ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, NarrativeEventSuggestionInfo,
    ProposedToolInfo,
};
```

#### Step 2.2: Delete Duplicate Struct Definitions

Delete lines 94-142 (~49 lines) containing:
- `ProposedToolInfo` struct
- `ChallengeSuggestionInfo` struct
- `ChallengeSuggestionOutcomes` struct
- `NarrativeEventSuggestionInfo` struct

**File**: `crates/engine-ports/src/outbound/mod.rs`

#### Step 2.3: Update Exports

Change lines 417-421 from:
```rust
pub use dm_approval_queue_service_port::{
    ApprovalDecisionType, ApprovalQueueItem, ApprovalRequest, ApprovalUrgency,
    ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, DmApprovalDecision,
    DmApprovalQueueServicePort, NarrativeEventSuggestionInfo, ProposedToolInfo,
};
```

To:
```rust
pub use dm_approval_queue_service_port::{
    ApprovalDecisionType, ApprovalQueueItem, ApprovalRequest, ApprovalUrgency,
    DmApprovalDecision, DmApprovalQueueServicePort,
};
// Re-export protocol types for API compatibility
pub use wrldbldr_protocol::{
    ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, NarrativeEventSuggestionInfo,
    ProposedToolInfo,
};
```

---

### Phase 3: Player-Ports Refactor

**File**: `crates/player-ports/src/inbound/player_events.rs`

#### Step 3.1: Add Protocol Import

Add near top of file:
```rust
use wrldbldr_protocol::{
    ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, NarrativeEventSuggestionInfo,
    ProposedToolInfo,
};
```

#### Step 3.2: Delete Duplicate Struct Definitions

Delete lines 145-187 (~43 lines) containing the 4 struct definitions.

**Impact**: These types will gain `Serialize, Deserialize` derives (acceptable).

**File**: `crates/player-ports/src/inbound/mod.rs`

#### Step 3.3: Update Exports

Update the re-exports to source from protocol:
```rust
// Re-export protocol types
pub use wrldbldr_protocol::{
    ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, NarrativeEventSuggestionInfo,
    ProposedToolInfo,
};
```

---

### Phase 4: Player-Adapters Cleanup

**File**: `crates/player-adapters/src/infrastructure/message_translator.rs`

Since player-ports now re-exports directly from protocol, the translation functions become identity transforms and should be deleted.

#### Step 4.1: Delete Translation Functions

Delete:
- `translate_proposed_tool_info` (lines 925-931)
- `translate_challenge_suggestion_info` (lines 934-951)
- `translate_narrative_event_suggestion_info` (lines 954-966)

#### Step 4.2: Update Call Sites

In the `ApprovalRequired` match arm (~lines 267, 276-277):
```rust
// Before:
proposed_tools: proposed_tools.into_iter().map(translate_proposed_tool_info).collect(),
challenge_suggestion: challenge_suggestion.map(translate_challenge_suggestion_info),
narrative_event_suggestion: narrative_event_suggestion.map(translate_narrative_event_suggestion_info),

// After:
proposed_tools,  // Direct assignment - same type now
challenge_suggestion,
narrative_event_suggestion,
```

In the `ChallengeOutcomePending` match arm (~lines 423-427):
```rust
// Before:
outcome_triggers: outcome_triggers.into_iter().map(translate_proposed_tool_info).collect(),

// After:
outcome_triggers,  // Direct assignment - same type now
```

---

### Phase 5: Player-App Re-exports (Optional Cleanup)

**Files**:
- `crates/player-app/src/application/dto/player_events.rs`
- `crates/player-app/src/application/dto/mod.rs`

These files re-export types from player-ports. After the refactor, they will automatically get the protocol types via the re-export chain. No changes strictly required, but imports can be simplified if desired.

---

## Verification Checklist

### After Each Phase

```bash
cargo check -p <crate-being-modified>
```

### After All Phases

```bash
# Full workspace compilation
cargo check --workspace

# Run all tests
cargo test --workspace

# Architecture verification
cargo xtask arch-check

# Clippy lint check
cargo clippy --workspace
```

---

## Summary of Changes

| Crate | Files Modified | Lines Deleted | Lines Added |
|-------|---------------|---------------|-------------|
| engine-dto | 1 | ~170 | ~90 |
| engine-ports | 2 | ~49 | ~15 |
| player-ports | 2 | ~43 | ~10 |
| player-adapters | 1 | ~50 | ~-10 (simplification) |
| **Total** | **6** | **~310** | **~105** |

**Net reduction**: ~200 lines of duplicate code

---

## Rollback Plan

If issues are discovered:
1. All changes are in separate files per crate
2. Git revert individual commits if needed
3. The protocol types are already stable and unchanged

---

## Future Considerations

1. **Test Coverage**: Consider adding tests for the new conversion functions
2. **Documentation**: The conversion functions should have doc comments explaining the domain ↔ protocol mapping
3. **Consistency**: Similar patterns may exist elsewhere that could be consolidated using this approach

---

## Execution Order

1. **Phase 1**: engine-dto (most complex, sets the pattern)
2. **Phase 2**: engine-ports (depends on protocol, not engine-dto)
3. **Phase 3**: player-ports (independent of engine-side)
4. **Phase 4**: player-adapters (simplification pass)
5. **Phase 5**: player-app (optional cleanup)

Each phase can be committed separately for easier review and rollback.
