# Architecture Remediation Master Plan (v2)

**Status**: ACTIVE (authoritative remediation plan)  
**Created**: 2026-01-01  
**Last Updated**: 2026-01-02  
**Supersedes**: `HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md`

This is the consolidated plan addressing all architectural issues identified in the comprehensive code review. It builds upon the completed work from the previous master plan (Phases 1-7) and adds new phases to address remaining gaps.

## Progress Summary

| Track | Completed | Remaining | Next Priority |
|-------|-----------|-----------|---------------|
| A: Engine Domain/DTOs | 2/3 | 1 | 1B (DTO Consolidation) |
| B: Composition/Ports | 3/4 | 1 | 2A (Composition Root Refactor) |
| C: Player/Docs | 1/3 | 2 | 3B (UI Error Feedback Audit) |
| D: Code Quality | 3/3 | 0 | **COMPLETE** |
| **Total** | **9/13** | **4** | |

### Completed Phases

| Phase | Description | Commit | Date |
|-------|-------------|--------|------|
| 2C | Fix player_events.rs docstring | `b8e76a0` | 2026-01-01 |
| 3A | Replace UI dice parsing with domain DiceFormula | `b8e76a0` | 2026-01-01 |
| 4A | Fix magic number in world_connection_manager.rs | `b8e76a0` | 2026-01-01 |
| 1A | Queue type architecture (REDESIGNED - see below) | `93837bc` | 2026-01-02 |
| 1C | Fix Utc::now() in App DTO | `93837bc` | 2026-01-02 |
| 4C | Cleanup orphaned services module | `93837bc` | 2026-01-02 |
| 4B | Remove blanket impl (SceneError already existed) | `93837bc` | 2026-01-02 |
| 2B | Port naming corrections (6 files renamed) | TBD | 2026-01-02 |
| 2D | Document protocol exceptions | TBD | 2026-01-02 |

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
                    │  │  - queue_data.rs   (queue payloads - correct)    │    │
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

- [x] Queue data types are correctly placed in domain (per queue-system.md) *(validated 2026-01-02)*
- [ ] No `Utc::now()` in production code outside tests
- [ ] Domain contains only: entities, value objects, typed IDs, domain events, business rules
- [x] No duplicate queue types in engine-ports (queue_types.rs deleted) *(completed 2026-01-02)*

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
| 1A | Queue Type Architecture (REDESIGNED) | Small | None | **DONE** |
| 1B | DTO Consolidation (ApprovalDecision) | Medium | None | Pending |
| 1C | Fix Utc::now() in App DTO | Small | None | **DONE** |

**Note on 1A**: Original plan to move queue types from domain to engine-ports was **CANCELLED** after analysis (see Phase 1A below for full rationale). Queue payloads are domain value objects per `queue-system.md`. The partial migration that created `engine-ports/outbound/queue_types.rs` has been reverted.

### Parallel Track B: Composition/Ports (HIGH PRIORITY)

| Phase | Description | Effort | Dependencies | Status |
|-------|-------------|--------|--------------|--------|
| 2A | Composition Root Refactor | Large | None | Pending |
| 2B | Port Naming Corrections | Small | None | **DONE** |
| 2C | Fix player_events.rs Docstring | Small | None | **DONE** |
| 2D | Document Protocol Exceptions | Small | None | **DONE** |

**Note on 2B**: ~~Rename ports_*.rs files to *_port_adapters.rs pattern.~~ **COMPLETED 2026-01-02**: Renamed 6 files. Trait rename (StoryEventQueryServicePort → StoryEventQueryPort) skipped due to name collision with existing StoryEventQueryPort repository trait.
**Note on 2C**: ~~Validation confirmed `player_events.rs` is **correctly placed** in `outbound/`. The file's docstring incorrectly claims it's in `inbound/`. Fix is to correct the docstring, not move the file.~~ **COMPLETED 2026-01-01**: Docstring updated to correctly describe outbound placement.
**Note on 2D**: **COMPLETED 2026-01-02**: Added ARCHITECTURE EXCEPTION documentation to dm_approval_queue_service_port.rs and mod.rs protocol imports.

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
| 4B | Remove Blanket Impl | Small | None | **DONE** |
| 4C | Cleanup Orphaned Module | Small | None | **DONE** |

