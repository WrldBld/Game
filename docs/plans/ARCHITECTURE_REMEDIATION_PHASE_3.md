# Architecture Remediation Phase 3

> **Status**: MOSTLY COMPLETE (Phase 3 deferred)
> **Created**: 2026-01-02
> **Validated**: 2026-01-02
> **Type**: Code review findings remediation (verified issues)
> **Depends On**: ARCHITECTURE_REMEDIATION_PHASE_2.md (COMPLETED)
> **Approach**: BIG BANG REFACTOR - No backward compatibility needed, no releases, no data to migrate

## Executive Summary

This plan addresses verified issues from the comprehensive code review. All issues have been **validated against the codebase** with specific findings documented. This plan explicitly avoids duplicating or rolling back work from:
- `ARCHITECTURE_REMEDIATION_MASTER_PLAN.md` (13 phases completed)
- `ARCHITECTURE_REMEDIATION_PHASE_2.md` (completed + documented tech debt)

**Key Decision**: This is a big-bang refactor. We will DELETE dead code rather than deprecate, and make breaking changes without migration paths.

---

## Already Addressed (No Action Needed)

These items from the review are already handled:

| Issue | Status | Reference |
|-------|--------|-----------|
| StoryEventService "god trait" | **FALSE** - Only 30 methods, ISP split already completed | 5 focused ports exist in `engine-app/services/internal/` |
| MockGameConnectionPort in ports | **TECH DEBT** - Cannot be moved due to dependency direction | ARCHITECTURE_REMEDIATION_PHASE_2.md §1.3 |
| 39 `*_port.rs` naming | **COMPLETED** - Already renamed to `*_service.rs` | Commit `8f71811` |
| DmApprovalDecision consolidation | **COMPLETED** - Three-type model in place | ARCHITECTURE_REMEDIATION_MASTER_PLAN.md Phase 1B |
| ChallengeSuggestionInfo, etc. | **CORRECT** - Already re-exported from protocol | engine-dto/queue.rs:14-17 |

---

## Phase 1: Documentation Fixes (15 minutes)

### 1.1 Fix AGENTS.md Source of Truth Reference

**File**: `AGENTS.md` (lines 9-11)

**Current** (incorrect):
```markdown
The single source-of-truth refactor plan to reach that target is:
- `docs/plans/HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md`
```

**Should be**:
```markdown
The single source-of-truth refactor plan to reach that target is:
- `docs/plans/ARCHITECTURE_REMEDIATION_MASTER_PLAN.md`
```

**Rationale**: HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md is marked DEPRECATED.

**Validation**: Confirmed - the deprecated file says "Superseded by `ARCHITECTURE_REMEDIATION_MASTER_PLAN.md`"

---

## Phase 2: Type Duplication Consolidation (2-3 hours)

### 2.1 Problem (VALIDATED)

`crates/player-ports/src/outbound/player_events.rs` duplicates types from `crates/protocol/src/`.

### 2.2 Validated Analysis

**Types SAFE to consolidate (19 types)** - exact field matches:

| Type | Can Re-export | Notes |
|------|---------------|-------|
| SceneData | YES | Exact match |
| CharacterData | YES | Exact match |
| GameTime | YES | Exact match |
| InteractionData | YES | Exact match |
| DialogueChoice | YES | Exact match |
| RegionData | YES | Exact match |
| NpcPresenceData | YES | Exact match |
| NavigationData | YES | Exact match |
| NavigationTarget | YES | Exact match |
| NavigationExit | YES | Exact match |
| RegionItemData | YES | Exact match |
| SplitPartyLocation | YES | Exact match |
| OutcomeDetailData | YES | Exact match |
| OutcomeBranchData | YES | Exact match |
| StagedNpcInfo | YES | Exact match |
| PreviousStagingInfo | YES | Exact match |
| WaitingPcInfo | YES | Exact match |
| NpcPresentInfo | YES | Exact match |
| NpcDispositionData | YES | Exact match |
| GoalData | YES | Exact match |

**Types that MUST NOT be consolidated (9 types)** - intentionally different:

| Type | Reason |
|------|--------|
| WorldRole | player-ports: `String` wrapper; protocol: typed enum |
| JoinError | player-ports: simple struct; protocol: rich enum |
| ResponseResult | player-ports: flat struct; protocol: tagged enum |
| ConnectedUser | Different `role` field type |
| WantData | String fields vs typed enums |
| WantTargetData | String fields vs typed enums |
| ActantialViewData | String fields vs typed enums |
| EntityChangedData | String fields vs typed enums |
| PlayerEvent | Main event enum - must stay in player-ports |

**Special case - CharacterPosition**:
- Can consolidate but requires updating `CharacterPositionStyle` trait impl in player-ui to handle `Unknown` variant

