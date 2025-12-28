# Consolidated Implementation Plan

**Created**: 2025-12-26
**Last Updated**: 2025-12-27 (cleanup sprint complete)
**Status**: ACTIVE
**Purpose**: Single source of truth for remaining implementation work

This document consolidates remaining work from all active planning documents into a prioritized backlog.

---

## ✅ COMPLETED: P0.1 WebSocket-First Services Refactor

### Status: COMPLETE (2025-12-27)

All player-app services have been migrated from REST (ApiPort) to WebSocket (GameConnectionPort).
The refactor successfully migrated 13 services to WebSocket, with 3 services intentionally remaining REST.

### Key Decisions (Implemented):
- **Remove session concept entirely** - connect to worlds, not sessions ✅
- **AssetService remains fully REST** - file uploads require HTTP multipart ✅
- **WorkflowService remains REST** - large JSON payloads (50-500KB ComfyUI workflows) ✅
- **SettingsService remains REST** - global/admin config, not session-tied ✅
- **Request timeout: 2 minutes** - configurable via `WRLDBLDR_REQUEST_TIMEOUT_MS` env var ✅
- **Global error handler** for WebSocket request errors ✅

### Phase 1: Infrastructure ✅
| Task | Status |
|------|--------|
| Add `RequestError` type to protocol | ✅ |
| Extend `GameConnectionPort` trait with `request()` method | ✅ |
| Implement desktop pending request tracking (tokio::sync::oneshot) | ✅ |
| Implement WASM pending request tracking (futures::channel::oneshot) | ✅ |
| Add missing `RequestPayload` variants (GetSheetTemplate, etc.) | ✅ |

### Phase 2: Service Layer Refactor ✅
| Task | Status | Notes |
|------|--------|-------|
| Create `ServiceError` type in player-app | ✅ | |
| Refactor WorldService (remove session methods, add WebSocket) | ✅ | Keeps REST for exports |
| Refactor PlayerCharacterService | ✅ | |
| Refactor CharacterService | ✅ | |
| Refactor LocationService | ✅ | |
| Refactor ChallengeService | ✅ | |
| Refactor NarrativeEventService | ✅ | |
| Refactor SkillService | ✅ | |
| Refactor EventChainService | ✅ | |
| Refactor StoryEventService | ✅ | |
| Refactor ObservationService | ✅ | |
| Refactor ActantialService | ✅ | |
| Refactor GenerationService | ✅ | |
| Refactor SuggestionService | ✅ | With auto-enrichment from world data |
| SettingsService | N/A | Intentionally REST (admin config) |
| WorkflowService | N/A | Intentionally REST (large payloads) |
| AssetService | N/A | Intentionally REST (file uploads) |

### Phase 3: UI Layer Updates ✅
| Task | Status |
|------|--------|
| Update Services bundle to use GameConnectionPort | ✅ |
| Remove session-related UI components | ✅ |
| Update service hooks | ✅ |
| Add global WebSocket error handler | ✅ |

### Phase 4: Cleanup ✅
| Task | Status |
|------|--------|
| Remove ApiPort from migrated services | ✅ |
| Remove session-related code from WorldService | ✅ |
| Clean up unused RawApiPort if applicable | ✅ (still used for world export) |
| Update Cargo dependencies | ✅ |
| Verify compilation for desktop and WASM | ✅ |

---

## Verified Status (as of 2025-12-27)

| Item | Plan Status | Verified | Notes |
|------|-------------|----------|-------|
| P0.1 | **COMPLETE** | ✅ | All 13 services migrated to WebSocket |
| P0.2 | **COMPLETE** | ✅ | Added FromStr to CampbellArchetype |
| P0.3 | **COMPLETE** | ✅ | Added FromStr to RelationshipType with all family types |
| P0.4 | **COMPLETE** | ✅ | MonomythStage variants aligned |
| P1.1a | **COMPLETE** | ✅ | Staging status event-driven UI + DM broadcast |
| P1.1b | **COMPLETE** | ✅ | DirectorialContext SQLite persistence |
| P1.2 | **COMPLETE** | ✅ | Dialogue context propagation + LLM topic extraction |
| P1.3 | **COMPLETE** | ✅ | PlaceItemInRegion and CreateAndPlaceItem DM APIs |
| P1.4 | **COMPLETE** | ✅ | All LLM context TODOs wired |
| P1.5 | **COMPLETE** | ✅ | All 4 WebSocket memory leaks fixed |
| P1.6 | **COMPLETE** | ✅ | GetSheetTemplate, GetMyPlayerCharacter handlers, sheet_data parsing |
| P2.1 | ~~Not Started~~ | **COMPLETE** | websocket.rs already split into 15 files |
| P2.5 | **COMPLETE** | ✅ | Mock moved to player-ports, WASM compiles |

