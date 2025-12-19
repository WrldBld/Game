# WrldBldr Monorepo Consolidation & Refactoring Plan

## Overview

This plan addresses all issues found in the comprehensive code review, including consolidating 3 separate repositories into a single monorepo with a shared protocol crate.

## Current State (Before)

```
WrldBldr/                    # Parent directory (not a git repo)
├── Engine/                  # Separate git repo
├── Player/                  # Separate git repo  
├── plans/                   # Separate git repo
├── docker-compose.yml
├── .env.example
└── various CODE_REVIEW*.md files
```

## Target State (After)

```
WrldBldr/
└── Game/                           # Single git repo
    ├── Cargo.toml                  # Workspace root
    ├── Cargo.lock                  # Shared lockfile
    ├── rust-toolchain.toml         # Pin Rust version
    ├── .cargo/
    │   └── config.toml             # Cargo aliases (xtask)
    │
    ├── crates/
    │   ├── protocol/               # NEW: Shared types (IDs, messages, rule system)
    │   │   ├── Cargo.toml
    │   │   └── src/
    │   │
    │   ├── engine/                 # Backend (moved from Engine/)
    │   │   ├── Cargo.toml
    │   │   └── src/
    │   │
    │   └── player/                 # Frontend (moved from Player/)
    │       ├── Cargo.toml
    │       ├── Dioxus.toml
    │       ├── Trunk.toml
    │       └── src/
    │
    ├── docs/                       # Documentation (moved from plans/)
    │   ├── architecture/
    │   ├── progress/
    │   ├── systems/
    │   └── CLAUDE.md
    │
    ├── docker/
    │   └── docker-compose.yml
    ├── .env.example
    └── README.md
```

---

## Implementation Phases

### Phase 0: Repository Consolidation
**Goal**: Merge 3 repos into 1 monorepo with Cargo workspace
**Status**: COMPLETE

| Task | Description | Status |
|------|-------------|--------|
| 0.1 | Create new git repo at Game/ | ✅ |
| 0.2 | Create workspace root `Cargo.toml` | ✅ |
| 0.3 | Create `rust-toolchain.toml` pinning Rust version | ✅ |
| 0.4 | Create `.cargo/config.toml` with aliases | ✅ |
| 0.5 | Move `Engine/` to `crates/engine/`, update paths | ✅ |
| 0.6 | Move `Player/` to `crates/player/`, update paths | ✅ |
| 0.7 | Move `plans/` to `docs/` | ✅ |
| 0.8 | Update `docker-compose.yml` paths, move to `docker/` | ✅ |
| 0.9 | Update all import paths, verify builds | ✅ |

---

### Phase 1: Protocol Crate Creation
**Goal**: Extract ~1,300 lines of shared types, establish clean API contract
**Status**: IN PROGRESS

| Task | Description | Status |
|------|-------------|--------|
| 1.1 | Create `crates/protocol/` scaffold with `Cargo.toml` | ✅ |
| 1.2 | Move ID types from Engine `domain/value_objects/ids.rs` | ✅ |
| 1.3 | Unify Player IDs to use UUID (currently String) | ⏳ |
| 1.4 | Move WebSocket message types (`ClientMessage`, `ServerMessage`, etc.) | ⏳ |
| 1.5 | Move `RuleSystemConfig`, `RuleSystemType`, `RuleSystemVariant` | ⏳ |
| 1.6 | Move shared enums: `ParticipantRole`, `DiceInputType`, `CampbellArchetype` | ✅ |
| 1.7 | Update Engine imports to use `protocol::` | ⏳ |
| 1.8 | Update Player imports to use `protocol::` | ⏳ |
| 1.9 | Remove duplicate type definitions | ⏳ |
| 1.10 | Verify WASM build still works with protocol crate | ⏳ |

**Protocol crate dependencies** (minimal):
```toml
[dependencies]
serde = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
```

---

### Phase 2: WebSocket Handler Refactoring (Complete)
**Goal**: Fix critical architecture violation - extract 1,961 lines of business logic
**Status**: PENDING

#### 2.1 Create New Application Services

| Service | Responsibility | Methods |
|---------|---------------|---------|
| `NavigationApplicationService` | All movement/travel operations | `travel_to_location`, `move_to_region`, `exit_to_location`, `resolve_split_party` |
| `ChallengeApplicationService` | Challenge workflow orchestration | `submit_roll`, `trigger_ad_hoc_challenge`, `resolve_challenge` |
| `ObservationApplicationService` | Knowledge/observation management | `share_npc_location`, `create_approach_observation`, `record_location_observation` |
| `SessionEventService` | Session state changes | `advance_game_time`, `broadcast_to_session` |
| `PlayerActionApplicationService` | Centralized player action dispatch | `handle_player_action` |

#### 2.2 Refactoring Tasks

