# Code Review Refactor Plan

> **Created**: 2026-01-18
> **Status**: COMPLETED
> **Orchestrator**: Claude Agent

This plan addresses all issues identified in the comprehensive code review. Work is tracked here with status updates as each task completes.

---

## Overview

| Priority | Tasks | Completed |
|----------|-------|-----------|
| HIGH | 3 | 3 |
| MEDIUM | 6 | 6 |
| LOW | 4 | 4 |
| **TOTAL** | **13** | **13** |

---

## HIGH Priority Tasks

### H1. Move Protocol Conversions Out of Use Cases
**Status**: CLOSED - NOT NEEDED
**Files**: `engine/src/use_cases/staging/types.rs`

**Problem**: Use case layer contains `to_protocol()` methods that convert to `wrldbldr_shared` types.

**Review Finding**: The `to_protocol()` helper methods are ALREADY architecturally correct:
1. They are **called** at the API boundary (handlers call them) - this is correct
2. The project has **documented this pattern as acceptable** (CLEAN_CODEBASE_REMEDIATION_PLAN.md:201)
3. Architecture checks in xtask explicitly **exempt** these methods
4. Moving method definitions would **break encapsulation** and add tech debt

**Decision**: No refactor needed. The pattern is intentional and correct.

**Review Notes**:
- [x] Review agent assessment - DOES NOT ADD VALUE, would add tech debt
- [x] Implementation complete - N/A (not needed)
- [x] Post-implementation review - N/A

**Progress Log**:
```
2026-01-18: Review agent determined this is NOT NEEDED
  - Methods are called at API boundary (correct)
  - Pattern documented as acceptable
  - xtask exempts to_protocol() methods
  - Moving would break encapsulation
```

---

### H2. Move Input Conversion to API Boundary
**Status**: CLOSED - NOT NEEDED
**Files**: 
- `engine/src/use_cases/session/directorial.rs:22`
- `engine/src/use_cases/session/join_world_flow.rs:59`
- `engine/src/use_cases/management/player_character.rs:61,94`

**Problem**: Use cases accept `wrldbldr_shared` types as inputs.

**Review Finding**: The architecture is ALREADY CORRECT:
1. **DirectorialContext**: Uses `ports::DirectorialContext` (domain type) internally. The `from_protocol()` helper converts at API boundary.
2. **WorldRole**: Has `From` trait implementations between wire and domain types. Conversion happens in handlers.
3. **CharacterSheetValues**: Is NOT a wire type - it's a DOMAIN type (`domain/src/types/character_sheet.rs`) that shared re-exports via `pub use wrldbldr_domain::types::*`.

**Decision**: No refactor needed. The pattern follows AGENTS.md "shared vocabulary types" architecture.

**Review Notes**:
- [x] Review agent assessment - DOES NOT ADD VALUE, architecture already correct
- [x] Implementation complete - N/A (not needed)
- [x] Post-implementation review - N/A

**Progress Log**:
```
2026-01-18: Review agent determined this is NOT NEEDED
  - DirectorialContext: domain type exists, from_protocol() is correct pattern
  - WorldRole: From traits handle conversion at API boundary
  - CharacterSheetValues: is a domain type re-exported by shared, not wire format
```

---

### H3. Rename CRUD Use Cases to Manage Pattern
**Status**: COMPLETED
**Files**:
- `engine/src/use_cases/management/character.rs` - `CharacterCrud` -> `CharacterManagement`
- `engine/src/use_cases/management/player_character.rs` - `PlayerCharacterCrud` -> `PlayerCharacterManagement`
- `engine/src/use_cases/management/location.rs` - `LocationCrud` -> `LocationManagement`
- `engine/src/use_cases/management/world.rs` - `WorldCrud` -> `WorldManagement`
- `engine/src/use_cases/management/scene.rs` - `SceneCrud` -> `SceneManagement`
- `engine/src/use_cases/management/act.rs` - `ActCrud` -> `ActManagement`
- All handlers that reference these

**Problem**: `*Crud` suffix doesn't follow `{Verb}{Noun}` naming convention. This came up in multiple reviews.

**Solution**:
1. Rename all `*Crud` structs to `*Management`
2. Update all imports and usages throughout codebase
3. Update `App` struct field names
4. Ensure tests still pass

**Review Notes**:
- [x] Review agent assessment - ADDS VALUE (consistency, follows existing patterns)
- [x] Implementation complete - All 10 structs renamed, doc comments updated
- [x] Post-implementation review - Verified, only `ChallengeCrudError` alias remains (intentional)

