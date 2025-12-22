# Active Development

Active implementation tracking for WrldBldr user stories.

**Current Phase**: Phase B - Player Knowledge & Agency (COMPLETE)  
**Last Updated**: 2025-12-20

---

## Phase Overview

| Phase | Focus | Status | Est. Effort |
|-------|-------|--------|-------------|
| A | Core Player Experience | **COMPLETE** | 3-4 days |
| B | Player Knowledge & Agency | **COMPLETE** | 4-5 days |
| C | DM Tools & Advanced Features | **NEXT** | 5-7 days |

---

## Phase A: Core Player Experience - COMPLETE

All Phase A stories have been implemented. See Completed section for details.

---

## Phase B: Player Knowledge & Agency - COMPLETE

All Phase B stories have been implemented. See Completed section for details.

---

## Phase C: DM Tools & Advanced Features

Improve DM workflow. These don't block player gameplay.

### US-STG-013 / US-OBS-006: Hidden NPCs + Unrevealed Interactions

| Field | Value |
|-------|-------|
| **Status** | In Progress |
| **Priority** | High |
| **Effort** | 2-3 days |
| **Systems** | [Staging](../systems/staging-system.md), [Observation](../systems/observation-system.md) |

**Goal**: Support NPCs that are staged as present-but-hidden from players, while still allowing DM-triggered approach events that may or may not reveal identity.

**Player-facing behavior**:
- Hidden NPCs do not appear in `SceneChanged.npcs_present` or `StagingReady.npcs_present`.
- Unrevealed approaches display as **"Unknown Figure"** with no sprite/portrait.
- Unrevealed interactions are recorded as observations and shown as **"Unknown Figure"** in Known NPCs.

**Implementation checklist**:
- [ ] Protocol: add `reveal` to approach events
- [ ] Protocol: add `is_hidden_from_players` to staged/approved NPCs
- [ ] Engine: persist hidden flag in staging (`INCLUDES_NPC`)
- [ ] Engine: filter hidden NPCs from player presence messages
- [ ] Engine: add `is_revealed_to_player` to observations + persistence
- [ ] Engine: scrub observation API for unrevealed entries
- [ ] Engine: approach event handler supports `reveal=false`
- [ ] Player UI: staging approval + pre-stage support hidden toggle
- [ ] Player UI: observations refresh via shared state; show Unknown Figure
- [ ] Security: stop `DerivedSceneRequest.pc_id` from auto-creating observations
- [ ] Maintenance: audit/remove unused HTTP endpoints (e.g. `POST /api/regions/{region_id}/scene`)
- [ ] Validate: `cargo check --workspace` and `cargo xtask arch-check`

**Key files**:
- `crates/protocol/src/messages.rs`
- `crates/domain/src/entities/staging.rs`
- `crates/domain/src/entities/observation.rs`
- `crates/engine-adapters/src/infrastructure/websocket.rs`
- `crates/engine-adapters/src/infrastructure/persistence/staging_repository.rs`
- `crates/engine-adapters/src/infrastructure/persistence/observation_repository.rs`
- `crates/engine-adapters/src/infrastructure/http/observation_routes.rs`
- `crates/engine-adapters/src/infrastructure/http/region_routes.rs`
- `crates/player-ui/src/presentation/components/dm_panel/staging_approval.rs`
- `crates/player-ui/src/presentation/components/dm_panel/location_staging.rs`

---

## Architecture Maintenance

### ARCH-SHIM-001: Remove internal `pub use crate::...` and `pub(crate) use ...` shims

| Field | Value |
|-------|-------|
| **Status** | Done |
| **Priority** | High |
| **Effort** | 0.5 days |
|
**Goal**: Reduce redundant module-level shims and make imports point at the true owning module.

**Planned work (checklist)**:
- [x] Remove `pub use crate::...` re-exports (player-ui session_state facade)
- [x] Remove `pub use crate::...` re-export (player-app services -> dto)
- [x] Remove `pub(crate) use ...` re-export (engine-adapters export mod)
- [x] Update call sites to import from source modules
- [x] Extend `cargo xtask arch-check` to forbid:
  - `pub use crate::...`
  - `pub(crate) use ...`
- [x] Run `cargo xtask arch-check` and `cargo check --workspace`

**Target files**:
- `crates/player-ui/src/presentation/state/session_state.rs`
- `crates/player-app/src/application/services/mod.rs`
- `crates/engine-adapters/src/infrastructure/export/mod.rs`
- `crates/engine-adapters/src/infrastructure/persistence/world_repository.rs`
- `crates/player-ui/src/presentation/components/settings/mod.rs`
- `crates/xtask/src/main.rs`


### US-CHAL-010: Region-level Challenge Binding

| Field | Value |
|-------|-------|
| **Status** | Not Started |
| **Priority** | Medium |
| **Effort** | 2 days |
| **System** | [Challenge](../systems/challenge-system.md) |

