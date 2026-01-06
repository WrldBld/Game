# Behavior Testing + TDD Plan

## Goals

- Add **behavior-based tests** that demonstrate each system’s user-visible behavior exists and stays within spec.
- Prefer testing at the **use case boundary** (engine) and **protocol boundary** (WebSocket/HTTP) over testing private helpers.
- Enable incremental **TDD**: when adding or changing a behavior, add/adjust a test first, then implement.

## Non-goals

- Snapshot-heavy UI testing.
- Testing trivial getters/setters.
- Perfect coverage. We want a small set of high-signal tests per system.

## Test Pyramid (This Repo)

1. **Domain unit tests** (fast, pure): verify invariants and deterministic logic.
2. **Engine use case tests** (fast-ish, mock ports): verify orchestration and decisions.
3. **Engine integration tests** (slower, real deps): Neo4j + queue + Axum WebSocket; verify end-to-end behaviors.
4. **Player tests** (fast): state reducers/message handling and key UI behavior in isolation.

## Tooling + Conventions

- Use `mockall` for mocking port traits.
  - Port traits should use `#[cfg_attr(test, mockall::automock)]` as needed.
- Prefer `#[tokio::test]` for async tests.
- Use **arrange/act/assert** structure and name tests by behavior:
  - `when_<context>__then_<expected_behavior>`

### Suggested Test Support

Create a small `test_support` module (engine + player) to reduce boilerplate:

- Engine
  - `builders::{challenge(), pc(), world(), staging(), scene()}`
  - `fixtures::{fixed_clock(), fixed_random()}`
  - `mocks::{repos_with_defaults()}`

- Player
  - `fixtures::{ws_message(), state_with_world()}`

This keeps tests readable and minimizes copy/paste.

## Execution Phases (How We’ll Run This Plan)

This plan is intentionally executed in **small, verifiable phases**.

Principles:

- Each phase produces at least **one new regression/behavior guarantee**.
- Prefer **use-case/entity boundary tests first**, then add **integration tests** once we have a harness.
- **Commit at the end of each phase** (so we can bisect failures and keep PRs reviewable).

### Phase 0 — Baseline snapshot (completed)

Deliverables:

- Repository is clean and a baseline commit exists for the already-completed CR6 work.

Exit criteria:

- `cargo test -p wrldbldr-engine --lib` passes.

### Phase 1 — First TDD footholds (completed)

Deliverables:

- A small number of port traits are mockable (`mockall`) where it provides leverage.
- At least one high-signal behavior test exists for a previously-fixed bug.

Exit criteria:

- Engine library tests pass and the new behavior test guards a real failure mode.

### Phase 2 — Test support + more behavior tests (in progress)

Goal: expand use-case/entity behavior coverage across the highest-risk systems without needing real Neo4j/WS.

Deliverables:

- Minimal `engine` test helpers (only if they reduce duplication meaningfully).
- Additional use-case/entity behavior tests:
  - Game time suggestion behavior (manual/no-cost/auto-as-suggested)
  - Narrative trigger wiring is world-scoped at the port boundary

Exit criteria:

- Engine library tests pass.
- New tests fail if the regression reappears (e.g., dropping `world_id` from trigger calls).

### Phase 3 — WebSocket integration harness

Goal: enable end-to-end protocol tests without requiring production wiring.

Deliverables:

- A test harness that can:
  - spin up an Axum router with the `/ws` route
  - connect 1–2 WebSocket clients
  - send/receive protocol messages deterministically
- At least one end-to-end approval-flow test (time suggestion approval or challenge approval).

Exit criteria:

- Harness runs in CI/local without manual setup.
- One happy-path test proves a DM-only decision produces a world broadcast.

### Phase 4 — Persistence integration tests (Neo4j + SQLite)

Goal: lock down query correctness/performance risks and persistence behavior.

Deliverables:

- Neo4j repo integration tests for:
  - narrative trigger fallback is **world-scoped** and **bounded**
  - staging pending save is **batched** (no N+1)
- SQLite queue integration tests for DM approval persistence and recovery.

Exit criteria:

- Tests run using testcontainers (or an equivalent local-only approach).
- Regression tests cover the historical perf/correctness issues.

### Phase 5 — Player behavior tests (reducers / message handling)

Goal: ensure player state updates correctly in response to server messages.

Deliverables:

