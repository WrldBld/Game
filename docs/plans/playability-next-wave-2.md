**Created:** January 27, 2026
**Status:** Draft
**Owner:** OpenCode
**Scope:** Next wave playability plan (post P2 completion)

---

## Ordered Priorities

1) **Map asset generation workflow** (P0) - ✅ COMPLETE
    - Implement `generate_map` pipeline via ComfyUI workflow
    - UI toggle should fail fast when workflow missing
    - Backend only - enqueues map asset generation to queue

2) **Lore linking + common lore** (P1) - ✅ COMPLETE (Backend)
    - ABOUT_* edges and COMMON_LORE exposure in player journal
    - Lore discovery tool wiring for LLM suggestions

3) **Challenge region availability** (P1)
    - Persist AVAILABLE_AT_REGION edges
    - Expose in challenge editor and enforce in availability checks

4) **Observation snapshot + live updates** (P1)
    - Player receives observation snapshot + incremental updates
    - Observation UI panel in player view

5) **Story arc event chains** (P2)
    - Editor workflows for chains, activation, and visibility
    - DM event chain overview in Story Arc tab

---

## Expansion Needed

- [x] Expand map asset generation plan (backend + frontend + tests) - Backend complete
- [ ] Expand lore linking plan
- [ ] Expand challenge region availability plan
- [ ] Expand observation snapshot plan
- [ ] Expand story arc chain plan

---

## Progress

- [x] P0 Map asset generation complete (Backend)
  - Added `map_workflow_name` to AppSettings
  - Added `map_workflow` to GenerateVisualStateRequest protocol
  - Implemented map generation with validation in `VisualStateCatalog`
  - Map generation enqueues `LocationStateMapOverlay` asset generation
  - Added unit tests for validation and enqueueing
  - Updated neo4j-schema.md to document LocationState/RegionState
- [ ] P0 Map asset generation frontend integration
- [x] P1 Lore linking + common lore complete (Backend)
  - Added `AboutTarget` enum in `domain/src/value_objects/about_target.rs` with typed IDs
  - Added ABOUT_* edge methods to `LoreRepo` trait: `link_lore_about`, `unlink_lore_about`, `get_lore_about_targets`
  - Added COMMON_LORE edge methods to `LoreRepo` trait: `mark_common_lore`, `unmark_common_lore`, `get_location_common_lore`
  - Implemented ABOUT_* and COMMON_LORE methods in `Neo4jLoreRepo`
  - Added use case methods in `LoreOps`: `link_lore_about`, `unlink_lore_about`, `get_lore_about_targets`, `mark_common_lore`, `unmark_common_lore`, `get_location_common_lore`
  - Added protocol types: `AboutTargetData` and new `LoreRequest` variants
  - Updated WebSocket handler `ws_lore.rs` with link/unlink/common lore request handling
  - Updated `docs/architecture/neo4j-schema.md` with Lore node and ABOUT_*/COMMON_LORE edges
  - Updated `docs/systems/lore-system.md` with implementation status
- [ ] P1 Challenge region availability complete
- [ ] P1 Observation snapshot + updates complete
- [ ] P2 Story arc chains complete
- [ ] Code review + architecture review
- [ ] Checks/tests clean
