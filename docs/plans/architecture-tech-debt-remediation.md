**Created:** January 27, 2026
**Status:** In progress
**Owner:** OpenCode
**Scope:** Remediate remaining architecture tech debt

---

## Findings (verbatim)

1) Neo4j query duplication across repos (character/staging/narrative/scene/observation/world/item)
2) High fan-in “god” use cases: movement enter_region/exit_location, narrative execute_effects
3) PlayerCharacter mutations return () without outcome signaling (update_location/update_region/touch)

---

## Plan

### Phase 1 — Query deduplication (engine)
- [x] Inventory common Cypher fragments and add helpers
- [x] Replace duplicated MATCH/RETURN in character, staging, narrative, scene, observation, world, item repos

### Phase 2 — Decompose god use cases (engine)
- [x] Extract cohesive sub-ops for movement (staging resolution, narrative triggers, location update)
- [x] Extract effect execution sub-ops in execute_effects
- [x] Reduce constructor dependency fan-in

### Phase 3 — Domain events for PlayerCharacter mutations (domain + engine)
- [x] Introduce event enums for location/region/touch updates
- [x] Update mutation methods to return events
- [x] Update use cases to handle outcomes

---

## Progress

- [x] Phase 1 complete
- [x] Phase 2 complete
- [x] Phase 3 complete
- [ ] Architecture review (re-run)
- [x] Checks/tests clean