---

## Priority Legend

| Priority | Definition | SLA |
|----------|------------|-----|
| P0 | Critical bug/blocker - breaks production | Immediate |
| P1 | High priority - core functionality gaps | Next sprint |
| P2 | Medium priority - feature completion | 2-4 weeks |
| P3 | Low priority - polish/nice-to-have | Backlog |

---

## P0: Critical (Runtime Failures)

### ~~P0.1: Fix Player-App REST Service Calls~~ ✅ COMPLETE
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T1.1
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 4-6 hours (actual: ~8 hours across multiple sessions)
**Severity**: CRITICAL - These services call deleted REST endpoints

**Resolution**: All 13 player-app services migrated from REST to WebSocket:
- WorldService, CharacterService, LocationService, PlayerCharacterService
- ChallengeService, NarrativeEventService, StoryEventService, EventChainService
- ObservationService, ActantialService, SkillService, GenerationService, SuggestionService

Three services intentionally remain REST:
- AssetService (file uploads require HTTP multipart)
- WorkflowService (large JSON payloads 50-500KB)
- SettingsService (global admin config)

**Bonus**: SuggestionService migration included auto-enrichment feature that fetches
world data to enhance LLM suggestion context when `world_setting` is not provided.

---

### ~~P0.2: Fix parse_archetype Inconsistency~~ ✅ COMPLETE
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T1.2
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 30 minutes

**Resolution**: Added `FromStr` impl to `CampbellArchetype` in domain with case-insensitive
matching. Supports PascalCase, snake_case, lowercase, and space-separated formats.
Removed duplicate `parse_archetype` functions from request_handler.rs and dto/character.rs.

---

### ~~P0.3: Fix parse_relationship_type Inconsistency~~ ✅ COMPLETE
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T1.3
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 30 minutes

**Resolution**: Added `FromStr` impl to `RelationshipType` and `FamilyRelation` in domain.
Now supports all 9 family relation types (parent, child, sibling, spouse, grandparent,
grandchild, aunt/uncle, niece/nephew, cousin) plus aliases (friend, mentor, enemy).
Removed duplicate functions from request_handler.rs and dto/character.rs.

---

### ~~P0.4: Fix MonomythStage Variant Name Mismatches~~ ✅ COMPLETE
**Source**: Code review 2025-12-26
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 10 minutes

**Resolution**: Aligned protocol variant names to match domain:
- `ApproachToTheInmostCave` → `ApproachToInnermostCave`
- `ReturnWithTheElixir` → `ReturnWithElixir`

---

## P1: High Priority (Core Functionality)

### ~~P1.1: WebSocket Migration Phase 5 Completion~~ ✅ COMPLETE
**Source**: [WEBSOCKET_MIGRATION_COMPLETION.md](./WEBSOCKET_MIGRATION_COMPLETION.md)
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 3.5-5 hours

| Task | Status | Notes |
|------|--------|-------|
| ~~DirectorialUpdate persistence~~ | ~~**COMPLETE**~~ | SQLite persistence via settings_pool (P1.1b) |
| ~~Wire NPC Mood Panel~~ | ~~**COMPLETE**~~ | Handlers exist, DM panel wired via SetNpcMood/GetNpcMoods |
| ~~Region Item Placement~~ | ~~**COMPLETE**~~ | Implemented in P1.3: PlaceItemInRegion, CreateAndPlaceItem APIs |
| ~~Staging Status API~~ | ~~**COMPLETE**~~ | Event-driven via RegionStagingStatus enum + DM broadcast (P1.1a) |

**P1.1a Resolution** (Staging Status):
- Added `RegionStagingStatus` enum to `GameState` with `None`, `Pending`, `Active` variants
- Updated `session_message_handler.rs` to set status on `StagingApprovalRequired`, `StagingPending`, `StagingReady` events
- Updated `LocationStagingPanel` to read from state instead of hardcoding `None`
- Added `broadcast_to_dms()` call in `staging.rs` so DMs receive `StagingReady` events

