# Hexagonal Architecture Gap Remediation Plan

**Status**: FINAL - Ready for Implementation  
**Created**: 2025-12-28  
**Updated**: 2025-12-28 (Final validation with deep codebase analysis)  
**Priority**: High - Clean architecture finalization  
**Estimated Total Effort**: 13-16 hours

## Executive Summary

This plan finalizes the hexagonal architecture refactor for WrldBldr. After extensive validation by multiple agents, we have identified the true remaining work and eliminated phantom issues.

### Key Validation Findings

1. **G1 is INVALID** - Use-case-specific DTOs and ports are valid hexagonal patterns
2. **G2 should be DELETED** - `app_event_repository_port.rs` is a true violation (protocol in ports layer)
3. **G3 does NOT require GameConnectionPort refactor** - The generic `request(RequestPayload)` pattern is correct; we only need app-layer DTOs
4. **Engine-app has port consolidation issues** - 6 ports need attention (duplicates, infrastructure ports in wrong place)
5. **arch-check passes** - Current exemptions are documented and tracked

### Architecture Status

```
┌─────────────────────────────────────────────────────────────────────┐
│                        CURRENT ARCHITECTURE                          │
├─────────────────────────────────────────────────────────────────────┤
│  Domain Layer              ✅ CLEAN (0 violations)                   │
│  Protocol Layer            ⚠️  8 re-exports from domain (approved)   │
│  Engine-Ports Layer        ⚠️  1 violation (app_event_repository)    │
│  Engine-App Layer          ⚠️  6 ports need consolidation            │
│  Engine-Adapters Layer     ✅ CLEAN                                  │
│  Player-Ports Layer        ⚠️  2 protocol imports (tracked Phase P2) │
│  Player-App Layer          ❌ 14 services import protocol types      │
│  Player-Adapters Layer     ✅ CLEAN                                  │
│  Player-UI Layer           ✅ CLEAN                                  │
└─────────────────────────────────────────────────────────────────────┘
```

### Gaps Summary

| ID | Severity | Issue | Status |
|----|----------|-------|--------|
| G1 | ~~Critical~~ | ~~DirectorialContext duplication~~ | **INVALID** |
| G2 | **High** | `app_event_repository_port.rs` imports protocol | Active |
| G3 | **High** | Player-app services import protocol Create/Update types | Active |
| G4 | **Low** | 2 handlers not using `IntoServerError` | Active |
| G5 | **Low** | Player-app dto re-exports protocol types | Active |
| G6 | **Medium** | Handler split incomplete | Deferred |
| G7 | **Low** | Protocol re-exports domain types | Document as Exception |
| G8 | **Low** | Player-app DTO re-exports | Document as Exception |
| G9 | **Medium** | Engine-app port consolidation | **NEW** |

---

## Phase 0: Baseline Verification

**Estimated Time**: 30 minutes  
**Purpose**: Document current state before changes

### Steps

```bash
# 1. Verify build
cargo check --workspace

# 2. Run architecture check
cargo xtask arch-check

# 3. Document current violations
grep -r "AppEventRepositoryPort" crates/ | wc -l
grep -l "use wrldbldr_protocol::" crates/player-app/src/application/services/*.rs | wc -l
```

### Expected Results

- arch-check: PASS (with documented exemptions)
- AppEventRepositoryPort usages: ~6 files
- Player-app services with protocol imports: 14 files

---

## Phase 1: Foundation & Documentation (2-3 hours)

### 1.1 Update CLAUDE.md with Architecture Rules

Add the following section to `CLAUDE.md`:

```markdown
## Hexagonal Architecture Rules

### Layer Structure

```
RUNNERS: Composition root, wires adapters to ports
PRESENTATION: UI components, views (player-ui only)
ADAPTERS: Implements ports, handles I/O, external systems
         ONLY layer that should construct protocol types for wire conversion
APPLICATION: Services, use cases, app-layer DTOs
            May define use-case-specific port traits (dependency injection)
PORTS: Infrastructure port traits (repos, external services)
PROTOCOL: Wire-format DTOs, shared Engine↔Player types
          May re-export stable domain types for serialization
