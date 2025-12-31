# Hexagonal Architecture Refactor — Master Plan (Source of Truth)

**Status**: ACTIVE (authoritative refactor plan)

This is the single, consolidated plan to reach the **idealized hexagonal architecture** defined in:

- `docs/architecture/hexagonal-architecture.md` (**Hexagonal Architecture (Target)**)

All other plans that cover “hexagonal correctness” (ports/adapters correctness, DTO ownership correctness, protocol boundary correctness, and dependency inversion correctness) are **subsumed by this plan** and should be considered **superseded** once this is adopted.

## What “done” means (acceptance contract)

When this plan is complete, all of the following will be true:

1. **Crate dependency DAG matches target**
   - `cargo xtask arch-check` passes.
   - No forbidden internal crate deps.

2. **Inbound/outbound taxonomy is correct**
   - App code depends only on **outbound** ports (plus domain + context DTOs).
   - Inbound ports are implemented by use cases and only used by driving adapters/UI.

3. **No adapter-wrapper anti-patterns**
   - No `engine-adapters/src/infrastructure/ports/*_adapter.rs` wrapper structs whose sole job is “port-to-port forwarding”.

4. **Dependency inversion everywhere**
   - No app use case depends on another use case’s concrete type.
   - No app service depends on another service’s concrete type.
   - Composition roots store/wire **ports**, not concrete structs, except at the actual construction boundary.

5. **DTO ownership is single-source-of-truth**
   - No “shadow copies” (e.g., `engine_dto::X` duplicating `engine_ports::...::X`).
   - Protocol DTOs remain wire-only.

6. **Protocol remains at the boundary**
   - Domain does not import protocol.
   - Ports do not import protocol except explicit, documented, whitelisted boundary files.
   - App use cases/services are protocol-free.

7. **Tooling enforces the above**
   - `cargo xtask arch-check` contains checks for the key invariants.
   - Warning-mode checks have been incrementally upgraded to enforcement mode.

---

## Ground rules while refactoring

- Always keep `cargo check --workspace` green.
- Prefer small, mechanical steps with a tight feedback loop.
- Every phase below includes:
  - **mechanical steps**,
  - **measurable verification**, and
  - **a “stop condition”** (what must be true before moving on).

---

## Phase 0 — Baseline & instrumentation (make progress measurable)

### 0.1 Freeze the target architecture spec

- Confirm `docs/architecture/hexagonal-architecture.md` is the canonical target.
- Any exceptions must be captured as ADRs (or a documented whitelist in `xtask`).

**Verify**
- `cargo xtask arch-check` runs successfully.

### 0.2 Tighten arch-check to surface current drift (warning mode first)

**Goal**: Make the architecture drift visible with stable signals.

Steps:
1. Keep existing checks (crate DAG, shims, handler size, protocol isolation).
2. Add / maintain these warning-mode checks:
   - **App must not depend on inbound ports** (engine-app + player-app)
   - **No engine-dto type shadows engine-ports type names** (heuristic)
   - **No glob re-exports** (already warning mode)

**Verify**
- `cargo xtask arch-check` prints warnings for violations but does not fail (yet).

**Stop condition**
- These warning signals are stable (few false positives), and the team agrees they represent the target direction.

---

## Authoritative inventories (the itemized checklist)

This section is intentionally “mechanical” and is meant to be kept up to date as the refactor progresses.

### Inventory A — Port taxonomy migration table

**Purpose**: resolve inbound/outbound drift by explicitly deciding where each trait belongs.

**How to use**:
- Anything in **“Suggested target = outbound”** should be moved/renamed so app code depends only on outbound ports.
- Anything in **“Suggested target = inbound”** should be referenced only by driving adapters/UI.

%% PORT TAXONOMY %%

#### Engine inbound taxonomy

