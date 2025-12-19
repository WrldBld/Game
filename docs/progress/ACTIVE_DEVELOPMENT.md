# Active Development

Active implementation tracking for WrldBldr user stories.

**Current Phase**: Phase B - Player Knowledge & Agency (COMPLETE)  
**Last Updated**: 2025-12-18

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
- `Engine/src/application/dto/item.rs`
- `Player/src/presentation/components/inventory_panel.rs`
- `Player/src/application/services/character_service.rs`
- `Player/src/application/dto/world_snapshot.rs`

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
- `Player/src/presentation/components/known_npcs_panel.rs`
- `Player/src/application/services/observation_service.rs`
- `Player/src/presentation/views/pc_view.rs`

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
- `Player/src/presentation/components/mini_map.rs`
- `Player/src/application/services/location_service.rs`
- `Player/src/presentation/views/pc_view.rs`

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
- `Player/src/presentation/components/navigation_panel.rs`
- `Player/src/presentation/state/game_state.rs`
- `Player/src/application/ports/outbound/game_connection_port.rs`
- `Player/src/infrastructure/websocket/game_connection_adapter.rs`

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
- `Player/src/presentation/components/navigation_panel.rs` (GameTimeDisplay)
- `Player/src/presentation/state/game_state.rs` (GameTimeData)
- `Player/src/presentation/handlers/session_message_handler.rs`

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
- `Player/src/presentation/components/event_overlays.rs`
- `Player/src/presentation/state/game_state.rs`
- `Player/src/presentation/handlers/session_message_handler.rs`
- `Player/src/presentation/views/pc_view.rs`

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
- `Player/src/presentation/components/event_overlays.rs`
- `Player/src/presentation/state/game_state.rs`
- `Player/src/presentation/handlers/session_message_handler.rs`
- `Player/src/presentation/views/pc_view.rs`

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
- `Player/src/presentation/components/tactical/challenge_roll.rs`
- `Player/src/presentation/components/tactical/skills_display.rs`

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
- `Engine/src/domain/value_objects/context_budget.rs`
- `Engine/src/infrastructure/http/settings_routes.rs`

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