**Description**: Bind challenges to specific regions, not just locations.

**Implementation Notes**:
- Engine: Schema referenced but not implemented
- Player: Not started
- Add `AVAILABLE_AT_REGION` edge to challenge repository
- Add `list_by_region()` repository method
- Add region filter to challenge service
- Update Director Mode to show region-bound challenges

---

### US-SCN-009: Scene Entry Conditions

| Field | Value |
|-------|-------|
| **Status** | Partial |
| **Priority** | Medium |
| **Effort** | 0.5 days |
| **System** | [Scene](../systems/scene-system.md) |

**Description**: Evaluate conditions before showing a scene.

**Implementation Notes**:
- Engine: `SceneCondition` enum exists, evaluation missing
- Player: N/A (engine feature)
- Add `evaluate_conditions()` helper function
- Call from `scene_resolution_service.resolve_scene()`
- Check CompletedScene, HasItem, KnowsCharacter, FlagSet conditions

---

### US-NAR-009: Visual Trigger Condition Builder

| Field | Value |
|-------|-------|
| **Status** | Not Started |
| **Priority** | Low |
| **Effort** | 3-4 days |
| **System** | [Narrative](../systems/narrative-system.md) |

**Description**: Visual builder for narrative trigger conditions.

**Implementation Notes**:
- Engine: Trigger schema exists
- Player: Not started
- Add `/api/triggers/schema` endpoint for available types
- Create visual builder component with dropdowns
- Support all trigger types (location, NPC, challenge, time, etc.)
- Add AND/OR/AtLeast logic selection

---

### US-AST-010: Advanced Workflow Parameter Editor

| Field | Value |
|-------|-------|
| **Status** | Not Started |
| **Priority** | Low |
| **Effort** | 2 days |
| **System** | [Asset](../systems/asset-system.md) |

**Description**: Edit ComfyUI workflow parameters in UI.

**Implementation Notes**:
- Engine: Complete (workflow config exists)
- Player: Basic config exists
- Add prompt mapping editor
- Add locked inputs configuration
- Add style reference detection display
- Optional: Raw JSON viewer/editor

---

## Completed

Stories moved here when fully implemented.

### US-CHAR-009: Inventory Panel

| Field | Value |
|-------|-------|
| **Completed** | 2025-12-18 |
| **System** | [Character](../systems/character-system.md) |

**Implementation**: Full inventory panel with item categories and actions.
- Engine: `GET /api/characters/{id}/inventory` endpoint
- Player: `InventoryPanel` component with category tabs (All/Equipped/Consumables/Key)
- `ItemData`, `InventoryItemData` DTOs
- `get_inventory()` on CharacterService
- Use item action wired to player actions

**Files**:
- `crates/engine-app/src/application/dto/item.rs`
- `crates/player-ui/src/presentation/components/inventory_panel.rs`
- `crates/player-app/src/application/services/character_service.rs`
- `crates/player-app/src/application/dto/world_snapshot.rs`

---

### US-OBS-004/005: Known NPCs Panel

| Field | Value |
|-------|-------|
| **Completed** | 2025-12-18 |
| **System** | [Observation](../systems/observation-system.md) |

**Implementation**: Panel showing observed NPCs with last seen info.
- `KnownNpcsPanel` component with observation cards
- `ObservationService` with `list_observations()` method
- Observation type icons (direct/heard/deduced)
- Display last seen location and game time
- Click NPC to initiate talk action

**Files**:
- `crates/player-ui/src/presentation/components/known_npcs_panel.rs`
- `crates/player-app/src/application/services/observation_service.rs`
- `crates/player-ui/src/presentation/views/pc_view.rs`

---

### US-NAV-010: Mini-map with Clickable Regions

| Field | Value |
|-------|-------|
| **Completed** | 2025-12-18 |
| **System** | [Navigation](../systems/navigation-system.md) |

**Implementation**: Visual map with clickable region overlays.
- `MiniMap` component with map image overlay and grid fallback
- `MapRegionData`, `MapBounds` types for region positioning
- `get_regions()` on LocationService
- Click navigable region to move
- Legend showing current/available/locked regions

**Files**:
- `crates/player-ui/src/presentation/components/mini_map.rs`
- `crates/player-app/src/application/services/location_service.rs`
- `crates/player-ui/src/presentation/views/pc_view.rs`

---

### US-NAV-008: Navigation Options UI

| Field | Value |
|-------|-------|
| **Completed** | 2025-12-18 |
| **System** | [Navigation](../systems/navigation-system.md) |

**Implementation**: Full navigation UI for region movement and location exits.
- `NavigationPanel` modal component with region/exit buttons
- `NavigationButtons` compact inline variant
- `move_to_region()` and `exit_to_location()` on GameConnectionPort
- Region buttons show locked/unlocked state
- Map button in action panel opens navigation modal