### 2.3 Implementation Steps

1. Add re-exports for the 19 safe types:
   ```rust
   pub use wrldbldr_protocol::{
       SceneData, CharacterData, GameTime, InteractionData, DialogueChoice,
       RegionData, NpcPresenceData, NavigationData, NavigationTarget,
       NavigationExit, RegionItemData, SplitPartyLocation, OutcomeDetailData,
       OutcomeBranchData, StagedNpcInfo, PreviousStagingInfo, WaitingPcInfo,
       NpcPresentInfo, NpcDispositionData, GoalData, CharacterPosition,
   };
   ```

2. Delete the 19 duplicate struct/enum definitions from player_events.rs

3. Update `player-ui/src/presentation/utils/position_styles.rs`:
   ```rust
   impl CharacterPositionStyle for wrldbldr_protocol::CharacterPosition {
       fn as_tailwind_classes(&self) -> &'static str {
           match self {
               Self::Left => "left-[10%]",
               Self::Center => "left-1/2 -translate-x-1/2",
               Self::Right => "right-[10%]",
               Self::OffScreen | Self::Unknown => "hidden",
           }
       }
   }
   ```

4. Simplify `message_translator.rs` - remove translation functions for consolidated types

### 2.4 Types to Document (Keep Separate by Design)

Add comment to player_events.rs explaining why 9 types are intentionally different:

```rust
// NOTE: The following types are intentionally different from their protocol
// equivalents. Protocol types use typed enums for wire format; these use
// String representations for UI binding simplicity:
// - WorldRole, JoinError, ResponseResult, ConnectedUser
// - WantData, WantTargetData, ActantialViewData, EntityChangedData
```

---

## Phase 3: engine-dto Imports in engine-ports (1-2 hours)

### 3.1 Problem (VALIDATED)

4 files in `engine-ports` import from `engine-dto`:

| File | Types | Action |
|------|-------|--------|
| `outbound/llm_port.rs` | 9 LLM types | Move to engine-ports |
| `outbound/llm_suggestion_queue_port.rs` | SuggestionContext | Keep in engine-dto (has behavior) |
| `inbound/queue_use_case_port.rs` | 5 types | 3 already from protocol, 2 need review |
| `inbound/request_handler.rs` | RequestContext | Move to engine-ports |

### 3.2 Validated Type Analysis

**Types to MOVE to engine-ports (10 types)**:
- LLM types: `ChatMessage`, `FinishReason`, `ImageData`, `LlmRequest`, `LlmResponse`, `MessageRole`, `TokenUsage`, `ToolCall`, `ToolDefinition`
- Request types: `RequestContext`

**Types to KEEP in engine-dto**:
- `SuggestionContext` - has `Default` impl and methods (behavior allowed in engine-dto)

**Types ALREADY correctly handled (via re-export chain)**:
- `ChallengeSuggestionInfo` - protocol → engine-dto → engine-ports ✓
- `NarrativeEventSuggestionInfo` - protocol → engine-dto → engine-ports ✓
- `ProposedToolInfo` - protocol → engine-dto → engine-ports ✓
- `DmApprovalDecision` - protocol → engine-dto → engine-ports (three-type model) ✓

### 3.3 Implementation

1. Create `crates/engine-ports/src/outbound/llm_types.rs`:
   - Move 9 LLM-related types from engine-dto/llm.rs

2. Create `crates/engine-ports/src/inbound/request_types.rs`:
   - Move RequestContext from engine-dto/request_context.rs

3. Update engine-dto to re-export FROM engine-ports:
   ```rust
   // engine-dto/llm.rs
   pub use wrldbldr_engine_ports::outbound::llm_types::*;
   ```

4. Remove `wrldbldr-engine-dto` dependency from engine-ports Cargo.toml

5. Add `wrldbldr-engine-ports` dependency to engine-dto Cargo.toml (dependency swap)

### 3.4 Circular Dependency Check

**Current**: engine-ports → engine-dto
**After**: engine-dto → engine-ports (swap, not cycle)

This is safe because:
- engine-dto needs ports for type re-exports
- engine-ports no longer needs engine-dto

---

## Phase 4: Naming Convention Fixes (1 hour)

### 4.1 RequestHandler Rename (VALIDATED)

**Scope**: 52 occurrences across 15 files

| Category | Count |
|----------|-------|
| Trait definition | 1 |
| Implementation | 1 |
| Arc<dyn RequestHandler> | 9 |
| Import statements | ~15 |
| Documentation/comments | ~27 |

**Action**: Rename `RequestHandler` → `RequestHandlerPort`

### 4.2 Manage*UseCase Traits (VALIDATED - DEAD CODE)

**Finding**: These 4 traits are **completely unused**:
- File marked with `#![allow(dead_code)]`
- Comment: "Currently unused - services will implement these traits in the future"
- No implementations exist
- Not exported from mod.rs

