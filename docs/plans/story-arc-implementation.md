**Created:** January 30, 2026
**Status:** In progress
**Owner:** OpenCode
**Scope:** Story Arc (Event Chain) integration + architecture fixes

---

## Gameplay Impact

### Executive Summary

The pending items tighten narrative state visibility and actionability across approvals, challenges, lore, and time. Phase 3 hooks in `docs/plans/story-arc-implementation.md` are the next gameplay unlocks; StoryEvent edge-backed data in `docs/architecture/neo4j-schema.md` and `docs/systems/narrative-system.md` is what makes "what happened" queryable for both players and DMs. ADR-011 protocol boundary fixes keep that growth stable by preventing wire concerns from leaking into use cases.

### Feature-to-Gameplay Mapping (per item)

#### 1) ADR-011 protocol boundary fixes & StoryEvent protocol updates

Why it matters: Story events are the "ledger" of what happened. Making the protocol consistent and boundary-safe allows story history to evolve without breaking gameplay logic.

Example: The DM sees "Baron's Arrival triggered by Talk to NPC: Marcus at Rusty Anchor / Bar Counter, Day 3 Evening."

Refs: `docs/architecture/ADR-011-protocol-conversion-boundaries.md`, `docs/plans/story-arc-implementation.md`.

#### 2) StoryEvent edge-backed fields

Why it matters: Edge-backed context (scene, time, location, participants) makes narrative history a usable gameplay feedback loop.

Example: A player recap can show "You spoke to Marcus at the Rusty Anchor," driven by `AT_LOCATION`, `AT_TIME`, and `INVOLVES_*` edges.

Refs: `docs/architecture/neo4j-schema.md`, `docs/systems/narrative-system.md`.

#### 3) EventChain color policy decision

Why it matters: Color is a UX affordance for the DM's story-arc scanability. Persisting it ensures consistent visual grouping across sessions.

Example: DM labels "Baron's Downfall" in amber for Act II; the color is saved and round-tripped, allowing consistent arc visualization.

Refs: `docs/plans/story-arc-implementation.md`, `docs/systems/narrative-system.md`.

#### 4) Phase 3 story-arc integration hooks

- Trigger evaluation after approvals: DM approvals directly advance story arcs.
  Example: Approving a bartender response immediately triggers "Baron's Arrival."
- Challenge success into TriggerContext: outcomes drive story flow.
  Example: Winning a duel unlocks a safehouse scene.
- Lore reveal effect: narrative events surface knowledge to players.
  Example: Event reveals "Baron frequents docks at night" in the journal.
- GameTime trigger alignment: time-based beats fire predictably.
  Example: "Day 3 Morning: Festival begins."

Refs: `docs/plans/story-arc-implementation.md`, `docs/systems/narrative-system.md`, `docs/systems/game-time-system.md`.

### Player Experience Impact

- Approvals and challenge results produce visible, immediate story consequences.
- Lore reveal effects make story beats tangible (journal/knowledge).
- Time-based beats feel coherent and predictable.
- Dialogue history gains meaningful context (where/when/who).

### DM Experience Impact

- Fast audit of "what fired, where, and why."
- Reliable chain progression from approvals and challenge outcomes.
- Cleaner visual navigation via EventChain color (persisted).
- Better pacing control from aligned time triggers.

### Risks if Not Implemented

- Approvals/challenge outcomes won't advance story arcs; narrative stalls.
- Story history stays opaque; recaps and filtering are weak.
- Protocol drift causes fragile gameplay changes (ADR-011).
- Time-based beats remain inconsistent with docs, eroding DM trust.

### Validation / Acceptance Criteria

- Use cases return domain/use-case types; protocol conversion happens at API boundary.
- StoryEvent responses include context from edges (scene, location/region, time, participants).
- EventChain color persisted and round-tripped.
- Dialogue approvals and challenge results can trigger narrative evaluation.
- Lore reveal effects produce player-visible updates.
- GameTime trigger behavior matches docs (or docs aligned to implementation).

## Findings (verbatim summaries)

**Code Review (Event Chains)**
- `set_event_active` is a no-op (EnableEvent/DisableEvent effects dropped).
- `get_completed_events` always returns empty; EventCompleted triggers never fire.
- `get_triggers_for_region` ignores region + chain state; triggers can fire anywhere.
- `selected_outcome` not persisted/hydrated; outcome-based triggers never match.
- Chains don’t advance on event trigger; `completed_events` not updated.
- EventChain ops don’t validate event existence/world ownership.

**Architecture Review (Event Chains)**
- ADR-009: EventChainOps depends on NarrativeOps wrapper (pass-through CRUD).
- ADR-011: use case builds protocol-shaped DTOs; API serializes directly.
- ADR-008: ChainStatus is presentation DTO in domain; EventChain lives under entities not aggregates.
- Domain/UI field: `color: Option<String>` stored in domain.

