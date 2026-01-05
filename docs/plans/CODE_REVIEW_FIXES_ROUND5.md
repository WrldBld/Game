# Code Review Fixes - Round 5

## Overview

This plan addresses issues found in the comprehensive code review of January 2026.
Organized by priority with estimated complexity. Excludes authentication-related issues.

**Total Items**: 45
**Estimated Phases**: 12

---

## Phase 1: Critical - Remove APOC Dependencies (Complexity: High)

APOC functions may not be available in all Neo4j installations, causing silent failures.

### CR5-1.1 - Remove APOC from player_character_repo.rs
**Status**: COMPLETE
**File**: `crates/engine/src/infrastructure/neo4j/player_character_repo.rs`
**Lines**: 320-356

**Issue**: `modify_stat` uses `apoc.convert.fromJsonMap`, `apoc.convert.toJson`, `apoc.map.setEntry`

**Tasks**:
- [ ] Implement read-modify-write pattern in Rust
- [ ] Fetch current stats JSON, parse in Rust, modify, write back
- [ ] Remove APOC function calls

---

### CR5-1.2 - Remove APOC from staging_repo.rs
**Status**: COMPLETE
**File**: `crates/engine/src/infrastructure/neo4j/staging_repo.rs`
**Lines**: 177-218 (save_pending_staging)

**Issue**: Uses `apoc.convert.fromJsonList` for parsing NPCs

**Tasks**:
- [ ] Pass NPCs as native Neo4j list parameter instead of JSON string
- [ ] Use UNWIND with list parameter directly
- [ ] Remove APOC function calls

---

## Phase 2: Critical - Fix Dangerous World Deletion (Complexity: Medium)

### CR5-2.1 - Safe World Deletion with Explicit Node Types
**Status**: COMPLETE
**File**: `crates/engine/src/infrastructure/neo4j/world_repo.rs`
**Lines**: 155-171

**Issue**: `MATCH (w)-[*]->(related) DETACH DELETE` could delete unintended nodes

**Tasks**:
- [ ] Replace wildcard relationship with explicit node type matching
- [ ] Delete in order: Events -> Scenes -> Regions -> Locations -> Characters -> World
- [ ] Add batching for large worlds (LIMIT + loop)

---

## Phase 3: Critical - Fix Message Loss (Complexity: High)

### CR5-3.1 - Add Backpressure for Critical Messages
**Status**: COMPLETE
**Files**: 
- `crates/engine/src/api/connections.rs`
- `crates/engine/src/api/websocket.rs`

**Issue**: `try_send()` drops messages when channel full

**Tasks**:
- [x] Create `send_critical()` method that uses `send().await` with timeout
- [x] Identify critical message types (state changes, approvals, errors)
- [x] Use `send_critical()` for those, keep `try_send()` for non-critical
- [x] Add channel fullness logging/metrics

**Implementation Notes**:
- Added `CriticalSendError` enum with `ConnectionNotFound`, `ChannelClosed`, `Timeout` variants
- Added `send_critical()` for single connection with 5-second timeout
- Added `broadcast_critical_to_world()` for all connections in a world
- Added `broadcast_critical_to_dms()` for DM connections only
- Added `send_critical_to_pc()` for specific player character's connection
- All critical methods log errors but don't fail the caller (except `send_critical` which returns Result)

---

## Phase 4: High - Fix Incomplete TriggerContext (Complexity: High)

### CR5-4.1 - Populate TriggerContext Properly
**Status**: COMPLETE
**File**: `crates/engine/src/entities/narrative.rs`
**Lines**: 328-344

**Issue**: TriggerContext has empty/hardcoded values for flags, events, challenges, etc.

**Tasks**:
- [x] Add Flag entity dependency to Narrative
- [x] Fetch and populate `flags` from Flag entity
- [x] Add Scene entity dependency for `current_scene`
- [x] Accept `turn_count` as parameter from caller
- [x] Document which fields are caller's responsibility

**Implementation Notes**:
- Added `flag_repo: Arc<dyn FlagRepo>` and `scene_repo: Arc<dyn SceneRepo>` dependencies
- `flags` now populated from both world and PC-scoped flags via FlagRepo
- `current_scene` now fetched from SceneRepo using `get_current(world_id)`
- Session-specific fields (turn_count, event_outcomes, turns_since_event, recent_dialogue_topics, recent_player_action) documented as caller responsibility - these are transient session state not stored in DB

---

## Phase 5: High - Add Missing Entity Operations (Complexity: Medium)

### CR5-5.1 - Add World Game Time Operations
**Status**: PENDING
**File**: `crates/engine/src/entities/world.rs`

**Tasks**:
- [ ] Add `advance_time(id, minutes)` method
- [ ] Add `get_current_time(id)` method  
- [ ] Add `set_time(id, game_time)` method
- [ ] Add `pause_time(id)` / `resume_time(id)` methods

---

### CR5-5.2 - Fix Staging TTL Check
**Status**: PENDING
**File**: `crates/engine/src/entities/staging.rs`
**Lines**: 121-131

**Issue**: `resolve_for_region()` ignores TTL/expiry

