# Consolidated Implementation Plan

**Created**: 2025-12-26
**Last Updated**: 2025-12-27 (reviewed)
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

### P1.1: WebSocket Migration Phase 5 Completion
**Source**: [WEBSOCKET_MIGRATION_COMPLETION.md](./WEBSOCKET_MIGRATION_COMPLETION.md)
**Status**: In Progress
**Effort**: 3-4 hours remaining

| Task | Status | Notes |
|------|--------|-------|
| DirectorialUpdate persistence | Not Started | Currently in-memory only via WorldStateManager |
| ~~Wire NPC Mood Panel~~ | ~~**COMPLETE**~~ | Handlers exist, DM panel wired via SetNpcMood/GetNpcMoods |
| ~~Region Item Placement~~ | ~~**COMPLETE**~~ | Implemented in P1.3: PlaceItemInRegion, CreateAndPlaceItem APIs |
| Staging Status API | **Partial** | Messages exist; UI hardcodes status; need `GetStagingStatus` request |

---

### P1.2: Dialogue Persistence System - Complete Gap Analysis
**Source**: [dialogue-system.md](../systems/dialogue-system.md) US-DLG-011/012/013
**Status**: **Mostly Complete** (2025-12-26 code review)
**Effort**: 0.5-1 day (gaps only)

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

**Remaining Gaps**:
- Player dialogue text not captured (empty string passed)
- Topics not extracted (empty vector passed)
- Context missing: scene_id, location_id, game_time passed as None

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

### P2.2: Delete Dead Code Modules
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T2.1
**Status**: Not Started (verified 2025-12-26)
**Effort**: 2-3 hours
**Lines to Remove**: ~680

| Module | Location | Verification |
|--------|----------|--------------|
| `json_exporter.rs` | engine-adapters/infrastructure/export/ | Exists, not exported, superseded by world_snapshot.rs |
| `config_routes.rs` | engine-adapters/infrastructure/http/ | Exists, routes never registered in router |
| `tool_parser.rs` functions | engine-app/services/llm/ | `parse_tool_calls()`, `parse_single_tool()`, `validate_tool_calls()` only used in tests |
| `common_goals` module | domain/entities/goal.rs | Exists (lines 44-107), not exported, UI duplicates the data |

---

### P2.3: Create Shared Row Converters Module
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T2.2
**Status**: Not Started (verified 2025-12-26 - no converters.rs exists)
**Effort**: 3-4 hours
**Lines to Consolidate**: ~400

**Duplications found**:
| Function | Copies | Locations |
|----------|--------|-----------|
| `row_to_item()` | 4 | item_repository.rs:309, character_repository.rs:1610, player_character_repository.rs:454, region_repository.rs:700 |
| `row_to_character()` | 2 | character_repository.rs:1512, region_repository.rs:648 |
| `row_to_region()` | 2 | region_repository.rs:370, location_repository.rs:657 |

**Fix**: Create `crates/engine-adapters/src/infrastructure/persistence/converters.rs`

---

### P2.4: Move DTOs from Ports to App/Domain
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T2.4
**Status**: Not Started (verified 2025-12-26)
**Effort**: 4-6 hours

**Violations found in engine-ports**:

| File | Issue | Lines |
|------|-------|-------|
| `queue_port.rs` | `QueueItem<T>` with `new()` constructor | 19-52 |
| `llm_port.rs` | `LlmRequest` with builder pattern (`with_*` methods) | 23-67 |
| `llm_port.rs` | `ChatMessage` with `user()`, `assistant()`, `system()` constructors | 76-97 |
| `request_handler.rs` | `RequestContext` with 4 constructors + 5 validation methods | 46-169 |

**Fix**: Move DTOs to domain, keep only trait definitions in ports.

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

### P2.6: Update Stale Documentation
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T3.4
**Status**: Not Started
**Effort**: 2-3 hours

**Verified 2025-12-26**: System docs reference 7 deleted REST route files:

| Doc File | References | Status |
|----------|------------|--------|
| navigation-system.md | `location_routes.rs`, `region_routes.rs` | **DELETED** |
| observation-system.md | `observation_routes.rs` | **DELETED** |
| narrative-system.md | `narrative_event_routes.rs` | **DELETED** |
| challenge-system.md | `challenge_routes.rs` | **DELETED** |
| character-system.md | `want_routes.rs`, `goal_routes.rs` | **DELETED** |

**Existing route files** (these references are valid):
- `settings_routes.rs`, `prompt_template_routes.rs`, `asset_routes.rs`
- `export_routes.rs`, `queue_routes.rs`, `workflow_routes.rs`
- `rule_system_routes.rs`, `config_routes.rs` (but config_routes unused)

---

## P3: Low Priority (Polish)

### P3.1: Mood & Expression System
**Source**: [MOOD_EXPRESSION_SYSTEM.md](../plans/MOOD_EXPRESSION_SYSTEM.md)
**Status**: Planning
**Effort**: 3-4 days

Redesign mood from static DM value to dynamic dialogue-embedded expression system.

Key features:
- Inline mood markers: `*happy*` or `*excited|happy*`
- Expression changes during typewriter playback
- Tool-based permanent mood changes
- PC mood marker support

---

### P3.2: Remove Unused Cargo Dependencies
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T3.1
**Status**: Not Started
**Effort**: 30 minutes

**Verified Unused** (2025-12-26 review):
| Crate | Dependency | Status |
|-------|------------|--------|
| `wrldbldr-player-ui` | `thiserror` | Unused - can remove |

**Previously Listed as Unused but Actually Used**:
- `wrldbldr-domain`: `anyhow` - USED in `FromStr` impls (region.rs, observation.rs)
- `wrldbldr-domain`: `serde_json` - USED heavily in settings.rs, workflow_config.rs

> **Note**: Original audit overestimated unused deps. Only `thiserror` in player-ui confirmed unused.

---

### P3.3: Consolidate Duplicate Type Definitions
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T3.2
**Status**: Partial (MonomythStage fixed in P0.4)
**Effort**: 4-6 hours

Types defined in both domain and protocol with incompatibilities:

| Type | Domain | Protocol | Issue |
|------|--------|----------|-------|
| `CampbellArchetype` | No serde, rich methods | Has serde, minimal | Incompatible |
| `GameTime` | `DateTime<Utc>` based | `day/hour/minute` fields | Different structures |
| ~~`MonomythStage`~~ | ~~`ApproachToInnermostCave`~~ | ~~`ApproachToTheInmostCave`~~ | ~~Fixed in P0.4~~ |

**Recommended approach**: Protocol should re-export from domain for shared enums.

---

### P3.4: Remove Legacy Protocol Messages
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T3.3
**Status**: Not Started
**Effort**: 2-3 hours

**Current State** (2025-12-26 review):
- All 4 messages still exist in protocol, **none marked `#[deprecated]`**
- `JoinSession` handler returns error with code "DEPRECATED" (runtime only)
- Player WebSocket client still sends `JoinSession` in some paths
- `SessionJoined`, `PlayerJoined`, `PlayerLeft` still actively handled in UI

**Messages to Remove**:
| Message | Type | Status |
|---------|------|--------|
| `JoinSession` | ClientMessage | Functionally deprecated, still sent by player |
| `SessionJoined` | ServerMessage | Still handled by player UI |
| `PlayerJoined` | ServerMessage | Still handled by player UI |
| `PlayerLeft` | ServerMessage | Still handled by player UI |

**Preferred**: `JoinWorld`, `WorldJoined`, `UserJoined`, `UserLeft`

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