**Note on 4A**: ~~Conversation limit already has env var (`WRLDBLDR_MAX_CONVERSATION_TURNS`). Only fix needed is inline `256` in `world_connection_manager.rs`.~~ **COMPLETED 2026-01-01**: Added `DEFAULT_BROADCAST_CHANNEL_BUFFER` constant.
**Note on 4B**: ~~The blanket `impl ErrorCode for String` is used by `SceneUseCaseError = String`. Removing it requires creating a proper `SceneUseCaseError` enum first.~~ **COMPLETED 2026-01-02**: `SceneError` already existed in `use_case_errors.rs`. Changed type alias to re-export and removed blanket impl.
**Note on 4C**: ~~Only `engine-runner/src/composition/services/mod.rs` is orphaned. `player-ports/src/mod.rs` is NOT orphaned.~~ **COMPLETED 2026-01-02**: Deleted orphaned services module and updated mod.rs.

---

## Detailed Phases

### Phase 1A: Queue Type Architecture (REDESIGNED)

**Status**: COMPLETE (redesigned approach)

**Original Plan**: Move "infrastructure" queue types from domain to engine-ports, classifying types with timestamps as "infrastructure."

**Problem Discovered**: The partial migration (Phase 1A.1) was executed, creating `engine-ports/outbound/queue_types.rs`, but Phase 1A.2 was not completed. This left the same types defined in THREE places:
- `domain/value_objects/queue_data.rs` (original)
- `engine-ports/outbound/queue_types.rs` (partial migration)
- `engine-dto/queue.rs` (serialization DTOs)

This caused `cargo xtask arch-check` to fail with:
```
Error: arch-check failed: engine-dto shadows engine-ports types
- SuggestionContext declared in BOTH engine-dto and engine-ports
```

**Root Cause Analysis**: The original classification was flawed:

1. **`DateTime<Utc>` is a value, not I/O** - having a timestamp field doesn't make something "infrastructure"
2. **Queue payloads ARE domain concepts** - `PlayerActionData` represents "what the player wants to do" - this is business data
3. **`queue-system.md:267` explicitly states** queue payload value objects belong in domain
4. **`SuggestionContext` is a pure value object** - no timestamps, retries, or callbacks at all

**Redesigned Approach (Option C)**:

Instead of moving types to engine-ports, we maintain the correct three-tier separation:

| Tier | Location | Purpose | Types |
|------|----------|---------|-------|
| **Domain** | `domain/value_objects/queue_data.rs` | Business value objects | `PlayerActionData`, `DmActionData`, `SuggestionContext`, etc. |
| **DTO** | `engine-dto/queue.rs` | Serialization with `#[serde(other)]` | `PlayerActionItem`, `DMActionItem`, etc. (raw UUIDs) |
| **Ports** | `engine-ports/outbound/queue_port.rs` | Trait definitions only | `QueuePort<T>`, `ApprovalQueuePort<T>` |

**Actions Taken**:

1. **DELETED** `engine-ports/outbound/queue_types.rs` - reverted the partial migration
2. **KEPT** domain types as canonical source - they're already well-defined
3. **KEPT** engine-dto types for serialization - they handle forward compatibility
4. **UPDATED** engine-app service imports to use domain types

**Why This Is Correct**:

1. **Matches `queue-system.md`**: "Queue data value objects - pure domain representations" (line 1 of queue_data.rs)
2. **Eliminates duplication**: No more three-way type definitions
3. **Fixes arch-check**: No shadowed types between engine-dto and engine-ports
4. **Preserves existing conversions**: engine-dto already has From/Into implementations

**What About Serde Derives in Domain?**

The domain types have `#[derive(Serialize, Deserialize)]`. This is acceptable because:
- Serde is a pure derive macro with no I/O
- The queue_data.rs header explicitly documents this: "Serde derives are included to support queue storage backends"
- This is consistent with how other domain value objects are defined

**Verification**:
```bash
cargo xtask arch-check  # Should pass (no shadowed types)
cargo check --workspace
```

---

### Phase 1B: DTO Consolidation (ApprovalDecision)

**Goal**: Reduce duplication in approval types while respecting hexagonal architecture.

**Current State** (ApprovalDecision):