**Files**:
- `crates/player-ui/src/presentation/components/navigation_panel.rs`
- `crates/player-ui/src/presentation/state/game_state.rs`
- `crates/player-ports/src/outbound/game_connection_port.rs`
- `crates/player-adapters/src/infrastructure/websocket/game_connection_adapter.rs`

---

### US-NAV-009: Game Time Display

| Field | Value |
|-------|-------|
| **Completed** | 2025-12-18 |
| **System** | [Navigation](../systems/navigation-system.md) |

**Implementation**: Game time display with time-of-day icons.
- `GameTimeDisplay` component shows current time with icons
- `GameTimeData` struct with display, time_of_day, is_paused
- Updates from `GameTimeUpdated` WebSocket message
- Shows pause indicator when time is stopped

**Files**:
- `crates/player-ui/src/presentation/components/navigation_panel.rs` (GameTimeDisplay)
- `crates/player-ui/src/presentation/state/game_state.rs` (GameTimeData)
- `crates/player-ui/src/presentation/handlers/session_message_handler.rs`

---

### US-NPC-008: Approach Event Display

| Field | Value |
|-------|-------|
| **Completed** | 2025-12-18 |
| **System** | [NPC](../systems/npc-system.md) |

**Implementation**: Visual overlay when NPC approaches player.
- `ApproachEventOverlay` modal component
- Shows NPC sprite (if available) with description
- "Continue" button to dismiss
- `ApproachEventData` in game state
- Triggered by `ApproachEvent` WebSocket message

**Files**:
- `crates/player-ui/src/presentation/components/event_overlays.rs`
- `crates/player-ui/src/presentation/state/game_state.rs`
- `crates/player-ui/src/presentation/handlers/session_message_handler.rs`
- `crates/player-ui/src/presentation/views/pc_view.rs`

---

### US-NPC-009: Location Event Display

| Field | Value |
|-------|-------|
| **Completed** | 2025-12-18 |
| **System** | [NPC](../systems/npc-system.md) |

**Implementation**: Banner notification for location-wide events.
- `LocationEventBanner` component at top of screen
- Click anywhere to dismiss
- `LocationEventData` in game state
- Triggered by `LocationEvent` WebSocket message

**Files**:
- `crates/player-ui/src/presentation/components/event_overlays.rs`
- `crates/player-ui/src/presentation/state/game_state.rs`
- `crates/player-ui/src/presentation/handlers/session_message_handler.rs`
- `crates/player-ui/src/presentation/views/pc_view.rs`

---

### US-CHAL-009: Skill Modifiers Display During Rolls

| Field | Value |
|-------|-------|
| **Completed** | 2025-12-18 (discovered already complete) |
| **System** | [Challenge](../systems/challenge-system.md) |

**Implementation**: Full skill modifier display in challenge rolls.
- `SkillsDisplay` component shows all skills with modifiers
- `ChallengeRollModal` shows modifier in header and result breakdown
- Roll display: dice + modifier + skill = total

**Files**:
- `crates/player-ui/src/presentation/components/tactical/challenge_roll.rs`
- `crates/player-ui/src/presentation/components/tactical/skills_display.rs`

---

### US-DLG-009: Context Budget Configuration

| Field | Value |
|-------|-------|
| **Completed** | 2025-12-18 (discovered already complete) |
| **System** | [Dialogue](../systems/dialogue-system.md) |

**Implementation**: Full context budget configuration via Settings API.
- `GET/PUT /api/settings` exposes all 10 `ContextBudgetConfig` fields
- Per-world settings at `/api/worlds/{world_id}/settings`
- Metadata endpoint for UI field rendering

**Files**:
- `crates/domain/src/value_objects/context_budget.rs`
- `crates/engine-adapters/src/infrastructure/http/settings_routes.rs`

---

## Progress Log

| Date | Phase | Story | Change |
|------|-------|-------|--------|
| 2025-12-18 | A | US-NAV-008 | Implemented navigation panel with region/exit buttons |
| 2025-12-18 | A | US-NAV-009 | Implemented game time display with time-of-day icons |
| 2025-12-18 | A | US-NPC-008 | Implemented approach event overlay for NPC approaches |
| 2025-12-18 | A | US-NPC-009 | Implemented location event banner for location events |
| 2025-12-18 | A | - | **Phase A Complete** |
| 2025-12-18 | - | US-CHAL-009 | Marked complete (already implemented) |
| 2025-12-18 | - | US-DLG-009 | Marked complete (already implemented) |
| 2025-12-18 | - | - | Created ACTIVE_DEVELOPMENT.md |
| 2025-12-18 | B | US-CHAR-009 | Implemented inventory panel with item categories |
| 2025-12-18 | B | US-OBS-004/005 | Implemented known NPCs panel with observations |
| 2025-12-18 | B | US-NAV-010 | Implemented mini-map with clickable regions |
| 2025-12-18 | B | - | **Phase B Complete** |