| Item | Kind | Defined in | Used in app/UI (examples) | Suggested target | Notes |
|---|---|---|---|---|---|
| `AppStatePort` | trait | crates/engine-ports/src/inbound/app_state_port.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `ChallengeDmApprovalQueuePort` | trait | crates/engine-ports/src/inbound/use_case_ports.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `ChallengeUseCasePort` | trait | crates/engine-ports/src/inbound/challenge_use_case_port.rs | crates/engine-app/src/application/use_cases/challenge.rs | inbound | Keep in inbound; ensure only adapters/UI import |
| `CharacterSummaryDto` | struct | crates/engine-ports/src/inbound/use_cases.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `ConnectionUseCasePort` | trait | crates/engine-ports/src/inbound/connection_use_case_port.rs | crates/engine-app/src/application/use_cases/connection.rs | inbound | Keep in inbound; ensure only adapters/UI import |
| `CreateCharacterRequest` | struct | crates/engine-ports/src/inbound/use_cases.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `CreateLocationRequest` | struct | crates/engine-ports/src/inbound/use_cases.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `CreateWorldRequest` | struct | crates/engine-ports/src/inbound/use_cases.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `InteractionServicePort` | trait | crates/engine-ports/src/inbound/use_case_ports.rs | crates/engine-app/src/application/use_cases/scene.rs | outbound (misplaced today) | Move to outbound; update app deps; remove inbound re-export |
| `InventoryUseCasePort` | trait | crates/engine-ports/src/inbound/inventory_use_case_port.rs | crates/engine-app/src/application/use_cases/inventory.rs | inbound | Keep in inbound; ensure only adapters/UI import |
| `LocationSummaryDto` | struct | crates/engine-ports/src/inbound/use_cases.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `ManageCharacterUseCase` | trait | crates/engine-ports/src/inbound/use_cases.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `ManageLocationUseCase` | trait | crates/engine-ports/src/inbound/use_cases.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `ManageSceneUseCase` | trait | crates/engine-ports/src/inbound/use_cases.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `ManageWorldUseCase` | trait | crates/engine-ports/src/inbound/use_cases.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `MovementUseCasePort` | trait | crates/engine-ports/src/inbound/movement_use_case_port.rs | crates/engine-app/src/application/use_cases/movement.rs | inbound | Keep in inbound; ensure only adapters/UI import |
| `NarrativeEventUseCasePort` | trait | crates/engine-ports/src/inbound/narrative_event_use_case_port.rs | crates/engine-app/src/application/use_cases/narrative_event.rs | inbound | Keep in inbound; ensure only adapters/UI import |
| `ObservationUseCasePort` | trait | crates/engine-ports/src/inbound/observation_use_case_port.rs | crates/engine-app/src/application/use_cases/observation.rs | inbound | Keep in inbound; ensure only adapters/UI import |
| `PlayerActionUseCasePort` | trait | crates/engine-ports/src/inbound/player_action_use_case_port.rs | crates/engine-app/src/application/use_cases/player_action.rs | inbound | Keep in inbound; ensure only adapters/UI import |
| `RequestHandler` | trait | crates/engine-ports/src/inbound/request_handler.rs | crates/engine-app/src/application/handlers/request_handler.rs | inbound (boundary trait) | Keep in inbound; ensure it does not leak into services |
| `SceneDmActionQueuePort` | trait | crates/engine-ports/src/inbound/use_case_ports.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `SceneServicePort` | trait | crates/engine-ports/src/inbound/use_case_ports.rs | crates/engine-app/src/application/use_cases/scene.rs | outbound (misplaced today) | Move to outbound; update app deps; remove inbound re-export |
| `SceneSummaryDto` | struct | crates/engine-ports/src/inbound/use_cases.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `SceneUseCaseError` | type | crates/engine-ports/src/inbound/scene_use_case_port.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `SceneUseCasePort` | trait | crates/engine-ports/src/inbound/scene_use_case_port.rs | crates/engine-app/src/application/use_cases/scene.rs | inbound | Keep in inbound; ensure only adapters/UI import |
| `StagingServiceExtPort` | trait | crates/engine-ports/src/inbound/use_case_ports.rs | crates/engine-app/src/application/use_cases/staging.rs | outbound (misplaced today) | Move to outbound; update app deps; remove inbound re-export |
| `StagingServicePort` | trait | crates/engine-ports/src/inbound/use_case_ports.rs | crates/engine-app/src/application/use_cases/movement.rs | outbound (misplaced today) | Move to outbound; update app deps; remove inbound re-export |
| `StagingStateExtPort` | trait | crates/engine-ports/src/inbound/use_case_ports.rs | crates/engine-app/src/application/use_cases/staging.rs | outbound (misplaced today) | Move to outbound; update app deps; remove inbound re-export |
| `StagingStatePort` | trait | crates/engine-ports/src/inbound/use_case_ports.rs | crates/engine-app/src/application/use_cases/movement.rs | outbound (misplaced today) | Move to outbound; update app deps; remove inbound re-export |
| `StagingUseCasePort` | trait | crates/engine-ports/src/inbound/staging_use_case_port.rs | crates/engine-app/src/application/use_cases/staging.rs | inbound | Keep in inbound; ensure only adapters/UI import |
| `UseCaseContext` | struct | crates/engine-ports/src/inbound/use_case_context.rs | crates/engine-app/src/application/use_cases/challenge.rs, crates/engine-app/src/application/use_cases/inventory.rs, crates/engine-app/src/application/use_cases/mod.rs (+6 more) | inbound (boundary DTO) | Keep in inbound; ensure it does not leak into services |
| `UseCaseError` | enum | crates/engine-ports/src/inbound/use_cases.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `WorldDto` | struct | crates/engine-ports/src/inbound/use_cases.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |
| `WorldStatePort` | trait | crates/engine-ports/src/inbound/use_case_ports.rs | crates/engine-app/src/application/use_cases/scene.rs | outbound (misplaced today) | Move to outbound; update app deps; remove inbound re-export |
| `WorldSummaryDto` | struct | crates/engine-ports/src/inbound/use_cases.rs |  | inbound | Keep in inbound; ensure only adapters/UI import |