**P1.1b Resolution** (DirectorialUpdate Persistence):
- Created `DirectorialContextRepositoryPort` trait in engine-ports
- Implemented `SqliteDirectorialContextRepository` using existing settings_pool
- Wired repository to AppState and WebSocket handlers
- Load on `JoinWorld`, persist on `DirectorialUpdate`

---

### ~~P1.2: Dialogue Persistence System - Complete Gap Analysis~~ ✅ COMPLETE
**Source**: [dialogue-system.md](../systems/dialogue-system.md) US-DLG-011/012/013
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 3-4 hours

| User Story | Description | Status |
|------------|-------------|--------|
| US-DLG-011 | Persist dialogue exchanges as StoryEvents | **DONE** - `record_dialogue_exchange()` implemented |
| US-DLG-012 | Query last dialogues with specific NPC | **DONE** - `get_dialogues_with_npc()` implemented |
| US-DLG-013 | Track SPOKE_TO relationships with metadata | **DONE** - `update_spoke_to_edge()` implemented |

**Already Implemented**:
- `SPOKE_TO` edge with `last_dialogue_at`, `last_topic`, `conversation_count`
- `record_dialogue_exchange()` creates `StoryEvent::DialogueExchange`
- `get_dialogues_with_npc()` retrieves dialogue history
- `get_dialogue_summary_for_npc()` for LLM context
- Called from `DMApprovalQueueService` after dialogue approval

**Gap Resolution** (2025-12-27):
- Added `scene_id`, `location_id`, `game_time` fields to `GamePromptRequest` (domain)
- Populated fields in `build_prompt_from_action()` (engine-adapters)
- Added fields to `ApprovalItem` DTO with `player_dialogue`, `topics` (engine-app)
- Added `topics` field to `LLMGameResponse` with parsing from `<topics>` tag
- Updated prompt template `DIALOGUE_RESPONSE_FORMAT` to request topic extraction
- Updated `record_dialogue_exchange()` call to use context from `ApprovalItem`

**Enables**: Staging System LLM context about recent NPC interactions

---

### ~~P1.3: Region Item Placement (Complete the System)~~ ✅ COMPLETE
**Source**: Previously identified as US-REGION-ITEMS (archived)
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 1.5 hours

**Resolution**: Added DM-only WebSocket APIs for placing items in regions:
- `PlaceItemInRegion`: Place an existing item into a region
- `CreateAndPlaceItem`: Create a new item and place it in a region

**Changes**:
- Added `PlaceItemInRegion` and `CreateAndPlaceItem` to RequestPayload enum
- Added `CreateItemData` struct for item creation parameters
- Added `place_item_in_region()` and `create_and_place_item()` to ItemService trait
- Wired `RegionRepositoryPort` into `ItemServiceImpl` via `with_region_repository()`
- Added `parse_item_id()` helper to request handler
- Implemented handlers with DM-only access check

**Remaining Work** (P2 priority):
- Pick up item from region flow (PickupItem already exists via WebSocket message)
- LLM context wiring done in P1.4

---

### ~~P1.4: Wire LLM Context - Region Items~~ ✅ COMPLETE
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T2.3
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 30 minutes

**Resolution**: Updated `build_prompt_from_action()` in `websocket_helpers.rs` to:
1. Get PC's current region from `pc_repo.get(pc_id)`
2. Fetch items via `region_repo.get_region_items(region_id)`
3. Convert to `Vec<RegionItemContext>` for LLM prompt
4. NPCs now aware of visible items during dialogue

| TODO | Location | Status |
|------|----------|--------|
| ~~`region_items: Vec::new()`~~ | ~~websocket_helpers.rs:73~~ | ~~**DONE** (2025-12-27)~~ |
| ~~`current_mood: None`~~ | ~~websocket_helpers.rs:199~~ | ~~**DONE** (2025-12-26)~~ |
| ~~`motivations: None`~~ | ~~websocket_helpers.rs:200~~ | ~~**DONE** (2025-12-26)~~ |
| ~~`featured_npc_names: Vec::new()`~~ | ~~websocket_helpers.rs:298~~ | ~~**DONE** (2025-12-26)~~ |

