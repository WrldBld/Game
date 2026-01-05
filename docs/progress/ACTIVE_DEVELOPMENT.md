# Active Development

Active implementation tracking for WrldBldr user stories and features.

**Last Updated**: 2026-01-04  
**Branch**: `new-arch`

---

## Recently Completed (2026-01-04)

### WebSocket Implementation - COMPLETE
All WebSocket TODOs resolved:
1. ✅ `WorldJoined` response now fetches PC data when role is Player
2. ✅ Region items returned in `SceneChanged` response via `build_region_items()` helper
3. ✅ Challenge outcome decision uses stored approval data (not hardcoded)
4. ✅ `ChallengeOutcomeDecision::Edit` broadcasts with DM's modified description
5. ✅ `DirectorialContext` stored in per-world cache in ConnectionManager
6. ✅ `ChallengeOutcomePending` sent to DMs when rolls require approval
7. ✅ `ChallengeOutcomeDecision::Suggest` queues LLM request for outcome suggestions

### Challenge Resolution Flow - COMPLETE
- Extended `RollResult` with challenge/character metadata for DM notifications
- Added `challenge_outcome: Option<ChallengeOutcomeData>` to `ApprovalRequestData`
- Handler uses stored approval data for outcome type, roll, modifier, character info
- `execute_for_pc()` variant passes PC context for trigger execution

### LLM Outcome Suggestions - COMPLETE
- Added `LlmRequestType::OutcomeSuggestion` variant with challenge context
- Queue processor handles OutcomeSuggestion and generates 3 alternatives
- Added `BroadcastEvent` type for queue->main communication
- Main loop broadcasts `OutcomeSuggestionReady` to DMs in world

### Code Quality Improvements (2026-01-04)
- Extracted UUID parsing helpers (30+ occurrences reduced to helper calls)
- Extracted DM authorization helpers (`require_dm`, `require_dm_for_request`)
- Extracted `build_navigation_data()` async helper
- Extracted `build_region_items()` async helper
- Added `StagedNpc::is_visible_to_players()` method to domain
- Removed unused `ConnectionManager` trait from ports.rs
- Wired `ExecuteEffects` into narrative event approval handler

### Flag Storage System (2026-01-04)
- Added `FlagRepo` port trait with world and PC-scoped flag operations
- Implemented Neo4j flag repository
- Created `Flag` entity module
- Wired into `EnterRegion` use case for scene condition evaluation

---

## Current Focus

### Outstanding Engine TODOs - COMPLETE (2026-01-04 Session 2)

All high-priority TODOs resolved:

1. ✅ **Broadcast OutcomeSuggestionReady to DMs** - Added `BroadcastEvent` enum, handled in main loop
2. ✅ **PC Name in Dialogue Events** - Now fetches from `player_character_repo`
3. ✅ **Event Completion Tracking** - `get_completed_events()` queries event chains
4. ✅ **Challenge Completion Tracking** - `get_resolved_challenges()` queries resolved challenges

### Remaining TODOs - LOW PRIORITY

1. **Game Time System** (`observation.rs:99`)
   - Using real time instead of in-game time for observations
   - Requires designing time progression mechanics

2. **Event Outcomes Tracking** (`narrative.rs:279`)
   - `event_outcomes` HashMap for trigger evaluation
   - Would enable "trigger if event X had outcome Y"

3. **Challenge Success/Failure Tracking** (`narrative.rs:290`)
   - `challenge_successes` HashMap for trigger evaluation
   - Would enable "trigger if challenge X was succeeded/failed"

4. **Per-PC Flags System** (`narrative.rs:279`)
   - `flags` HashMap for trigger evaluation
   - Would enable custom state tracking per PC

### Documentation Updates - IN PROGRESS

All system documentation files reference old file paths and need updating. Scene system docs are current and can be used as template.

---

## Upcoming Features

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

### P3.1: Mood & Expression System

| Field | Value |
|-------|-------|
| **Status** | Planning Complete - Ready for Implementation |
| **Priority** | Low (Polish) |
| **Effort** | 30-35 hours (4-5 days) |
| **Full Plan** | [MOOD_EXPRESSION_SYSTEM_IMPLEMENTATION.md](../plans/MOOD_EXPRESSION_SYSTEM_IMPLEMENTATION.md) |

**Description**: Implement a three-tier emotional model with clear terminology:

1. **Disposition** (persistent NPC→PC relationship) - renamed from MoodLevel
2. **Mood** (semi-persistent NPC state) - set during staging
3. **Expression** (transient dialogue state) - inline markers that change sprites

**Key Features**:
- Clear terminology separation: Disposition vs Mood vs Expression
- Expression markers in dialogue: `*happy*` or `*excited|happy*`
- LLM context includes both disposition AND mood
- DM editable dialogue with live marker validation
- Expression sheet generation via ComfyUI

