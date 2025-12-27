# WrldBldr Implementation Backlog

**Created**: 2025-12-25  
**Last Updated**: 2025-12-25 (Sprint 5 Complete)

This document tracks all unimplemented features in priority order with implementation details.

---

## Development Diary

### 2025-12-25 Session 2: Sprint 1 Implementation

**Starting Sprint 1**: Critical Gaps + Quick Wins

| Item | Status | Notes |
|------|--------|-------|
| P0.1: Challenge Reasoning Approval | DEAD CODE | `LLMRequestType::ChallengeReasoning` is never instantiated - skip |
| P0.2: Observation Check in Scene Resolution | DONE | Added `ObservationRepositoryPort`, wired into scene resolution |
| P2.6: Calculate Relative Time for NPCs | DONE | Added `format_observation_time()` helper, formatted display |
| P2.7: Manual ComfyUI Health Check | DONE | Full E2E implementation |
| P1.2: Hidden NPCs Extension | DONE | Already implemented - verified all components |

**P0.1 Finding**: Dead code - `LLMRequestType::ChallengeReasoning` never instantiated. Challenge flow uses `ChallengeOutcomeApprovalService`.

**P0.2 Completed**:
- Created `ObservationRepositoryPort` trait in `engine-ports/outbound/repository_port.rs`
- Implemented trait for `Neo4jObservationRepository`
- Added `observation_repository` to `SceneResolutionServiceImpl`
- Fixed `KnowsCharacter` condition to use proper observation check
- Files: `repository_port.rs`, `observation_repository.rs`, `scene_resolution_service.rs`, `state/mod.rs`

**P2.6 Completed**:
- Added `format_observation_time()` helper to parse RFC3339 and format as "Dec 25, 2:30 PM"
- Replaced "..." placeholder with actual formatted time
- Added tooltip with full timestamp
- File: `known_npcs_panel.rs`

**P2.7 Completed**:
- Added `ClientMessage::CheckComfyUIHealth` to protocol
- Added `health_check()` to `ComfyUIPort` trait and implemented in `ComfyUIClient`
- Added WebSocket handler for `CheckComfyUIHealth` that triggers health check and broadcasts state
- Added `check_comfyui_health()` to `GameConnectionPort` (both WASM and desktop variants)
- Implemented in `WebSocketGameConnectionAdapter` and `MockGameConnectionPort`
- Wired "Retry Now" button in `ComfyUIBanner` to call `check_comfyui_health()`
- Files: `messages.rs`, `comfyui_port.rs`, `comfyui.rs`, `websocket.rs`, `game_connection_port.rs`, `game_connection_adapter.rs`, `mock_game_connection_port.rs`, `comfyui_banner.rs`

**P1.2 Verified (Already Implemented)**:
- `is_hidden_from_players: bool` already exists on `StagedNpc` in domain
- Protocol types (`StagedNpcInfo`, `NpcPresentInfo`, `ApprovedNpcInfo`) all have the field
- `Staging.present_visible_npcs()` method filters hidden NPCs
- Staging repository persists `is_hidden_from_players` in Neo4j `INCLUDES_NPC` relationship
- WebSocket handlers filter hidden NPCs from `SceneChanged.npcs_present` (lines 1773, 2206, 2613)
- Staging approval UI has full "Hidden" toggle support in `NpcSelectionRow` component
- Files verified: `staging.rs`, `messages.rs`, `staging_repository.rs`, `websocket.rs`, `staging_approval.rs`, `game_state.rs`, `session_message_handler.rs`

---

### 2025-12-25 Session 3: Sprint 2 Implementation

**Starting Sprint 2**: Feature Complete - Items & Challenge Triggers

| Item | Status | Notes |
|------|--------|-------|
| P1.1: US-REGION-ITEMS Phases 1-4 | ✅ COMPLETED | Drop + Pickup implementation |
| P1.3: Challenge Outcome Trigger Execution | ✅ COMPLETED | Session 3 |

**P1.1 Completed (Phases 1-4)**:
- **Phase 1**: Implemented `RegionRepositoryPort` item methods in Neo4j:
  - `add_item_to_region()` - Creates `(Region)-[:CONTAINS_ITEM]->(Item)` edge with placement metadata
  - `get_region_items()` - Returns items in region
  - `remove_item_from_region()` - Deletes the relationship edge
  - Added `row_to_item()` helper for Neo4j result conversion