> **Note**: All LLM context TODOs are now complete.

---

### ~~P1.5: Fix WebSocket Memory Leaks~~ ✅ COMPLETE
**Source**: Code review 2025-12-27
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 1.5 hours

**Resolution**: Fixed all 4 memory leak issues:

| Issue | Fix |
|-------|-----|
| Desktop disconnect | Clear `pending_requests` HashMap on disconnect |
| WASM disconnect | Clear `pending_requests` on disconnect |
| WASM timeout | Add `request_with_timeout()` to client that removes request on timeout |
| WASM closure leak | Store closures in `WasmClosures` struct, drop on disconnect/reconnect |

**Files Modified**:
- `client.rs`: Added cleanup to both disconnect methods, added WASM `request_with_timeout()`, added `WasmClosures` struct
- `game_connection_adapter.rs`: Delegate WASM timeout to client method

---

### ~~P1.6: Implement Missing Engine Handlers~~ ✅ COMPLETE
**Source**: Code review 2025-12-27
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 1.5 hours

**Resolution**: Implemented all missing handlers:
- `GetSheetTemplate`: Returns `SheetTemplateResponseDto` for world's default template
- `GetMyPlayerCharacter`: Uses existing `get_pc_by_user_and_world()` service method
- `UpdatePlayerCharacter`: Fixed `sheet_data` parsing from protocol JSON to `CharacterSheetData`

**Changes**:
- Wired `SheetTemplateService` to `AppRequestHandler` (as `Arc<SheetTemplateService>`)
- Updated `PlayerServices` to use `Arc<SheetTemplateService>` for sharing
- Added import for `SheetTemplateResponseDto` in handler
- Added import for `CharacterSheetData` for JSON parsing

---

## P2: Medium Priority (Feature Completion)

### ~~P2.1: WebSocket Migration Phase 6 (Technical Debt)~~ ✅ COMPLETE
**Source**: [WEBSOCKET_MIGRATION_COMPLETION.md](./WEBSOCKET_MIGRATION_COMPLETION.md)
**Status**: ✅ COMPLETE (verified 2025-12-27)
**Effort**: Already done

| Task | Status |
|------|--------|
| Split websocket.rs | ✅ Split into 15 files |
| Error handling audit | Partial - standardized in new structure |
| Remove unused code | ✅ Done during split |

**Structure verified** (2025-12-27):
```
crates/engine-adapters/src/infrastructure/websocket/
├── mod.rs (147 lines)
├── dispatch.rs (311 lines)
├── converters.rs (110 lines)
├── messages.rs (6 lines)
└── handlers/
    ├── mod.rs, challenge.rs, connection.rs, inventory.rs
    ├── misc.rs, movement.rs, narrative.rs, player_action.rs
    ├── request.rs, scene.rs, staging.rs
```

**Note**: 4 handler files exceed 500 lines and could be further split:
- `movement.rs` (973 lines)
- `challenge.rs` (816 lines)
- `staging.rs` (595 lines)
- `inventory.rs` (517 lines)

---

### ~~P2.2: Delete Dead Code Modules~~ ✅ COMPLETE
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T2.1
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 30 minutes
**Lines Removed**: ~815

**Resolution**:
- Deleted `json_exporter.rs` from engine-adapters/infrastructure/export/
- Deleted `config_routes.rs` from engine-adapters/infrastructure/http/
- Removed `parse_tool_calls()`, `parse_single_tool()`, `validate_tool_calls()`, `ParsedToolCall` from tool_parser.rs
- Removed `common_goals` module from domain/entities/goal.rs

---

### ~~P2.3: Create Shared Row Converters Module~~ ✅ COMPLETE
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T2.2
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 1 hour

**Resolution**: Created `crates/engine-adapters/src/infrastructure/persistence/converters.rs` with shared converters:
- `row_to_item()` - consolidated from 4 locations
- `row_to_want()` - consolidated from 2 locations  
- `row_to_region()` - consolidated from 2 locations

Updated 6 repository files to use shared converters:
- character_repository.rs, player_character_repository.rs, item_repository.rs
- region_repository.rs, want_repository.rs, location_repository.rs

> **Note**: `row_to_character()` NOT consolidated - implementations differ significantly between files.

---

