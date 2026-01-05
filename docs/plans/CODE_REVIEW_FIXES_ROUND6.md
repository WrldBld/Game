# Code Review Fixes - Round 6

## Overview

Comprehensive code review performed on 2026-01-05 using 6 parallel analysis agents covering:
1. Engine game systems (movement, conversation, challenge, approval, time)
2. Engine infrastructure (Neo4j repos, ports, external integrations)
3. Player App services and DTOs
4. Player UI components and state management
5. Protocol and Domain layers
6. System documentation vs implementation gaps

## Summary of Findings

| Category | Critical | High | Medium | Low | Total |
|----------|----------|------|--------|-----|-------|
| Engine Game Systems | 0 | 8 | 13 | 5 | 26 |
| Engine Infrastructure | 1 | 5 | 5 | 7 | 18 |
| Player App Services | 1 | 4 | 5 | 5 | 15 |
| Player UI | 2 | 4 | 6 | 6 | 18 |
| Protocol & Domain | 2 | 3 | 5 | 5 | 15 |
| **Total Bugs/Issues** | **6** | **24** | **34** | **28** | **92** |

Plus: 15+ unimplemented features documented in system specs.

---

## Phase 1: CRITICAL Bugs (Must Fix)

### CR6-1.1 - Race Condition in `modify_stat` (Player Character Repo)
**Severity**: CRITICAL
**File**: `crates/engine/src/infrastructure/neo4j/player_character_repo.rs:319-367`

**Issue**: Read-modify-write pattern without transaction isolation. Another request could modify the same stat between read and write, causing lost updates.

**Tasks**:
- [ ] Wrap read+write in Neo4j transaction
- [ ] Or use atomic Cypher JSON modification

---

### CR6-1.2 - Auto Mode Time Never Actually Advances
**Severity**: CRITICAL
**File**: `crates/engine/src/use_cases/time/mod.rs:128-143`

**Issue**: Despite being named "Auto-advance time immediately", the `SuggestTime::execute` method returns a result but **never persists** the time change. Callers just log the result.

**Tasks**:
- [ ] Either persist time in `SuggestTime::execute` for Auto mode
- [ ] Or ensure all callers persist the returned time change

---

### CR6-1.3 - Dice Roll Range Off By One
**Severity**: CRITICAL
**File**: `crates/engine/src/use_cases/challenge/mod.rs:132-134`

**Issue**: Assuming `gen_range` uses exclusive upper bound (Rust standard), D20 generates 1-19, D100 generates 1-99, missing the maximum values.

**Tasks**:
- [ ] Verify `RandomPort::gen_range` semantics
- [ ] Fix to `gen_range(1, 21)` and `gen_range(1, 101)` if exclusive

---

### CR6-1.4 - Data Loss in UpdateEventChainRequest Conversion
**Severity**: CRITICAL
**File**: `crates/player-app/src/application/services/event_chain_service.rs:121-128`

**Issue**: `From<&UpdateEventChainRequest>` only converts 2 of 7 fields. `events`, `act_id`, `tags`, `color`, `is_active` are silently discarded.

**Tasks**:
- [ ] Update `From` implementation to include all fields
- [ ] Update protocol `UpdateEventChainData` if missing fields

---

### CR6-1.5 - Typewriter Effect Race Condition
**Severity**: CRITICAL
**File**: `crates/player-ui/src/presentation/state/dialogue_state.rs:313-366`

**Issue**: If `apply_dialogue` is called while typewriter is running, old task continues briefly causing visual glitches.

**Tasks**:
- [ ] Add generation counter to typewriter effect
- [ ] Check version hasn't changed before continuing animation

---

### CR6-1.6 - Duplicate AdHocOutcomes Type Definition
**Severity**: CRITICAL
**Files**: 
- `crates/protocol/src/messages.rs:1157-1166`
- `crates/domain/src/value_objects/ad_hoc_outcomes.rs:12-23`

**Issue**: Identical type defined in both protocol and domain, creating maintenance burden and divergence risk.

**Tasks**:
- [ ] Protocol should import from domain OR
- [ ] Add clear From/Into conversions

---

## Phase 2: HIGH Priority Bugs

### CR6-2.1 - N+1 Query in save_pending_staging
**File**: `crates/engine/src/infrastructure/neo4j/staging_repo.rs:159-216`

**Issue**: NPCs added one at a time in loop. N NPCs = N database round trips.

**Tasks**:
- [ ] Use Cypher `UNWIND` to batch-insert all NPCs

---