**Progress Log**:
```
2026-01-18: Review agent confirmed value - improves naming consistency
2026-01-18: Implementation agent renamed 10 structs across 14 files
2026-01-18: Verification agent confirmed - cargo check/clippy pass, all tests pass
```

---

## MEDIUM Priority Tasks

### M1. Add Domain Events to Location Aggregate
**Status**: CLOSED - NOT NEEDED
**Files**: `domain/src/aggregates/location.rs:272-315`

**Problem**: Location setters return `()` while Character aggregate returns domain events.

**Review Finding**: Location is CORRECT; Character has unused tech debt:
1. **AGENTS.md explicitly says**: "Pure setters have exactly one outcome - the value is now set - so events would be ceremony without value"
2. **Character's `CharacterUpdate` events are NEVER USED** - All callers ignore the return value
3. **Location setters are pure setters** - No multiple outcomes, no guard clauses
4. The fix would be to REMOVE Character's unused events, not add more to Location

**Decision**: Location's `()` returns are correct per AGENTS.md. No change needed.

**Future consideration**: Remove unused `CharacterUpdate` enum from Character aggregate (~50 lines of dead code).

**Review Notes**:
- [x] Review agent assessment - DOES NOT ADD VALUE, would add ceremony without value
- [x] Implementation complete - N/A (not needed)
- [x] Post-implementation review - N/A

**Progress Log**:
```
2026-01-18: Review agent determined this is NOT NEEDED
  - Location setters are pure setters (one outcome)
  - AGENTS.md says pure setters should return ()
  - Character's events are unused tech debt, not a pattern to copy
```

---

### M2. Add Domain Events to World Aggregate
**Status**: CLOSED - NOT NEEDED
**Files**: `domain/src/aggregates/world.rs:236-245`

**Problem**: World time mutations (`set_time_mode`, `set_time_costs`) return `()`.

**Review Finding**: These are pure setters with single outcomes:
- Caller provides the value they want to set
- No business logic branching (no "AlreadySet", "InvalidTransition", etc.)
- Use case caller builds their own result for broadcasting

**Decision**: Per AGENTS.md, pure setters should return `()`. No change needed.

**Review Notes**:
- [x] Review agent assessment - PURE SETTER, no events needed
- [x] Implementation complete - N/A
- [x] Post-implementation review - N/A

**Progress Log**:
```
2026-01-18: Review agent confirmed pure setter - no events needed
```

---

### M3. Add Domain Events to PlayerCharacter Location Changes
**Status**: CLOSED - NOT NEEDED
**Files**: `domain/src/aggregates/player_character.rs:414-436`

**Problem**: `update_location` returns `()` while state changes return `PlayerCharacterStateChange` events.

**Review Finding**: `update_location` is a pure setter:
- Single outcome: location updated, region cleared
- Caller explicitly requests this specific change
- No conditional logic (no "AlreadyAtLocation", "LocationUnreachable")
- State changes (`kill`, `deactivate`) have multiple outcomes - those correctly return events

**Decision**: Per AGENTS.md, pure setters should return `()`. The inconsistency is intentional - state transitions have multiple outcomes, location updates don't.

**Review Notes**:
- [x] Review agent assessment - PURE SETTER, no events needed
- [x] Implementation complete - N/A
- [x] Post-implementation review - N/A

**Progress Log**:
```
2026-01-18: Review agent confirmed pure setter - no events needed
```

---

### M4. Define CharacterSheetValues in Domain
**Status**: CLOSED - ALREADY DONE
**Files**: 
- `domain/src/types/character_sheet.rs` (canonical definition)
- `shared/src/lib.rs` (re-export)

**Problem**: `CharacterSheetValues` appears to be in shared but represents core domain data.

**Review Finding**: This was already correctly implemented:
1. **Canonical definition is in domain**: `domain/src/types/character_sheet.rs:47-51`
2. **Shared just re-exports it**: `shared/src/lib.rs:112` - `pub use wrldbldr_domain::types::{CharacterSheetValues, SheetValue};`
3. This is the correct "shared vocabulary type" pattern per AGENTS.md

**Decision**: Already correctly implemented. Documented in ADR-011.

**Review Notes**:
- [x] Review agent assessment - Already in domain, shared re-exports (H2 analysis)
- [x] Implementation complete - N/A (already correct)
- [x] Post-implementation review - N/A

**Progress Log**:
```
2026-01-18: Already addressed in H2 review - CharacterSheetValues is a domain type
```

---

### M5. Split ws_core.rs
**Status**: COMPLETED
**Files**: `engine/src/api/websocket/ws_core.rs` (deleted - split into 4 new modules)