### ~~P2.4: Move DTOs from Ports to App/Domain~~ ✅ COMPLETE
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T2.4
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 1.5 hours

**Resolution**: Created new `crates/engine-dto/` crate for DTO types:
- `src/llm.rs` - LlmRequest, ChatMessage, LlmResponse, ToolCall, FinishReason
- `src/queue.rs` - QueueItem, QueueItemStatus, QueueError  
- `src/request_context.rs` - RequestContext with validation methods

**Changes**:
- Added `engine-dto` to workspace Cargo.toml
- Updated `engine-ports` to depend on and re-export from engine-dto
- Added engine-dto to arch-check rules in xtask

Port files now contain only trait definitions, re-exporting DTOs from engine-dto.

---

### ~~P2.5: Hexagonal Architecture Test Violations~~ ✅ COMPLETE
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T1.4
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 30 minutes

**Resolution**: Moved `MockGameConnectionPort` from `player-adapters` to `player-ports`.
Updated imports in action_service.rs, narrative_event_service.rs, challenge_service.rs.
Also fixes WASM compilation by adding `#[cfg(not(target_arch = "wasm32"))]` guard.

**Changes**:
- Created `player-ports/src/outbound/testing/` module with desktop-only mock
- Updated 3 test files to import from `wrldbldr_player_ports::outbound`
- Removed mock from `player-adapters/infrastructure/testing/`
- Added `tokio` dev-dependency to player-app for async tests

---

### ~~P2.6: Update Stale Documentation~~ ✅ COMPLETE
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T3.4
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 30 minutes

**Resolution**: Updated 5 system docs to reference WebSocket handlers instead of deleted HTTP routes:
- `navigation-system.md` - now references `handlers/movement.rs`
- `observation-system.md` - now references `handlers/misc.rs`
- `narrative-system.md` - now references `handlers/narrative.rs`
- `challenge-system.md` - now references `handlers/challenge.rs`
- `character-system.md` - now references `handlers/inventory.rs`, `handlers/misc.rs`

Also updated `docs/systems/_template.md` to use WebSocket handler pattern.

---

## P3: Low Priority (Polish)

### P3.1: Mood & Expression System
**Source**: [MOOD_EXPRESSION_SYSTEM_IMPLEMENTATION.md](../plans/MOOD_EXPRESSION_SYSTEM_IMPLEMENTATION.md)
**Status**: Planning Complete - Ready for Implementation
**Effort**: 30-35 hours (4-5 days)

Implement a **three-tier emotional model** with clear terminology:
1. **Disposition** (persistent NPC→PC relationship) - renamed from MoodLevel
2. **Mood** (semi-persistent NPC state) - set during staging, cached until next staging
3. **Expression** (transient dialogue state) - inline markers that change sprites

**Key Features**:
- Clear terminology separation: Disposition vs Mood vs Expression
- Disposition persists in Neo4j per NPC-PC pair (renamed from "mood")
- Mood set by DM during staging approval, cached with staging
- Expression markers in dialogue: `*happy*` or `*excited|happy*`
- LLM context includes both disposition AND mood for richer responses
- LLM tool calls: `change_disposition` and `change_mood` (both require DM approval)
- DM editable dialogue with live marker validation (green/red/gray underlines)
- Expression sheet generation via ComfyUI (3x3 grids)

**Implementation Phases** (13 phases):
0. Disposition Rename Refactor (prerequisite)
1. New Mood System (Domain & Protocol)
2. Staging System Mood Integration
3. Persistence & Repository Updates
4. LLM Prompt Updates
5. Expression Sheet Generation
6. Typewriter with Expression Changes
7. Character Sprite Updates
8. Player Input Validation
9. Expression Config Editor UI
10. Expression Sheet Generation UI
11. DM Approval Marker Support
12. Testing & Polish

See full implementation plan: [MOOD_EXPRESSION_SYSTEM_IMPLEMENTATION.md](../plans/MOOD_EXPRESSION_SYSTEM_IMPLEMENTATION.md)

---

### ~~P3.2: Remove Unused Cargo Dependencies~~ ✅ COMPLETE
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T3.1
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 5 minutes

**Resolution**: Removed unused `thiserror` dependency from `crates/player-ui/Cargo.toml`.

> **Note**: Only `thiserror` in player-ui was actually unused. Other deps (anyhow, serde_json) are used.

---