### CR6-2.2 - Unbounded Query in get_triggers_for_region Fallback
**File**: `crates/engine/src/infrastructure/neo4j/narrative_repo.rs:377-408`

**Issue**: Fetches ALL active, non-triggered events in database (no world filter).

**Tasks**:
- [ ] Add `world_id` parameter to query
- [ ] Add `LIMIT` clause

---

### CR6-2.3 - Missing Pagination on List Queries
**Files**: Multiple repos (character_repo, location_repo, item_repo, lore_repo)

**Issue**: List methods can return unbounded result sets.

**Tasks**:
- [ ] Add optional `limit` and `offset` parameters to list methods

---

### CR6-2.4 - No Conversation State Persistence
**File**: `crates/engine/src/use_cases/conversation/start.rs:113`

**Issue**: `conversation_id` generated but never persisted. No way to:
- Track active conversations
- Prevent multiple simultaneous conversations
- Resume interrupted conversations

**Tasks**:
- [ ] Design conversation tracking mechanism
- [ ] Or document conversations are client-side only

---

### CR6-2.5 - NPC Can Be Unstaged Mid-Conversation
**File**: `crates/engine/src/use_cases/conversation/continue_conversation.rs:95-106`

**Issue**: Every continue message re-checks staging TTL. If staging expires mid-conversation, it's abruptly terminated.

**Tasks**:
- [ ] Consider "conversation lock" to prevent unstaging during active conversations

---

### CR6-2.6 - Target PC Not Passed to Challenge Resolution
**File**: `crates/engine/src/use_cases/challenge/mod.rs:259-264`

**Issue**: Public `execute` method passes `None` for `target_pc_id`, causing triggers like `GiveItem` to silently fail.

**Tasks**:
- [ ] Remove execute() without PC parameter, OR
- [ ] Return error if triggers require a PC

---

### CR6-2.7 - Non-Atomic Staging Operations
**File**: `crates/engine/src/use_cases/approval/mod.rs:68-71`

**Issue**: NPCs staged individually without transaction. Partial failures leave inconsistent state.

**Tasks**:
- [ ] Implement batch staging operation
- [ ] Or wrap in transaction

---

### CR6-2.8 - Toggle Operations Require Two Round-Trips (Race Condition)
**Files**: 
- `crates/player-app/src/application/services/narrative_event_service.rs:64-87`
- `crates/player-app/src/application/services/challenge_service.rs:125-141`

**Issue**: GET then SET creates race condition in multi-user scenarios.

**Tasks**:
- [ ] Add atomic `ToggleFavorite` request type, OR
- [ ] Accept desired state as parameter instead of toggling

---

### CR6-2.9 - Missing Loading State in PC Selection Flow
**File**: `crates/player-ui/src/routes/player_routes.rs:18-45`

**Issue**: Async PC check with no visible loading indicator.

**Tasks**:
- [ ] Add loading signal
- [ ] Show "Checking character..." state

---

### CR6-2.10 - Inventory Not Refreshing After Actions
**File**: `crates/player-ui/src/presentation/views/pc_view.rs:702-766`

**Issue**: Inventory panel doesn't re-fetch after equip/unequip/drop.

**Tasks**:
- [ ] Watch `inventory_refresh_counter` and re-fetch when changed

---

### CR6-2.11 - Missing GameTime Conversions Between Protocol and Domain
**Files**:
- `crates/protocol/src/types.rs:138-197`
- `crates/domain/src/game_time.rs:263-449`

**Issue**: Protocol and domain have completely different `GameTime` structures with no `From` implementation.

**Tasks**:
- [ ] Verify adapter layer has proper conversion
- [ ] Add `From` implementations if missing

---

### CR6-2.12 - Inconsistent ID Types in Protocol (String vs Uuid)
**File**: `crates/protocol/src/messages.rs` (multiple locations)

**Issue**: Many messages use `String` for IDs where `Uuid` would provide validation.

**Tasks**:
- [ ] Audit all ID fields
- [ ] Standardize on `Uuid` or document reasoning for String

---

## Phase 3: MEDIUM Priority Issues

### Infrastructure
- CR6-3.1: Extra round trip in `get_npc_mood` - use single query with COALESCE
- CR6-3.2: Dead code `load_staging_npcs` function - remove
- CR6-3.3: Missing connection pool configuration in Neo4j setup
- CR6-3.4: Queue cleanup missing - old items never deleted
- CR6-3.5: World delete not atomic - 14 sequential queries

### Player App
- CR6-3.6: Inconsistent service DI patterns (generic vs Arc<dyn>)
- CR6-3.7: Missing Deserialize on several request DTOs
- CR6-3.8: Hardcoded "GLOBAL" fallback in GenerationService
- CR6-3.9: CreateWantRequest.tells only supports single tell

