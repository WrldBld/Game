# Active Development

Active implementation tracking for WrldBldr user stories.

**Current Phase**: Sprint 5 - Actantial Completion + Cleanup (COMPLETE)  
**Last Updated**: 2025-12-25

---

## Phase Overview

| Phase | Focus | Status | Est. Effort |
|-------|-------|--------|-------------|
| A | Core Player Experience | **COMPLETE** | 3-4 days |
| B | Player Knowledge & Agency | **COMPLETE** | 4-5 days |
| P | Feature Parity Gap Removal | **COMPLETE** | 5 days |
| C | DM Tools & Advanced Features | **IN PROGRESS** | 5-7 days |

---

## Phase A: Core Player Experience - COMPLETE

All Phase A stories have been implemented. See Completed section for details.

---

## Phase B: Player Knowledge & Agency - COMPLETE

All Phase B stories have been implemented. See Completed section for details.

---

## Phase P: Feature Parity Gap Removal - COMPLETE

Restore functionality and UX that exists in docs/UI but isn't fully wired. See [FEATURE_PARITY_GAP_REMOVAL.md](./FEATURE_PARITY_GAP_REMOVAL.md) for detailed analysis.

### GAP-DM-CHAL-001: Ad-hoc Challenge Creation

| Field | Value |
|-------|-------|
| **Status** | **COMPLETE** |
| **Priority** | High |
| **Effort** | 0.5 days |
| **Completed** | 2025-12-24 |

**Goal**: Wire the existing ad-hoc challenge modal to the backend.

**Implementation checklist**:
- [x] Player Ports: Add `create_adhoc_challenge()` to `GameConnectionPort` trait
- [x] Player Adapters: Implement in websocket client (message already exists in protocol)
- [x] Player UI: Wire `dm_view.rs` callback to call the new port method
- [x] Validate: `cargo check --workspace` and `cargo xtask arch-check`

---

### GAP-UI-NAV-001: Mini-map Background Image

| Field | Value |
|-------|-------|
| **Status** | **COMPLETE** |
| **Priority** | High |
| **Effort** | 0.5 days |
| **Completed** | 2025-12-24 |

**Goal**: Display location map image in the mini-map component.

**Implementation checklist**:
- [x] Protocol: Add `map_asset: Option<String>` to `RegionData`
- [x] Engine: Include location `map_asset` in `SceneChanged` message (all 5 RegionData constructions)
- [x] Player UI: Pass `map_asset` to `MiniMap` component from game state
- [x] Validate: `cargo check --workspace` and `cargo xtask arch-check`

---

### GAP-UI-INV-001/002: Inventory Equip/Drop

| Field | Value |
|-------|-------|
| **Status** | **COMPLETE** |
| **Priority** | High |
| **Effort** | 1.5 days |
| **Completed** | 2025-12-24 |

**Goal**: Wire inventory equip/unequip and drop actions end-to-end.

**Implementation checklist**:
- [x] Protocol: Add `EquipItem`, `UnequipItem`, `DropItem` client messages
- [x] Protocol: Add `ItemEquipped`, `ItemUnequipped`, `ItemDropped`, `InventoryUpdated` server messages
- [x] Engine Ports: Add `get_inventory_item()` to `CharacterRepositoryPort`
- [x] Engine Adapters: Implement in `Neo4jCharacterRepository`
- [x] Engine WebSocket: Add handlers for new client messages
- [x] Player Ports: Add `equip_item()`, `unequip_item()`, `drop_item()` to `GameConnectionPort`
- [x] Player Adapters: Implement in websocket client
- [x] Player UI: Wire `on_toggle_equip` and `on_drop_item` callbacks in `pc_view.rs`
- [x] Player UI: Add `trigger_inventory_refresh()` to GameState and handle server messages
- [x] Validate: `cargo check --workspace` and `cargo xtask arch-check`

**Note**: Drop currently destroys items. Future work will place items in regions when that system exists.

---

### GAP-ENG-AST-001: Asset Generation Queue Persistence

