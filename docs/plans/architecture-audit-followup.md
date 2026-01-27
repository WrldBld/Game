**Created:** January 27, 2026
**Status:** In progress
**Owner:** OpenCode
**Scope:** Remediate full-architecture audit findings

---

## Findings (verbatim)

**Audit A**
1) ADR-011 boundary leaks
- use_cases/challenge parses UUIDs from String IDs
- use_cases/staging/approve uses Option<String> IDs
- use_cases/lore uses String IDs/timestamps in DTOs
- use_cases/queues parses typed IDs from strings inside use case

2) ADR-008 tier mismatch
- value_objects/relationship.rs documented as simple data but implemented as encapsulated accessors

3) Tech debt
- Cypher duplication across character/lore/staging repos
- String validation duplication across domain entities (common/string.rs not reused)

**Audit B**
1) Typed ID drift in engine/store DTOs
- use_cases/assets uses raw Uuid
- challenge resolution uses String parsed to Uuid
- staging request IDs are Uuid strings
- TimeSuggestionStore keyed by Uuid
- queue_types uses raw Uuid in conversation_id and action_item_id
- staging approve input uses Option<String>

---

## Plan

### Phase 1 — Typed IDs + ADR-011 boundaries (engine/shared)
- [x] Convert use-case inputs to typed IDs (challenge, staging approve, queues, assets, lore)
- [x] Move string/UUID parsing to API/queue boundary
- [x] Replace raw UUIDs in stores/queue_types with typed IDs
- [x] Update tests and API handlers

### Phase 2 — ADR-008 alignment (domain)
- [x] Make Relationship a Tier‑4 simple data struct (public fields) or reclassify docs
- [x] Consolidate string validation using common/string.rs helpers

### Phase 3 — Neo4j query deduplication
- [x] Extend helpers.rs query fragments to lore/character repos
- [x] Replace duplicated MATCH/RETURN fragments where practical

---

## Progress

- [x] Phase 1 complete
- [x] Phase 2 complete
- [x] Phase 3 complete
- [x] Code review + architecture review
- [x] Checks/tests clean
