# Consolidated Implementation Plan

**Created**: 2025-12-26
**Last Updated**: 2025-12-27
**Status**: ACTIVE
**Purpose**: Single source of truth for remaining implementation work

This document consolidates remaining work from all active planning documents into a prioritized backlog.

---

## 🚧 ACTIVE WORK: P0.1 WebSocket-First Services Refactor

### Status: IN PROGRESS (2025-12-27)

The player-app services currently use REST (ApiPort) for entity CRUD operations, but the engine-side
REST endpoints were deleted during the WebSocket migration. This refactor migrates all 16 REST services
to use WebSocket request-response pattern.

### Key Decisions:
- **Remove session concept entirely** - connect to worlds, not sessions
- **AssetService remains fully REST** - file uploads require HTTP multipart
- **Request timeout: 2 minutes** - configurable via `WRLDBLDR_REQUEST_TIMEOUT_MS` env var
- **Global error handler** for WebSocket request errors

### Phase 1: Infrastructure ⏳
| Task | Status |
|------|--------|
| Add `RequestError` type to protocol | ⏳ |
| Extend `GameConnectionPort` trait with `request()` method | ⏳ |
| Implement desktop pending request tracking (tokio::sync::oneshot) | ⏳ |
| Implement WASM pending request tracking (futures::channel::oneshot) | ⏳ |
| Add missing `RequestPayload` variants (GetSheetTemplate, etc.) | ⏳ |

### Phase 2: Service Layer Refactor
| Task | Status |
|------|--------|
| Create `ServiceError` type in player-app | ⏳ |
| Refactor WorldService (remove session methods, add WebSocket) | ⏳ |
| Refactor PlayerCharacterService | ⏳ |
| Refactor CharacterService | ⏳ |
| Refactor LocationService | ⏳ |
| Refactor ChallengeService | ⏳ |
| Refactor NarrativeEventService | ⏳ |
| Refactor SkillService | ⏳ |
| Refactor EventChainService | ⏳ |
| Refactor StoryEventService | ⏳ |
| Refactor ObservationService | ⏳ |
| Refactor ActantialService | ⏳ |
| Refactor SettingsService | ⏳ |
| Refactor WorkflowService | ⏳ |
| Refactor SuggestionService | ⏳ |
| Refactor GenerationService | ⏳ |

### Phase 3: UI Layer Updates
| Task | Status |
|------|--------|
| Update Services bundle to use GameConnectionPort | ⏳ |
| Remove session-related UI components | ⏳ |
| Update service hooks | ⏳ |
| Add global WebSocket error handler | ⏳ |

### Phase 4: Cleanup
| Task | Status |
|------|--------|
| Remove ApiPort from migrated services | ⏳ |
| Remove session-related code from WorldService | ⏳ |
| Clean up unused RawApiPort if applicable | ⏳ |
| Update Cargo dependencies | ⏳ |
| Verify compilation for desktop and WASM | ⏳ |

---

## Verified Status (as of 2025-12-27)

| Item | Plan Status | Verified | Notes |
|------|-------------|----------|-------|
| P0.1 | Not Started | **VALID - CRITICAL** | 16 services call deleted REST endpoints |
| P0.2 | Not Started | **VALID** | 2 inconsistent parse_archetype implementations |
| P0.3 | Not Started | **PARTIALLY FIXED** | request_handler.rs fixed, dto/character.rs still missing family |
| P0.4 | Not Started | **VALID** | MonomythStage variant mismatches |
| P1.4 | Partial | **4/5 DONE** | Only region_items TODO remains |
| P2.1 | Not Started | **DONE** | websocket.rs already split into 14 files |

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

### P0.1: Fix Player-App REST Service Calls
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T1.1
**Status**: Not Started
**Effort**: 4-6 hours
**Severity**: CRITICAL - These services call deleted REST endpoints

| File | Issue |
|------|-------|
| `player_character_service.rs` | Calls deleted `/api/sessions/{id}/player-characters/*` |
| `world_service.rs` | Calls deleted `/api/sessions`, `/api/worlds/{id}/sessions` |

**Fix**: Refactor services to use WebSocket `RequestPayload` instead of REST calls.

---

### P0.2: Fix parse_archetype Inconsistency
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T1.2
**Status**: Not Started
**Effort**: 1 hour
**Severity**: HIGH - Case sensitivity inconsistency