- Tests that validate player state transitions for:
  - time suggestion arrival
  - game time advanced broadcast
  - staging pending/approved

Exit criteria:

- Message handling logic can change safely without UI snapshots.

### Phase 6 — Coverage expansion (system-by-system)

Goal: add 1–2 behavior tests per system’s “critical flows”.

Deliverables:

- Each system has at least:
  - 1 use-case/entity behavior test
  - 1 integration test (WS or persistence) for the main happy path

Exit criteria:

- New bugs in core systems are more likely to break a test than ship silently.

## What To Test (By System)

Below, each section lists:
- **Primary behavior(s)** we need to lock down.
- **Recommended test layer(s)**.
- **Concrete test cases** (initial set).

---

## Game Time System

Spec: `docs/systems/game-time-system.md`

### Behaviors
- Time **never auto-advances** due to player actions.
- Player actions may create **time suggestions**; time advances only when DM approves/modifies or DM manually advances.
- Advancing time broadcasts an update to players.

### Tests
- Engine use case tests
  - `SuggestTime` maps action → suggestion (minutes/hours) and includes a stable reason string.
  - Movement/conversation/challenge use cases produce time suggestions but **do not mutate world time**.
- WebSocket integration tests
  - DM approves time suggestion → world time changes and `GameTimeAdvanced` (or equivalent) broadcast occurs.

Initial cases
- `when_player_moves__then_time_suggestion_created_and_time_not_advanced`
- `when_dm_approves_time_suggestion__then_time_advances_and_broadcasts`

---

## Navigation / Movement System

Spec: `docs/systems/navigation-system.md`

### Behaviors
- Entering a region triggers staging/scene resolution.
- Region entry should drive observation recording (when applicable).
- Movement should not bypass DM approval flows.

### Tests
- Engine use case tests
  - `EnterRegion` requests staging resolution and returns expected payload.
  - Entering a region with no valid connection returns a clear error.
- WebSocket integration tests
  - Player sends move → DM receives staging approval request (when needed).

Initial cases
- `when_entering_unstaged_region__then_dm_approval_required`
- `when_entering_staged_region_with_valid_ttl__then_no_new_approval`

---

## Staging System

Spec: `docs/systems/staging-system.md`

### Behaviors
- NPC presence is **DM-approved** before players see it.
- Hidden NPCs are excluded from player-facing presence.
- Approved staging persists for TTL and is reused when still valid.

### Tests
- Engine entity/use case tests (mock repos)
  - Rule-based suggestion contains correct candidate NPCs and relationship metadata.
  - Approval application persists staging with correct NPC edge properties (including `is_hidden_from_players`).
- Neo4j integration tests
  - `save_pending_staging` writes all NPC edges in one batch (regression guard for N+1).
  - `get_history` ordering and TTL semantics.
- WebSocket integration tests
  - Player enters region → receives `StagingPending`, later `StagingReady` after DM approval.

Initial cases
- `when_dm_approves_staging__then_players_receive_only_visible_npcs`
- `when_staging_ttl_valid__then_reuse_previous_without_new_approval`

---

## Visual State System

Spec: `docs/systems/visual-state-system.md`

### Behaviors
- State resolution honors:
  - hard rules
  - priority
  - default fallback
- Resolved states propagate to player-facing payloads.

### Tests
- Engine unit tests (already exist in `use_cases/visual_state/resolve_state.rs`)
  - Add tests for priority ties, multiple matches, default selection.
- Staging integration tests
  - During staging, resolved visual state is attached and preserved through approval.

Initial cases
- `when_multiple_states_match__then_highest_priority_wins`
- `when_no_states_match__then_default_state_used`

---

## Challenge System

Spec: `docs/systems/challenge-system.md`

### Behaviors
- Rolls always produce a DM approval request before effects apply.
- DM accept/edit triggers effects and marks challenge resolved.
- PC-dependent triggers apply to the correct target PC.

### Tests
- Engine use case tests
  - `RollChallenge::execute` produces an approval request with `pc_id` set.
  - `ResolveOutcome::execute_for_pc` executes PC-dependent triggers (GiveItem/ModifyStat/Reveal persistent info).
- WebSocket integration tests
  - Player rolls → DM receives outcome approval payload.
  - DM accepts → players receive resolved message; effects persisted.