**Action**: DELETE the use_cases.rs file entirely (188 lines of dead code). Big-bang refactor - no backward compatibility needed.

### 4.3 ConnectionLifecyclePort Collision (VALIDATED - LOW PRIORITY)

**Finding**: Two different traits, same name, different crates:
- Engine: 1 method (`unregister_connection`) - 10 usages
- Player: 5 methods (lifecycle management) - 18 usages

**Recommendation**: Rename engine version to `ConnectionCleanupPort` (reflects actual purpose)

**Priority**: LOW - no actual conflicts since they're in different crate namespaces

---

## Phase 5: Primitive Obsession Fix (2-3 hours)

### 5.1 Validated Scope

**`ConnectionId` does NOT exist** - must be added to domain/ids.rs

**Structs using raw Uuid**:
- `ConnectionInfo`: 4 fields
- `WorldConnectionState`: 4 fields
- `WorldConnectionManager`: 3 fields
- `BroadcastError`: 3 variants
- `ConnectionManagerError`: 3 variants (in engine-ports)

**Port traits using mixed types**:
- `DmNotificationPort`: Already uses `WorldId` ✓
- `ConnectionQueryPort`: Partially uses typed IDs
- Others: Use raw `Uuid`

### 5.2 Implementation Steps

1. Add to `domain/ids.rs`:
   ```rust
   define_id!(ConnectionId);
   ```

2. Update internal structs (adapters layer):
   - `ConnectionInfo`, `WorldConnectionState`, `WorldConnectionManager`

3. Update error enums (both adapters and ports):
   - `BroadcastError`, `ConnectionManagerError`

4. Update port trait signatures to use typed IDs consistently

5. Use `.as_uuid()` / `.from_uuid()` only at serialization boundaries

### 5.3 Breaking Change Mitigation

Consider updating ports first, then adapters, to maintain compile-time checking.

---

## Phase 6: Business Logic in Adapter (45 minutes)

### 6.1 Validated Finding

`item_repository.rs:226-254` contains quantity business logic.

Same pattern exists correctly in `inventory.rs:222-234` (application layer) - domain logic should be shared.

### 6.2 Implementation

Create `crates/domain/src/value_objects/quantity.rs`:

```rust
/// Result of a quantity subtraction operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuantityChangeResult {
    Updated(u32),
    Depleted,
}

impl QuantityChangeResult {
    pub fn subtract(current: u32, amount: u32) -> Self {
        if amount >= current {
            Self::Depleted
        } else {
            Self::Updated(current - amount)
        }
    }

    pub fn should_remove(&self) -> bool {
        matches!(self, Self::Depleted)
    }

    pub fn new_quantity(&self) -> Option<u32> {
        match self {
            Self::Updated(qty) => Some(*qty),
            Self::Depleted => None,
        }
    }
}
```

Update both `item_repository.rs` AND `inventory.rs` to use this value object.

---

## Phase 7: anyhow::Result Migration - DEFERRED

### 7.1 Validated Scope

~229 async methods use `anyhow::Result` in engine-ports/outbound/

### 7.2 Recommendation: DEFER

This is correctly deferred. The effort is high and requires:
1. Error handling patterns RFC
2. Team agreement on error enum design
3. Incremental migration by domain area

---

## Phase 8: Missing #[serde(other)] Variants (30 minutes)

### 8.1 Validated Finding

**Missing in engine-dto/persistence.rs**:
- `SectionLayoutDto` (line 571)
- `ItemListTypeDto` (line 630)

**Already correct**: 10 other enums in engine-dto have `#[serde(other)]`
**Protocol crate**: 100% coverage (19 enums have `#[serde(other)]`)

### 8.2 Risk Assessment: LOW

These are internal persistence DTOs, not wire format. Risk is database migration compatibility, not runtime crashes.

### 8.3 Implementation

Add Unknown variants and update From impls to map Unknown → sensible defaults:
- `SectionLayoutDto::Unknown` → `SectionLayout::Vertical`
- `ItemListTypeDto::Unknown` → `ItemListType::Inventory`

---

## Phase 9: Dead Code Cleanup (1 hour) - NEW

### 9.1 Validated Findings

**Dead ISP port containers** in `engine-runner/src/composition/factories/repositories.rs`:
- 9 structs marked `#[allow(dead_code)]`: `CharacterPorts`, `LocationPorts`, `RegionPorts`, `ItemPorts`, `ScenePorts`, `StoryEventPorts`, `ChallengePorts`, `StagingPorts`, `PcPorts`
- These appear to be ISP refactoring artifacts that were never integrated

**Dead use_cases.rs** in engine-ports/inbound:
- 188 lines of planned but never implemented traits