**Fix**: Add `FromStr` impl on `CampbellArchetype` in domain with case-insensitive matching.

---

### P0.3: Fix parse_relationship_type Inconsistency
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T1.3
**Status**: Not Started
**Effort**: 1 hour
**Severity**: HIGH - Missing family relationship variants

**Fix**: Add canonical `FromStr` in domain with all variants.

---

### P0.4: Fix MonomythStage Variant Name Mismatches
**Source**: Code review 2025-12-26
**Status**: Not Started
**Effort**: 30 minutes
**Severity**: HIGH - Cross-crate type inconsistency will cause serialization failures

**Problem**: Two variant naming mismatches between domain and protocol:

| Domain (`entities/world.rs`) | Protocol (`types.rs`) |
|------------------------------|----------------------|
| `ApproachToInnermostCave` | `ApproachToTheInmostCave` |
| `ReturnWithElixir` | `ReturnWithTheElixir` |

**Fix**: Align variant names across both crates (prefer domain naming).

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
| Region Item Placement | **Partial** | Read/drop works; missing `place_item()` API for DM manual placement |
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

### P1.3: Region Item Placement (Complete the System)
**Source**: Previously identified as US-REGION-ITEMS (archived)
**Status**: Partial
**Effort**: 1-2 days (reduced - partial implementation exists)

**Already Implemented**:
- `get_region_items()` in RegionRepository - reads items via `[:CONTAINS_ITEM]` edge
- `add_item_to_region()` in RegionRepository - creates edge with metadata
- Drop item flow places items in regions
- `SceneChanged` message includes `region_items` field
- `RegionItemsPanel` UI component exists

**Still Needed**:
- `place_item()` standalone API for DM to manually place items
- Pick up item from region flow
- LLM context wiring (currently hardcoded to empty)

**Blocked by**: None
**Enables**: US-NAV-014 (region items in LLM context)

---

### P1.4: Wire LLM Context - Region Items
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T2.3
**Status**: Mostly Complete
**Effort**: 30 minutes (once P1.3 complete)

| TODO | Location | Status |
|------|----------|--------|
| `region_items: Vec::new()` | websocket_helpers.rs:73 | Blocked by P1.3 |
| ~~`current_mood: None`~~ | ~~websocket_helpers.rs:199~~ | ~~**DONE** (2025-12-26)~~ |
| ~~`motivations: None`~~ | ~~websocket_helpers.rs:200~~ | ~~**DONE** (2025-12-26)~~ |
| ~~`featured_npc_names: Vec::new()`~~ | ~~websocket_helpers.rs:298~~ | ~~**DONE** (2025-12-26)~~ |

> **Note**: Only `region_items` remains. This is blocked by P1.3 Region Item Placement.

---

## P2: Medium Priority (Feature Completion)

### P2.1: WebSocket Migration Phase 6 (Technical Debt)
**Source**: [WEBSOCKET_MIGRATION_COMPLETION.md](./WEBSOCKET_MIGRATION_COMPLETION.md)
**Status**: Not Started
**Effort**: 3-4 hours

| Task | Description |
|------|-------------|
| Split websocket.rs | ~3700 lines -> modules |
| Error handling audit | Standardize error types |
| Remove unused code | Compiler warnings cleanup |

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

### P2.5: Hexagonal Architecture Test Violation
**Source**: [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) T1.4
**Status**: Not Started (verified 2025-12-26)
**Effort**: 1-2 hours

**Violation at** `player-app/action_service.rs:84`:
```rust
use wrldbldr_player_adapters::infrastructure::testing::MockGameConnectionPort;
```

Application layer imports from adapters layer in test code, breaking hexagonal architecture.

**Fix Options**:
1. Move `MockGameConnectionPort` to `player-ports` with `#[cfg(test)]`
2. Use `mockall` to generate mocks from trait
3. Define mock in `player-app` test module

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
**Status**: Not Started
**Effort**: 4-6 hours

Types defined in both domain and protocol with incompatibilities:

| Type | Domain | Protocol | Issue |
|------|--------|----------|-------|
| `CampbellArchetype` | No serde, rich methods | Has serde, minimal | Incompatible |
| `GameTime` | `DateTime<Utc>` based | `day/hour/minute` fields | Different structures |
| `MonomythStage` | `ApproachToInnermostCave` | `ApproachToTheInmostCave` | Variant name mismatch (see P0.4) |

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