DOMAIN: Entities, value objects, domain events
        Zero external dependencies
```

### Port Ownership Rules

**Infrastructure ports** (`*-ports` crates):
- Repository traits (CharacterRepositoryPort, LocationRepositoryPort)
- External service traits (LlmPort, ComfyUIPort, BroadcastPort)
- Connection/transport traits (GameConnectionPort)

**Use-case ports** (`*-app` crates, allowed):
- Service facade traits injected into use cases
- Use-case-specific abstractions for dependency injection
- Example: `SceneServicePort`, `ChallengeResolutionPort`

### Protocol Import Rules

| Layer | Protocol Imports | Rationale |
|-------|------------------|-----------|
| domain | FORBIDDEN | Pure business logic |
| *-ports | FORBIDDEN (except API boundaries) | Infrastructure contracts |
| *-app/use_cases | FORBIDDEN | Business logic orchestration |
| *-app/services | Use app-layer DTOs, convert before calling port | Service layer isolation |
| *-app/handlers | ALLOWED | Boundary layer, converts protocol↔domain |
| *-adapters | ALLOWED | Implements wire-format conversion |
| player-ui | ALLOWED | Presentation boundary |

### GameConnectionPort Pattern

The `GameConnectionPort::request(payload: RequestPayload)` method is the correct design:
- Generic method handles all 118+ RequestPayload variants
- Individual methods exist only for operations needing special handling
- Services create app-layer DTOs, convert to RequestPayload before calling `request()`
- This is NOT a violation to fix
```

### 1.2 Delete app_event_repository_port.rs (G2)

**Problem**: Ports layer imports protocol type directly.

**Files to modify**:

| File | Action |
|------|--------|
| `crates/engine-ports/src/outbound/app_event_repository_port.rs` | DELETE |
| `crates/engine-ports/src/outbound/mod.rs` | Remove export |
| `crates/engine-adapters/src/infrastructure/repositories/sqlite_app_event_repository.rs` | Convert to internal adapter module |
| `crates/engine-adapters/src/infrastructure/event_bus/sqlite_event_bus.rs` | Use DomainEventRepositoryPort, convert internally |
| `crates/engine-adapters/src/infrastructure/websocket_event_subscriber.rs` | Use DomainEventRepositoryPort, convert internally |
| `crates/engine-adapters/src/infrastructure/state/event_infra.rs` | Remove app_event_repository field |
| `crates/engine-adapters/src/infrastructure/state/mod.rs` | Update wiring |
| `crates/xtask/src/main.rs` | Remove exemption for app_event_repository_port.rs |

**Implementation approach**:
1. `SqliteEventBus` converts `DomainEvent` → `AppEvent` internally before storage
2. `WebSocketEventSubscriber` receives `DomainEvent`, converts to `ServerMessage` internally
3. `SqliteAppEventRepository` becomes an internal adapter module (not a port implementation)

### 1.3 Document Protocol Re-exports (G7)

**Files to modify**:

| File | Action |
|------|--------|
| `crates/protocol/src/types.rs` | Add ARCHITECTURE EXCEPTION comment |
| `crates/protocol/src/rule_system.rs` | Add ARCHITECTURE EXCEPTION comment |

**Comment to add**:
```rust
// ARCHITECTURE EXCEPTION: [APPROVED 2025-12-28]
// Re-exports stable domain types for wire serialization. Domain remains
// the canonical source. These types have serde derives and are used
// unchanged in protocol messages.
// See: docs/architecture/hexagonal-architecture.md
```

---

## Phase 2: Player-App DTO Cleanup (6-8 hours)

### Important: NOT a GameConnectionPort Refactor

The validation confirmed:
- `GameConnectionPort::request(RequestPayload)` is the **correct** pattern
- 111+ RequestPayload variants would require 150+ individual methods (anti-pattern)
- We only need to isolate **service-level** protocol type construction

### Special Case: actantial_service.rs