#### Player inbound taxonomy

_No player inbound items found._ (Player server-event DTOs are currently defined in `crates/player-ports/src/outbound/player_events.rs`.)

_Regenerate with `task arch:inventories`._

%% /PORT TAXONOMY %%
### Inventory B — DTO ownership / duplication table

**Purpose**: enforce the “single canonical home” model and eliminate shadow copies.

**Rule**:
- If a type is a stable app↔adapter boundary contract: it belongs in `engine-ports` (near the owning port).
- If a type is internal glue (queues, projections, internal orchestration): it belongs in `engine-dto`.

%% DTO OWNERSHIP %%

| Type name | engine-dto (examples) | engine-ports (examples) | Suggested owner | Notes |
|---|---|---|---|---|
| _(none)_ |  |  |  |  |

_Regenerate with `task arch:inventories`._

%% /DTO OWNERSHIP %%
---

---

## Phase 1 — Remove the easy, mechanical debt (low risk)

### 1.1 Eliminate glob re-exports

**Goal**: remove implicit exports so rust-analyzer + dead-code detection improve.

Steps:
1. In `crates/engine-dto/src/lib.rs`, replace:
   - `pub use llm::*;` etc.
   with explicit exports.
2. Repeat for any other crates once the check is expanded.

**Verify**
- `cargo xtask arch-check` shows **0 glob re-export violations**.

**Progress**
- ✅ Completed 2025-12-31: removed glob re-exports from `crates/engine-dto/src/lib.rs`.

**Stop condition**
- Glob re-export check can be moved from warning-mode to enforcement.

---

## Phase 2 — DTO ownership consolidation (remove type duplication)

### 2.1 Make a DTO ownership inventory

**Goal**: identify every DTO that lives in the wrong “home”.

Steps:
1. Identify all DTOs in:
   - `engine-dto`
   - `engine-ports` boundary DTOs
   - `protocol`
2. For each DTO, decide the canonical owner:
   - domain vs protocol vs ports vs engine-dto

**Deliverable**
- A table in this doc listing DTO name → canonical crate/module → current locations → action.

### 2.2 Fix confirmed duplication: `StagingProposal`

**Goal**: One canonical definition.

Steps:
1. Choose canonical owner (target: `engine-ports` outbound boundary DTO).
2. Update all imports/usages from `wrldbldr_engine_dto::StagingProposal` to the canonical path.
3. Remove the duplicate definition from `engine-dto`.
4. Fix misleading comments (e.g., `world_state_manager.rs` comment).

**Verify**
- New arch-check duplication warning no longer reports `StagingProposal`.
- `cargo check --workspace` passes.

**Progress**
- ✅ Completed 2025-12-31: migrated staging DTO usage to `engine-ports` and deleted the duplicate `engine-dto` definitions.