| Field | Value |
|-------|-------|
| **Status** | **COMPLETE** |
| **Priority** | Medium |
| **Effort** | 1 day |
| **Completed** | 2025-12-24 |

**Goal**: Generated images should persist as Asset records.

**Implementation checklist**:
- [x] Implement ComfyUI `/history/{prompt_id}` polling until complete (with 5-minute timeout)
- [x] Download generated images from ComfyUI output
- [x] Save images to `data/generated_assets/` directory
- [x] Create Asset records via `AssetRepositoryPort`
- [x] Associate assets with target entity (character, location, etc.)
- [x] Handle errors properly (fail queue item with message)
- [x] Validate: `cargo check --workspace` and `cargo xtask arch-check`

---

### GAP-ENG-CHAL-002: Challenge Outcome Triggers

| Field | Value |
|-------|-------|
| **Status** | **COMPLETE** |
| **Priority** | Medium |
| **Effort** | 0.5 days |
| **Completed** | 2025-12-24 |

**Goal**: Execute outcome triggers when challenge resolves.

**Implementation checklist**:
- [x] Add `original_triggers: Vec<OutcomeTriggerRequestDto>` to `ChallengeOutcomeApprovalItem` to preserve trigger data
- [x] Convert DTOs to domain `OutcomeTrigger` on approval
- [x] Execute triggers via existing `OutcomeTriggerService`
- [x] Validate: `cargo check --workspace` and `cargo xtask arch-check`

---

### GAP-ENG-SCN-001: Scene Broadcast on Character Endpoints

| Field | Value |
|-------|-------|
| **Status** | **COMPLETE** |
| **Priority** | Medium |
| **Effort** | 0.5 days |
| **Completed** | 2025-12-24 |

**Goal**: Broadcast scene updates after PC scene resolution.

**Implementation checklist**:
- [x] After resolving scene for PC in `create_player_character`, construct `ServerMessage::SceneUpdate`
- [x] After resolving scene for PC in `update_player_character_location`, construct `ServerMessage::SceneUpdate`
- [x] Send to player via `AsyncSessionPort.send_to_participant()`
- [x] Validate: `cargo check --workspace` and `cargo xtask arch-check`

---

### GAP-UI-STATE-001: State Updates in Message Handler

| Field | Value |
|-------|-------|
| **Status** | **COMPLETE** |
| **Priority** | Low |
| **Effort** | 0.5 days |
| **Completed** | 2025-12-24 |

**Goal**: Update UI state after server messages.

**Implementation checklist**:
- [x] After `NpcLocationShared`: Add `observations_refresh_counter` to GameState and trigger refresh
- [x] After `PcSelected`: Update `selected_pc_id` signal in game state
- [x] Validate: `cargo check --workspace` and `cargo xtask arch-check`

---

## Phase C: DM Tools & Advanced Features

Improve DM workflow. These don't block player gameplay.

### US-STG-013 / US-OBS-006: Hidden NPCs + Unrevealed Interactions

| Field | Value |
|-------|-------|
| **Status** | Done |
| **Priority** | High |
| **Effort** | 2-3 days |
| **Systems** | [Staging](../systems/staging-system.md), [Observation](../systems/observation-system.md) |

**Goal**: Support NPCs that are staged as present-but-hidden from players, while still allowing DM-triggered approach events that may or may not reveal identity.

**Player-facing behavior**:
- Hidden NPCs do not appear in `SceneChanged.npcs_present` or `StagingReady.npcs_present`.
- Unrevealed approaches display as **"Unknown Figure"** with no sprite/portrait.
- Unrevealed interactions are recorded as observations and shown as **"Unknown Figure"** in Known NPCs.