### ~~P3.3: Consolidate Duplicate Type Definitions~~ ✅ COMPLETE
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T3.2
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 3.5 hours

**Resolution**: Established domain as single source of truth for shared types:

| Change | Description |
|--------|-------------|
| Architecture rules | Updated xtask to allow `protocol → domain` dependency |
| Domain types | Added serde derives to `CampbellArchetype`, `MonomythStage`, `RuleSystemType`, `RuleSystemVariant`, `RuleSystemConfig`, `StatDefinition`, `DiceSystem`, `SuccessComparison` |
| Protocol cleanup | Replaced duplicate types with re-exports from domain |
| Engine-adapters | Simplified world_repository.rs (direct serialize/deserialize) and world_snapshot.rs (removed conversion function) |
| Player-app | Replaced duplicate types with protocol re-exports + extension traits for UI methods (`RuleSystemTypeExt`, `RuleSystemVariantExt`) |
| Engine-app DTOs | Removed redundant `RuleSystemTypeDto`, `RuleSystemVariantDto`, `RuleSystemConfigDto`, etc. (~200 lines deleted) |
| GameTime utility | Added `to_protocol_game_time()` in engine-adapters converters.rs |
| Serde consistency | Fixed `RuleSystemConfig` and `StatDefinition` to use `snake_case` (was `camelCase`) |

**Intentionally not unified** (per D7):
- `GameTime`: Domain uses `chrono::DateTime<Utc>`, protocol uses simple `{day, hour, minute, is_paused}` for wire efficiency
- Request handler GameTime conversions stay inline (engine-app cannot import from engine-adapters)

**Documentation updated**: HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md D3 and D5 now reflect reality.

---

### ~~P3.4: Remove Legacy Protocol Messages~~ ✅ COMPLETE
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T3.3
**Status**: ✅ COMPLETE (2025-12-27)
**Effort**: 1.5 hours

**Resolution**:
- Renamed `join_session()` → `join_world()` in `GameConnectionPort` trait
- Updated signature to `fn join_world(&self, world_id: &str, user_id: &str, role: ParticipantRole)`
- Updated WASM and Desktop clients to send `JoinWorld` message
- Removed `JoinSession` from `ClientMessage` enum in protocol
- Removed `SessionJoined`, `PlayerJoined`, `PlayerLeft` from `ServerMessage` enum
- Removed legacy handlers from player-ui `session_message_handler.rs`
- Removed deprecated `handle_join_session()` from engine dispatch

**Note**: `session_id` field remains in `ConnectionState` as UI components still reference it.
This can be cleaned up in a future refactor when UI fully migrates to `world_id`.

---

### P3.5: Visual Trigger Condition Builder
**Source**: [ACTIVE_DEVELOPMENT.md](./ACTIVE_DEVELOPMENT.md) US-NAR-009
**Status**: Not Started
**Effort**: 3-4 days

Visual builder UI for narrative trigger conditions.

---

### P3.6: Advanced Workflow Parameter Editor
**Source**: [ACTIVE_DEVELOPMENT.md](./ACTIVE_DEVELOPMENT.md) US-AST-010
**Status**: Not Started
**Effort**: 2 days

Edit ComfyUI workflow parameters in UI.

---

## Deferred (Future Consideration)

### Authentication & Authorization
**Status**: Not Started
**Priority**: Post-MVP

Currently no auth system. Requires:
- User accounts
- World ownership
- Role-based access (DM vs Player)
- Session tokens

---

### Save/Load System
**Status**: Not Started
**Priority**: Post-MVP

No persistence across server restarts. Requires:
- Define save file format
- Export world state
- Import/restore world state
- Autosave mechanism

---

### Tactical Combat
**Status**: Not Started
**Priority**: Future

Grid-based combat system:
- Combat service (turn order, movement, attack resolution)
- Combat WebSocket messages
- Grid renderer
- Combat UI

---

### Audio System
**Status**: Not Started
**Priority**: Future

Sound and music:
- Audio manager
- Scene audio integration
- Volume controls

---

## Progress Tracking