- **Phase 2**: Skipped separate service - functionality integrated directly in existing systems
- **Phase 3**: Updated `DropItem` WebSocket handler:
  - Gets PC's current region from `pc.current_region_id`
  - Places item in region via `add_item_to_region()` before removing from inventory
  - Adds rollback on failure to maintain data consistency
  - Includes proper error handling for PCs not in regions
- **Phase 4**: Implemented `PickupItem` functionality (Session 3):
  - Added `ClientMessage::PickupItem` and `ServerMessage::ItemPickedUp` to protocol
  - Added `pickup_item()` method to both desktop and WASM `GameConnectionPort` traits
  - Implemented `pickup_item()` in `WebSocketGameConnectionAdapter` and `MockGameConnectionPort`
  - Added comprehensive `PickupItem` WebSocket handler with full validation:
    - Input validation (empty strings, invalid UUIDs)
    - Region validation (PC in region, item in region)
    - Duplicate item prevention (PC can't already own the item)
    - Atomic operations with rollback (remove from region, add to inventory)
    - Enhanced logging and error handling
  - Updated player UI `session_message_handler.rs` to handle `ItemPickedUp` messages
  - Full integration: conversation log entries and inventory refresh
- Files: `messages.rs`, `game_connection_port.rs`, `game_connection_adapter.rs`, `mock_game_connection_port.rs`, `websocket.rs`, `session_message_handler.rs`

---

## Priority Tiers

| Tier | Focus | Criteria |
|------|-------|----------|
| **P0** | Critical Gaps | Blocking core functionality, broken features |
| **P1** | Feature Complete | Missing pieces of existing systems |
| **P2** | UX Polish | Quality of life, workflow improvements |
| **P3** | Future Features | New capabilities, major systems |

---

## P0: Critical Gaps

### P0.1: Challenge Reasoning Approval
| Field | Value |
|-------|-------|
| **Status** | Not Implemented |
| **Effort** | 4 hours |
| **Location** | `crates/engine-app/src/application/services/llm_queue_service.rs:495-500` |

**Problem**: Challenge reasoning requests log info but don't execute any approval flow.

**Implementation**:
1. Define `ChallengeReasoningApproval` DTO with challenge context
2. Add to `ApprovalQueuePort` processing in `llm_queue_service.rs`
3. Send to DM for approval via existing `dm_approval_queue_service`
4. On approval, execute challenge with reasoning context

**Files to Modify**:
- `crates/engine-app/src/application/dto/queue_items.rs` (add DTO)
- `crates/engine-app/src/application/services/llm_queue_service.rs` (implement handler)
- `crates/engine-app/src/application/services/dm_approval_queue_service.rs` (add decision type)

---

### P0.2: Observation Check in Scene Resolution
| Field | Value |
|-------|-------|
| **Status** | Incomplete |
| **Effort** | 2 hours |
| **Location** | `crates/engine-app/src/application/services/scene_resolution_service.rs:102` |

**Problem**: Scene condition `HasObservedNpc` doesn't properly check observation records.

**Implementation**:
1. Inject `ObservationRepositoryPort` into `SceneResolutionServiceImpl`
2. Implement `check_observed_npc()` using repository
3. Replace placeholder with actual query

**Files to Modify**:
- `crates/engine-app/src/application/services/scene_resolution_service.rs`
- `crates/engine-adapters/src/infrastructure/state/mod.rs` (wire dependency)

---

## P1: Feature Complete

### P1.1: US-REGION-ITEMS - Region Item Placement
| Field | Value |
|-------|-------|
| **Status** | Stubs Only |
| **Effort** | 2-3 days |
| **Plan File** | (completed and archived) |

**Problem**: Dropped items are destroyed; no items can exist in world locations.

**Phases**:

#### Phase 1: Repository Implementation (4 hours)
```rust
// Already stubbed in RegionRepositoryPort:
async fn add_item_to_region(&self, region_id: RegionId, item_id: ItemId) -> Result<()>;
async fn get_region_items(&self, region_id: RegionId) -> Result<Vec<Item>>;
async fn remove_item_from_region(&self, region_id: RegionId, item_id: ItemId) -> Result<()>;
```

Neo4j queries:
```cypher
// add_item_to_region
MATCH (r:Region {id: $region_id}), (i:Item {id: $item_id})
CREATE (r)-[:CONTAINS_ITEM {placed_at: datetime(), placed_by: $placed_by, visibility: 'visible'}]->(i)

// get_region_items
MATCH (r:Region {id: $region_id})-[c:CONTAINS_ITEM]->(i:Item)
RETURN i, c.visibility, c.placed_at

// remove_item_from_region
MATCH (r:Region {id: $region_id})-[c:CONTAINS_ITEM]->(i:Item {id: $item_id})
DELETE c
```

**Files**:
- `crates/engine-adapters/src/infrastructure/persistence/region_repository.rs`

#### Phase 2: Service Layer (2 hours)
Create `RegionItemService`:
```rust
pub trait RegionItemService: Send + Sync {
    async fn place_item(&self, region_id: RegionId, item_id: ItemId, placed_by: &str) -> Result<()>;
    async fn pickup_item(&self, region_id: RegionId, item_id: ItemId, pc_id: PlayerCharacterId) -> Result<()>;
    async fn drop_item(&self, pc_id: PlayerCharacterId, item_id: ItemId) -> Result<()>;
    async fn list_visible_items(&self, region_id: RegionId) -> Result<Vec<Item>>;
}
```

**Files**:
- NEW: `crates/engine-app/src/application/services/region_item_service.rs`
- MOD: `crates/engine-app/src/application/services/mod.rs`

#### Phase 3: Drop Item Integration (2 hours)
Update `DropItem` WebSocket handler:
```rust
// Instead of destroying item:
// 1. Get PC's current region
// 2. Check region capacity (max_items)
// 3. If capacity ok, call region_item_service.drop_item()
// 4. If full, return error message
```

**Files**:
- `crates/engine-adapters/src/infrastructure/websocket.rs` (lines ~3050-3100)

#### Phase 4: Pickup Integration (4 hours)
Add new WebSocket message:
```rust
ClientMessage::PickupItem { item_id: String }
```

Handler:
1. Validate item is in PC's current region
2. Check if contested (multiple PCs reaching)
3. Simple case: add to PC inventory, remove from region
4. Contested: queue for DM decision

**Files**:
- `crates/protocol/src/messages.rs`
- `crates/engine-adapters/src/infrastructure/websocket.rs`
- `crates/player-ports/src/outbound/game_connection_port.rs`
- `crates/player-adapters/src/infrastructure/websocket/game_connection_adapter.rs`

#### Phase 5: LLM Context (2 hours)
Include region items in NPC response context:
```rust
// In prompt_builder.rs scene context:
region_items: Vec<ItemSummary> // name, description, type
```

**Files**:
- `crates/engine-app/src/application/services/llm/prompt_builder.rs`

#### Phase 6: UI Updates (4 hours)
- Show region items in location panel
- Add "Pick up" button for each item
- Update inventory on pickup

**Files**:
- `crates/player-ui/src/presentation/views/pc_view.rs`
- NEW: `crates/player-ui/src/presentation/components/region_items_panel.rs`

---

### P1.2: Hidden NPCs Extension (STAGING Part H)
| Field | Value |
|-------|-------|
| **Status** | ✅ COMPLETED (Already Implemented) |
| **Effort** | 0 hours (was already done) |
| **Plan File** | (completed and archived) |

**Problem**: NPCs cannot be staged as "present but hidden" for surprise reveals.

**Implementation** (Verified Complete):
1. ✅ `is_hidden_from_players: bool` on `StagedNpc` struct
2. ✅ Filter hidden NPCs from `SceneChanged.npcs_present` (3 locations in websocket.rs)
3. ✅ `reveal: bool` on `TriggerApproachEvent` and `ApproachEvent` messages
4. ✅ Staging approval UI has "Hidden" toggle checkbox

**Files** (Already Complete):
- `crates/domain/src/entities/staging.rs` - Field + `present_visible_npcs()` method
- `crates/protocol/src/messages.rs` - All staging types have field, approach events have reveal
- `crates/engine-adapters/src/infrastructure/persistence/staging_repository.rs` - Persists to Neo4j
- `crates/engine-adapters/src/infrastructure/websocket.rs` - Filters at lines 1773, 2206, 2613
- `crates/player-ui/src/presentation/components/dm_panel/staging_approval.rs` - Full UI

---

### P1.3: Challenge Outcome Trigger Execution ✅ COMPLETED
| Field | Value |
|-------|-------|
| **Status** | ✅ **COMPLETED** |
| **Effort** | 3 hours |
| **Location** | `crates/engine-app/src/application/services/challenge_outcome_approval_service.rs` |

**Problem**: Challenge outcome triggers (GiveItem, RevealInfo, etc.) were approved but not executed.

**Root Cause**: StateChange objects from trigger execution were not processed to apply actual game state changes.

**Solution Implemented**:
1. **Trigger Execution**: ✅ Already working - `OutcomeTriggerService.execute_triggers()` correctly converts DTOs and runs triggers
2. **State Change Processing**: ✅ Added `process_state_changes()` method to handle `StateChange::ItemAdded` events
3. **Item Creation & Inventory**: ✅ GiveItem triggers now create actual items and add them to PC inventories
4. **Repository Integration**: ✅ Added ItemRepository and PlayerCharacterRepository dependencies

**Key Changes**:
- **File**: `challenge_outcome_approval_service.rs` - Added state change processor after trigger execution
- **Method**: `process_state_changes()` - Creates items and adds to inventory for ItemAdded state changes  
- **Integration**: Service constructor updated with PC and Item repositories
- **State Module**: Service instantiation updated with new repository dependencies

**Behavior**: GiveItem triggers now create persistent items in the world and add them to the rolling PC's inventory.

---

### P1.4: Character Mood and Relationship Tracking
| Field | Value |
|-------|-------|
| **Status** | Stub |
| **Effort** | 4 hours |
| **Location** | `crates/engine-adapters/src/infrastructure/websocket_helpers.rs:135-137` |

**Problem**: `current_mood` and `relationship_to_player` are always `None`.

**Implementation**:
1. Add `mood: Option<String>` to `Character` entity
2. Create `MOOD_TOWARD` edge: `(NPC)-[:MOOD_TOWARD {mood: "friendly", updated_at: datetime()}]->(PC)`
3. Query mood when building NPC data for scene
4. Update mood based on dialogue/interaction outcomes

**Files**:
- `crates/domain/src/entities/character.rs`
- `crates/engine-ports/src/outbound/repository_port.rs` (add mood methods)
- `crates/engine-adapters/src/infrastructure/persistence/character_repository.rs`
- `crates/engine-adapters/src/infrastructure/websocket_helpers.rs`

---

### P1.5: Actantial Model System (Expanded from GoalRepositoryPort)
| Field | Value |
|-------|-------|
| **Status** | Planning Complete |
| **Effort** | 15-17 hours (5 phases across 3 sessions) |
| **Plan File** | (completed and archived) |

**Problem**: The Actantial Model (Wants, Goals, Helper/Opponent views) is stored in Neo4j but completely invisible to the LLM and DM. NPCs have rich motivational data that isn't being used.

**Scope Expansion**: Originally just GoalRepository, now a full Actantial Context System:
- Goals: Abstract desire targets (Power, Revenge, Redemption)
- Wants with Targets: NPCs desire Characters, Items, or Goals
- Actantial Views: NPCs view others as Helpers/Opponents/Senders/Receivers
- Secret Motivations: Hidden wants with behavioral guidance and "tells"
- NPC → PC Views: NPCs can view player characters as allies/enemies
- LLM Integration: Full motivational context in every NPC response
- DM Tools: Visual panels for viewing and editing NPC motivations

**Implementation Phases**:
1. **Phase 1**: Goal Repository (2h) - Foundation
2. **Phase 2**: Actantial Context Service (4-5h) - Aggregation layer
3. **Phase 3**: LLM Context Integration (3h) - Prompt enrichment
4. **Phase 4**: DM Panel Integration (4h) - UI for viewing/editing
5. **Phase 5**: Goal Management UI (2-3h) - Creator Mode

**Session Breakdown**:
- Session A: Phase 1 + 2 (6-7h) - Backend infrastructure
- Session B: Phase 3 + 5 (5-6h) - LLM integration + Goal UI
- Session C: Phase 4 (4h) - DM tools

**Plan Status**: Completed and archived

---

## P2: UX Polish

### P2.1: View-as-Character Mode
| Field | Value |
|-------|-------|
| **Status** | ✅ COMPLETED (Sprint 4) |
| **Effort** | 4 hours |
| **Completed** | 2025-12-25 |

**Implementation** (Complete):
- Added `ViewMode` enum (Director | ViewingAsCharacter) to GameState
- Added helper methods: `start_viewing_as()`, `stop_viewing_as()`, `is_viewing_as_character()`
- Created `ViewAsCharacterMode` component with read-only perspective
- Blue banner with "Exit View" button, shows NPCs/items visible to character

---

### P2.2: Location Preview Modal
| Field | Value |
|-------|-------|
| **Status** | ✅ COMPLETED (Sprint 4) |
| **Effort** | 2 hours |
| **Completed** | 2025-12-25 |

**Implementation** (Complete):
- Created `LocationPreviewModal` component
- Shows location details, regions, connections, hidden secrets
- Wired `on_preview` callback in LocationNavigator

---

### P2.3: Story Arc Visual Timeline
| Field | Value |
|-------|-------|
| **Status** | ✅ COMPLETED (Sprint 4) |
| **Effort** | 6 hours |
| **Completed** | 2025-12-25 |

**Implementation** (Complete):
- Created horizontal zoomable/pannable timeline as new "Visual" sub-tab
- Clustering: events within 30px grouped, stacking up to 3 before "+N more"
- Zoom controls (0.25x to 4.0x) with pan buttons
- Date markers, hover tooltips, click to open detail modal
- Filtered-out events shown greyed

---

### P2.4: Split Party Warning UX
| Field | Value |
|-------|-------|
| **Status** | ✅ COMPLETED (Sprint 4) |
| **Effort** | 1 hour |
| **Completed** | 2025-12-25 |

**Implementation** (Complete):
- Added `split_party_locations` signal to GameState
- Created `SplitPartyBanner` component with collapsible location list
- Wired to Director view (top of left panel)

---

### P2.5: Style Reference for Asset Generation
| Field | Value |
|-------|-------|
| **Status** | ✅ COMPLETED (Sprint 4) |
| **Effort** | 3 hours |
| **Completed** | 2025-12-25 |

**Implementation** (Complete):
- Added `style_reference_asset_id` to world settings (backend + frontend)
- "Use as Reference" button saves to world settings
- Purple border + star indicator for current reference
- Pre-populates generation modals with world default

---

### P2.6: Calculate Relative Time for NPCs
| Field | Value |
|-------|-------|
| **Status** | TODO in code |
| **Effort** | 1 hour |
| **Location** | `crates/player-ui/src/presentation/components/known_npcs_panel.rs:317` |

**Implementation**:
1. Parse game time string
2. Calculate difference from current game time
3. Display as "2 hours ago", "yesterday", etc.

**Files**:
- `crates/player-ui/src/presentation/components/known_npcs_panel.rs`

---

### P2.7: Manual ComfyUI Health Check
| Field | Value |
|-------|-------|
| **Status** | ✅ COMPLETED |
| **Effort** | 1 hour |
| **Location** | `crates/player-ui/src/presentation/components/creator/comfyui_banner.rs:53` |

**Implementation**:
1. ✅ Add WebSocket message `ClientMessage::CheckComfyUIHealth`
2. ✅ Engine re-checks ComfyUI connection
3. ✅ Broadcasts status update to all sessions

**Files Modified**:
- `crates/protocol/src/messages.rs` - Added `CheckComfyUIHealth` variant
- `crates/engine-ports/src/outbound/comfyui_port.rs` - Added `health_check()` method
- `crates/engine-adapters/src/infrastructure/comfyui.rs` - Implemented health check
- `crates/engine-adapters/src/infrastructure/websocket.rs` - Handler for message
- `crates/player-ports/src/outbound/game_connection_port.rs` - Port method (both WASM/desktop)
- `crates/player-adapters/src/infrastructure/websocket/game_connection_adapter.rs` - Implementation
- `crates/player-adapters/src/infrastructure/testing/mock_game_connection_port.rs` - Mock impl
- `crates/player-ui/src/presentation/components/creator/comfyui_banner.rs` - UI wiring

---

## P3: Future Features

### P3.1: US-NAR-009 - Visual Trigger Condition Builder
| Field | Value |
|-------|-------|
| **Status** | Not Started |
| **Effort** | 3-4 days |

**Implementation**:
1. Create `/api/triggers/schema` endpoint returning available trigger types
2. Build visual condition builder with dropdowns
3. Support AND/OR/AtLeast logic selection
4. Generate trigger JSON from visual selections

**Files**:
- NEW: `crates/engine-adapters/src/infrastructure/http/trigger_routes.rs`
- NEW: `crates/player-ui/src/presentation/components/story_arc/trigger_builder.rs`

---

### P3.2: US-AST-010 - Advanced Workflow Parameter Editor
| Field | Value |
|-------|-------|
| **Status** | Not Started |
| **Effort** | 2 days |

**Implementation**:
1. Parse workflow JSON to extract editable parameters
2. Create parameter editor UI with type-appropriate inputs
3. Support prompt mappings, locked inputs
4. Show style reference detection

**Files**:
- `crates/player-ui/src/presentation/components/settings/workflow_config_editor.rs`

---

### P3.3: Typed Error Handling
| Field | Value |
|-------|-------|
| **Status** | Not Started |
| **Effort** | 3-4 days |

**Implementation**:
1. Define `DomainError`, `ApplicationError`, `InfrastructureError` enums
2. Replace `anyhow::Result` with typed errors in critical paths
3. Map errors to HTTP status codes
4. Add error context for debugging

**Scope**: Start with one vertical slice (e.g., Challenge system), then expand.

---

### P3.4: Testing Infrastructure
| Field | Value |
|-------|-------|
| **Status** | Not Started |
| **Effort** | 1-2 weeks |

**Implementation**:
1. Set up test database (in-memory Neo4j or test containers)
2. Create test fixtures for common entities
3. Add unit tests for domain entities
4. Add integration tests for repository ports
5. Add API tests for HTTP endpoints

---

### P3.5: Token Budget Enforcement
| Field | Value |
|-------|-------|
| **Status** | Not Started |
| **Effort** | 4-6 hours |
| **Priority** | Medium |

**Problem**: `ContextBudgetConfig` settings exist and are exposed via the Settings API, but the actual token counting and budget enforcement is not implemented. Users can configure budgets but they have no effect.

**Related Types** (already exist in `domain/value_objects/context_budget.rs`):
- `ContextBudgetConfig` - Per-category token limits, stored in world settings
- `ContextCategory` - Enum for budget categories (Scene, Character, Challenges, etc.)
- `TokenCounter` - Token estimation with hybrid char/word counting

**Implementation**:
1. Inject `TokenCounter` into `websocket_helpers::build_prompt_from_action()`
2. After building each context section, count tokens
3. If over budget, truncate using `TokenCounter::truncate_to_budget()`
4. Optionally use LLM to summarize (if `enable_summarization` is true)
5. Log when truncation/summarization occurs for debugging

**Files to Modify**:
- `crates/engine-adapters/src/infrastructure/websocket_helpers.rs`
- `crates/engine-app/src/application/services/llm/prompt_builder.rs` (optional)

---

### P3.6: WebSocket-First Architecture
| Field | Value |
|-------|-------|
| **Status** | Planning |
| **Effort** | 2-3 days |
| **Priority** | High |

**Problem**: Game logic is split between REST and WebSocket, causing:
- Duplicate code paths (actantial CRUD exists in both REST and WebSocket)
- Missing broadcasts (REST game actions don't always notify clients)
- Inconsistent multiplayer behavior

**Proposed Architecture**:
- **REST API**: CRUD operations, queries, initial state fetch (Creator Mode)
- **WebSocket**: All game actions, DM controls, real-time broadcasts (Play Mode)

**Implementation**:
1. Audit all REST endpoints for game-state-modifying operations
2. Add broadcasts to REST endpoints that modify game state, OR
3. Deprecate REST game actions in favor of WebSocket
4. Remove duplicate WebSocket CRUD (keep REST for data, WS for suggestions)
5. Document the pattern for future development

**See**: `docs/plans/WEBSOCKET_ARCHITECTURE.md` for detailed analysis

---

## Implementation Order Recommendation

### Sprint 1 (3-4 days): Critical Gaps + Quick Wins
1. P0.1: Challenge Reasoning Approval (4h)
2. P0.2: Observation Check in Scene Resolution (2h)
3. P2.6: Calculate Relative Time for NPCs (1h)
4. P2.7: Manual ComfyUI Health Check (1h)
5. P1.2: Hidden NPCs Extension (1.5h)

### Sprint 2 (3-4 days): Feature Complete - Items
1. P1.1: US-REGION-ITEMS Phases 1-3 (8h)
2. P1.3: Challenge Outcome Trigger Execution (3h)

### Sprint 3 (3 sessions): Feature Complete - Characters & Actantial Model
1. P1.4: Character Mood and Relationship Tracking (4h) ✅ COMPLETED
2. P1.5: Actantial Model System (15-17h) - See detailed plan
   - Session A: Goal Repository + Actantial Context Service (6-7h)
   - Session B: LLM Context Integration + Goal UI (5-6h)
   - Session C: DM Panel Integration (4h)
3. P1.1: US-REGION-ITEMS Phases 5-6 (6h) - Deferred to Sprint 4

### Sprint 4 (3-4 days): UX Polish - ✅ COMPLETE
1. P2.1: View-as-Character Mode (4h) ✅
2. P2.2: Location Preview Modal (2h) ✅
3. P2.3: Story Arc Visual Timeline (6h) ✅
4. P2.4: Split Party Warning UX (1h) ✅
5. P2.5: Style Reference for Asset Generation (3h) ✅
6. Session ID Refactor: Remove session_id from story events ✅

### Sprint 5 (Dec 25, 2025): Actantial Completion + Cleanup - ✅ COMPLETE
1. Dead code removal: LLMContextService, AssembledContext, etc. ✅
2. WebSocket state updates for actantial messages ✅
3. Character search dropdown (CharacterPicker component) ✅
4. WebSocket architecture documentation ✅

### Sprint 6+ (Future): Major Features
- P3.1: Visual Trigger Condition Builder
- P3.2: Advanced Workflow Parameter Editor
- P3.3: Typed Error Handling
- P3.4: Testing Infrastructure
- P3.5: Token Budget Enforcement (wire ContextBudgetConfig into prompt building)
- P3.6: WebSocket-First Architecture (consolidate game logic to WebSocket)

---

## Session Progress Diary

### Session 3 (Dec 25, 2025)
**Target**: P1.3 Challenge Outcome Trigger Execution + P1.1 Phase 4 Pickup Items

**Work Completed**:
- ✅ **P1.3: Challenge Outcome Trigger Execution** - Fixed missing state change processing
  - **Root Cause**: Trigger execution was working, but StateChange objects were not applied to game state
  - **Solution**: Added `process_state_changes()` method to actually create items and update PC inventories
  - **Files Modified**: `challenge_outcome_approval_service.rs`, `state/mod.rs`
  - **Key Fix**: GiveItem triggers now create persistent items and add them to rolling PC's inventory

- ✅ **P1.1 Phase 4: Pickup Item Integration** - Complete end-to-end pickup functionality
  - **Feature**: Players can now pick up items from their current region
  - **Implementation**: 136 lines of code across 6 files following established DropItem pattern
  - **Protocol**: Added `PickupItem` and `ItemPickedUp` messages  
  - **Validation**: Comprehensive input/region/duplicate item validation with enhanced logging
  - **Error Handling**: 9 distinct error cases with proper rollback on failure
  - **UI Integration**: Full conversation log and inventory refresh support
  - **Files Modified**: `messages.rs`, `game_connection_port.rs`, `game_connection_adapter.rs`, `mock_game_connection_port.rs`, `websocket.rs`, `session_message_handler.rs`

**Validation**: ✅ `cargo check --workspace && cargo xtask arch-check` both pass

**Architecture Impact**: Hexagonal architecture maintained - all operations use repository ports with proper transaction safety

**Session Status**: Sprint 2 major progress - P1.1 Phases 1-4 and P1.3 complete! Core item management fully functional (Drop + Pickup)

---

### Session 4 (Dec 25, 2025)
**Target**: P1.4 Character Mood and Relationship Tracking + P1.5 Actantial Model Planning

**Work Completed**:
- ✅ **P1.4: Character Mood and Relationship Tracking** - Full implementation
  - **Phase 1-2**: Domain layer with `MoodLevel`, `RelationshipLevel`, `NpcMoodState` (previously done)
  - **Phase 3**: `MoodService` trait and `MoodServiceImpl` wired into `GameServices`
  - **Phase 4**: LLM context integration - `current_mood` and `relationship_to_player` now populated
  - **Phase 5**: DM Controls - Protocol messages (`SetNpcMood`, `SetNpcRelationship`, `GetNpcMoods`)
  - **Phase 5**: WebSocket handlers for all mood control messages
  - **Phase 5**: DM UI components (`NpcMoodPanel`, `NpcMoodListPanel`)
  - **Files Created**: `npc_mood_panel.rs`
  - **Files Modified**: `mood_service.rs`, `game_services.rs`, `state/mod.rs`, `websocket_helpers.rs`, `websocket.rs`, `messages.rs`, `session_message_handler.rs`

- ✅ **P1.5: Actantial Model System - Planning Complete**
  - **Analysis**: Researched how Goals integrate with Wants, Actantial Views, and LLM context
  - **Design Decision**: Expanded from simple GoalRepository to full Actantial Model System
  - **Secret Wants**: Designed tiered visibility (Known/Suspected/Hidden) with behavioral guidance
  - **NPC → PC Views**: NPCs can now view player characters as Helpers/Opponents
  - **UI Mockups**: Created detailed mockups for Actantial Panel, Want Editor, Goal Manager
  - **Plan Document**: ACTANTIAL_MODEL_IMPLEMENTATION_PLAN.md (completed and archived)
  - **Phases Defined**: 5 phases across 3 sessions (15-17 hours total)

**Validation**: ✅ `cargo check --workspace && cargo xtask arch-check` both pass

**Session Status**: Sprint 3 - P1.4 complete, P1.5 planning complete. Ready to begin Actantial Model implementation in Session A.

---

### Session 5 (Dec 25, 2025)
**Target**: Sprint 5 - Actantial Completion + Dead Code Cleanup

**Work Completed**:

- ✅ **Dead Code Removal** (~850 lines removed)
  - Deleted `LLMContextService` (~770 lines) - never instantiated
  - Deleted `AssembledContext`, `CategoryContext` - unused context assembly
  - Removed empty `want.rs` module and `services/` directory
  - Cleaned imports in `prompt_builder.rs` and `context_budget.rs`
  - Documented remaining types (`ContextBudgetConfig`, `TokenCounter`) as pending P3.5

- ✅ **WebSocket State Updates for Actantial Messages**
  - Added `actantial_refresh_counter` signal to `GameState`
  - Added `trigger_actantial_refresh()` helper method
  - Updated 16 message handlers to trigger refresh on actantial changes
  - Connected `motivations_tab.rs` to global refresh signal

- ✅ **Character Search Dropdown** (`CharacterPicker` component)
  - Created `crates/player-ui/src/presentation/components/common/character_picker.rs` (~290 lines)
  - Searchable dropdown for NPCs and PCs
  - Visual distinction: NPC = blue, PC = green badges
  - Returns selection as `{type}:{id}` format
  - Integrated into `ActantialViewsEditor` (replaces manual UUID entry)
  - Added `world_id` prop threading through WantsSection → WantCard → ActantialViewsEditor

- ✅ **WebSocket Architecture Documentation**
  - Created `docs/plans/WEBSOCKET_ARCHITECTURE.md`
  - Analyzes REST vs WebSocket split
  - Proposes WebSocket-first architecture with request-response pattern
  - Migration plan: 5 phases over 2-3 days

**Files Created**:
- `docs/plans/WEBSOCKET_ARCHITECTURE.md`
- `crates/player-ui/src/presentation/components/common/character_picker.rs`

**Files Modified**:
- `crates/player-ui/src/presentation/state/game_state.rs`
- `crates/player-ui/src/presentation/handlers/session_message_handler.rs`
- `crates/player-ui/src/presentation/components/creator/motivations_tab.rs`
- `crates/player-ui/src/presentation/components/common/mod.rs`

**Validation**: ✅ `cargo check --workspace && cargo xtask arch-check` both pass

**Session Status**: Sprint 5 COMPLETE - All P1.5 refinements done, dead code cleaned, architecture documented.

---

## Notes

- All estimates assume familiarity with codebase
- Run `cargo check --workspace && cargo xtask arch-check` after each feature
- Update `ACTIVE_DEVELOPMENT.md` when completing features
- NixOS environment - use `nix-shell` for all commands