This service has **deeper protocol coupling** than others and requires extra work:
- Returns `NpcActantialContextData` directly (complex nested type)
- Embeds 5 protocol enums in its DTOs (`WantVisibilityData`, `WantTargetTypeData`, `ActorTypeData`, `ActantialRoleData`)
- Requires creating app-layer versions of nested types

**Additional effort**: +1-2 hours on top of standard service updates

### 2.1 Create player-app/dto/requests.rs

**File**: `crates/player-app/src/application/dto/requests.rs`

Create app-layer DTOs for the 11 protocol types currently imported:

| App DTO | Replaces Protocol Type |
|---------|------------------------|
| `CreateWorldRequest` | `CreateWorldData` |
| `CreateChallengeRequest` | `CreateChallengeData` |
| `UpdateChallengeRequest` | `UpdateChallengeData` |
| `CreateSkillRequest` | `CreateSkillData` |
| `UpdateSkillRequest` | `UpdateSkillData` |
| `CreateCharacterRequest` | `CreateCharacterData` |
| `UpdateCharacterRequest` | `UpdateCharacterData` |
| `ChangeArchetypeRequest` | `ChangeArchetypeData` |
| `CreateLocationRequest` | `CreateLocationData` |
| `UpdateLocationRequest` | `UpdateLocationData` |
| `CreateLocationConnectionRequest` | `CreateLocationConnectionData` |
| `CreateEventChainRequest` | `CreateEventChainData` |
| `UpdateEventChainRequest` | `UpdateEventChainData` |
| `CreateNarrativeEventRequest` | `CreateNarrativeEventData` |
| `CreateGoalRequest` | `CreateGoalData` |
| `UpdateGoalRequest` | `UpdateGoalData` |
| `CreateWantRequest` | `CreateWantData` |
| `UpdateWantRequest` | `UpdateWantData` |
| `CreatePlayerCharacterRequest` | `CreatePlayerCharacterData` |
| `UpdatePlayerCharacterRequest` | `UpdatePlayerCharacterData` |
| `CreateDmMarkerRequest` | `CreateDmMarkerData` |

### 2.2 Add From Conversions

**Location**: Can be in `player-app/dto/requests.rs` (Rust orphan rules allow `impl From<LocalType> for ForeignType`)

```rust
impl From<CreateWorldRequest> for wrldbldr_protocol::CreateWorldData {
    fn from(req: CreateWorldRequest) -> Self {
        Self {
            name: req.name,
            description: req.description,
            // ... fields
        }
    }
}
```

### 2.3 Update Services

Update these 11 service files to use app-layer DTOs:

| Service File | Protocol Imports to Replace |
|--------------|----------------------------|
| `world_service.rs` | `CreateWorldData` |
| `challenge_service.rs` | `CreateChallengeData`, `UpdateChallengeData` |
| `skill_service.rs` | `CreateSkillData`, `UpdateSkillData` |
| `character_service.rs` | `CreateCharacterData`, `UpdateCharacterData`, `ChangeArchetypeData` |
| `location_service.rs` | `CreateLocationData`, `UpdateLocationData`, `CreateLocationConnectionData` |
| `event_chain_service.rs` | `CreateEventChainData`, `UpdateEventChainData` |
| `narrative_event_service.rs` | `CreateNarrativeEventData` |
| `actantial_service.rs` | `CreateGoalData`, `UpdateGoalData`, `CreateWantData`, `UpdateWantData` + enum types (see below) |
| `player_character_service.rs` | `CreatePlayerCharacterData`, `UpdatePlayerCharacterData` |
| `story_event_service.rs` | `CreateDmMarkerData` |

**Pattern for services**:
```rust
// Before:
use wrldbldr_protocol::CreateChallengeData;
let data = CreateChallengeData { ... };
let payload = RequestPayload::CreateChallenge { world_id, data };

// After:
use crate::application::dto::requests::CreateChallengeRequest;
let request = CreateChallengeRequest { ... };
let payload = RequestPayload::CreateChallenge { world_id, data: request.into() };
```

### 2.4 Special Handling: actantial_service.rs

This service requires additional app-layer types due to deep protocol coupling:

**Enums to create** (in `player-app/dto/actantial.rs`):
| App Enum | Protocol Enum |
|----------|---------------|
| `AppWantVisibility` | `WantVisibilityData` |
| `AppWantTargetType` | `WantTargetTypeData` |
| `AppActorType` | `ActorTypeData` |
| `AppActantialRole` | `ActantialRoleData` |

**Structs to create**:
| App Struct | Protocol Struct |
|------------|-----------------|
| `AppWantTarget` | `WantTargetData` |
| `AppNpcActantialContext` | `NpcActantialContextData` |
| `AppWantData` | `WantData` |
| `AppActantialActor` | `ActantialActorData` |

**Method to update**:
- `get_actantial_context()` - Change return type from `NpcActantialContextData` to `AppNpcActantialContext`

### 2.5 Update xtask Exemptions

After cleaning services, remove them from the exempt list in `crates/xtask/src/main.rs`:

```rust
// Remove from exempt_files:
// "skill_service.rs", "challenge_service.rs", "character_service.rs",
// "location_service.rs", "event_chain_service.rs", "narrative_event_service.rs",
// "actantial_service.rs", "world_service.rs", "story_event_service.rs",
// "player_character_service.rs"
```

---

## Phase 3: Engine Port Consolidation (3-4 hours)

### 3.1 Fix Internal Duplicates

**Issue**: `WorldStatePort` is defined twice within engine-app:
- `connection.rs:211`
- `scene.rs:227`

**Action**: Consolidate into a single definition, likely in a shared module.

### 3.2 Remove Cross-Crate Duplicates

| Port in engine-app | Equivalent in engine-ports | Action |
|--------------------|---------------------------|--------|
| `ObservationRepositoryPort` (observation.rs:100) | `ObservationRepositoryPort` (repository_port.rs:1709) | Remove from engine-app, use engine-ports |
| `DirectorialContextRepositoryPort` (scene.rs:250) | `DirectorialContextRepositoryPort` (directorial_context_port.rs:14) | Keep in engine-app (use-case specific signature) |

### 3.3 Move Infrastructure Ports to engine-ports

| Port | Current Location | Recommendation |
|------|-----------------|----------------|
| `ConnectionManagerPort` | connection.rs:117 | MOVE to engine-ports |
| `WorldStatePort` | connection.rs:211, scene.rs:227 | CONSOLIDATE then MOVE |
| `StagingStatePort` | movement.rs:130 | MOVE to engine-ports |
| `StagingStateExtPort` | staging.rs:134 | MOVE with StagingStatePort |

### 3.4 Notification Ports - Keep Separate (NOT Duplicates)

These ports serve **different purposes** at different layers and should NOT be consolidated:

| Port | Location | Purpose | Recommendation |
|------|----------|---------|----------------|
| `BroadcastPort` | engine-ports | Generic WebSocket broadcast | Keep - infrastructure |
| `DmNotificationPort` | engine-app/player_action.rs | DM-specific action notifications | Keep - use-case specific |
| `WorldMessagePort` | engine-app/observation.rs | World event broadcasts | Keep - use-case specific |

These are proper hexagonal layering where use-cases define their notification needs and adapters implement them.

### 3.5 Document Use-Case Ports

These ports are **valid** in engine-app (use-case dependency injection pattern):

- `ChallengeResolutionPort`
- `ChallengeOutcomeApprovalPort`
- `DmApprovalQueuePort`
- `WorldServicePort`
- `PlayerCharacterServicePort`
- `SceneServicePort`
- `InteractionServicePort`
- `DmActionQueuePort`
- `StagingServicePort`
- `DmNotificationPort` (use-case specific notification)
- `WorldMessagePort` (use-case specific notification)

Add documentation explaining why they're in engine-app.

---

## Phase 4: Polish (1-2 hours)

### 4.1 G4: IntoServerError Migration

**Files**:
- `crates/engine-adapters/src/infrastructure/websocket/handlers/movement.rs`
- `crates/engine-adapters/src/infrastructure/websocket/handlers/narrative.rs`

**Action**: Replace local error conversion functions with `IntoServerError` trait.