**Gameplay Review (Value Opportunities)**
- Dialogue → narrative triggers not evaluated (DialogueTopic/NpcAction never fire).
- Challenge outcomes not added to trigger context; ChallengeCompleted unreliable.
- Lore reveal not available as narrative effect.
- Time-based triggers don’t use GameTime; doc/impl mismatch.

---

## Plan

### Phase 1 — Persistence + trigger correctness (backend)
-- [x] Implement `set_event_active` with Neo4j update
-- [x] Persist + hydrate `selected_outcome`
-- [x] Implement `get_completed_events` (chain completion source)
-- [x] Scope `get_triggers_for_region` by region + chain state
-- [x] Advance chains on event trigger (update chain completion)
-- [x] Validate event existence/world ownership in EventChainOps

### Phase 2 — ADR compliance (backend)
- [x] EventChainOps injects NarrativeRepo directly (remove NarrativeOps wrapper)
- [x] Move protocol conversion to API layer
- [x] Move ChainStatus out of domain; relocate EventChain to aggregates
- [x] Decide on `color` (value object vs UI-only) - **Decision**: Persisted as string field

### Phase 3 — Integration hooks (backend)
- [ ] Evaluate narrative triggers after dialogue approvals
- [ ] Populate challenge successes in TriggerContext
- [ ] Add Lore reveal narrative effect
- [ ] Use GameTime period/day triggers (or align docs to code)

#### Phase 3 Implementation Plan

**Ordered Steps**
1. Add dialogue approval trigger evaluation entrypoint in the approval flow and wire it to the narrative trigger evaluator.
2. Extend TriggerContext with challenge success outcomes and hydrate it in challenge resolution paths.
3. Introduce Lore reveal as a narrative effect and apply it via the narrative effect executor.
4. Align GameTime-trigger behavior with documentation (either implement period/day evaluation or update docs to match current engine behavior).

**File Targets**
- engine/src/use_cases/approval/ (dialogue approval flow)
- engine/src/use_cases/narrative/trigger_evaluator.rs
- engine/src/use_cases/challenge/ (challenge resolution + outcomes)
- engine/src/use_cases/narrative/types.rs (TriggerContext additions)
- engine/src/use_cases/narrative/effects/ (Lore reveal effect)
- docs/architecture/websocket-protocol.md (if protocol needs new lore reveal message)
- docs/systems/ (if narrative effect needs system docs)
- docs/architecture/ADR-011-protocol-conversion-boundaries.md (if conversion boundaries change)
- docs/architecture/ADR-008-tiered-encapsulation.md (if new domain type requires tier decision)

**Tests**
- engine/tests/use_cases/approval_dialogue_triggers.rs (approval triggers invoke narrative evaluation)
- engine/tests/use_cases/challenge_trigger_context.rs (challenge success feeds TriggerContext)
- engine/tests/use_cases/narrative_lore_reveal.rs (effect produces expected outcome)
- engine/tests/use_cases/narrative_game_time_triggers.rs (period/day trigger evaluation vs docs)
- Update/add domain tests if new value objects or events are introduced

**Remediation Plan**
- Dialogue approval trigger evaluation
  - Fix: Invoke narrative trigger evaluation after dialogue approval is persisted.
  - Target: engine/src/use_cases/approval/
  - Order: Step 1
  - Tests: approval_dialogue_triggers.rs
  - ADR updates: None expected
- Challenge successes in TriggerContext
  - Fix: Add challenge outcome flags to TriggerContext and populate during challenge resolution.
  - Target: engine/src/use_cases/challenge/, engine/src/use_cases/narrative/types.rs
  - Order: Step 2
  - Tests: challenge_trigger_context.rs
  - ADR updates: None expected
- Lore reveal narrative effect
  - Fix: Add effect type + executor branch; optionally protocol message in API layer.
  - Target: engine/src/use_cases/narrative/effects/, engine/src/api/
  - Order: Step 3
  - Tests: narrative_lore_reveal.rs
  - ADR updates: ADR-011 only if conversion boundary changes
- GameTime triggers or doc alignment
  - Fix: Implement period/day checks against GameTime or update docs to reflect current behavior.
  - Target: engine/src/use_cases/narrative/trigger_evaluator.rs, docs/architecture/websocket-protocol.md
  - Order: Step 4
  - Tests: narrative_game_time_triggers.rs
  - ADR updates: None unless behavior change affects documented tier rules

### Phase 4 — UI enhancements (frontend)
- [ ] Apply Story Arc UI improvements from narrative-system.md
- [ ] Event chain list: readiness + blockers + quick actions
- [ ] Editor: validation banner + event roster
- [ ] Visualizer: badges + inspector panel

---

## Progress

- [x] Phase 1 complete
- [x] Phase 2 complete
- [ ] Phase 3 complete
- [ ] Phase 4 complete
- [ ] Code review + architecture review
- [ ] Checks/tests clean