### 2.3 Repeat for all shadow DTOs

Steps:
1. For each collision reported by arch-check:
   - migrate usages
   - remove duplicate
   - add tests where conversions changed

**Progress**
- ✅ Completed 2025-12-31: reduced DTO shadowing warnings to zero (removed public engine-dto shadow types for `ApprovalItem`, `QueueItem*`, `OutcomeDetail`, and resolved the `LlmResponse` name collision).

**Stop condition**
- DTO shadowing warnings go to zero.
- The DTO shadowing check can be switched from warning to enforcement.

---

## Phase 3 — Fix port taxonomy drift (inbound vs outbound correctness)

This is the core “hexagonal correctness” phase.

### 3.1 Classify all port traits by direction

**Goal**: every trait is in the right folder and used from the right layer.

Steps:
1. For every trait under `engine-ports/src/inbound/`:
   - Determine whether it is truly inbound (implemented by app use case and called by driving adapter/UI), or
   - Actually outbound (depended on by app).
2. Create a mapping table:
   - Trait name → current module → target module (inbound/outbound) → primary users.

### 3.2 Move misclassified ports to outbound

**Common current pattern** (to eliminate):
- Use case depends on `wrldbldr_engine_ports::inbound::SomeServicePort`.

Steps:
1. Move the dependency trait into `engine-ports/src/outbound/`.
2. Update imports in:
   - engine-app use cases/services
   - engine-composition
   - engine-adapters implementations
3. If a trait name is misleading, rename with a migration.

**Verify**
- The inbound-dependency check emits fewer warnings.
- `cargo check --workspace` passes.

**Progress**
- ✅ Started 2025-12-31: moved use-case error enums (`ActionError`, `ChallengeError`, `InventoryError`, `NarrativeEventError`, `ObservationError`, `SceneError`, `StagingError`) from `engine-ports` inbound to outbound, and updated imports.
- ✅ Continued 2025-12-31: moved `ConnectionManagerPort` from `engine-ports` inbound (`use_case_ports.rs`) to `engine-ports` outbound.
- ✅ Continued 2025-12-31: moved `PlayerActionQueuePort` and `DmNotificationPort` from `engine-ports` inbound (`use_case_ports.rs`) to `engine-ports` outbound.
- ✅ Continued 2025-12-31: moved connection DTO ports from `engine-ports` inbound to outbound (renamed to avoid collisions): `WorldServicePort` → `WorldSnapshotJsonPort`, `PlayerCharacterServicePort` → `PlayerCharacterDtoPort`, `DirectorialContextPort` → `DirectorialContextQueryPort`.
- ✅ Continued 2025-12-31: moved challenge use-case dependency ports from `engine-ports` inbound to outbound: `ChallengeResolutionPort`, `ChallengeOutcomeApprovalPort`, and standardized on outbound `NarrativeRollContext`.
- ✅ Continued 2025-12-31: moved scene directorial-context DTO persistence port from `engine-ports` inbound to outbound (renamed to avoid collision with the domain repo port): `DirectorialContextRepositoryPort` → `DirectorialContextDtoRepositoryPort`.

### 3.3 Normalize “context DTOs”

**Goal**: allow the *few* context DTOs app needs without using inbound module as a dumping ground.

Steps:
1. Identify context-only DTOs (e.g., `UseCaseContext`, `RequestContext`).
2. Decide canonical module for each:
   - If it’s strictly use-case invocation context: stay with inbound ports, but isolate in a dedicated `context` module.
   - If it’s boundary request context: keep near handler boundary and avoid spreading into services.
3. Update arch-check allowlist to reflect these explicit exceptions.

**Stop condition**
- App no longer imports traits from inbound modules (only DTO exceptions remain).
- The inbound-dependency check can move from warning mode to enforcement.

---

## Phase 4 — Delete the “port adapter” wrapper layer

### 4.1 Remove wrappers in `engine-adapters/src/infrastructure/ports/`

**Goal**: adapters implement outbound ports directly; no indirection.

Steps:
1. For each wrapper adapter struct:
   - identify the real underlying service/repo it forwards to
   - change use cases to depend on the outbound port implemented by the real adapter