### 4.2 G5/G8: Re-export Cleanup

**Option A (Recommended)**: Document as approved exceptions with comments.

**Option B**: Remove re-exports and update consumers to import directly.

### 4.3 Enable Full arch-check Enforcement

After all phases complete:
1. Remove remaining xtask exemptions
2. Uncomment the `bail!` in `check_player_ports_protocol_isolation()` (line 776)
3. Verify all checks pass

---

## Implementation Order

| Phase | Description | Effort | Dependencies |
|-------|-------------|--------|--------------|
| 0 | Baseline verification | 30 min | None |
| 1 | Foundation + G2 + G7 | 2-3 hrs | None |
| 2 | Player-app DTOs (G3) | 6-8 hrs | Phase 1 |
| 2a | Standard services (10 files) | 4-5 hrs | - |
| 2b | actantial_service.rs (special) | 2-3 hrs | - |
| 3 | Engine port consolidation (G9) | 3-4 hrs | Phase 1 |
| 4 | Polish (G4, G5, G8) | 1-2 hrs | Phases 2, 3 |

**Total**: 13-16 hours

Phases 2 and 3 can be done in parallel after Phase 1.

---

## Verification Commands

```bash
# After Phase 1
grep -r "AppEventRepositoryPort" crates/  # Should be 0
cargo xtask arch-check  # Should pass

# After Phase 2
grep -r "use wrldbldr_protocol::Create" crates/player-app/src/application/services/  # Should be 0
grep -r "use wrldbldr_protocol::Update" crates/player-app/src/application/services/  # Should be 0
cargo check --workspace

# After Phase 3
grep -rn "trait WorldStatePort" crates/engine-app/  # Should be 1 or 0
cargo check --workspace

# Final
cargo xtask arch-check  # Should pass with no exemptions needed
```

---

## Success Criteria

| Metric | Before | After |
|--------|--------|-------|
| arch-check | Pass (with exemptions) | Pass (minimal exemptions) |
| AppEventRepositoryPort usages | 6 files | 0 |
| Protocol Create/Update in player-app services | 14 files | 0 |
| Duplicate ports in engine-app | 2 | 0 |
| Infrastructure ports in engine-app | 4 | 0 |
| Documented architecture patterns | Incomplete | Complete |

---

## Appendix A: Files Affected Summary

### To Delete
- `crates/engine-ports/src/outbound/app_event_repository_port.rs`

### To Create
- `crates/player-app/src/application/dto/requests.rs`
- `crates/player-app/src/application/dto/actantial.rs` (app-layer enums/structs for actantial service)

### To Modify (Phase 1)
- `crates/engine-ports/src/outbound/mod.rs`
- `crates/engine-adapters/src/infrastructure/event_bus/sqlite_event_bus.rs`
- `crates/engine-adapters/src/infrastructure/websocket_event_subscriber.rs`
- `crates/engine-adapters/src/infrastructure/state/event_infra.rs`
- `crates/engine-adapters/src/infrastructure/state/mod.rs`
- `crates/protocol/src/types.rs`
- `crates/protocol/src/rule_system.rs`
- `CLAUDE.md`

### To Modify (Phase 2)
- `crates/player-app/src/application/dto/mod.rs`
- 11 player-app service files

### To Modify (Phase 3)
- `crates/engine-app/src/application/use_cases/connection.rs`
- `crates/engine-app/src/application/use_cases/scene.rs`
- `crates/engine-app/src/application/use_cases/movement.rs`
- `crates/engine-app/src/application/use_cases/staging.rs`
- `crates/engine-app/src/application/use_cases/observation.rs`
- `crates/engine-ports/src/outbound/mod.rs` (add moved ports)

### To Modify (Phase 4)
- `crates/engine-adapters/src/infrastructure/websocket/handlers/movement.rs`
- `crates/engine-adapters/src/infrastructure/websocket/handlers/narrative.rs`
- `crates/player-app/src/application/dto/mod.rs`
- `crates/xtask/src/main.rs`

---

## Appendix B: Critical Design Decisions

### GameConnectionPort is Correct