**Implementation checklist**:
- [x] Protocol: add `reveal` to approach events
- [x] Protocol: add `is_hidden_from_players` to staged/approved NPCs
- [x] Engine: persist hidden flag in staging (`INCLUDES_NPC`)
- [x] Engine: filter hidden NPCs from player presence messages
- [x] Engine: add `is_revealed_to_player` to observations + persistence
- [x] Engine: scrub observation API for unrevealed entries
- [x] Engine: approach event handler supports `reveal=false`
- [x] Engine: approach event targets specific PC (not broadcast to all)
- [x] Player UI: staging approval + pre-stage support hidden toggle
- [x] Player UI: observations refresh via shared state; show Unknown Figure
- [x] Security: removed `POST /api/regions/{region_id}/scene` endpoint (had auto-observation bug)
- [x] Maintenance: removed unused `engine-app/src/domain/` folder (orphaned code)
- [x] Validate: `cargo check --workspace` and `cargo xtask arch-check`

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
| **Status** | Done |
| **Priority** | Medium |
| **Effort** | 0.5 days |
| **Completed** | 2025-12-24 |
| **System** | [Challenge](../systems/challenge-system.md) |

**Description**: Bind challenges to specific regions, not just locations.

**Implementation checklist**:
- [x] Add `ChallengeRegionAvailability` entity to domain
- [x] Add `AVAILABLE_AT_REGION` edge to Neo4j schema docs
- [x] Add `list_by_region()`, `add_region_availability()`, `get_region_availabilities()`, `remove_region_availability()` to `ChallengeRepositoryPort`
- [x] Implement in `Neo4jChallengeRepository`
- [x] Validate: `cargo check --workspace` and `cargo xtask arch-check`

**Key files**:
- `crates/domain/src/entities/challenge.rs` (ChallengeRegionAvailability)
- `crates/engine-ports/src/outbound/repository_port.rs` (4 new methods)
- `crates/engine-adapters/src/infrastructure/persistence/challenge_repository.rs`
- `docs/architecture/neo4j-schema.md`

---

### US-SCN-009: Scene Entry Conditions

| Field | Value |
|-------|-------|
| **Status** | Done |
| **Priority** | Medium |
| **Effort** | 1 day |
| **Completed** | 2025-12-24 |
| **System** | [Scene](../systems/scene-system.md) |

**Description**: Evaluate conditions before showing a scene.

**Implementation checklist**:
- [x] Add `GameFlag` entity with `FlagScope` enum (World/PC)
- [x] Add `FlagRepositoryPort` trait with world and PC-scoped flag methods
- [x] Implement `Neo4jFlagRepository`
- [x] Add scene completion tracking to `SceneRepositoryPort` (mark_scene_completed, is_scene_completed, get_completed_scenes)
- [x] Implement scene completion in `Neo4jSceneRepository` via `COMPLETED_SCENE` edge
- [x] Add `evaluate_conditions()` to `SceneResolutionServiceImpl`
- [x] Filter scenes by entry conditions in `resolve_scene_for_pc()`
- [x] Validate: `cargo check --workspace` and `cargo xtask arch-check`

**Known limitation**: `HasItem` condition logs a warning and treats as met due to PC inventory system gap (see US-INV-001).

**Key files**:
- `crates/domain/src/entities/game_flag.rs` (new)
- `crates/engine-ports/src/outbound/repository_port.rs` (FlagRepositoryPort, scene completion)
- `crates/engine-adapters/src/infrastructure/persistence/flag_repository.rs` (new)
- `crates/engine-adapters/src/infrastructure/persistence/scene_repository.rs`
- `crates/engine-app/src/application/services/scene_resolution_service.rs`

---

### US-INV-001: Fix Player Character Inventory System

| Field | Value |
|-------|-------|
| **Status** | **COMPLETE** |
| **Priority** | High |
| **Effort** | 3-4 days |
| **Completed** | 2025-12-24 |
| **System** | [Character](../systems/character-system.md) |

**Description**: PC inventory operations were broken. WebSocket handlers used `CharacterId` type and queried `:Character` nodes, but PCs are `:PlayerCharacter` nodes.

