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

3) **Challenge region availability** (P1) - ✅ COMPLETE (Backend + Architecture Review)
    - Persist AVAILABLE_AT / AVAILABLE_AT_REGION edges
    - Expose in challenge editor and enforce in availability checks
    - Backend implementation:
      - Added `set_availability_at_location` and `set_availability_at_region` to ChallengeRepo trait
      - Implemented edge management in Neo4jChallengeRepo with `always_available` and `time_restriction` fields
      - Added `get_availability`, `list_available_at_location`, `list_available_at_region`, and `remove_availability` methods
      - Added availability fields to ChallengeSummary DTO (`available_at_location_id`, `available_at_region_id`, `availability_time_restriction`, `always_available`)
      - Updated CreateChallengeInput and UpdateChallengeInput with availability parameters
      - Updated CreateChallengeData and UpdateChallengeData in shared requests
      - Updated ws_challenge handler to parse availability fields as typed IDs
      - Added ChallengeAvailability struct to ports
      - **Architecture review remediations (backend only):**
        - ✅ Enforced availability in TriggerChallengePrompt and RollChallenge use cases
        - ✅ Added input validation to reject both location_id and region_id
        - ✅ Added input validation to enforce always_available constraints
        - ✅ Added availability edge cleanup when switching types or always_available
        - ✅ Fixed get_availability query to avoid cartesian join (using WITH + COALESCE)
        - ✅ Fixed default value to always_available=true when no edges exist
        - ✅ Added AvailabilityTimeRestriction enum value object in domain
        - ✅ Implemented ADR-011: ChallengeSummary uses typed IDs, conversion to wire at API layer
        - ⏳ N+1 query optimization for list_for_world_with_skills (TODO added)
    - Frontend implementation:
      - Added availability fields to player DTO `ChallengeData` (`available_at_location_id`, `available_at_region_id`, `availability_time_restriction`, `always_available`)
      - Updated `ChallengeService` to map availability fields from protocol in create/update operations
      - Updated `CreateChallengeRequest` and `UpdateChallengeRequest` DTOs with availability fields
      - Updated `ChallengeFormModal` with availability UI:
        - "Available everywhere" checkbox
        - Location selector dropdown (filtered by available locations)
        - Region selector dropdown (filtered by selected location)
        - Time restriction text input
        - Proper disable states when "always_available" is checked
      - Added availability badges to challenge list cards:
        - "Always Available" badge (blue)
        - "Location-Specific" or "Region-Specific" badge (purple)
      - Wired location/region data loading in `ChallengeLibrary` component
      - All hooks at component root (Dioxus 0.7 compliance)

4) **Observation snapshot + live updates** (P1)
   - Player receives observation snapshot + incremental updates
   - Observation UI panel in player view
   - **Additional remediation (validated):**
     - Enforce auth on ListObservations (pc_id must match or DM)
     - Canonicalize ObservationType persistence (no Display strings)
     - Direct sightings should upsert/overwrite non-Direct observations
     - ShareNpcLocation uses world game time, not wall clock
     - Validate region/location consistency for shared observations
     - Deduced info should surface in Observations (or remove Deduced type)
     - Approach events should create unrevealed observations

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
- [x] P1 Challenge region availability complete (Backend + Frontend)
- [ ] P1 Observation snapshot + updates complete
- [ ] P2 Story arc chains complete
- [ ] Code review + architecture review
- [ ] Checks/tests clean