### 9.2 Implementation

1. DELETE dead ISP port containers from repositories.rs
2. DELETE use_cases.rs entirely (dead code)

Big-bang refactor - no backward compatibility needed, delete all dead code.

---

## Phase 10: TODO/Technical Debt Tracking (30 minutes) - NEW

### 10.1 High Priority TODOs Found

| Location | Issue | Action |
|----------|-------|--------|
| `challenge_outcome_approval_service.rs:627` | Queue item ID → resolution_id mapping missing | Create tracking issue |
| `challenge_outcome_approval_service.rs:790` | Branch storage in approval items | Create tracking issue |
| `scene_builder.rs:285,294` | Region item system incomplete | Document in roadmap |

### 10.2 Implementation

Create `docs/progress/TECH_DEBT_TRACKING.md` to consolidate:
- TODO items from code review
- Phase 0.D / Phase 0.H references (found in interaction_repository.rs)
- Backward compatibility notes (71 NOTE comments)

---

## Implementation Order (Updated)

| Phase | Effort | Priority | Dependencies |
|-------|--------|----------|--------------|
| Phase 1 | 15 min | HIGH | None |
| Phase 2 | 2-3 hrs | HIGH | None |
| Phase 3 | 1-2 hrs | HIGH | None |
| Phase 4 | 1 hr | MEDIUM | None |
| Phase 5 | 2-3 hrs | MEDIUM | None |
| Phase 6 | 45 min | MEDIUM | None |
| Phase 7 | - | DEFERRED | - |
| Phase 8 | 30 min | LOW | None |
| Phase 9 | 1 hr | MEDIUM | None |
| Phase 10 | 30 min | LOW | None |

**Sprint Plan**:
- Sprint 1: Phases 1, 2, 3, 8, 10 (HIGH priority + quick wins + tracking)
- Sprint 2: Phases 4, 5, 6, 9 (MEDIUM priority)
- Backlog: Phase 7 (DEFERRED)

---

## Verification

After each phase:

```bash
cargo xtask arch-check      # Must pass
cargo check --workspace     # Must compile
cargo test --workspace      # Must pass
```

---

## Success Criteria

- [x] AGENTS.md references correct master plan (Phase 1 - DONE)
- [x] 20 player-ports types re-exported from protocol (9 documented as intentionally different) (Phase 2 - DONE)
- [ ] engine-ports does not import from engine-dto (Phase 3 - DEFERRED, dependency swap requires separate PR)
- [x] RequestHandler renamed to RequestHandlerPort (Phase 4.1 - DONE)
- [x] Dead use_cases.rs removed (Phase 4.2 - DONE, 188 lines deleted)
- [x] ConnectionId added to domain, used in WorldConnectionManager (Phase 5 - DONE)
- [x] QuantityChangeResult value object in domain (Phase 6 - DONE)
- [x] persistence.rs enums have #[serde(other)] variants (Phase 8 - DONE)
- [x] ISP port containers verified as used (Phase 9 - no action needed, not dead code)
- [ ] Tech debt tracking document created (Phase 10 - deferred)
- [x] `cargo xtask arch-check` passes (verified)

---

## Appendix A: Items NOT Addressed (Out of Scope)

| Issue | Reason |
|-------|--------|
| Protocol imports in player-app services | Architectural decision needed - services are at boundary |
| Anemic domain entities | Requires domain modeling discussion |
| StoryEventService god trait | Already split - review was incorrect |
| MockGameConnectionPort duplication | Documented tech debt - dependency constraint |
| Clone optimization in broadcast paths | Premature optimization without profiling |

---

## Appendix B: Validation Summary

| Original Claim | Validation Result |
|----------------|-------------------|
| 14+ type duplications in player-ports | **CONFIRMED** - 19 can consolidate, 9 intentionally different |
| engine-dto imports in engine-ports | **CONFIRMED** - 10 types to move, rest correctly handled |
| 5 naming violations | **CONFIRMED** - 1 real (RequestHandler), 4 dead code |
| Primitive obsession in WorldConnectionManager | **CONFIRMED** - ConnectionId missing, ~50+ Uuid usages |
| Business logic in item_repository | **CONFIRMED** - same pattern exists correctly in inventory.rs |
| Missing #[serde(other)] | **CONFIRMED** - 2 enums missing, 10 others correct |
| ~100+ anyhow::Result methods | **CONFIRMED** - actually ~229 methods |
| ConnectionLifecyclePort collision | **CONFIRMED** - two different traits, same name |
| Dead code in use_cases.rs | **CONFIRMED** - 188 lines unused, marked dead_code |
| Dead ISP port containers | **NEW FINDING** - 9 structs in repositories.rs |
| High-priority TODOs | **NEW FINDING** - 3 critical TODOs need tracking |
