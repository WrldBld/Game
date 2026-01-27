**Created:** January 27, 2026
**Status:** In progress
**Owner:** OpenCode
**Scope:** Post-audit remediation (Jan 27 full architecture sweep)

---

## Findings (verbatim)

1) StatBlock HP invariants not enforced
2) PlayerCharacter description silently drops invalid input
3) WantContext invariants not enforced (intensity/priority)
4) CalendarDefinition validation gaps
5) Queue DTOs use raw String/Uuid IDs
6) join_world_flow uses serde_json::Value (ADR-011 leak)
7) RollResult percent bounds not validated (0–100)
8) visual_state catalog uses activation_rules_json (serde_json::Value) in use case
9) visual_state catalog returns generation_batch_id as String (should be typed)
10) ProposedTool.arguments uses untyped serde_json::Value through queues

---

## Plan

### Phase 1 — Domain invariants
- [x] Enforce HP invariants in StatBlock setters (return Result or make private)
- [x] Replace PlayerCharacter description .ok() fallback with fail-fast Result
- [x] Validate WantContext intensity (0..=1) + priority (>=1)
- [x] Add CalendarDefinition validation (non-empty, positive values)
- [x] Validate RollResult percent bounds (0–100)

### Phase 2 — Typed IDs + ADR-011 boundary
- [ ] Replace raw IDs in queue_types with typed IDs (ConversationId, ApprovalRequestId, NarrativeEventId, ChallengeId)
- [ ] Remove serde_json::Value from join_world_flow use case outputs; move conversion to API
- [ ] Replace activation_rules_json input with typed rules/logic at API boundary
- [ ] Replace generation_batch_id String with typed BatchId internally, stringify at boundary

---

## Progress

- [x] Phase 1 complete
- [ ] Phase 2 complete
- [ ] Phase 3 complete (tool argument typing)
- [ ] Code review + architecture review
- [ ] Checks/tests clean
