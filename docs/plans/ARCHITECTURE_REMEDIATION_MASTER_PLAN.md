# Architecture Remediation Master Plan (v2)

**Status**: ACTIVE (authoritative remediation plan)  
**Created**: 2026-01-01  
**Last Updated**: 2026-01-01  
**Supersedes**: `HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md`

This is the consolidated plan addressing all architectural issues identified in the comprehensive code review. It builds upon the completed work from the previous master plan (Phases 1-7) and adds new phases to address remaining gaps.

## Progress Summary

| Track | Completed | Remaining | Next Priority |
|-------|-----------|-----------|---------------|
| A: Engine Domain/DTOs | 0/4 | 4 | 1A.1 (Queue Data Migration) |
| B: Composition/Ports | 1/4 | 3 | 2A (Composition Root Refactor) |
| C: Player/Docs | 1/3 | 2 | 3B (UI Error Feedback Audit) |
| D: Code Quality | 1/3 | 2 | 4B (Remove Blanket Impl) |
| **Total** | **3/14** | **11** | |

### Completed Phases

| Phase | Description | Commit | Date |
|-------|-------------|--------|------|
| 2C | Fix player_events.rs docstring | `b8e76a0` | 2026-01-01 |
| 3A | Replace UI dice parsing with domain DiceFormula | `b8e76a0` | 2026-01-01 |
| 4A | Fix magic number in world_connection_manager.rs | `b8e76a0` | 2026-01-01 |

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Diagrams](#architecture-diagrams)
3. [Acceptance Criteria](#acceptance-criteria)
4. [Ground Rules](#ground-rules)
5. [Phase Overview](#phase-overview)
6. [Detailed Phases](#detailed-phases)
7. [Verification Checklist](#verification-checklist)
8. [Appendices](#appendices)

---

## Executive Summary

### What Was Completed (Previous Plan)

The original `HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md` successfully addressed:

- **Phase 1**: Eliminated glob re-exports
- **Phase 2**: Consolidated DTO ownership (StagingProposal, shadow types)
- **Phase 3**: Fixed port taxonomy (inbound/outbound classification)
- **Phase 4**: Removed port adapter wrapper anti-patterns
- **Phase 5**: Dependency inversion in engine-app
- **Phase 6**: Fixed IoC violations (services constructing services)
- **Phase 7**: Composition-root purity (no concrete types where ports exist)

### What Remains (This Plan)

Issues identified in the comprehensive code review that need remediation:

| Category | Count | Priority |
|----------|-------|----------|
| Domain Layer Violations | 1 major (queue_data.rs) | HIGH |
| DTO Duplication | 4 types duplicated 2-4x | HIGH |
| Composition Root Issues | 2 (duplicate instantiation, complexity) | HIGH |
| Port Layer Corrections | 4 (naming, placement, docs) | MEDIUM |
| Player-Side Architecture | 2 (dice logic, error handling) | MEDIUM |
| Documentation Drift | 5 items | LOW |
| Code Quality | 4 items | LOW |

### Parallelization Strategy

The phases are organized to maximize parallel work:

```
PARALLEL TRACK A          PARALLEL TRACK B          PARALLEL TRACK C
(Engine Domain/DTOs)      (Composition/Ports)       (Player/Docs)
─────────────────────     ─────────────────────     ─────────────────────
Phase 1A: Queue Data  ──► Phase 2A: Composition ──► Phase 3A: Player UI
Phase 1B: DTO Consol. ──► Phase 2B: Port Naming ──► Phase 3B: Documentation
                          Phase 2C: Port Placement
```

---

## Architecture Diagrams

### Idealized System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                    PLAYER CLIENT                                         │
│  ┌─────────────────────────────────────────────────────────────────────────────────────┐│
│  │                              player-ui (Dioxus)                                      ││
│  │   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐            ││
│  │   │  Components  │  │    Views     │  │    State     │  │   Handlers   │            ││
│  │   │  (present.)  │  │  (routes)    │  │  (signals)   │  │  (events)    │            ││
│  │   └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘            ││
│  │          │                 │                 │                 │                     ││
│  │          └─────────────────┴────────┬────────┴─────────────────┘                     ││
│  │                                     ▼                                                ││
│  │                          ┌──────────────────┐                                        ││
│  │                          │  player-app      │                                        ││
│  │                          │  (services)      │◄──── Boundary: Uses protocol DTOs     ││
│  │                          └────────┬─────────┘                                        ││
│  │                                   │                                                  ││
│  │                          ┌────────▼─────────┐                                        ││
│  │                          │  player-ports    │                                        ││
│  │                          │  (abstractions)  │                                        ││
│  │                          └────────┬─────────┘                                        ││
│  │                                   │                                                  ││
│  │                          ┌────────▼─────────┐                                        ││
│  │                          │ player-adapters  │                                        ││
│  │                          │ (WebSocket, etc) │                                        ││
│  │                          └────────┬─────────┘                                        ││
│  └───────────────────────────────────┼──────────────────────────────────────────────────┘│
└──────────────────────────────────────┼──────────────────────────────────────────────────┘
                                       │
                          ═══════════════════════════
                          ║   WebSocket (protocol)  ║  ◄── Wire format: JSON messages
                          ═══════════════════════════
                                       │
┌──────────────────────────────────────┼──────────────────────────────────────────────────┐
│                                 ENGINE SERVER                                            │
│  ┌───────────────────────────────────┼──────────────────────────────────────────────────┐│
│  │                          ┌────────▼─────────┐                                        ││
│  │                          │ engine-adapters  │                                        ││
│  │    ┌─────────────────────┤ (driving side)   ├─────────────────────┐                  ││
│  │    │                     │  - Axum/HTTP     │                     │                  ││
│  │    │                     │  - WebSocket     │                     │                  ││
│  │    │                     └────────┬─────────┘                     │                  ││
│  │    │                              │                               │                  ││
│  │    │                     ┌────────▼─────────┐                     │                  ││
│  │    │                     │  engine-ports    │                     │                  ││
│  │    │                     │  (inbound)       │                     │                  ││
│  │    │                     │  - UseCasePorts  │                     │                  ││
│  │    │                     │  - RequestHandler│                     │                  ││
│  │    │                     └────────┬─────────┘                     │                  ││
│  │    │                              │                               │                  ││
│  │    │                     ┌────────▼─────────┐                     │                  ││
│  │    │                     │   engine-app     │                     │                  ││
│  │    │                     │  ┌────────────┐  │                     │                  ││
│  │    │                     │  │  Handlers  │◄─┼── Protocol DTOs     │                  ││
│  │    │                     │  └─────┬──────┘  │     (boundary)      │                  ││
│  │    │                     │        │         │                     │                  ││
│  │    │                     │  ┌─────▼──────┐  │                     │                  ││
│  │    │                     │  │ Use Cases  │  │                     │                  ││
│  │    │                     │  └─────┬──────┘  │                     │                  ││
│  │    │                     │        │         │                     │                  ││
│  │    │                     │  ┌─────▼──────┐  │                     │                  ││
│  │    │                     │  │  Services  │  │  ◄── Pure business  │                  ││
│  │    │                     │  └─────┬──────┘  │      orchestration  │                  ││
│  │    │                     └────────┼─────────┘                     │                  ││
│  │    │                              │                               │                  ││
│  │    │                     ┌────────▼─────────┐                     │                  ││
│  │    │                     │  engine-ports    │                     │                  ││
│  │    │                     │  (outbound)      │                     │                  ││
│  │    │                     │  - Repositories  │                     │                  ││
│  │    │                     │  - ServicePorts  │                     │                  ││
│  │    │                     │  - CachePorts    │                     │                  ││
│  │    │                     └────────┬─────────┘                     │                  ││
│  │    │                              │                               │                  ││
│  │    │                     ┌────────▼─────────┐                     │                  ││
│  │    └────────────────────►│ engine-adapters  │◄────────────────────┘                  ││
│  │                          │ (driven side)    │                                        ││
│  │                          │  - Neo4j repos   │                                        ││
│  │                          │  - Ollama LLM    │                                        ││
│  │                          │  - ComfyUI       │                                        ││
│  │                          │  - SQLite queues │                                        ││
│  │                          │  - In-memory     │                                        ││
│  │                          └────────┬─────────┘                                        ││
│  └───────────────────────────────────┼──────────────────────────────────────────────────┘│
└──────────────────────────────────────┼──────────────────────────────────────────────────┘
                                       │
              ┌────────────────────────┼────────────────────────┐
              │                        │                        │
              ▼                        ▼                        ▼
    ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
    │     Neo4j       │    │     Ollama      │    │    ComfyUI      │
    │  (Graph DB)     │    │  (LLM Server)   │    │ (Image Gen)     │
    │                 │    │                 │    │                 │
    │  - Entities     │    │  - NPC dialogue │    │  - Portraits    │
    │  - Relations    │    │  - Suggestions  │    │  - Scenes       │
    │  - Game state   │    │  - Narratives   │    │  - Assets       │
    └─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Idealized Hexagonal Architecture

```
                              ┌─────────────────────────────────────┐
                              │         DRIVING ADAPTERS            │
                              │  (Primary / Left Side of Hexagon)   │
                              │                                     │
                              │  ┌─────────────┐  ┌──────────────┐  │
                              │  │ Axum HTTP   │  │  WebSocket   │  │
                              │  │  Handlers   │  │  Handlers    │  │
                              │  └──────┬──────┘  └──────┬───────┘  │
                              │         │                │          │
                              └─────────┼────────────────┼──────────┘
                                        │                │
                                        ▼                ▼
                              ┌─────────────────────────────────────┐
                              │         INBOUND PORTS               │
                              │   (What the application OFFERS)     │
                              │                                     │
                              │   - RequestHandler                  │
                              │   - ChallengeUseCasePort            │
                              │   - MovementUseCasePort             │
                              │   - StagingUseCasePort              │
                              │   - SceneUseCasePort                │
                              │   - ... (all *UseCasePort traits)   │
                              └──────────────────┬──────────────────┘
                                                 │
                                                 │ implements
                                                 ▼
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                                                                                          │
│                                    APPLICATION CORE                                      │
│                                                                                          │
│   ┌────────────────────────────────────────────────────────────────────────────────────┐ │
│   │                              HANDLERS (Boundary)                                   │ │
│   │   - Protocol DTO <-> Domain conversion                                             │ │
│   │   - Request validation                                                             │ │
│   │   - Response formatting                                                            │ │
│   └───────────────────────────────────────┬────────────────────────────────────────────┘ │
│                                           │                                              │
│                                           ▼                                              │
│   ┌────────────────────────────────────────────────────────────────────────────────────┐ │
│   │                              USE CASES                                             │ │
│   │   - Orchestrate complex workflows                                                  │ │
│   │   - Coordinate multiple services                                                   │ │
│   │   - Transaction boundaries                                                         │ │
│   │   - Implement inbound port traits                                                  │ │
│   └───────────────────────────────────────┬────────────────────────────────────────────┘ │
│                                           │                                              │
│                                           ▼                                              │
│   ┌────────────────────────────────────────────────────────────────────────────────────┐ │
│   │                              SERVICES                                              │ │
│   │   - Business logic orchestration                                                   │ │
│   │   - Coordinate domain operations                                                   │ │
│   │   - Depend on outbound ports (not concrete types)                                  │ │
│   └───────────────────────────────────────┬────────────────────────────────────────────┘ │
│                                           │                                              │
│                                           ▼                                              │
│   ┌────────────────────────────────────────────────────────────────────────────────────┐ │
│   │                              DOMAIN                                                │ │
│   │   - Entities (Character, World, Scene, Challenge, ...)                             │ │
│   │   - Value Objects (GameTime, Disposition, ActantialContext, ...)                   │ │
│   │   - Typed IDs (CharacterId, WorldId, SceneId, ...)                                 │ │
│   │   - Domain Events                                                                  │ │
│   │   - Business rules and invariants                                                  │ │
│   │   - NO framework dependencies, NO infrastructure types                             │ │
│   └────────────────────────────────────────────────────────────────────────────────────┘ │
│                                                                                          │
└──────────────────────────────────────────────────────────────────────────────────────────┘
                                                 │
                                                 │ depends on
                                                 ▼
                              ┌─────────────────────────────────────┐
                              │         OUTBOUND PORTS              │
                              │   (What the application NEEDS)      │
                              │                                     │
                              │   REPOSITORIES (Data Access)        │
                              │   - CharacterCrudPort               │
                              │   - CharacterWantPort               │
                              │   - WorldRepositoryPort             │
                              │   - SceneCrudPort, SceneQueryPort   │
                              │   - ... (ISP-split traits)          │
                              │                                     │
                              │   SERVICE PORTS (External Ops)      │
                              │   - LlmPort                         │
                              │   - ComfyUIPort                     │
                              │   - BroadcastPort                   │
                              │   - ConnectionBroadcastPort         │
                              │                                     │
                              │   CACHE PORTS (In-Memory State)     │
                              │   - SettingsCachePort               │
                              │   - PromptTemplateCachePort         │
                              │   - ActiveGenerationBatchesPort     │
                              │                                     │
                              │   INFRASTRUCTURE PORTS              │
                              │   - ClockPort                       │
                              │   - RandomPort                      │
                              │   - EventBusPort                    │
                              │   - QueueDataPorts (see below)      │
                              └──────────────────┬──────────────────┘
                                                 │
                                                 │ implements
                                                 ▼
                              ┌─────────────────────────────────────┐
                              │         DRIVEN ADAPTERS             │
                              │ (Secondary / Right Side of Hexagon) │
                              │                                     │
                              │  ┌─────────────┐  ┌──────────────┐  │
                              │  │   Neo4j     │  │   Ollama     │  │
                              │  │   Repos     │  │   Client     │  │
                              │  └─────────────┘  └──────────────┘  │
                              │                                     │
                              │  ┌─────────────┐  ┌──────────────┐  │
                              │  │  ComfyUI    │  │  In-Memory   │  │
                              │  │   Client    │  │   Caches     │  │
                              │  └─────────────┘  └──────────────┘  │
                              │                                     │
                              │  ┌─────────────┐  ┌──────────────┐  │
                              │  │   SQLite    │  │  WebSocket   │  │
                              │  │   Queues    │  │  Broadcast   │  │
                              │  └─────────────┘  └──────────────┘  │
                              └─────────────────────────────────────┘
```

### Crate Dependency Diagram (Target State)

```
                    ┌─────────────────────────────────────────────────────────┐
                    │                    COMPOSITION ROOTS                     │
                    │  ┌─────────────────────┐    ┌─────────────────────────┐  │
                    │  │   engine-runner     │    │    player-runner        │  │
                    │  │   (wires engine)    │    │    (wires player)       │  │
                    │  └──────────┬──────────┘    └────────────┬────────────┘  │
                    └─────────────┼────────────────────────────┼───────────────┘
                                  │                            │
                    ┌─────────────┼────────────────────────────┼───────────────┐
                    │             ▼           ADAPTERS          ▼              │
                    │  ┌─────────────────────┐    ┌─────────────────────────┐  │
                    │  │  engine-adapters    │    │   player-adapters       │  │
                    │  │  - Neo4j            │    │   - WebSocket           │  │
                    │  │  - Axum             │    │   - Platform            │  │
                    │  │  - Ollama           │    │                         │  │
                    │  │  - ComfyUI          │    │                         │  │
                    │  │  - In-Memory        │    │                         │  │
                    │  └──────────┬──────────┘    └────────────┬────────────┘  │
                    └─────────────┼────────────────────────────┼───────────────┘
                                  │                            │
                    ┌─────────────┼────────────────────────────┼───────────────┐
                    │             ▼         APPLICATION         ▼              │
                    │  ┌─────────────────────┐    ┌─────────────────────────┐  │
                    │  │    engine-app       │    │     player-app          │  │
                    │  │  - handlers         │    │   - services (boundary) │  │
                    │  │  - use_cases        │    │                         │  │
                    │  │  - services         │    │                         │  │
                    │  └──────────┬──────────┘    └────────────┬────────────┘  │
                    │             │                            │               │
                    │  ┌──────────▼──────────┐                 │               │
                    │  │ engine-composition  │                 │               │
                    │  │ (DI containers)     │                 │               │
                    │  └──────────┬──────────┘                 │               │
                    └─────────────┼────────────────────────────┼───────────────┘
                                  │                            │
                    ┌─────────────┼────────────────────────────┼───────────────┐
                    │             ▼            PORTS            ▼              │
                    │  ┌─────────────────────┐    ┌─────────────────────────┐  │
                    │  │   engine-ports      │    │    player-ports         │  │
                    │  │  - inbound/         │    │   - inbound/ (target)   │  │
                    │  │  - outbound/        │    │   - outbound/           │  │
                    │  └──────────┬──────────┘    └────────────┬────────────┘  │
                    └─────────────┼────────────────────────────┼───────────────┘
                                  │                            │
                    ┌─────────────┼────────────────────────────┼───────────────┐
                    │             ▼        SHARED KERNEL        ▼              │
                    │  ┌──────────────────────────────────────────────────┐    │
                    │  │                   protocol                        │    │
                    │  │  (Wire-format DTOs for Engine <-> Player)        │    │
                    │  └──────────────────────────────────────────────────┘    │
                    │                                                          │
                    │  ┌──────────────────────────────────────────────────┐    │
                    │  │                   common                          │    │
                    │  │  (Pure utilities: datetime, string helpers)      │    │
                    │  └──────────────────────────────────────────────────┘    │
                    └──────────────────────────────────────────────────────────┘
                                  │
                    ┌─────────────┼──────────────────────────────────────────────┐
                    │             ▼     ENGINE-INTERNAL (not shared kernel)      │
                    │  ┌──────────────────────────────────────────────────┐      │
                    │  │           engine-dto                              │      │
                    │  │  (Engine-internal glue: queue storage, persistence)│      │
                    │  └──────────────────────────────────────────────────┘      │
                    └────────────────────────────────────────────────────────────┘
                                  │
                    ┌─────────────┼────────────────────────────────────────────┐
                    │             ▼            DOMAIN                          │
                    │  ┌──────────────────────────────────────────────────┐    │
                    │  │                   domain                          │    │
                    │  │  - entities/       (25 entities)                 │    │
                    │  │  - value_objects/  (rich value objects)          │    │
                    │  │  - ids.rs          (28 typed IDs)                │    │
                    │  │  - events/         (domain events)               │    │
                    │  │  - error.rs        (domain errors)               │    │
                    │  │  NO queue_data.rs  (moved to engine-ports)       │    │
                    │  └──────────────────────────────────────────────────┘    │
                    │                                                          │
                    │  ┌──────────────────────────────────────────────────┐    │
                    │  │                 domain-types                      │    │
                    │  │  (Shared vocabulary: archetypes, monomyth, etc.) │    │
                    │  └──────────────────────────────────────────────────┘    │
                    └──────────────────────────────────────────────────────────┘
```

---

## Acceptance Criteria

When this plan is complete, all of the following will be true:

### 1. Domain Layer Purity

- [ ] No infrastructure types in domain (queue data moved to ports)
- [ ] No `Utc::now()` in production code outside tests
- [ ] Domain contains only: entities, value objects, typed IDs, domain events, business rules

### 2. DTO Single Source of Truth

- [ ] `ApprovalDecision` exists only in `protocol` (all 4 copies consolidated)
- [ ] Suggestion types (`ChallengeSuggestion`, etc.) have canonical owner with documented exceptions
- [ ] No shadow copies between `engine-dto` and `engine-ports`

### 3. Composition Root Simplicity

- [ ] No duplicate service instantiation in `app_state.rs`
- [ ] `new_app_state()` function < 400 lines
- [ ] All game services constructed via factory

### 4. Port Layer Correctness

- [x] `player_events.rs` docstring correctly describes its `outbound/` placement *(completed 2026-01-01)*
- [ ] All protocol exceptions explicitly documented
- [ ] Adapter files named consistently (not `ports_*.rs`)
- [ ] `StoryEventQueryServicePort` renamed to `StoryEventQueryPort`

### 5. Player-Side Architecture

- [x] UI uses domain's `DiceFormula::parse()` instead of duplicate regex-based parsing *(completed 2026-01-01)*
- [ ] Silent failures in UI components audited and user feedback added where needed

### 6. Documentation Accuracy

- [ ] AGENTS.md hexagon diagram correct (engine-dto not in shared kernel)
- [ ] Counts accurate (28 typed IDs, ~700 files)
- [ ] Naming conventions documented

### 7. Code Quality

- [x] No magic numbers (conversation limit, buffer sizes configurable) *(completed 2026-01-01)*
- [ ] No blanket `impl ErrorCode for String`
- [ ] No orphaned/deprecated modules

---

## Ground Rules

1. **Keep builds green**: `cargo check --workspace` must pass after each step
2. **Incremental commits**: Small, focused commits with clear messages
3. **Verify as you go**: Run `cargo xtask arch-check` frequently
4. **Update docs in same PR**: Documentation changes accompany code changes
5. **Parallelize where safe**: Use parallel tracks for independent work

---

## Phase Overview

### Parallel Track A: Engine Domain/DTOs (HIGH PRIORITY)

| Phase | Description | Effort | Dependencies | Status |
|-------|-------------|--------|--------------|--------|
| 1A.1 | Queue Data Migration (move types) | Medium | None | Pending |
| 1A.2 | Queue Data Consolidation (remove duplicates) | Medium | 1A.1 | Pending |
| 1B | DTO Consolidation (ApprovalDecision) | Medium | 1A.2 | Pending |
| 1C | Fix Utc::now() in App DTO | Small | None | Pending |

**Note**: `ApprovalUrgency` and `ApprovalDecisionType` are **business concepts** (what needs DM approval and how urgent) - they stay in domain. Only infrastructure queue types move.

### Parallel Track B: Composition/Ports (HIGH PRIORITY)

| Phase | Description | Effort | Dependencies | Status |
|-------|-------------|--------|--------------|--------|
| 2A | Composition Root Refactor | Large | None | Pending |
| 2B | Port Naming Corrections | Small | None | Pending |
| 2C | Fix player_events.rs Docstring | Small | None | **DONE** |
| 2D | Document Protocol Exceptions | Small | None | Pending |

**Note on 2C**: ~~Validation confirmed `player_events.rs` is **correctly placed** in `outbound/`. The file's docstring incorrectly claims it's in `inbound/`. Fix is to correct the docstring, not move the file.~~ **COMPLETED 2026-01-01**: Docstring updated to correctly describe outbound placement.

### Parallel Track C: Player/Docs (MEDIUM PRIORITY)

| Phase | Description | Effort | Dependencies | Status |
|-------|-------------|--------|--------------|--------|
| 3A | Remove Duplicate Dice Parsing | Small | None | **DONE** |
| 3B | UI Error Feedback Audit | Medium | None | Pending |
| 3C | Documentation Updates | Small | 1A, 2B | Pending |

**Note on 3A**: ~~Domain already has complete `DiceFormula` implementation. UI has duplicate with bugs (allows d1, no shorthand). Fix is to use domain directly, not create new service.~~ **COMPLETED 2026-01-01**: UI now imports `DiceFormula` from domain. Exported `DiceFormula` and `DiceRollResult` from `domain/value_objects`. Removed buggy local regex parser.

### Parallel Track D: Code Quality (LOW PRIORITY)

| Phase | Description | Effort | Dependencies | Status |
|-------|-------------|--------|--------------|--------|
| 4A | Fix Inline Buffer Size | Small | None | **DONE** |
| 4B | Remove Blanket Impl | Small | None | Pending |
| 4C | Cleanup Orphaned Module | Small | None | Pending |

**Note on 4A**: ~~Conversation limit already has env var (`WRLDBLDR_MAX_CONVERSATION_TURNS`). Only fix needed is inline `256` in `world_connection_manager.rs`.~~ **COMPLETED 2026-01-01**: Added `DEFAULT_BROADCAST_CHANNEL_BUFFER` constant.
**Note on 4C**: Only `engine-runner/src/composition/services/mod.rs` is orphaned. `player-ports/src/mod.rs` is NOT orphaned.

---

## Detailed Phases

### Phase 1A: Queue Data Analysis & Migration

**Goal**: Remove infrastructure types from domain layer while keeping business concepts.

**Current State**: `domain/src/value_objects/queue_data.rs` (427 lines) contains:

| Type | Category | Target Location |
|------|----------|-----------------|
| `PlayerActionData` | Queue infrastructure | `engine-ports/outbound/queue_types.rs` |
| `DmActionData` | Queue infrastructure | `engine-ports/outbound/queue_types.rs` |
| `DmActionType` | Queue infrastructure | `engine-ports/outbound/queue_types.rs` |
| `DmApprovalDecision` | Wire format duplicate | REMOVE (use protocol) |
| `LlmRequestData` | Queue infrastructure | `engine-ports/outbound/queue_types.rs` |
| `LlmRequestType` | Queue infrastructure | `engine-ports/outbound/queue_types.rs` |
| `SuggestionContext` | Queue infrastructure | `engine-ports/outbound/queue_types.rs` |
| `ProposedTool` | Wire format duplicate | REMOVE (use protocol) |
| `ChallengeSuggestion` | Wire format duplicate | REMOVE (use protocol) |
| `ChallengeSuggestionOutcomes` | Wire format duplicate | REMOVE (use protocol) |
| `NarrativeEventSuggestion` | Wire format duplicate | REMOVE (use protocol) |
| `ApprovalDecisionType` | **BUSINESS CONCEPT** | **KEEP IN DOMAIN** |
| `ApprovalUrgency` | **BUSINESS CONCEPT** | **KEEP IN DOMAIN** |
| `ApprovalRequestData` | Queue infrastructure | `engine-ports/outbound/queue_types.rs` |
| `ChallengeOutcomeData` | Queue infrastructure | `engine-ports/outbound/queue_types.rs` |
| `AssetGenerationData` | Queue infrastructure | `engine-ports/outbound/queue_types.rs` |

**Why keep `ApprovalDecisionType` and `ApprovalUrgency` in domain?**

These are **business rules**, not infrastructure:
- `ApprovalDecisionType` defines what kinds of things need DM approval (NpcResponse, ToolUsage, ChallengeSuggestion, etc.) - this is core game logic
- `ApprovalUrgency` defines priority levels (Normal, AwaitingPlayer, SceneCritical) - this is narrative pacing logic

The duplicates in `engine-ports` should be removed, with ports importing from domain.

#### Phase 1A.1: Move Infrastructure Types

**Steps**:

1. Create `crates/engine-ports/src/outbound/queue_types.rs`
2. Move infrastructure types (keep typed ID usage):
   - `PlayerActionData`, `DmActionData`, `DmActionType`
   - `LlmRequestData`, `LlmRequestType`, `SuggestionContext`
   - `ApprovalRequestData`, `ChallengeOutcomeData`, `AssetGenerationData`
3. Update imports in engine-app, engine-adapters, engine-dto
4. Keep `ApprovalDecisionType` and `ApprovalUrgency` in domain
5. Remove duplicates of these types from `engine-ports/src/outbound/dm_approval_queue_service_port.rs`

**Verification**:
```bash
cargo check --workspace
```

#### Phase 1A.2: Consolidate Wire Format Duplicates

**Steps**:

1. For duplicate types (`DmApprovalDecision`, `ProposedTool`, `ChallengeSuggestion`, etc.):
   - Add documented protocol exceptions to `queue_types.rs`
   - Re-export from protocol where needed
   - Remove domain copies
2. Delete remaining contents of `domain/src/value_objects/queue_data.rs`
3. Move `ApprovalDecisionType` and `ApprovalUrgency` to appropriate domain file (e.g., `dm_approval.rs`)
4. Update `domain/src/value_objects/mod.rs`
5. Add arch-check rule: no file matching `*queue*` in domain

**Verification**:
```bash
grep -r "queue_data" crates/domain  # Should return empty
cargo check --workspace
cargo xtask arch-check
```

---

### Phase 1B: DTO Consolidation (ApprovalDecision)

**Goal**: Single source of truth for approval types.

**Current State** (ApprovalDecision):

| Location | Type Name | Variants |
|----------|-----------|----------|
| `protocol/types.rs` | `ApprovalDecision` | Accept, AcceptWithRecipients, AcceptWithModification, Reject, TakeOver, Unknown |
| `player-ports/session_types.rs` | `ApprovalDecision` | Same minus Unknown |
| `engine-dto/queue.rs` | `DmApprovalDecision` | Same minus Unknown |
| `domain/queue_data.rs` | `DmApprovalDecision` | Same minus Unknown |

**Target State**:
- `protocol::ApprovalDecision` is the single source of truth
- Ports add documented exceptions to use protocol directly
- ~150 lines of `From` impl boilerplate removed

**Steps**:

1. In `engine-ports/outbound/queue_types.rs` (new file from 1A):
   ```rust
   // ARCHITECTURE EXCEPTION: [APPROVED 2026-01-01]
   // ApprovalDecision is the wire format and is used identically for queue storage.
   // Duplicating would require 50+ lines of From impls with no benefit.
   pub use wrldbldr_protocol::ApprovalDecision;
   ```

2. Update `engine-dto/queue.rs`:
   - Remove `DmApprovalDecision` enum definition
   - Import from `engine-ports::outbound::queue_types`
   - Remove bidirectional `From` impls (~60 lines)

3. Update `player-ports/session_types.rs`:
   - Remove `ApprovalDecision` enum definition
   - Add documented exception and re-export from protocol
   - Remove bidirectional `From` impls (~40 lines)

4. Update all usages across crates

5. Apply same pattern to:
   - `ProposedTool` / `ProposedToolInfo`
   - `ChallengeSuggestion` / `ChallengeSuggestionInfo`
   - `NarrativeEventSuggestion` / `NarrativeEventSuggestionInfo`

**Verification**:
```bash
grep -rn "enum DmApprovalDecision" crates/  # Should only find protocol
grep -rn "enum ApprovalDecision" crates/    # Should only find protocol  
cargo check --workspace
```

---

### Phase 1C: Fix Utc::now() in App DTO

**Goal**: Remove time impurity from app layer.

**Location**: `engine-app/src/application/dto/world_snapshot.rs:74`

**Steps**:

1. Remove `Default` impl from `WorldSnapshot`, OR
2. Replace `Utc::now()` with a fixed sentinel value:
   ```rust
   impl Default for WorldSnapshot {
       fn default() -> Self {
           // Use epoch as sentinel for "uninitialized" snapshot
           let epoch = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
           Self {
               world: World::new("Empty World", "A placeholder world", epoch),
               // ...
           }
       }
   }
   ```

**Verification**:
```bash
grep -rn "Utc::now()" crates/engine-app/src/application/  # Should only be in tests
cargo check --workspace
```

---

### Phase 2A: Composition Root Refactor

**Goal**: Eliminate duplicate service instantiation and reduce complexity.

**Current Issues**:

1. **Duplicate instantiation** (`app_state.rs:311-391`): Services created twice
2. **God function**: `new_app_state()` is 855 lines
3. **Missing factory**: Game services constructed inline

**Steps**:

#### 2A.1: Create Game Services Factory

1. Create `crates/engine-runner/src/composition/factories/game_services.rs`
2. Move game service construction from `app_state.rs`:
   - `StoryEventServiceImpl`
   - `NarrativeEventServiceImpl`
   - `ChallengeServiceImpl`
   - `StagingServiceImpl`
   - Related services

3. Follow existing factory pattern:
   ```rust
   pub struct GameServicePorts {
       pub story_event_service: Arc<dyn StoryEventServicePort>,
       pub narrative_event_service: Arc<dyn NarrativeEventServicePort>,
       // ...
   }
   
   pub fn create_game_service_ports(deps: GameServiceDependencies) -> GameServicePorts {
       // ...
   }
   ```

#### 2A.2: Eliminate Duplicate Service Instantiation

1. Identify services created twice (port version + app-layer version)
2. For each duplicate:
   - Keep the port-typed version
   - Ensure app-layer trait is implemented by same type
   - Remove duplicate construction
   - Use trait object coercion where needed

**Current pattern (to eliminate)**:
```rust
// Port version
let world_service_port = core_service_ports.world_service; // Arc<dyn WorldServicePort>

// DUPLICATE - app-layer version  
let world_service: Arc<dyn WorldService> = Arc::new(WorldServiceImpl::new(...));
```

**Target pattern**:
```rust
// Single instance, coerced as needed
let world_service = core_service_ports.world_service; // Arc<dyn WorldServicePort>
// Use world_service directly, or coerce if WorldService differs
```

#### 2A.3: Simplify new_app_state()

1. After factory extraction, target < 400 lines
2. Function should primarily:
   - Call factories in order
   - Wire dependencies between factory outputs
   - Construct final AppState and WorkerServices

**Verification**:
```bash
wc -l crates/engine-runner/src/composition/app_state.rs  # Target: < 600
cargo check --workspace
cargo xtask arch-check
```

---

### Phase 2B: Port Naming Corrections

**Goal**: Consistent naming in engine-adapters.

**Files to rename**:

| Current | Target |
|---------|--------|
| `ports.rs` | `port_adapters.rs` |
| `ports_scene_adapters.rs` | `scene_port_adapters.rs` |
| `ports_connection_adapters.rs` | `connection_port_adapters.rs` |
| `ports_challenge_adapters.rs` | `challenge_port_adapters.rs` |
| `ports_staging_service_adapter.rs` | `staging_port_adapters.rs` |
| `ports_player_action_adapters.rs` | `player_action_port_adapters.rs` |

**Also rename**:
- `StoryEventQueryServicePort` -> `StoryEventQueryPort` (it's read-only queries)

**Steps**:

1. Rename files using `git mv`
2. Update `mod.rs` declarations
3. Update all imports
4. Run `cargo check --workspace`

---

### Phase 2C: Fix player_events.rs Docstring

**Goal**: Correct the misleading docstring in `player_events.rs`.

**Issue**: The file's docstring incorrectly claims it should be in `inbound/`, but the file is correctly placed in `outbound/`.

**Why `outbound/` is correct**:
1. The game server is a "driven" dependency (something the app NEEDS)
2. `GameConnectionPort` is an outbound port that delivers `PlayerEvent`
3. Adapters CREATE these types (`message_translator.rs`), app/UI CONSUME them
4. This matches the outbound pattern: "implemented by adapters, depended on by use cases/services"

**The data flows inward** (server → app), but the **port category** is outbound because the server connection is a dependency the app needs, not a service the app offers.

**Steps**:

1. Update the docstring in `crates/player-ports/src/outbound/player_events.rs`:

```rust
//! Player events - application-layer types for server messages
//!
//! These types represent the application's view of server messages.
//! They are defined in the ports layer as the output contract of the
//! `GameConnectionPort` - the interface between adapters and application.
//!
//! # Hexagonal Architecture Placement
//!
//! This module is in `player-ports/outbound` because:
//!
//! 1. **Outbound dependency**: The game server is a "driven" system that
//!    the application depends on (via `GameConnectionPort`). The app doesn't
//!    offer services to the server - it consumes data from it.
//!
//! 2. **Adapter responsibility**: The adapters layer (`message_translator.rs`)
//!    translates wire-format `ServerMessage` into these `PlayerEvent` types.
//!
//! 3. **Port contract**: These types define what the outbound port returns
//!    to the application layer - they're the "output" side of the port.
//!
//! Note: The data flows *inward* to the application, but the port category
//! is "outbound" because the server connection is a dependency the app
//! *needs* (outbound), not a service the app *offers* (inbound).
```

**Verification**:
```bash
# Docstring updated, no file moves needed
cargo check --workspace
```

**Effort**: 0.5 hours (documentation only, zero risk)

---

### Phase 2D: Document Protocol Exceptions

**Goal**: All protocol usage in ports explicitly documented.

**Undocumented exceptions to fix**:

1. `engine-ports/src/outbound/dm_approval_queue_service_port.rs:18`
2. New exceptions from Phase 1A/1B

**Template**:
```rust
// ARCHITECTURE EXCEPTION: [APPROVED YYYY-MM-DD]
// Reason: <brief justification>
// Alternative considered: <what would happen if we didn't do this>
use wrldbldr_protocol::{...};
```

---

### Phase 3A: Remove Duplicate Dice Parsing

**Goal**: Eliminate duplicate dice parsing logic by using existing domain implementation.

**Discovery**: Domain already has a complete `DiceFormula` implementation at `crates/domain/src/value_objects/dice.rs` with:
- `DiceFormula::parse()` - Pure string parsing, no world context needed
- `DiceFormula::roll()` - Takes injected RNG (hexagonal-compliant)
- Support for shorthand (`d20` → `1d20`)
- Proper validation (die size ≥ 2)
- Methods: `min_roll()`, `max_roll()`, `display()`, `breakdown()`

**Problem**: The UI at `challenge_roll.rs:184-214` has duplicate parsing with bugs:
- Uses `regex_lite` (extra dependency)
- Allows `d1` (domain correctly rejects dies < 2)
- Doesn't support `d20` shorthand (domain does)
- Different limits (1-20 dice, 1-100 sides vs domain's 1-255)

**Solution**: Use domain's `DiceFormula` directly instead of creating a new service.

**Why this is architecturally acceptable**:
- `DiceFormula` is a pure value object with no I/O dependencies
- Dice formula parsing is world-independent (modifiers are applied separately)
- The "UI uses protocol not domain" is a guideline for infrastructure types, not pure VOs
- This eliminates code duplication and fixes bugs

**Steps**:

1. Add `wrldbldr-domain` dependency to `crates/player-ui/Cargo.toml`:
   ```toml
   # ARCHITECTURE NOTE: DiceFormula is a pure value object with no infrastructure
   # concerns. Importing it directly avoids duplicating parsing logic with bugs.
   wrldbldr-domain = { workspace = true }
   ```

2. Update `challenge_roll.rs` to use domain's `DiceFormula`:
   ```rust
   use wrldbldr_domain::value_objects::DiceFormula;

   // Replace the local parse_formula closure with:
   match DiceFormula::parse(&formula) {
       Ok(f) => {
           // Use f.dice_count, f.die_size, f.modifier
       }
       Err(e) => {
           error_message.set(Some(e.to_string()));
       }
   }
   ```

3. Remove the `regex_lite` dependency from `player-ui/Cargo.toml` (if no longer needed)

4. Delete the inline `parse_formula` closure (~30 lines)

**Verification**:
```bash
grep -n "regex_lite" crates/player-ui/Cargo.toml  # Should be removable
grep -n "parse_formula" crates/player-ui/  # Should not find the closure
cargo check --workspace
```

**Effort**: 1 hour
**Benefits**: Fixes bugs (d1 allowed, no shorthand), removes duplicate code, removes dependency

---

### Phase 3B: UI Error Feedback Audit

**Goal**: Ensure user-facing feedback for operations that can fail.

**Context**: The codebase already has an established pattern using `error: Signal<Option<String>>` in 30+ components. The issue is inconsistent application - some async operations log errors but don't show user feedback.

**Scope**: ~74 occurrences of `tracing::error!` or `tracing::warn!` in player-ui, of which ~30 are silent failures that need user feedback.

**Existing Pattern** (already in use):
```rust
let mut error: Signal<Option<String>> = use_signal(|| None);

// In async handler:
match some_operation().await {
    Ok(result) => { /* handle success */ }
    Err(e) => error.set(Some(format!("Failed: {}", e))),
}

// In component render:
if let Some(err) = error.read().as_ref() {
    rsx! { div { class: "error-message", "{err}" } }
}
```

**Steps**:

1. Audit silent failures in key components:
   - `pc_view.rs` (24 occurrences)
   - `generation_queue.rs` (11 occurrences)
   - `content.rs` (6 occurrences)
   - `motivations_tab.rs` (5 occurrences)

2. For each silent failure, determine if user feedback is needed:
   - **Yes**: User-initiated action that failed (show error)
   - **No**: Background sync, transient retry, non-critical warning

3. For failures needing feedback:
   - Ensure component has `error: Signal<Option<String>>`
   - Add `error.set(Some(...))` in error handler
   - Add error display in component render if missing

**NOT in scope** (to avoid over-engineering):
- Global toast/notification system
- New error state infrastructure
- Centralized error handling

**Verification**:
```bash
# After audit, spot-check key user actions show feedback on failure
cargo check --workspace
```

**Effort**: 2-4 hours (audit + targeted fixes)

---

### Phase 3B-OLD (Removed - kept for reference):

1. Create error signal pattern in state:
   ```rust
   // In appropriate state module
   pub struct ErrorState {
       pub message: Signal<Option<String>>,
       pub is_visible: Signal<bool>,
   }
   ```

2. Update async spawns to set error state on failure

3. Add error display component

---

### Phase 3C: Documentation Updates

**Goal**: Accurate documentation.

**Updates**:

1. **AGENTS.md hexagon diagram**: Move `engine-dto` out of Shared Kernel layer

2. **AGENTS.md counts**:
   - "26 typed IDs" -> "28 typed IDs"
   - "~650 Rust files" -> "~700 Rust files"

3. **Add naming convention section**:
   - `*Port` suffix for port traits
   - `*Provider` suffix for platform abstractions (player-ports)
   - `*RepositoryPort` for data access
   - `*ServicePort` for business operations
   - `*QueryPort` for read-only query operations

---

### Phase 4A: Fix Inline Buffer Size

**Goal**: Use named constant for inline magic number.

**Context from validation**:
- Conversation history limit (30) already has env var: `WRLDBLDR_MAX_CONVERSATION_TURNS`
- `event_infra.rs:26` already uses named constant: `const EVENT_CHANNEL_BUFFER: usize = 256;`
- Only issue: `world_connection_manager.rs:959` has inline `256`

**Single item to fix**:

`world_connection_manager.rs:959`:
```rust
// Current (inline magic number)
broadcast::channel::<ServerMessage>(256)

// Target (use named constant)
const CONNECTION_BROADCAST_BUFFER: usize = 256;
broadcast::channel::<ServerMessage>(CONNECTION_BROADCAST_BUFFER)
```

**Effort**: 0.5 hours

---

### Phase 4B: Remove Blanket Impl

**Goal**: No logic in ports layer.

**Location**: `engine-ports/src/outbound/use_case_types.rs:1066-1070`

```rust
// REMOVE THIS
impl ErrorCode for String {
    fn code(&self) -> &'static str {
        "USE_CASE_ERROR"
    }
}
```

Find usages and replace with proper error types.

---

### Phase 4C: Cleanup Orphaned Module

**Single item to remove**:

1. `engine-runner/src/composition/services/mod.rs` (empty, deprecated)

**Correction from validation**: `player-ports/src/mod.rs` is **NOT orphaned** - it's properly used by `lib.rs` as a re-export module. Do not remove it.

**Steps**:
1. Delete `crates/engine-runner/src/composition/services/mod.rs`
2. Remove `pub mod services;` from `crates/engine-runner/src/composition/mod.rs`

**Effort**: 0.25 hours

---

## Verification Checklist

After completing all phases:

```bash
# 1. Architecture check passes
cargo xtask arch-check

# 2. All tests pass
cargo test --workspace

# 3. No queue_data in domain (infrastructure types moved)
grep -r "queue_data" crates/domain && echo "FAIL" || echo "PASS"

# 4. No duplicate ApprovalDecision
grep -c "enum.*ApprovalDecision" crates/*/src/**/*.rs  # Should be 1 (in protocol)

# 5. No Utc::now() outside tests in app layer
grep -rn "Utc::now()" crates/engine-app/src/application/ | grep -v "#\[cfg(test)\]" | grep -v "mod tests" && echo "FAIL" || echo "PASS"

# 6. player_events.rs docstring is correct (not moved, just documented)
grep -A5 "Hexagonal Architecture Placement" crates/player-ports/src/outbound/player_events.rs | grep -q "outbound" && echo "PASS" || echo "FAIL"

# 7. Composition complexity
wc -l crates/engine-runner/src/composition/app_state.rs  # Should be < 600

# 8. Documentation counts
grep "28 typed IDs" AGENTS.md && echo "PASS" || echo "FAIL"

# 9. UI uses domain DiceFormula (no duplicate parsing)
grep -r "regex_lite" crates/player-ui/src/ && echo "FAIL (still has regex)" || echo "PASS"
grep -r "DiceFormula::parse" crates/player-ui/src/ && echo "PASS (uses domain)" || echo "FAIL"

# 10. ApprovalUrgency and ApprovalDecisionType still in domain (business concepts)
grep -l "ApprovalUrgency" crates/domain/src/ && echo "PASS" || echo "FAIL"
```

---

## Appendix A: Superseded Plans

This plan supersedes:

- `HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md` - Phases 1-7 complete, remaining work consolidated here

The previous plan should be marked deprecated with a reference to this document.

---

## Appendix B: Type Migration Reference

### Queue Types Moving to engine-ports

```rust
// crates/engine-ports/src/outbound/queue_types.rs

// Infrastructure types (move from domain)
pub struct PlayerActionData { ... }
pub struct DmActionData { ... }
pub enum DmActionType { ... }
pub struct LlmRequestData { ... }
pub enum LlmRequestType { ... }
pub struct SuggestionContext { ... }
pub struct ApprovalRequestData { ... }
pub struct ChallengeOutcomeData { ... }
pub struct AssetGenerationData { ... }

// Re-exports from protocol (documented exceptions)
pub use wrldbldr_protocol::ApprovalDecision;
pub use wrldbldr_protocol::ProposedToolInfo as ProposedTool;
pub use wrldbldr_protocol::ChallengeSuggestionInfo as ChallengeSuggestion;
pub use wrldbldr_protocol::ChallengeSuggestionOutcomes;
pub use wrldbldr_protocol::NarrativeEventSuggestionInfo as NarrativeEventSuggestion;
```

### Business Types Staying in domain

```rust
// crates/domain/src/value_objects/dm_approval.rs (or similar)

// These are BUSINESS CONCEPTS, not infrastructure:
// - ApprovalDecisionType defines what kinds of things need DM approval
// - ApprovalUrgency defines priority levels for narrative pacing

pub enum ApprovalDecisionType {
    NpcResponse,      // Business: NPC responding to player
    ToolUsage,        // Business: Using game tools
    ChallengeSuggestion,  // Business: Skill check suggestion
    SceneTransition,  // Business: Scene changes
    ChallengeOutcome, // Business: Challenge results
}

pub enum ApprovalUrgency {
    Normal,           // Workflow priority
    AwaitingPlayer,   // User experience consideration
    SceneCritical,    // Narrative pacing
}
```

---

## Appendix C: Estimated Effort (Revised After Validation)

| Phase | Estimated Hours | Parallelizable With | Notes |
|-------|----------------|---------------------|-------|
| 1A.1 | 3-4 | 2A, 2B, 3A | Move infrastructure types |
| 1A.2 | 2-3 | 2C, 2D | Consolidate with protocol |
| 1B | 2-3 | After 1A.2 | DTO consolidation |
| 1C | 0.5 | Any | Trivial fix |
| 2A | 6-8 | 1A, 3A, 3B | Complex, raised from 4-6 |
| 2B | 1-2 | Any | File renames |
| 2C | 0.5 | Any | **Reduced**: docstring fix only |
| 2D | 0.5 | Any | Documentation |
| 3A | 1 | 1A, 2A | **Reduced**: use existing domain |
| 3B | 2-4 | 1A, 2A | Audit + targeted fixes |
| 3C | 1-2 | After 1A, 2B | Documentation |
| 4A | 0.5 | Any | **Reduced**: single inline fix |
| 4B | 0.5 | Any | Delete blanket impl |
| 4C | 0.25 | Any | **Reduced**: single orphan |

**Total**: ~22-32 hours of focused work

**With parallelization**: ~12-16 hours elapsed time (2 developers)

### Validation-Driven Changes

| Phase | Original Plan | Validated Plan | Change |
|-------|---------------|----------------|--------|
| 2C | Move file + 11 imports | Fix docstring | -1.5 hrs, lower risk |
| 3A | Create new service | Use existing domain | -1 hr, fixes bugs |
| 4A | Make 2 items configurable | Fix 1 inline constant | -0.5 hrs |
| 4C | Remove 2 orphans | Remove 1 orphan | -0.25 hrs |

---

## Appendix D: Validation Summary

This plan was validated by multiple AI agents analyzing each phase before finalization. Key findings:

1. **Phase 2C**: `player_events.rs` is correctly placed in `outbound/`. The game server is a "driven" dependency, making this an outbound port pattern. The docstring is wrong, not the placement.

2. **Phase 3A**: Domain already has complete `DiceFormula` with proper validation. UI version has bugs (allows d1, no shorthand). Fix is to use domain, not create new service.

3. **Phase 1A**: `ApprovalDecisionType` and `ApprovalUrgency` are business concepts (what needs DM approval, urgency levels) and should stay in domain. Only infrastructure queue types move.

4. **Phase 4A**: Conversation limit already has env var. Only fix needed is one inline `256`.

5. **Phase 4C**: `player-ports/src/mod.rs` is NOT orphaned - validation confirmed it's used by `lib.rs`.