**Problem**: Module contained 5 unrelated domain handlers (1,547 lines) masquerading as "core".

**Solution Implemented**:
1. Created `ws_world.rs` - World CRUD handlers (~250 lines)
2. Created `ws_character.rs` - Character CRUD handlers (~265 lines)
3. Created `ws_npc.rs` - NPC disposition/mood/relationship handlers (~540 lines)
4. Created `ws_items.rs` - Item placement handlers (~90 lines)
5. Merged time handlers into existing `ws_time.rs` (~400 lines added)
6. Deleted `ws_core.rs`
7. Updated `mod.rs` dispatcher to route to new modules

**Review Notes**:
- [x] Review agent assessment - SPLIT recommended for consistency
- [x] Implementation complete - 4 new modules, 1 deleted, 1 merged
- [x] Post-implementation review - cargo check/clippy pass, all tests pass

**Progress Log**:
```
2026-01-18: Review agent recommended split - 5 domain handlers bundled arbitrarily
2026-01-18: Implementation agent extracted to domain-specific modules
2026-01-18: Verification agent confirmed - all checks pass
```

---

### M6. Rename Narrative Operations
**Status**: COMPLETED
**Files**: `engine/src/use_cases/narrative_operations.rs` + 14 consumer files

**Problem**: `Narrative` alias created confusion - the struct was already correctly named `NarrativeOps`.

**Review Finding**: The `{Verb}{Noun}` pattern doesn't apply - this is a multi-method orchestrator, not a single-action use case. The `{Domain}Ops` pattern (`NarrativeOps`, `EventChainOps`) is correct.

**Solution Implemented**:
1. Removed confusing `Narrative` type alias
2. Updated export in `mod.rs` to `NarrativeOps`
3. Updated 14 consumer files to use `NarrativeOps`

**Review Notes**:
- [x] Review agent assessment - `NarrativeOps` already correct, remove alias
- [x] Implementation complete - 14 files updated
- [x] Post-implementation review - clippy/tests pass

**Progress Log**:
```
2026-01-18: Review agent found struct already correctly named NarrativeOps
2026-01-18: Implementation agent removed alias, updated 14 consumers
2026-01-18: Verification - cargo clippy/test pass
```

---

## LOW Priority Tasks

### L1. Use Newtypes for Remaining Raw Strings
**Status**: CLOSED - NOT NEEDED
**Files**: Various

**Problem**: Raw strings where validated newtypes might be safer.

**Review Finding**: These are free-form text fields with NO invariants:
- `StagedNpc.name` - Denormalized display copy, not validated input
- `backdrop_override` - Free-form asset path
- `directorial_notes` - Free-form DM notes
- `description`, `scene_direction` - Free-form narrative for LLM

Per ADR-008 (Tiered Encapsulation): Fields without invariants don't need encapsulation. Adding newtypes would be **over-encapsulation anti-pattern**.

**Review Notes**:
- [x] Review agent assessment - SKIP, would add ceremony without value

**Progress Log**:
```
2026-01-18: Review agent determined NOT NEEDED - no invariants to protect
```

---

### L2. Scene Deprecated Field Migration
**Status**: COMPLETED
**Files**: `domain/src/aggregates/scene.rs`, `scene_events.rs`, `scene_repo.rs`, `ports.rs`, + 5 caller files

**Problem**: `location_id` and `featured_characters` were embedded in Scene aggregate, duplicating graph edges.

**Solution Implemented** (Pure Edge Approach):
1. Added `get_location()` and `set_location()` to SceneRepo port
2. Removed `location_id` and `featured_characters` fields from Scene aggregate
3. Removed related accessors, mutations, builders, and event variants
4. Updated all callers to use repository methods instead of aggregate accessors
5. Scene now stores relationships purely as Neo4j edges

**Files Modified**:
- `domain/src/aggregates/scene.rs` - Removed fields, accessors, mutations
- `domain/src/events/scene_events.rs` - Removed 5 event variants
- `engine/src/infrastructure/ports.rs` - Added 2 methods
- `engine/src/infrastructure/neo4j/scene_repo.rs` - Implemented edge queries
- `engine/src/use_cases/management/scene.rs` - Use repo for location
- `engine/src/use_cases/session/join_world.rs` - Use repo for location/characters
- `engine/src/api/websocket/ws_scene.rs` - Async fetch from repo
- `engine/src/api/websocket/ws_movement.rs` - Use repo for location

**Review Notes**:
- [x] Review agent assessment - Straightforward, infrastructure 80% existed
- [x] Implementation complete - Pure edge approach
- [x] Post-implementation review - All tests pass

