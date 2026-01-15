# Strict Review Remediation Plan

Status: Active
Owner: Codex
Last updated: 2026-01-14

## Scope

Address issues identified in the full code review (per `docs/architecture/review.md`) and adopt **strict** layering: use cases depend on repositories only, not port traits. This plan records each issue, its impact, and a concrete remediation path. It also records exclusions that are explicitly accepted for now.

## Constraints

- Domain crate remains pure (no I/O, no framework deps, no async).
- Use cases must inject repositories only (strict mode).
- Keep backward compatibility where noted (serialization wire formats, etc.).
- Free-form string fields are allowed for now (no newtypes), per user request.

## Issues and Remediation Paths

### 1) Hardcoded secret fallback
**Issue:** `NEO4J_PASSWORD` defaults to "password" if env var missing.
- Path: `crates/engine/src/main.rs:63`
- Severity: CRITICAL (secret in code)

**Remediation path:**
- Remove the hardcoded fallback; require env var or fail fast with a clear error message.
- Optionally support a `.env` default in dev via `dotenv` only (no literal in code).

**Acceptance criteria:**
- Engine startup fails with actionable error if `NEO4J_PASSWORD` unset.
- No default password literal in code.

---

### 2) Direct `Utc::now()` in use cases (Clock boundary violation)
**Issue:** Use cases call `Utc::now()` directly instead of going through `ClockPort`.
- Example paths: `crates/engine/src/use_cases/lore/mod.rs:245`, `crates/engine/src/use_cases/staging/request_approval.rs:90`, `crates/engine/src/use_cases/narrative/chains.rs:108`
- Severity: HIGH (boundary violation, testability)

**Remediation path:**
- Introduce repository-level time helpers or clock-aware repositories where needed.
- Inject repository wrappers into use cases; repositories consume `ClockPort` internally.
- Replace `Utc::now()` in use cases with repo or service calls that source time.

**Acceptance criteria:**
- `rg -n "Utc::now" crates/engine/src/use_cases` returns only tests.
- Use-case APIs do not accept `ClockPort` directly.

---

### 3) NarrativeEvent boolean state machine
**Issue:** Multiple booleans allow invalid combinations.
- Path: `crates/domain/src/aggregates/narrative_event.rs:112-120`, `crates/domain/src/aggregates/narrative_event.rs:136`
- Severity: MEDIUM (state integrity)

**Remediation path:**
- Replace boolean cluster with an enum state (e.g., `EventStatus` or `TriggerStatus`).
- Ensure serialization retains compatibility (if needed, use custom serde mapping).
- Update callers and tests to use the enum.

**Acceptance criteria:**
- No `is_active`, `is_triggered`, `is_repeatable`, `is_favorite` booleans on the aggregate.
- All state transitions are explicit and validated by the enum.

---

### 4) Mutation methods without domain events
**Issue:** Some aggregate mutations return `()` instead of events.
- Example paths: `crates/domain/src/aggregates/character.rs:505-517`, `crates/domain/src/aggregates/narrative_event.rs:551-564`, `crates/domain/src/aggregates/scene.rs:309-317`
- Severity: MEDIUM (loss of behavior observability)

**Remediation path:**
- Define specific domain events for these mutations (e.g., `CharacterStateChanged`, `NarrativeEventUpdated`, `SceneUpdated`).
- Update mutation signatures and callers accordingly.

**Acceptance criteria:**
- All public `&mut self` mutation methods return a domain event enum.

---

### 5) Use cases depend on port traits (strict mode violation)
**Issue:** Use cases depend directly on `Arc<dyn *Port>`.
- Example paths: `crates/engine/src/use_cases/approval/mod.rs:119` (QueuePort)
- Count: ~27 files use `Arc<dyn *Port>`, ~52 use-case files import `infrastructure::ports`
- Severity: LOW (architectural purity, test seams)

**Remediation path:**
- Create repository/service wrappers around non-repo ports (Queue, Clock, LLM, Random, Directorial, etc.).
- Update use cases to depend on these wrappers only.
- Update app composition and tests to pass wrappers.

**Acceptance criteria:**
- No `Arc<dyn *Port>` in `crates/engine/src/use_cases` (excluding tests if needed).
- `rg -n "infrastructure::ports" crates/engine/src/use_cases` shows only test-only uses.

---

### 6) Flaky timing in tests
**Issue:** `sleep()` in e2e tests can be nondeterministic.
- Path: `crates/engine/src/e2e_tests/approval_timeout_tests.rs:620`
- Severity: MEDIUM (test flakiness)

**Remediation path:**
- Replace time-based sleeps with controlled clocks or deterministic polling with explicit time advancement.
- Align with `ClockPort` or test harness controls.

**Acceptance criteria:**
- No sleeps for timing expectations in e2e tests; time control is deterministic.

---

## Explicit Exclusions (Documented)

- Validated string newtypes for asset paths/notes/tags are **deferred**. Free-form strings are allowed for now (user request). This remains a tracked tech-debt item and is intentionally not in this plan.

## Phases

1) **Security + Boundary Fixes** ✅
   - Remove hardcoded Neo4j password fallback.
   - Eliminate `Utc::now()` usage in use cases.

2) **Strict Port-to-Repository Enforcement** ✅
   - Add repo/service wrappers for ports.
   - Update use cases and wiring.
   - Update tests to mock repos only.

3) **Domain State Integrity** ✅
   - Refactor `NarrativeEvent` boolean state into enum.
   - Update serialization/tests.

4) **Domain Events for Mutations** ✅
   - Introduce event enums for `Character`, `NarrativeEvent`, `Scene` mutation methods.
   - Update callers.

5) **Test Determinism** ✅
   - Remove sleep-based timing tests and replace with deterministic clock/test harness approach.

## Validation Checklist

- [x] `rg -n "password" crates/engine/src/main.rs` has no default secret literals.
- [x] `rg -n "Utc::now" crates/engine/src/use_cases` only shows tests.
- [x] `rg -n "Arc<dyn .*Port" crates/engine/src/use_cases` is empty (excluding tests).
- [x] `NarrativeEvent` uses an enum state, no multi-boolean state.
- [x] All aggregate mutations return events (no `fn x(&mut self) {}` without return).
- [x] No sleep-based TTL assertions in e2e tests.