2. Delete wrapper structs and modules.
3. Remove the directory/module if it becomes empty.

**Verify**
- No remaining references to `engine-adapters/src/infrastructure/ports/` in codebase.
- `cargo check --workspace`.

**Stop condition**
- That directory is deleted (or contains only legitimate boundary code, not forwarders).

---

## Phase 5 — Dependency inversion in engine-app (no concrete dependencies)

### 5.1 Use cases must not depend on concrete use cases

**Example to fix**: `PlayerActionUseCase` depends on `Arc<MovementUseCase>`.

Steps:
1. Find all `Arc<SomeUseCase>` fields in use cases.
2. Replace with `Arc<dyn SomeUseCasePort>`.
3. Ensure the port trait lives in inbound (because it’s the app API), not outbound.
4. Wire in composition root using trait objects.

**Verify**
- Grep: no `Arc<...UseCase>` fields except the implementing struct itself.
- Unit tests compile (add mocks via `mockall` if needed).

### 5.2 Services must not depend on concrete services

Steps:
1. For each app service that takes `Arc<ConcreteService>`:
   - ensure an outbound port exists (or create it in `engine-ports/src/outbound/`)
   - depend on `Arc<dyn Port>`
2. Update construction in composition root.

**Verify**
- Grep: no `Arc<ConcreteService>` in `engine-app/src/application/services/**` constructors (except when constructing the service itself internally, which should be rare and usually removed).

---

## Phase 6 — Fix IoC violations (services constructing services)

**Goal**: only composition roots construct; services don’t new() other services.

Steps:
1. For each violation where service calls `OtherService::new()`:
   - extract dependency to constructor
   - wire in composition root
2. For spawned tasks that lazily construct services, refactor to use injected factories or injected ports.

**Verify**
- `cargo xtask arch-check` (extend with a check if needed: forbid `::new(` of known services inside `engine-app`).

---

## Phase 7 — Composition-root purity

### 7.1 Composition must not store concrete types when a port exists

Steps:
1. In `engine-runner` factories and containers:
   - remove fields holding both `Arc<dyn Port>` and `Arc<Concrete>` versions
2. If generics force concretes (e.g., `NarrativeEventApprovalService<N>`), introduce a trait object abstraction.

**Verify**
- Add/enable an arch-check that flags `Arc<SomeConcreteService>` fields in composition modules.

---

## Phase 8 — Boundary purity follow-ups (optional but recommended)

These are not the core hexagonal rules, but they improve correctness and testability.

### 8.1 Remove direct `rand` usage behind `RandomPort`

Steps:
1. Create/confirm `RandomPort` in outbound ports.
2. Move direct `rand` usage from adapters/app into `engine-adapters` implementation.

### 8.2 Reduce multi-lock `Arc<Mutex>` patterns

Steps:
1. Consolidate related states into one lock or use atomics.
2. Keep concurrency primitives in adapters.

### 8.3 Fix partial batch failure semantics

Steps:
1. Move batch queue orchestration into an application service.
2. Define all-or-nothing vs partial-success semantics explicitly.

---

## Phase 9 — Switch checks from WARNING → FAIL

When the refactor reaches low-warning state, flip the enforcement switches:

1. Glob re-exports: fail on any occurrence.
2. App inbound dependencies: fail if app imports inbound traits (except explicit ctx DTO allowlist).
3. DTO shadow copies: fail on collisions.

**Verify**
- `cargo xtask arch-check` fails on regressions.
- CI gate uses `cargo xtask arch-check`.

---

## Appendix A — Superseded plans

The following documents contained overlapping scope and are superseded by this plan.

They have been deleted after consolidation to keep `docs/plans/` single-source-of-truth:

- `PORT_ADAPTER_TECH_DEBT_REMEDIATION.md`
- `PORT_ADAPTER_TECH_DEBT_VALIDATION.md`
- `ADDITIONAL_HEXAGONAL_VIOLATIONS.md`
- `ARCHITECTURE_GAP_REMEDIATION_PLAN.md`
- `CHALLENGE_APPROVAL_REFACTORING_PLAN.md`
- `TECH_DEBT_PHASE_2_PLAN.md`

If there are remaining non-hex items in those plans (feature refactors, performance work), they should be moved into system-specific plans.