**Implementation Phases** (13 phases):
0. Disposition Rename Refactor (prerequisite)
1. New Mood System (Domain & Protocol)
2. Staging System Mood Integration
3. Persistence & Repository Updates
4. LLM Prompt Updates
5. Expression Sheet Generation
6. Typewriter with Expression Changes
7. Character Sprite Updates
8. Player Input Validation
9. Expression Config Editor UI
10. Expression Sheet Generation UI
11. DM Approval Marker Support
12. Testing & Polish

---

## Architecture & Technical Debt

For architecture remediation, code quality, and cleanup work, see:

**[HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md](../plans/HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md)**

Current architecture score: 92/100 → Target: 98/100

Remaining work includes:
- Protocol → Domain dependency removal (C5)
- Implementation code in ports layer (C6)
- File decomposition (app_state.rs, request_handler.rs)
- God trait completion
- Clippy warning fixes

---

## Completed Phases

### Phase A: Core Player Experience ✅
- Navigation panel with region/exit buttons
- Game time display with time-of-day icons
- Approach event overlay for NPC approaches
- Location event banner

### Phase B: Player Knowledge & Agency ✅
- Inventory panel with item categories
- Known NPCs panel with observations
- Mini-map with clickable regions

### Phase P: Feature Parity Gap Removal ✅
- Ad-hoc challenge creation modal wired
- Mini-map background image from location data
- Inventory equip/drop end-to-end
- ComfyUI polling and asset persistence
- Challenge outcome triggers
- Scene broadcast after PC creation
- State updates for NpcLocationShared and PcSelected

### Sprint 4: UX Polish ✅
- Split Party Warning
- Location Preview
- View-as-Character
- Style Reference
- Visual Timeline

### Sprint 5: Actantial Completion ✅
- Dead code cleanup (~850 lines)
- CharacterPicker component
- WebSocket architecture doc
- MotivationsTab with wants, goals, actantial views

### Sprint 6: Code Quality Remediation ✅
- All P0 Critical items (REST→WebSocket migration)
- All P1 High Priority items (WebSocket migration Phase 5)
- All P2 Medium Priority items (DTO consolidation, dead code)
- P3.2-P3.4 Low Priority items (deps, type consolidation, legacy messages)

---

## Completed Stories (Summary)

| Story | Description | Completed |
|-------|-------------|-----------|
| US-NAV-008 | Navigation Options UI | 2025-12-18 |
| US-NAV-009 | Game Time Display | 2025-12-18 |
| US-NAV-010 | Mini-map with Clickable Regions | 2025-12-18 |
| US-NPC-008 | Approach Event Display | 2025-12-18 |
| US-NPC-009 | Location Event Display | 2025-12-18 |
| US-CHAR-009 | Inventory Panel | 2025-12-18 |
| US-OBS-004/005 | Known NPCs Panel | 2025-12-18 |
| US-CHAL-009 | Skill Modifiers Display | 2025-12-18 |
| US-DLG-009 | Context Budget Configuration | 2025-12-18 |
| US-CHAL-010 | Region-level Challenge Binding | 2025-12-24 |
| US-SCN-009 | Scene Entry Conditions | 2025-12-24 |
| US-INV-001 | PC Inventory System | 2025-12-24 |
| US-STG-013/US-OBS-006 | Hidden NPCs + Unrevealed Interactions | 2025-12-25 |
| ARCH-SHIM-001 | Remove internal shims | 2025-12-25 |
| P0.1-P0.4 | Critical fixes (REST→WS, parsing) | 2025-12-27 |
| P1.1-P1.6 | Core functionality (staging, dialogue, handlers) | 2025-12-27 |
| P2.1-P2.6 | Feature completion (websocket split, DTOs, docs) | 2025-12-27 |
| P3.2-P3.4 | Polish (deps, types, legacy messages) | 2025-12-27 |

---

## Progress Log

| Date | Change |
|------|--------|
| 2026-01-04 | WebSocket TODOs complete - PC data, region items, challenge flow, directorial context |
| 2026-01-04 | Challenge resolution flow rewritten - uses stored approval data, proper outcome types |
| 2026-01-04 | LLM outcome suggestions queued (broadcast pending) |
| 2026-01-04 | Flag storage system implemented |
| 2026-01-04 | Code quality: helper functions extracted |
| 2026-01-03 | Simplified architecture migration complete |
| 2025-12-30 | Consolidated documentation - moved remediation to ARCHITECTURE_GAP_REMEDIATION_PLAN |
| 2025-12-27 | Sprint 6 Code Quality Remediation complete (P0-P3.4) |
| 2025-12-25 | Sprint 5 Actantial Completion complete |
| 2025-12-25 | Sprint 4 UX Polish complete |
| 2025-12-24 | Phase P Feature Parity complete |
| 2025-12-24 | Phase C started |
| 2025-12-18 | Phase A & B complete |