**Tasks**:
- [ ] Add `current_game_time` parameter to `resolve_for_region()`
- [ ] Check staging expiry before returning NPCs
- [ ] Update all callers to pass game time

---

### CR5-5.3 - Fix Conversation TTL Check
**Status**: PENDING
**Files**: 
- `crates/engine/src/use_cases/conversation/start.rs`
- `crates/engine/src/use_cases/conversation/continue_conversation.rs`

**Issue**: Uses `staging.resolve_for_region()` without TTL check

**Tasks**:
- [ ] Add World entity dependency to conversation use cases
- [ ] Fetch current game time from world
- [ ] Pass game time to staging resolution

---

## Phase 6: High - Fix N+1 Queries (Complexity: Medium)

### CR5-6.1 - Fix get_active_staging N+1
**Status**: PENDING
**File**: `crates/engine/src/infrastructure/neo4j/staging_repo.rs`
**Lines**: 236-263

**Issue**: Fetches Staging then makes second query for NPCs

**Tasks**:
- [ ] Combine into single query using COLLECT (like get_pending_staging)
- [ ] Reuse `row_to_staging_with_npcs` helper

---

### CR5-6.2 - Fix get_triggers_for_region Inefficiency
**Status**: PENDING
**File**: `crates/engine/src/infrastructure/neo4j/narrative_repo.rs`
**Lines**: 349-383

**Issue**: Fetches ALL events then filters in Rust

**Tasks**:
- [ ] Add Cypher filtering for region-specific triggers
- [ ] Consider adding TRIGGERS_AT relationship for indexed lookup

---

## Phase 7: High - Fix Domain Model Issues (Complexity: Low)

### CR5-7.1 - Fix CampbellArchetype::from_str Default
**Status**: PENDING
**File**: `crates/domain/src/types/archetype.rs`
**Lines**: 129-143

**Issue**: Defaults to `Ally` instead of `Unknown` for unrecognized strings

**Tasks**:
- [ ] Change default case to return `Self::Unknown`

---

### CR5-7.2 - Add Unknown to SkillCategory
**Status**: PENDING
**File**: `crates/domain/src/entities/skill.rs`
**Lines**: 69-92

**Tasks**:
- [ ] Add `#[serde(other)] Unknown` variant to SkillCategory

---

### CR5-7.3 - Add Missing PlayerCharacter Fields
**Status**: PENDING
**File**: `crates/domain/src/entities/player_character.rs`

**Tasks**:
- [ ] Add `is_alive: bool` field (default true)
- [ ] Add `is_active: bool` field (default true)

---

### CR5-7.4 - Make NpcDispositionState Serializable
**Status**: PENDING
**File**: `crates/domain/src/value_objects/disposition.rs`
**Lines**: 42-61

**Tasks**:
- [ ] Add `#[derive(Serialize, Deserialize)]` to NpcDispositionState
- [ ] Add `#[serde(rename_all = "camelCase")]`

---

## Phase 8: High - Fix Protocol Forward Compatibility (Complexity: Low)

### CR5-8.1 - Add Unknown to TimeSuggestionDecision
**Status**: PENDING
**File**: `crates/protocol/src/types.rs`
**Lines**: 358-367

**Tasks**:
- [ ] Add `#[serde(other)] Unknown` variant

---

### CR5-8.2 - Add Unknown to RuleSystemVariant
**Status**: PENDING
**File**: `crates/domain/src/types/rule_system.rs`
**Lines**: 27-47

**Tasks**:
- [ ] Add `#[serde(other)] Unknown` variant

---

### CR5-8.3 - Add Missing Act CRUD Operations
**Status**: PENDING
**File**: `crates/protocol/src/requests.rs`

**Tasks**:
- [ ] Add `GetAct { act_id: String }` to RequestPayload
- [ ] Add `UpdateAct { act_id: String, data: UpdateActData }` 
- [ ] Add `DeleteAct { act_id: String }`
- [ ] Create `UpdateActData` struct

---

## Phase 9: Medium - Fix Player UI Bugs (Complexity: Medium)

### CR5-9.1 - Fix Typewriter Effect Reactivity
**Status**: PENDING
**File**: `crates/player-ui/src/presentation/state/dialogue_state.rs`
**Lines**: 273-355

**Issue**: `use_future` captures state at creation, doesn't restart on new dialogue

**Tasks**:
- [ ] Restructure to watch for changes to dialogue content
- [ ] Use `use_effect` with proper dependencies or `use_resource`
- [ ] Ensure typewriter restarts when new dialogue arrives

---

### CR5-9.2 - Fix OutcomeRegenerated Race Condition
**Status**: PENDING
**File**: `crates/player-ui/src/presentation/handlers/session_message_handler.rs`
**Lines**: 305-339

**Issue**: Uses pre-computed index that could be stale

**Tasks**:
- [ ] Clone first, then find by request_id
- [ ] Remove index-based lookup

---

### CR5-9.3 - Add Bounds to Unbounded Collections
**Status**: PENDING
**File**: `crates/player-ui/src/presentation/state/game_state.rs`