| Task | Current Location | Target |
|------|-----------------|--------|
| 2.2.1 | Create `NavigationApplicationService` port + impl | New in `application/services/` |
| 2.2.2 | Extract `travel` handler (lines 294-524) | → `NavigationApplicationService::travel_to_location` |
| 2.2.3 | Extract `MoveToRegion` handler (lines 1581-1765) | → `NavigationApplicationService::move_to_region` |
| 2.2.4 | Extract `ExitToLocation` handler | → `NavigationApplicationService::exit_to_location` |
| 2.2.5 | Create `ChallengeApplicationService` port + impl | New in `application/services/` |
| 2.2.6 | Extract challenge roll handling (lines 800-827) | → `ChallengeApplicationService` |
| 2.2.7 | Create `ObservationApplicationService` port + impl | New in `application/services/` |
| 2.2.8 | Extract `ShareNpcLocation` (lines 1167-1282) | → `ObservationApplicationService` |
| 2.2.9 | Extract `TriggerApproachEvent` (lines 1288-1416) | → `ObservationApplicationService` |
| 2.2.10 | Create `SessionEventService` | New in `application/services/` |
| 2.2.11 | Extract `AdvanceGameTime` (lines 1466-1523) | → `SessionEventService` |
| 2.2.12 | Remove ALL direct repository access from websocket.rs | ~15 locations |
| 2.2.13 | Refactor websocket.rs to pure message routing | Delegate all logic to services |

---

### Phase 3: Dependency Injection Fixes
**Goal**: Make all services properly injectable and testable
**Status**: PENDING

| Task | File | Change |
|------|------|--------|
| 3.1 | `dm_approval_queue_service.rs:33` | Inject `ToolExecutionService` via constructor |
| 3.2 | `llm_queue_service.rs:69` | Inject `LLMService` via constructor |
| 3.3 | `llm_queue_service.rs:420` | Inject `SuggestionService` via constructor |
| 3.4 | `challenge_outcome_approval_service.rs:214,506` | Inject `OutcomeSuggestionService` |
| 3.5 | Update `AppState` initialization to wire new dependencies | `infrastructure/state/mod.rs` |

---

### Phase 4: Player Component Refactoring
**Goal**: Split large components, fix error handling, improve type safety
**Status**: PENDING

#### 4.1 Component Splitting

| Component | Lines | Split Into |
|-----------|-------|-----------|
| `generation_queue.rs` | 900 | `GenerationQueueList`, `GenerationQueueFilters`, `GenerationBatchActions`, `QueueItemRow` |
| `challenge_roll.rs` | 740 | `ChallengeInputPhase`, `ChallengeWaitingPhase`, `ChallengeResultPhase` |
| `approval_popup.rs` | 705 | `ApprovalHeader`, `DialogueApproval`, `ChallengeApproval`, `NarrativeApproval` |
| `workflow_config_editor.rs` | 834 | `WorkflowEditor`, `WorkflowTester`, `WorkflowSectionEditor` |
| `skills_panel.rs` | 675 | `SkillsList`, `SkillEditor`, `SkillCategoryGroup` |

#### 4.2 Error Handling Fixes

| Location | Current | Fix |
|----------|---------|-----|
| `edit_character_modal.rs:52-55` | `Err(_) => {}` | Log error with `tracing::warn!` |
| `character_panel.rs:37-40` | `Err(_) => {}` | Log error, show user feedback |
| `challenge_roll.rs:458` | `Err(_) => {}` | Log parse error |
| `websocket/client.rs:142` | `.unwrap()` | Use `?` or `.expect()` with context |

#### 4.3 Type Safety Improvements

| Task | Change |
|------|--------|
| 4.3.1 | Update `Character.id` from `String` to `CharacterId` |
| 4.3.2 | Update `Scene.id`, `Scene.location_id` to use domain types |
| 4.3.3 | Update `Location.id` to `LocationId` |
| 4.3.4 | Update `World.id` to `WorldId` |
| 4.3.5 | Propagate type changes through components |

---

### Phase 5: DTO Validation & Boundary Cleanup
**Goal**: Proper validation, clean layer boundaries
**Status**: PENDING

#### 5.1 Add DTO Validation

Add `validator` crate and validation derives:

| DTO | Validations |
|-----|-------------|
| `CreateWorldRequestDto` | `name`: non-empty, max 255 chars |
| `CreateCharacterRequestDto` | `name`: non-empty, max 255 chars |
| `CreateLocationRequestDto` | `name`: non-empty, max 255 chars; `description`: max 10000 chars |
| `CreateChallengeRequestDto` | `skill_id`: valid UUID format |
| `CreateSceneRequestDto` | `name`: non-empty |

#### 5.2 Move DTOs from Route Files