| Location | Type Name | Variants |
|----------|-----------|----------|
| `protocol/types.rs` | `ApprovalDecision` | Accept, AcceptWithRecipients, AcceptWithModification, Reject, TakeOver, Unknown |
| `player-ports/session_types.rs` | `ApprovalDecision` | 5 variants (no Unknown) + ~130 lines From impls |
| `engine-dto/queue.rs` | `DmApprovalDecision` | 6 variants (has Unknown with #[serde(other)]) + ~60 lines From impls |
| `domain/queue_data.rs` | `DmApprovalDecision` | 5 variants (no Unknown) |

**CRITICAL ARCHITECTURE CONSTRAINT**: Domain layer CANNOT depend on protocol. The original plan to have domain re-export from protocol violates hexagonal architecture (dependencies must point inward).

**Revised Target State (Three-Type Model)**:
1. `domain::DmApprovalDecision` - Canonical business type (5 variants, no Unknown)
2. `protocol::ApprovalDecision` - Wire format (6 variants with Unknown for forward compat)
3. Remove duplicates in `engine-dto` and `player-ports` - re-export from protocol

**Steps**:

1. Keep `domain/value_objects/queue_data.rs` as-is (5 variants, canonical business type)

2. Update `engine-dto/queue.rs`:
   - Remove `DmApprovalDecision` enum definition (~35 lines)
   - Re-export from protocol: `pub use wrldbldr_protocol::ApprovalDecision;`
   - Keep `From<domain::DmApprovalDecision>` impl (domain→protocol conversion)
   - Remove reverse `From` impl

3. Update `player-ports/session_types.rs`:
   - Remove `ApprovalDecision` enum definition (~25 lines)
   - Add documented exception and re-export from protocol
   - Remove bidirectional `From` impls (~130 lines)

4. Update `engine-adapters` conversion code to handle domain↔protocol at boundaries

5. **NOT applying to ProposedTool, ChallengeSuggestion, NarrativeEventSuggestion** - these need separate analysis as they have different domain vs wire representations (typed IDs vs strings)

**Estimated savings**: ~250 lines (revised from 150)

**Effort**: Medium (2-3 hours) - revised from original estimate

**Verification**:
```bash
# Domain should have its own type (not re-exported)
grep -rn "enum DmApprovalDecision" crates/domain/  # Should find 1 match
# engine-dto and player-ports should re-export from protocol
grep -rn "pub use.*protocol.*ApprovalDecision" crates/engine-dto/
grep -rn "pub use.*protocol.*ApprovalDecision" crates/player-ports/
cargo check --workspace
```

---

### Phase 1C: Fix Utc::now() in App DTO

**Status**: COMPLETE

**Goal**: Remove time impurity from app layer.

**Location**: `engine-app/src/application/dto/world_snapshot.rs:74`

**Solution Applied**: Replaced `Utc::now()` with Unix epoch (1970-01-01T00:00:00Z) as a sentinel value for "uninitialized" snapshots. This avoids impure time calls while preserving the Default impl functionality.

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
2. **God function**: `new_app_state()` is 842 lines (verified)
3. **Missing factory**: Game services constructed inline

**ROOT CAUSE ANALYSIS** (from code review):

The core problem is **trait interface mismatch**: `AppRequestHandler` uses app-layer traits (`*Service`) while factories return port traits (`*ServicePort`). These are different traits, so services get constructed twice to satisfy both.

Verified duplicates:
- `WorldServiceImpl`: 2x (core_services.rs:279, app_state.rs:316)
- `CharacterServiceImpl`: 2x (core_services.rs:289, app_state.rs:324)
- `SkillServiceImpl`: 3x (core_services.rs:324, app_state.rs:358, app_state.rs:451)
- `PlayerCharacterServiceImpl`: 3x (core_services.rs:346, app_state.rs:432, app_state.rs:441)
- `ChallengeServiceImpl`: 2x (app_state.rs:373, app_state.rs:385)

Some duplicates are **intentional** due to Rust generics requiring concrete types (comments note "Keep concrete version for ChallengeResolutionService generics").

**REVISED EFFORT ESTIMATE**: 10-14 hours (not 6-8) due to trait mismatch complexity.

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
2. `engine-ports/src/outbound/mod.rs:588` (protocol re-exports)
3. New exceptions from Phase 1A/1B

**Note**: `engine-ports/src/inbound/request_handler.rs:40` is already documented with proper `// ARCHITECTURE EXCEPTION: [APPROVED 2025-12-28]`.

**Template**:
```rust
// ARCHITECTURE EXCEPTION: [APPROVED YYYY-MM-DD]
// Reason: <brief justification>
// Alternative considered: <what would happen if we didn't do this>
use wrldbldr_protocol::{...};
```

**Effort**: 0.5-1 hour

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

**DISCOVERY**: A proper `SceneError` enum already exists at `engine-ports/src/outbound/use_case_errors.rs:257-289` with full `impl ErrorCode`. The fix is straightforward:

**Steps**:
1. In `scene_use_case_port.rs`: Change `pub type SceneUseCaseError = String;` to:
   ```rust
   pub use crate::outbound::SceneError as SceneUseCaseError;
   ```
2. In `engine-app/src/application/use_cases/scene.rs`: Remove the 3 `.map_err(|e| e.to_string())` calls (lines 266, 276, 286)
3. In `use_case_types.rs`: Delete the blanket impl (lines 1065-1070)

**Effort**: 1-2 hours (revised from 0.5 - need to verify no regressions)

---

### Phase 4C: Cleanup Orphaned Module

**Status**: COMPLETE

**Single item removed**:

1. `engine-runner/src/composition/services/` directory (empty, deprecated)

**Correction from validation**: `player-ports/src/mod.rs` is **NOT orphaned** - it's properly used by `lib.rs` as a re-export module. Do not remove it.

**Steps completed**:
1. Deleted `crates/engine-runner/src/composition/services/` directory
2. Removed `pub mod services;` from `crates/engine-runner/src/composition/mod.rs`
3. Updated module docstring to reflect current structure

---

## Verification Checklist

After completing all phases:

```bash
# 1. Architecture check passes (critical - validates no shadowed types)
cargo xtask arch-check

# 2. All tests pass
cargo test --workspace

# 3. Queue types are in domain (correct location per queue-system.md)
test -f crates/domain/src/value_objects/queue_data.rs && echo "PASS" || echo "FAIL"

# 4. No queue_types.rs in engine-ports (was deleted in Phase 1A redesign)
test ! -f crates/engine-ports/src/outbound/queue_types.rs && echo "PASS" || echo "FAIL"

# 5. No duplicate ApprovalDecision (after Phase 1B)
grep -c "enum.*ApprovalDecision" crates/*/src/**/*.rs  # Should be 1 (in protocol)

# 6. No Utc::now() outside tests in app layer
grep -rn "Utc::now()" crates/engine-app/src/application/ | grep -v "#\[cfg(test)\]" | grep -v "mod tests" && echo "FAIL" || echo "PASS"

# 7. player_events.rs docstring is correct (not moved, just documented)
grep -A5 "Hexagonal Architecture Placement" crates/player-ports/src/outbound/player_events.rs | grep -q "outbound" && echo "PASS" || echo "FAIL"

# 8. Composition complexity
wc -l crates/engine-runner/src/composition/app_state.rs  # Should be < 600

# 9. Documentation counts
grep "28 typed IDs" AGENTS.md && echo "PASS" || echo "FAIL"

# 10. UI uses domain DiceFormula (no duplicate parsing)
grep -r "regex_lite" crates/player-ui/src/ && echo "FAIL (still has regex)" || echo "PASS"
grep -r "DiceFormula::parse" crates/player-ui/src/ && echo "PASS (uses domain)" || echo "FAIL"

# 11. ApprovalUrgency and ApprovalDecisionType still in domain (business concepts)
grep -l "ApprovalUrgency" crates/domain/src/ && echo "PASS" || echo "FAIL"
```

---

## Appendix A: Superseded Plans

This plan supersedes:

- `HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md` - Phases 1-7 complete, remaining work consolidated here

The previous plan should be marked deprecated with a reference to this document.

---

## Appendix B: Type Architecture Reference (REVISED)

### Queue Type Ownership (Three-Tier Model)

After the Phase 1A redesign, queue types follow this ownership model:

```
┌─────────────────────────────────────────────────────────────────────┐
│                         DOMAIN LAYER                                 │
│  crates/domain/src/value_objects/queue_data.rs                      │
│                                                                      │
│  CANONICAL BUSINESS TYPES (with typed IDs):                         │
│  - PlayerActionData, DmActionData, DmActionType                     │
│  - LlmRequestData, LlmRequestType, SuggestionContext                │
│  - ApprovalRequestData, ChallengeOutcomeData, AssetGenerationData   │
│  - ApprovalDecisionType, ApprovalUrgency (business concepts)        │
│  - ProposedTool, ChallengeSuggestion, NarrativeEventSuggestion      │
│                                                                      │
│  Note: Serde derives included for queue storage (documented)        │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
                                   │ From/Into conversions
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       ENGINE-DTO LAYER                               │
│  crates/engine-dto/src/queue.rs                                     │
│                                                                      │
│  SERIALIZATION DTOS (with raw UUIDs, #[serde(other)]):              │
│  - PlayerActionItem, DMActionItem, DMAction                         │
│  - LLMRequestItem, LLMRequestType                                   │
│  - ApprovalItem, ChallengeOutcomeApprovalItem                       │
│  - AssetGenerationItem                                              │
│  - DmApprovalDecision, DecisionType, DecisionUrgency                │
│  - SuggestionContext (DTO version with String world_id)             │
│                                                                      │
│  Purpose: Forward-compatible serialization for SQLite storage       │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
                                   │ Trait definitions only
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       ENGINE-PORTS LAYER                             │
│  crates/engine-ports/src/outbound/queue_port.rs                     │
│                                                                      │
│  PORT TRAITS (no type definitions):                                  │
│  - QueuePort<T>           - Generic queue operations                 │
│  - ApprovalQueuePort<T>   - Approval-specific operations            │
│  - Uses domain types as T parameter                                  │
│                                                                      │
│  NO queue_types.rs file (deleted in Phase 1A redesign)              │
└─────────────────────────────────────────────────────────────────────┘
```

### Why Domain Types Are Correct

| Argument | Response |
|----------|----------|
| "They have timestamps" | `DateTime<Utc>` is a value, not I/O |
| "They're queue infrastructure" | No - they represent business data (what player wants to do) |
| "Domain should be pure" | They ARE pure - no framework deps, no I/O |
| "queue-system.md says..." | It says queue payloads ARE domain value objects (line 267) |

### Business Concepts (Stay in Domain)

```rust
// crates/domain/src/value_objects/queue_data.rs

// These define WHAT needs approval and HOW urgent - core game rules:

pub enum ApprovalDecisionType {
    NpcResponse,          // NPC responding to player
    ToolUsage,            // Using game tools
    ChallengeSuggestion,  // Skill check suggestion
    SceneTransition,      // Scene changes
    ChallengeOutcome,     // Challenge results
}

pub enum ApprovalUrgency {
    Normal,           // Standard workflow
    AwaitingPlayer,   // Player is waiting
    SceneCritical,    // Narrative pacing critical
}
```

---

## Appendix C: Estimated Effort (Revised After Validation)

| Phase | Estimated Hours | Parallelizable With | Notes |
|-------|----------------|---------------------|-------|
| 1A | **DONE** | - | Queue type architecture (redesigned) |
| 1B | 2-3 | Any | DTO consolidation (three-type model) |
| 1C | **DONE** | - | Utc::now() fix |
| 2A | 10-14 | None | **Revised**: Complex, trait mismatch root cause |
| 2B | 1-2 | Any | File renames |
| 2C | **DONE** | - | Docstring fix |
| 2D | 0.5-1 | Any | Documentation (includes mod.rs:588) |
| 3A | **DONE** | - | Use existing domain DiceFormula |
| 3B | 2-4 | Any | Audit + targeted fixes |
| 3C | 1-2 | After 1B, 2B | Documentation |
| 4A | **DONE** | - | Single inline fix |
| 4B | 1-2 | Any | **Revised**: SceneError already exists |
| 4C | **DONE** | - | Orphan module removed |

**Total remaining**: ~18-28 hours of focused work

**With parallelization**: ~10-14 hours elapsed time (2 developers)

### Validation-Driven Changes (2026-01-02 Review)

| Phase | Original Plan | Validated Plan | Change |
|-------|---------------|----------------|--------|
| 1B | Domain re-exports protocol | Three-type model | Architecture fix |
| 2A | 6-8 hours | 10-14 hours | Trait mismatch complexity |
| 2D | 1 exception | 2 exceptions | Found mod.rs:588 |
| 4B | Create SceneUseCaseError | Use existing SceneError | Much simpler |

---

## Appendix D: Validation Summary

This plan was validated by multiple AI agents analyzing each phase before finalization. Key findings:

1. **Phase 2C**: `player_events.rs` is correctly placed in `outbound/`. The game server is a "driven" dependency, making this an outbound port pattern. The docstring is wrong, not the placement.

2. **Phase 3A**: Domain already has complete `DiceFormula` with proper validation. UI version has bugs (allows d1, no shorthand). Fix is to use domain, not create new service.

3. **Phase 1A**: `ApprovalDecisionType` and `ApprovalUrgency` are business concepts (what needs DM approval, urgency levels) and should stay in domain. Only infrastructure queue types move.

4. **Phase 4A**: Conversation limit already has env var. Only fix needed is one inline `256`.

5. **Phase 4C**: `player-ports/src/mod.rs` is NOT orphaned - validation confirmed it's used by `lib.rs`.