**Tasks**:
- [ ] Add MAX_TIME_SUGGESTIONS constant (50)
- [ ] Add bounds checking to `add_time_suggestion()`
- [ ] Add MAX_NPC_MOODS constant (200)
- [ ] Implement LRU eviction for npc_moods HashMap

---

### CR5-9.4 - Fix ComfyUI State Default
**Status**: PENDING
**File**: `crates/player-ui/src/presentation/state/connection_state.rs`
**Lines**: 97, 206

**Tasks**:
- [ ] Change default from "connected" to "unknown"
- [ ] Update clear() to set "unknown" instead of "connected"

---

## Phase 10: Medium - Fix Player Services (Complexity: Low)

### CR5-10.1 - Add Request Timeouts to ChallengeService
**Status**: PENDING
**File**: `crates/player-app/src/application/services/challenge_service.rs`

**Tasks**:
- [ ] Replace all `request()` calls with `request_with_timeout()`
- [ ] Use `get_request_timeout_ms()` for timeout value

---

### CR5-10.2 - Add Request Timeouts to NarrativeEventService
**Status**: PENDING
**File**: `crates/player-app/src/application/services/narrative_event_service.rs`

**Tasks**:
- [ ] Replace all `request()` calls with `request_with_timeout()`

---

### CR5-10.3 - Fix EventChain Request Propagation
**Status**: PENDING
**File**: `crates/player-app/src/application/services/event_chain_service.rs`
**Lines**: 95-115

**Tasks**:
- [ ] Update protocol CreateEventChainData to include all fields
- [ ] Update From impl to propagate events, act_id, tags, color, is_active

---

## Phase 11: Medium - Code Deduplication (Complexity: Medium)

### CR5-11.1 - Extract Shared CharacterSheetDataApi
**Status**: PENDING
**Files**:
- `crates/player-app/src/application/services/character_service.rs`
- `crates/player-app/src/application/services/player_character_service.rs`

**Tasks**:
- [ ] Move CharacterSheetDataApi to dto module
- [ ] Update imports in both services

---

### CR5-11.2 - Extract StagedNpc Conversion Helper
**Status**: PENDING
**File**: `crates/player-ui/src/presentation/handlers/session_message_handler.rs`

**Tasks**:
- [ ] Create `staged_npc_to_data()` helper function
- [ ] Replace 3 duplicate conversion blocks

---

### CR5-11.3 - Create Neo4j Error Mapping Extension
**Status**: PENDING
**File**: `crates/engine/src/infrastructure/neo4j/helpers.rs`

**Tasks**:
- [ ] Create `GraphExt` trait with `run_or_err()` and `execute_or_err()`
- [ ] Implement for `Graph`
- [ ] Update repos to use new methods (can be done incrementally)

---

## Phase 12: Low - Missing Features (Complexity: High)

### CR5-12.1 - Add Region Connection Validation
**Status**: PENDING
**File**: `crates/engine/src/use_cases/movement/enter_region.rs`

**Issue**: Players can teleport to any region in a location

**Tasks**:
- [ ] Check that a connection exists from current region to target
- [ ] Add `EnterRegionError::NoPathToRegion` variant
- [ ] Allow exception for initial spawn (no current region)

---

### CR5-12.2 - Add EndConversation Use Case
**Status**: PENDING
**File**: `crates/engine/src/use_cases/conversation/mod.rs`

**Tasks**:
- [ ] Create `EndConversation` use case
- [ ] Mark conversation as ended
- [ ] Record final dialogue state
- [ ] Broadcast conversation end to clients

---

### CR5-12.3 - Store CharacterLoreResponse Data
**Status**: PENDING
**File**: `crates/player-ui/src/presentation/handlers/session_message_handler.rs`
**Lines**: 1585-1598

**Tasks**:
- [ ] Add `lore_summaries` signal to LoreState
- [ ] Store summaries when CharacterLoreResponse received

---

---

## Progress Tracking

| Phase | Items | Completed | Status |
|-------|-------|-----------|--------|
| Phase 1 | 2 | 2 | COMPLETE |
| Phase 2 | 1 | 1 | COMPLETE |
| Phase 3 | 1 | 1 | COMPLETE |
| Phase 4 | 1 | 1 | COMPLETE |
| Phase 5 | 3 | 0 | PENDING |
| Phase 6 | 2 | 0 | PENDING |
| Phase 7 | 4 | 0 | PENDING |
| Phase 8 | 3 | 0 | PENDING |
| Phase 9 | 4 | 0 | PENDING |
| Phase 10 | 3 | 0 | PENDING |
| Phase 11 | 3 | 0 | PENDING |
| Phase 12 | 3 | 0 | PENDING |
| **Total** | **30** | **0** | **PENDING** |

---

## Commit History

| Commit | Phase | Description |
|--------|-------|-------------|
| 91cb9d0 | Phase 1 | Remove APOC dependencies from Neo4j repositories |
| 282b843 | Phase 2 | Safe world deletion with explicit node types |
| 16ff0dc | Phase 3 | Add backpressure for critical messages |
| - | Phase 4 | Populate TriggerContext properly |

