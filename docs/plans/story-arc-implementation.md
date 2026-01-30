**Created:** January 30, 2026
**Status:** In progress
**Owner:** OpenCode
**Scope:** Story Arc (Event Chain) integration + architecture fixes

---

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
- [ ] EventChainOps injects NarrativeRepo directly (remove NarrativeOps wrapper)
- [ ] Move protocol conversion to API layer
- [ ] Move ChainStatus out of domain; relocate EventChain to aggregates
- [ ] Decide on `color` (value object vs UI-only)

### Phase 3 — Integration hooks (backend)
- [ ] Evaluate narrative triggers after dialogue approvals
- [ ] Populate challenge successes in TriggerContext
- [ ] Add Lore reveal narrative effect
- [ ] Use GameTime period/day triggers (or align docs to code)

### Phase 4 — UI enhancements (frontend)
- [ ] Apply Story Arc UI improvements from narrative-system.md
- [ ] Event chain list: readiness + blockers + quick actions
- [ ] Editor: validation banner + event roster
- [ ] Visualizer: badges + inspector panel

---

## Progress

- [x] Phase 1 complete
- [ ] Phase 2 complete
- [ ] Phase 3 complete
- [ ] Phase 4 complete
- [ ] Code review + architecture review
- [ ] Checks/tests clean