Initial cases
- `when_player_rolls_challenge__then_dm_approval_enqueued`
- `when_dm_accepts_outcome__then_effects_execute_and_mark_resolved`

---

## Dialogue / Conversation System

Spec: `docs/systems/dialogue-system.md`

### Behaviors
- Conversation start/continue/end flows preserve world + NPC context.
- LLM suggestions go through DM approval (NPC response).

### Tests
- Engine use case tests
  - `StartConversation` fails if NPC not present/staged.
  - `ContinueConversation` enqueues LLM request with correct prompt context.
- Queue processing tests
  - Processing a dialogue LLM request creates a DM approval request.

Initial cases
- `when_npc_not_present__then_cannot_start_conversation`
- `when_llm_response_generated__then_dm_approval_created`

---

## Narrative System + Narrative Resolution

Specs: `docs/systems/narrative-system.md`, `docs/systems/narrative-resolution-system.md`

### Behaviors
- Trigger evaluation is bounded and world-scoped.
- Trigger execution updates flags/scene/etc deterministically.

### Tests
- Engine use case tests
  - Trigger search is world-scoped and limited (regression guard for unbounded scans).
  - Trigger results are stable given the same inputs.
- Neo4j integration tests
  - Ensure queries use parameters (no string concat); validate bounded query plans where feasible.

Initial cases
- `when_region_triggers_fallback__then_world_scoped_and_limited`

---

## Inventory System

Spec: `docs/systems/inventory-system.md`

### Behaviors
- Equip/unequip/drop/pickup enforce inventory membership.
- `GiveItem` trigger creates item in PC’s world and adds to PC inventory.

### Tests
- Engine entity tests
  - `give_item_to_pc` creates item and adds edge.
  - Drop removes inventory edge and places item in region.

Initial cases
- `when_give_item_trigger__then_item_created_and_attached_to_pc`

---

## Observation System

Spec: `docs/systems/observation-system.md`

### Behaviors
- Recording visits creates observations only for visible NPCs.
- Persistent “deduced info” is recorded when requested.

### Tests
- Engine entity tests
  - `record_visit` creates observations for visible NPCs only.
  - `record_deduced_info` persists journal entry.

---

## Lore System

Spec: `docs/systems/lore-system.md`

### Behaviors
- CRUD and query filtering (by tags/category) are correct.

### Tests
- Engine entity tests + Neo4j integration tests

---

## Asset System

Spec: `docs/systems/asset-system.md`

### Behaviors
- Asset generation requests enqueue jobs.
- Expression sheet slicing fails loudly (no fake success).

### Tests
- Engine use case tests
  - Enqueue asset generation.
  - Expression sheet slicing error is surfaced.

---

## Prompt Template System

Spec: `docs/systems/prompt-template-system.md`

### Behaviors
- Prompt templates render deterministically and include required context fields.

### Tests
- Domain/unit tests

---

## WebSocket Protocol (Cross-Cutting)

Spec: `docs/architecture/websocket-protocol.md`

### Behaviors
- Role-gating: DM-only messages are rejected for players.
- Broadcast scoping: world-only events go only to that world.
- Approval flows: each approval type triggers correct downstream effects.

### Tests
- WebSocket integration tests (spawn Axum app, connect client(s), send protocol messages)

Initial cases
- `when_non_dm_sends_dm_command__then_error`
- `when_dm_approves_staging__then_world_broadcast`

---

## Implementation Phases

### Phase 1 (Now): Establish TDD footholds
- Add/expand port mocks (done selectively via `automock`).
- Add 1–2 use case tests per high-risk system:
  - Game time suggestion vs advancement
  - Challenge roll → approval
  - Challenge outcome → triggers execute (started)
  - Narrative trigger bounds

### Phase 2: Stabilize core multiplayer flows
- WebSocket integration test harness
- Staging approval flow end-to-end
- Time suggestion approval end-to-end

### Phase 3: Persistence correctness
- Neo4j testcontainers for repo integration tests
- Queue persistence tests (SQLite)

### Phase 4: Player behavior tests
- Player state reducers: incoming messages update UI state correctly
- Minimal component-level tests where state logic is non-trivial

## “Definition of Done” for a System

A system is “well tested” when:
- There is at least one use case-level behavior test per critical flow.
- There is at least one integration test covering the main happy-path across WebSocket.
- Any prior bugfix has a regression test.