**Implementation checklist**:
- [x] Phase 1: Created `Neo4jItemRepository` with container support
- [x] Phase 2: Added PC inventory methods to `PlayerCharacterRepositoryPort` (add_inventory_item, get_inventory, get_inventory_item, update_inventory_item, remove_inventory_item)
- [x] Phase 2: Implemented in `Neo4jPlayerCharacterRepository` using `POSSESSES` edge
- [x] Phase 3: Fixed WebSocket handlers (`EquipItem`, `UnequipItem`, `DropItem`) to use `PlayerCharacterId`
- [x] Phase 4: Implemented container system with `CONTAINS` edge between Items
- [x] Phase 5: Created `ItemService` trait and `ItemServiceImpl` for item operations
- [x] Phase 5: Updated `DMApprovalQueueService` with DM recipient selection for give_item tools
- [x] Phase 5: Updated `ApprovalDecision` with `AcceptWithRecipients` and `item_recipients` field
- [x] Phase 6: Wired `HasItem` scene condition to check PC inventory
- [x] Phase 7: Added stub methods to `RegionRepositoryPort` for future region item placement
- [x] Phase 7: Created US-REGION-ITEMS user story for future region item system
- [x] Validate: `cargo check --workspace` and `cargo xtask arch-check`

**Key files**:
- `crates/engine-adapters/src/infrastructure/persistence/item_repository.rs` (new)
- `crates/engine-adapters/src/infrastructure/persistence/player_character_repository.rs`
- `crates/engine-adapters/src/infrastructure/websocket.rs`
- `crates/engine-app/src/application/services/item_service.rs` (new)
- `crates/engine-app/src/application/services/dm_approval_queue_service.rs`
- `crates/engine-app/src/application/services/scene_resolution_service.rs`
- `crates/protocol/src/types.rs` (ApprovalDecision)
- `docs/progress/US-INV-001-PLAN.md` (implementation plan)
- `docs/progress/US-REGION-ITEMS.md` (future region items story)

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
| 2025-12-20 | - | Prompt Templates | Configurable LLM prompts system complete |
| 2025-12-24 | - | Doc Alignment | System docs aligned with ACTIVE_DEVELOPMENT |
| 2025-12-24 | P | - | **Phase P Started** - Feature Parity Gap Removal |
| 2025-12-24 | P | GAP-DM-CHAL-001 | Wired ad-hoc challenge creation modal to backend |
| 2025-12-24 | P | GAP-UI-NAV-001 | Wired mini-map background image from location data |
| 2025-12-24 | P | GAP-UI-INV-001/002 | Implemented inventory equip/drop end-to-end |
| 2025-12-24 | P | GAP-ENG-AST-001 | Implemented ComfyUI polling and asset persistence |
| 2025-12-24 | P | GAP-ENG-CHAL-002 | Wired challenge outcome triggers to OutcomeTriggerService |
| 2025-12-24 | P | GAP-ENG-SCN-001 | Scene broadcast after PC creation and location update |
| 2025-12-24 | P | GAP-UI-STATE-001 | State updates for NpcLocationShared and PcSelected |
| 2025-12-24 | P | - | **Phase P Complete** |
| 2025-12-24 | C | US-CHAL-010 | Region-level challenge binding with 4 repository methods |
| 2025-12-24 | C | US-SCN-009 | Scene entry conditions with flag repository and completion tracking |
| 2025-12-24 | C | US-INV-001 | **COMPLETE** - Full PC inventory system with ItemService, DM approval, containers |
| 2025-12-25 | C | P1.5 | Actantial Model System - MotivationsTab with wants, goals, actantial views UI |
| 2025-12-25 | - | Sprint 4 | **Sprint 4: UX Polish COMPLETE** - Split Party Warning, Location Preview, View-as-Character, Style Reference, Visual Timeline |
| 2025-12-25 | - | Sprint 5 | Dead code removal (LLMContextService, ~850 lines), WebSocket state updates for actantial, CharacterPicker component |
| 2025-12-25 | - | Sprint 5 | **Sprint 5: Actantial Completion COMPLETE** - Dead code cleanup, CharacterPicker, WebSocket architecture doc |