| Source File | DTOs to Move | Target |
|-------------|-------------|--------|
| `character_routes.rs` | `CreateRegionRelationshipRequest`, `RegionRelationshipResponse` | `application/dto/character.rs` |
| `player_character_routes.rs` | 8 DTOs | `application/dto/player_character.rs` |
| `session_routes.rs` | `CreateSessionRequest`, `GameTimeResponse`, `AdvanceGameTimeRequest` | `application/dto/session.rs` |
| `region_routes.rs` | 7 DTOs | `application/dto/region.rs` |
| `observation_routes.rs` | 3 DTOs | `application/dto/observation.rs` |
| `suggestion_routes.rs` | `SuggestionQueuedResponse` | `application/dto/suggestion.rs` |

#### 5.3 Configuration Cleanup

| Task | File | Change |
|------|------|--------|
| 5.3.1 | `config.rs:77,81` | Change default URLs from `10.8.0.6` to `localhost` |
| 5.3.2 | `location_service.rs:16-17` | Remove duplicate constants, use `SettingsService` |
| 5.3.3 | `generation_service.rs:598-606` | Extract magic numbers to constants/config |

---

### Phase 6: Interface Segregation (Repository Traits)
**Goal**: Split large repository traits into cohesive interfaces
**Status**: PENDING

#### 6.1 `StoryEventRepositoryPort` (35 methods)

Split into:
- `StoryEventCrudPort` (create, get, update, delete)
- `StoryEventQueryPort` (list_by_*, search, filter)
- `StoryEventRelationsPort` (link_to_*, unlink_from_*)

#### 6.2 `CharacterRepositoryPort` (28 methods)

Split into:
- `CharacterCrudPort` (basic CRUD)
- `CharacterInventoryPort` (inventory operations)
- `CharacterLocationPort` (home, work, frequents, avoids regions)
- `CharacterWantsPort` (wants/desires management)

#### 6.3 `ChallengeRepositoryPort` (27 methods)

Split into:
- `ChallengeCrudPort` (basic CRUD)
- `ChallengePrerequisitesPort` (prerequisite management)
- `ChallengeAvailabilityPort` (location binding, unlocking)

---

### Phase 7: Cleanup & Testing Infrastructure
**Goal**: Remove dead code, add test infrastructure
**Status**: PENDING

| Task | Description |
|------|-------------|
| 7.1 | Remove or implement `GridMapRepositoryPort` |
| 7.2 | Remove or implement `ItemRepositoryPort` |
| 7.3 | Remove or implement `GoalRepositoryPort` |
| 7.4 | Remove or implement `WantRepositoryPort` |
| 7.5 | Remove or implement `RepositoryProvider` facade |
| 7.6 | Fix `MockGameConnectionPort` - add missing methods |
| 7.7 | Create `MockLlmPort` for Engine testing |
| 7.8 | Create `MockComfyUIPort` for Engine testing |
| 7.9 | Create `MockQueuePort` for Engine testing |
| 7.10 | Combine circuit breaker mutexes in `comfyui.rs:52-54` |
| 7.11 | Extract `parse_datetime_or_now()` utility |
| 7.12 | Remove duplicate datetime parsing (~20 locations) |

---

## Dependency Graph

```
Phase 0 (Repo Consolidation)
    │
    ▼
Phase 1 (Protocol Crate) ──────────────────┐
    │                                       │
    ▼                                       │
Phase 2 (WebSocket Refactor)                │
    │                                       │
    ├───────────────┬───────────────┐       │
    ▼               ▼               ▼       │
Phase 3         Phase 4         Phase 5     │
(DI Fixes)    (Player UI)    (DTO/Config)   │
    │               │               │       │
    └───────────────┴───────────────┘       │
                    │                       │
                    ▼                       │
              Phase 6 (Interface Split) ◄───┘
                    │
                    ▼
              Phase 7 (Cleanup)
```

---

## Estimated Effort Summary

| Phase | Tasks | Estimated Effort |
|-------|-------|------------------|
| 0 - Repo Consolidation | 9 | ✅ COMPLETE |
| 1 - Protocol Crate | 11 | 2-3 days |
| 2 - WebSocket Refactor | 13 | 3-5 days |
| 3 - DI Fixes | 5 | 0.5 days |
| 4 - Player Components | 12 | 2-3 days |
| 5 - DTO/Validation | 10 | 1-2 days |
| 6 - Interface Split | 4 | 1-2 days |
| 7 - Cleanup/Testing | 12 | 1-2 days |
| **Total** | **76 tasks** | **~12-19 days** |

---

## Notes

- Old repositories (Engine/, Player/, plans/) are kept as historical archives
- New Game repository is at `git@github.com:WrldBld/Game.git`
- Unused repository ports (GridMap, Item, Goal, Want) will be addressed in future feature work, not removed