The `GameConnectionPort::request(payload: RequestPayload)` method is the **right design**:
- 118 RequestPayload variants exist
- Individual methods would require 150+ trait methods (anti-pattern)
- The generic approach is clean and extensible
- Services construct app DTOs, convert to RequestPayload, then call `request()`

**This is NOT a violation to fix.**

### Use-Case Ports in engine-app are Valid

Having use-case-specific port traits in `engine-app` is a **valid hexagonal pattern**:
- They represent use-case dependencies (dependency injection)
- They're different from infrastructure ports (which go in `*-ports`)
- Examples: `SceneServicePort`, `ChallengeResolutionPort`

**Document this pattern, don't "fix" it.**

### Protocol Re-exports are Acceptable

Protocol re-exporting stable domain types (enums, simple structs) is an **approved exception**:
- Reduces import complexity for consumers
- Domain remains the canonical source
- Types are serde-ready and stable

**Document with ARCHITECTURE EXCEPTION comments.**

---

## Appendix C: actantial_service.rs Deep Analysis

This service has the deepest protocol coupling in player-app and requires special handling.

### Protocol Types Imported (11 total)

| Type | Category | Usage |
|------|----------|-------|
| `CreateGoalData` | Request DTO | Internal construction |
| `UpdateGoalData` | Request DTO | Internal construction |
| `CreateWantData` | Request DTO | Internal construction |
| `UpdateWantData` | Request DTO | Internal construction |
| `WantVisibilityData` | Enum | Embedded in request/response structs |
| `WantTargetTypeData` | Enum | Embedded in request structs |
| `ActorTypeData` | Enum | Embedded in request structs |
| `ActantialRoleData` | Enum | Embedded in request structs |
| `NpcActantialContextData` | Struct | **Returned directly from `get_actantial_context()`** |
| `WantTargetData` | Struct | Embedded in response structs |
| `RequestPayload` | Enum | WebSocket request construction |

### Methods Affected

| Method | Protocol Leak | Fix Required |
|--------|---------------|--------------|
| `get_actantial_context()` | Returns `NpcActantialContextData` | Change return type to app-layer DTO |
| `list_wants()` | `WantResponse` embeds protocol enums | Update `WantResponse` fields |
| `create_want()` | Same as above | Same fix |
| `update_want()` | Same as above | Same fix |
| `set_want_target()` | Request uses `WantTargetTypeData` | Update request type |
| `add_actantial_view()` | Request uses `ActorTypeData`, `ActantialRoleData` | Update request type |
| `remove_actantial_view()` | Same as above | Same fix |

### Estimated New Code

- ~150-200 lines of new DTO definitions
- ~80-100 lines of `From` trait implementations
- All existing method signatures can remain (except `get_actantial_context` return type)

---

## Appendix D: Notification Ports Clarification

Three notification-related ports exist, but they are **NOT duplicates**:

```
┌─────────────────────────────────────────────────────────────────┐
│  engine-ports (Infrastructure Layer)                            │
│  └── BroadcastPort                                               │
│      - Generic WebSocket broadcast to all/some connections       │
│      - Infrastructure concern                                    │
├─────────────────────────────────────────────────────────────────┤
│  engine-app (Application Layer)                                  │
│  ├── DmNotificationPort (player_action.rs)                       │
│  │   - Notifies DM when player action is queued                  │
│  │   - Use-case specific: "action queued" semantics              │
│  │                                                               │
│  └── WorldMessagePort (observation.rs)                           │
│      - Sends events to users in a world                          │
│      - Use-case specific: "approach event", "location event"     │
└─────────────────────────────────────────────────────────────────┘
```

The use-case ports define **what** needs to happen (domain semantics).
The infrastructure port defines **how** it happens (WebSocket mechanics).
Adapters bridge them.

---

## Appendix E: Related Documentation

- Architecture overview: `docs/architecture/hexagonal-architecture.md`
- Original master plan: `docs/plans/HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md`
- Queue system: `docs/architecture/queue-system.md`
- WebSocket protocol: `docs/architecture/websocket-protocol.md`