### Player UI
- CR6-3.10: Missing ARIA labels on interactive elements
- CR6-3.11: No keyboard navigation for dialogue advancement
- CR6-3.12: Inconsistent error display patterns
- CR6-3.13: SuggestionButton doesn't clean up on unmount
- CR6-3.14: NPC mood cache uses arbitrary eviction (not true LRU)

### Protocol/Domain
- CR6-3.15: Inconsistent serde rename_all (camelCase vs snake_case)
- CR6-3.16: Missing skip_serializing_if for optional fields
- CR6-3.17: LoreCategoryData duplicates domain LoreCategory
- CR6-3.18: Missing validation in domain Character entity
- CR6-3.19: Item.properties stores JSON as String without validation

---

## Phase 4: LOW Priority Issues

### Engine
- CR6-4.1: Hardcoded "Travel to" prefix needs localization
- CR6-4.2: Disposition format uses Debug trait
- CR6-4.3: Hardcoded staging TTL (24 hours)
- CR6-4.4: Workflow name ignored in ComfyUI build_workflow
- CR6-4.5: Silent nil UUID fallback on parse error

### Player
- CR6-4.6: Unused Log button handler
- CR6-4.7: Magic numbers for typewriter animation delays
- CR6-4.8: Duplicate client-check boilerplate in PC view
- CR6-4.9: Missing focus management in modals
- CR6-4.10: No confirmation for destructive actions (Drop item)

### Protocol/Domain
- CR6-4.11: Missing forward compatibility for some domain enums
- CR6-4.12: Inconsistent default handling on boolean fields
- CR6-4.13: Empty vectors serialize as [] instead of being omitted
- CR6-4.14: Domain error type could have more specific variants

---

## Phase 5: Unimplemented Features (Gameplay Gaps)

### Critical for MVP

| Feature | System | Effort | Description |
|---------|--------|--------|-------------|
| Game time suggestion flow | Time | Medium | US-TIME-003 to 009 not implemented |
| Staging WebSocket handlers | Staging | Medium | DM can't approve staging in real-time |
| Lore REST API + LLM integration | Lore | Medium | No lore CRUD or discovery |
| Visual state resolution | Visual | Medium | No dynamic backdrops |

### Important for Beta

| Feature | System | Effort | Description |
|---------|--------|--------|-------------|
| Item transfer between characters | Inventory | Low | Players can't trade |
| Container system | Inventory | Medium | No bags/chests |
| Travel time between regions | Navigation | Low | No travel time suggestions |
| Region items in LLM context | Navigation | Low | NPCs can't reference items |

### Future Enhancements

| Feature | System | Effort | Description |
|---------|--------|--------|-------------|
| Combat system | Narrative | High | StartCombat effect exists but no combat |
| XP/Rewards system | Narrative | Medium | AddReward effect exists but no XP |
| Multi-slot NPC schedules | NPC | Medium | NPCs limited to one time slot |
| Prompt template settings UI | Templates | Low | DMs must edit via API |

---

## Recommended Fix Order

### Week 1: Critical Bugs
1. CR6-1.3 - Dice roll range (simple fix, affects all challenges)
2. CR6-1.4 - UpdateEventChainRequest data loss
3. CR6-1.2 - Auto mode time advancement
4. CR6-1.1 - Race condition in modify_stat

### Week 2: High Priority
5. CR6-2.1 - N+1 in staging
6. CR6-2.2 - Unbounded narrative query
7. CR6-2.6 - Target PC for challenge resolution
8. CR6-2.8 - Toggle race conditions

### Week 3: UI/UX Fixes
9. CR6-1.5 - Typewriter race condition
10. CR6-2.9 - PC selection loading state
11. CR6-2.10 - Inventory refresh
12. CR6-3.10/11 - Accessibility improvements

### Week 4+: Features
- Game time suggestion flow
- Staging WebSocket completion
- Lore system integration

---

## Progress Tracking

| Phase | Items | Done | Status |
|-------|-------|------|--------|
| Phase 1 (Critical) | 6 | 0 | PENDING |
| Phase 2 (High) | 12 | 0 | PENDING |
| Phase 3 (Medium) | 19 | 0 | PENDING |
| Phase 4 (Low) | 14 | 0 | PENDING |
| Phase 5 (Features) | 8+ | 0 | PENDING |
| **Total** | **59+** | **0** | **PENDING** |

---

## Commit History

| Commit | Phase | Description |
|--------|-------|-------------|
| - | - | - |