| Date | Change |
|------|--------|
| 2025-12-27 | **P3.1 Planning Complete**: Created comprehensive implementation plan with 13 phases, three-tier emotional model (Disposition/Mood/Expression), UI mockups, staging mood integration |
| 2025-12-27 | **P3.3 COMPLETE (Part 2)**: Engine-app DTO removal (~200 lines), GameTime utility, serde casing fix |
| 2025-12-27 | **P3.3 COMPLETE (Part 1)**: Type consolidation - domain is now single source of truth for shared types; protocol re-exports from domain |
| 2025-12-27 | **P1.2 COMPLETE**: Dialogue gaps fixed - propagated context through LLM flow, added topic extraction |
| 2025-12-27 | **P1.1b COMPLETE**: DirectorialContext persisted to SQLite via settings_pool |
| 2025-12-27 | **P1.1a COMPLETE**: Staging status event-driven via RegionStagingStatus + DM broadcast |
| 2025-12-27 | **P1.6 COMPLETE**: Implemented GetSheetTemplate, GetMyPlayerCharacter handlers, fixed sheet_data parsing |
| 2025-12-27 | **P1.3 COMPLETE**: Added PlaceItemInRegion and CreateAndPlaceItem DM APIs |
| 2025-12-27 | **P2.5 COMPLETE**: Fixed hex arch violation, moved MockGameConnectionPort to player-ports |
| 2025-12-27 | Fixed WASM compilation by cfg-guarding mock to desktop-only |
| 2025-12-27 | **P1.5 COMPLETE**: Fixed all 4 WebSocket memory leaks (disconnect cleanup, timeout cleanup, closure storage) |
| 2025-12-27 | **P1.4 COMPLETE**: Wired region_items into LLM context via build_prompt_from_action() |
| 2025-12-27 | Code review: Added P1.5 (memory leaks) and P1.6 (missing handlers) |
| 2025-12-27 | Code review: P2.1 marked COMPLETE (websocket.rs split into 15 files) |
| 2025-12-27 | Code review: P2.5 updated with 2 additional test violations |
| 2025-12-27 | Code review: P3.3 updated (MonomythStage fixed) |
| 2025-12-27 | **ALL P0 ITEMS COMPLETE** |
| 2025-12-27 | **P0.4 COMPLETE**: MonomythStage variants aligned between domain and protocol |
| 2025-12-27 | **P0.3 COMPLETE**: Added FromStr to RelationshipType with all 9 family types |
| 2025-12-27 | **P0.2 COMPLETE**: Added FromStr to CampbellArchetype for case-insensitive parsing |
| 2025-12-27 | **P0.1 COMPLETE**: All 13 player-app services migrated to WebSocket |
| 2025-12-27 | SuggestionService: Added auto-enrichment (fetches world data for LLM context) |
| 2025-12-27 | Protocol: Added `SuggestionContextData`, `EnqueueContentSuggestion`, `CancelContentSuggestion` |
| 2025-12-27 | Engine: `SuggestionEnqueueAdapter` now auto-enriches `world_setting` from world repository |
| 2025-12-26 | Full code validation: All P0-P3 items verified against codebase |
| 2025-12-26 | P1.2 Dialogue Persistence: Marked as MOSTLY COMPLETE (was Not Started) |
| 2025-12-26 | P0.4: Added second mismatch (ReturnWithElixir vs ReturnWithTheElixir) |
| 2025-12-26 | P2.2-P2.6: Added specific line numbers and verification details |
| 2025-12-26 | P3.4: Added current state of legacy messages (none have #[deprecated]) |
| 2025-12-26 | Review corrections: NPC Mood Panel marked complete, Region Items/Staging marked partial |
| 2025-12-26 | Added P0.4 MonomythStage variant mismatch |
| 2025-12-26 | Corrected P3.2 unused deps (only thiserror in player-ui confirmed) |
| 2025-12-26 | Updated P3.3 with specific incompatibility details |
| 2025-12-26 | Initial consolidation from all active plans |

---

## Source Documents

This plan consolidates work from:
- [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) - Code quality audit findings
- [WEBSOCKET_MIGRATION_COMPLETION.md](./WEBSOCKET_MIGRATION_COMPLETION.md) - WebSocket migration phases 5-6
- [ACTIVE_DEVELOPMENT.md](./ACTIVE_DEVELOPMENT.md) - Active sprint tracking
- [MOOD_EXPRESSION_SYSTEM.md](../plans/MOOD_EXPRESSION_SYSTEM.md) - New mood system design
- System documentation in [docs/systems/](../systems/)
