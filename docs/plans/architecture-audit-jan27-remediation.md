**Created:** January 27, 2026
**Status:** In progress
**Owner:** OpenCode
**Scope:** Remediate Jan 27 architecture audits (A+B)

---

## Findings (verbatim)

**Audit A**
- ADR-011 boundary leaks: use_cases/challenge parses UUIDs from String IDs; use_cases/staging/approve uses Option<String> IDs; use_cases/lore uses String IDs/timestamps in DTOs; use_cases/queues parses typed IDs from strings inside use case.
- ADR-008 tier mismatch: value_objects/relationship.rs documented as simple data but implemented as encapsulated accessors.
- Tech debt: Cypher duplication across character/lore/staging repos; string validation duplication across domain entities (common/string.rs not reused).

**Audit B**
- Protocol boundary drift in use cases: session/directorial.rs uses wrldbldr_shared::DirectorialContext; join_world_flow.rs uses wrldbldr_shared::WorldRole.
- Typed ID leakage: story_events create returns raw Uuid; ai mod uses Option<Uuid> for world_id; queues and challenge resolution IDs use String/Uuid.
- God-object use case: queues/mod.rs depends on many repos.
- Error mapping loses context via e.to_string in use cases.

---

## Plan

### Phase 1 — Typed IDs + ADR-011 boundaries (engine)
- [x] Move protocol conversions out of use cases (session/directorial, join_world_flow)
- [x] Replace raw Uuid/String IDs in use-case inputs/outputs with typed IDs (story_events, ai, queues, challenge)
- [x] Parse/validate IDs at API boundary only
- [x] Update tests/fixtures for new signatures

### Phase 2 — Domain state modeling (domain + repos)
- [x] Replace boolean state flags with enums for NarrativeEvent + EventChain
- [x] Update persistence mapping in Neo4j repos
- [x] Add unit tests for state transitions
- [x] Remove legacy backward-compatibility handling from narrative_repo parse_* helpers
- [x] Remove legacy boolean field OR clauses from Cypher queries

### Phase 3 — Use case refactor + error mapping
- [ ] Decompose queue processing fan-in (introduce internal context/helper)
- [ ] Replace error stringification with typed errors (assets, visual_state, challenge)

### Phase 4 — Query/validation tech debt
- [ ] Extend Neo4j helper fragments to narrative/scene repos where duplication remains
- [ ] Consolidate string validation with common/string.rs where repeated

---

## Progress

- [x] Phase 1 complete
- [x] Phase 2 complete
- [ ] Phase 3 complete
- [ ] Phase 4 complete
- [ ] Code review + architecture review
- [ ] Checks/tests clean