**Progress Log**:
```
2026-01-18: Review agent analyzed scope - 6 production uses to update
2026-01-18: Implementation agent removed fields, updated 8 files
2026-01-18: Verification - cargo test passes
```

---

### L3. Standardize from_parts vs with_* Pattern
**Status**: CLOSED - ALREADY CONSISTENT
**Files**: Various domain entities

**Problem**: Some entities use `from_parts()` for reconstruction, others use `with_*` chaining.

**Review Finding**: The codebase is already consistent:
- All 6 aggregates use `with_id()` for database reconstitution
- No `from_parts` pattern found in aggregates
- This task appears to be based on outdated information

**Review Notes**:
- [x] Review agent assessment - SKIP, already consistent

**Progress Log**:
```
2026-01-18: Review agent found pattern already standardized on with_id()
```

---

### L4. Add Module Documentation
**Status**: COMPLETED
**Files**: `player/src/ui/presentation/components/` - 5 files updated

**Problem**: Some components lack module-level documentation.

**Solution Implemented**:
- Added module docs to 5 files that were missing them
- Most files (15+) already had good documentation with user story references
- Only `mod.rs` files and a few key components needed updates

**Files Updated**:
- `components/mod.rs` - Expanded organization description
- `components/common/mod.rs` - Added shared controls description
- `components/common/form_field.rs` - Added component description
- `components/pc/mod.rs` - Expanded character panel description
- `components/tactical/mod.rs` - Expanded dice/challenge description

**Review Notes**:
- [x] Review agent assessment - Most already documented
- [x] Implementation complete - 5 files updated
- [x] Post-implementation review - cargo check passes

**Progress Log**:
```
2026-01-18: Implementation agent added docs to 5 files (most already documented)
```

---

## Execution Log

### Session: 2026-01-18

| Time | Task | Action | Result |
|------|------|--------|--------|
| START | - | Created plan | - |
| 00:01 | H3 | Review agent assessment | ADDS VALUE |
| 00:02 | H3 | Implementation agent | 10 structs renamed |
| 00:03 | H3 | Verification agent | PASSED |
| 00:04 | H1 | Review agent assessment | NOT NEEDED - pattern is correct |
| 00:05 | H2 | Review agent assessment | NOT NEEDED - architecture already correct |
| 00:06 | - | Created ADR-011 | Documented protocol conversion patterns |
| 00:07 | - | Updated AGENTS.md | Added Protocol Conversion Patterns section |
| 00:08 | - | Updated review.md | Added NOT Violations table, Architecture Theater anti-pattern |
| 00:09 | M1 | Review agent assessment | NOT NEEDED - pure setters should return () |
| 00:10 | M2 | Review agent assessment | NOT NEEDED - pure setter |
| 00:11 | M3 | Review agent assessment | NOT NEEDED - pure setter |
| 00:12 | M4 | Review agent assessment | ALREADY DONE - CharacterSheetValues in domain |
| 00:13 | M5 | Review agent assessment | SPLIT recommended |
| 00:14 | M5 | Implementation agent | Split into 4 modules, merged time, deleted ws_core.rs |
| 00:15 | M5 | Verification agent | PASSED - all checks pass |
| 00:16 | M6 | Review agent assessment | Remove Narrative alias, keep NarrativeOps |
| 00:17 | M6 | Implementation agent | Removed alias, updated 14 consumers |
| 00:18 | L1 | Review agent assessment | NOT NEEDED - free-form text, no invariants |
| 00:19 | L2 | Review agent assessment | IMPLEMENT - graph edge migration |
| 00:20 | L2 | Implementation agent | Removed deprecated fields, updated 8 files |
| 00:21 | L3 | Review agent assessment | ALREADY CONSISTENT - with_id pattern used |
| 00:22 | L4 | Review agent assessment | Low value, defer to lint-based approach |
| 00:23 | L4 | Implementation agent | Added docs to 5 files |
| 00:24 | ALL | Final verification | All 13 tasks complete |

---

## Post-Refactor Verification

After all tasks complete:

```bash
# All tests pass
cargo test --workspace --lib

# Clippy clean
cargo clippy --workspace -- -D warnings

# E2E tests (if LLM code changed)
E2E_LLM_MODE=playback cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=1

# Re-record VCR cassettes if needed
E2E_LLM_MODE=record cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=1
```

---

## Notes

- No production data or releases - can change anything for correctness
- Authentication not a concern - client-generated user IDs are fine
- Big refactors are acceptable if they improve consistency
- serve-llm script running for VCR recording when needed
