**Created:** January 22, 2026
**Status:** Draft for expansion
**Owner:** OpenCode
**Scope:** Visual State Catalog + generation workflow

---

## Problem

Pre-staging currently requires visual state IDs, but those IDs are only guaranteed once a state is active. We need a catalog of visual states with IDs available before staging, plus a way to generate new states when none exist.

## Core Idea

Introduce a Visual State Catalog for each location/region. Visual states exist independently of staging as selectable options. Staging selects from existing visual states; if none fit, DM can generate and save a new one.

## Draft Data Model

VisualState (Location or Region scoped)
- `id`: derived from stable hash of prompt + model/workflow config
- `scope`: `LocationState` | `RegionState`
- `name`: human label (e.g., "Stormy Night")
- `description`: semantic description (e.g., "Rain-soaked alleys, flickering lanterns")
- `prompt`: generation prompt (optional for provenance)
- `backdrop_asset_id`: generated asset reference
- `map_asset_id`: optional, if map used
- `atmosphere`: optional field (fog/rain/etc)
- `rules`: optional triggers/conditions for rule-based auto-selection
- `tags`: optional categories for filtering
- `created_by`: DM/AI metadata
- `created_at`

## ID Strategy

IDs are derived from the generation prompt + model/workflow config (deterministic hash).
- Pros: same prompt yields same ID; avoids duplicates.
- Cons: prompt changes create new IDs (likely desired).
- Policy: allow DM to rename/describe without changing ID.

## Workflow

1) DM opens pre-stage and sees available visual states for region/location.
2) If none fit, DM clicks "Generate Visual State."
3) DM enters description or prompt; system runs generation workflow, saves VisualState, returns ID.
4) Pre-stage uses selected VisualState IDs for approval.

## Rule-Based Resolution

Visual state "rules" can determine defaults during staging:
- Time of day, weather, flags, current narrative state.
- Auto-resolution selects best matching visual state.
- DM can override.

## UI/UX Draft

Pre-stage modal:
- Dropdowns for Location/Region Visual State
- "Generate New" button next to each dropdown
- Inline preview (thumbnail + description)

Staging approval UI:
- Same dropdowns (already present) + "Generate New"
- "Details" drawer shows description + prompt + rules

## Implications

- A location/region can be staged without an active visual state as long as a catalog option is selected.
- Generation is part of DM toolchain for pre-stage and approval flows.
- Storage for prompts and assets is recommended.

---

## Next Steps (to expand)

- [x] Define user stories and acceptance criteria
- [x] Plan UI/UX mockups for DM flows - See: [Visual State Catalog UI Design](../designs/visual-state-catalog-ui.md)
- [ ] Identify existing asset generation workflows to reuse
- [ ] Propose concrete data structures + protocol contracts
- [ ] Define REST endpoints for catalog operations
- [ ] Define WebSocket messages for real-time updates

Specs in progress:
- `docs/plans/visual-state-catalog-spec.md`

---

## Draft User Stories (Proposed)

**US-VS-012:** DM can browse cataloged visual states for a location/region.
- Acceptance: pre-stage dropdowns show options with name + description + thumbnail.

**US-VS-013:** DM can generate a new visual state from pre-stage or approval.
- Acceptance: "Generate Visual State" action creates a new catalog entry and auto-selects it.

**US-VS-014:** Visual state IDs are deterministic from prompt + workflow.
- Acceptance: same prompt/workflow returns existing ID; renaming does not change ID.

**US-VS-015:** DM can view visual state details.
- Acceptance: details drawer shows prompt, rules, and asset references.

**US-VS-016:** DM can override resolved state in approval using catalog IDs.
- Acceptance: approval requires at least one selected visual state ID.

**US-VS-017:** System auto-resolves visual state from catalog rules.
- Acceptance: rule evaluation selects highest-priority match; DM can override.

---

## Reuse Patterns (Codebase)

**Visual state resolution**
- `crates/engine/src/use_cases/visual_state/resolve_state.rs`
- `crates/domain/src/entities/location_state.rs`
- `crates/domain/src/entities/region_state.rs`

**Asset generation pipeline**
- `crates/engine/src/use_cases/assets/mod.rs`
- `crates/domain/src/entities/gallery_asset.rs`
- `crates/engine/src/infrastructure/neo4j/asset_repo.rs`

**Workflow + prompt metadata**
- `crates/shared/src/workflow.rs`
- `crates/engine/src/prompt_templates.rs`
- `crates/engine/src/infrastructure/prompt_templates.rs`

**Shared visual state protocol types**
- `crates/shared/src/types.rs` (`ResolvedVisualStateData`, `StateOptionData`)
