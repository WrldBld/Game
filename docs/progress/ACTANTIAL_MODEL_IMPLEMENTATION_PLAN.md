# Actantial Model System - Implementation Plan

**Created**: 2025-12-25  
**Status**: ✅ Complete (All Phases Implemented)
**Total Estimated Effort**: 15-17 hours across 3 sessions

> **Implementation Complete**: All 5 phases have been implemented. See individual phase progress documents for details. Refinement work (wiring HTTP fetches, LLM suggestions, state updates) is pending.

---

## Executive Summary

This plan implements a comprehensive Actantial Model system based on Greimas' narrative theory. The system provides rich motivational context for NPCs that integrates with LLM prompts and DM tools.

### Key Features
- **Goals**: Abstract desire targets (Power, Revenge, Redemption, etc.)
- **Wants with Targets**: NPCs desire specific Characters, Items, or Goals
- **Actantial Views**: NPCs view others as Helpers, Opponents, Senders, or Receivers
- **Secret Motivations**: Hidden wants with behavioral guidance and "tells"
- **NPC → PC Views**: NPCs can view player characters as allies/enemies
- **LLM Integration**: Full motivational context in every NPC response
- **DM Tools**: Visual panels for viewing and editing NPC motivations

---

## Data Model

### Core Types

```rust
/// Visibility level for a Want
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WantVisibility {
    /// Player knows this motivation
    Known,
    /// Player suspects something but doesn't know details  
    Suspected,
    /// Player has no idea
    Hidden,
}

/// Target of an actantial view (can be NPC or PC)
#[derive(Debug, Clone, PartialEq)]
pub enum ActantialTarget {
    Npc(CharacterId),
    Pc(PlayerCharacterId),
}

/// A resolved want target
#[derive(Debug, Clone, PartialEq)]
pub enum WantTarget {
    Character { id: CharacterId, name: String },
    Item { id: ItemId, name: String },
    Goal { id: GoalId, name: String, description: Option<String> },
}

/// An actor in the actantial model
#[derive(Debug, Clone)]
pub struct ActantialActor {
    pub target: ActantialTarget,
    pub name: String,
    pub reason: String,
}

/// Complete actantial context for a character
#[derive(Debug, Clone)]
pub struct ActantialContext {
    pub character_id: CharacterId,
    pub character_name: String,
    pub wants: Vec<WantContext>,
    pub social_views: SocialViewSummary,
}

/// A want with its full context
#[derive(Debug, Clone)]
pub struct WantContext {
    pub want_id: WantId,
    pub description: String,
    pub intensity: f32,
    pub priority: u32,
    pub visibility: WantVisibility,
    pub target: Option<WantTarget>,
    
    /// Behavioral guidance when probed about this want
    pub deflection_behavior: Option<String>,
    
    /// Subtle behavioral tells that hint at this want
    pub tells: Vec<String>,
    
    /// Actantial roles for this want
    pub helpers: Vec<ActantialActor>,
    pub opponents: Vec<ActantialActor>,
    pub sender: Option<ActantialActor>,
    pub receiver: Option<ActantialActor>,
}

/// Summary of social views across all wants
#[derive(Debug, Clone)]
pub struct SocialViewSummary {
    pub allies: Vec<(ActantialTarget, String, Vec<String>)>, // target, name, reasons
    pub enemies: Vec<(ActantialTarget, String, Vec<String>)>,
}
```

### Want Entity Updates

```rust
// Updated Want entity (crates/domain/src/entities/want.rs)
pub struct Want {
    pub id: WantId,
    pub description: String,
    pub intensity: f32,
    pub created_at: DateTime<Utc>,
    
    // UPDATED: Replace known_to_player with visibility
    pub visibility: WantVisibility,
    
    // NEW: Behavioral guidance for hidden wants
    pub deflection_behavior: Option<String>,
    
    // NEW: Subtle behavioral tells
    pub tells: Vec<String>,
}
```

### Neo4j Schema

```cypher
// Goal Node
(:Goal {
    id: uuid,
    world_id: uuid,
    name: string,
    description: string?
})

// World owns Goals
(world:World)-[:CONTAINS_GOAL]->(goal:Goal)

// Want targets (polymorphic)
(want:Want)-[:TARGETS]->(target)  // Character, Item, or Goal

// Actantial views (NPC → NPC or NPC → PC)
(npc:Character)-[:VIEWS_AS_HELPER {want_id, reason, assigned_at}]->(target)
(npc:Character)-[:VIEWS_AS_OPPONENT {want_id, reason, assigned_at}]->(target)
(npc:Character)-[:VIEWS_AS_SENDER {want_id, reason, assigned_at}]->(target)
(npc:Character)-[:VIEWS_AS_RECEIVER {want_id, reason, assigned_at}]->(target)
// where target can be :Character or :PlayerCharacter
```

---

## UI Mockups

### Actantial Panel (DM View - Director Mode)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Marcus the Redeemed - Motivations                              [Collapse] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  WANTS                                                          [+ Add Want]│
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Priority 1                                                    [Edit] [X]││
│  │ ┌─────────────────────────────────────────────────────────────────────┐ ││
│  │ │ "Atone for the village massacre"                                    │ ││
│  │ │                                                           [Suggest] │ ││
│  │ └─────────────────────────────────────────────────────────────────────┘ ││
│  │                                                                         ││
│  │ Target: [Goal ▼] [Redemption_____________] [Select...]                  ││
│  │                                                                         ││
│  │ Intensity: [████████░░] 0.8  Strong                                     ││
│  │                                                                         ││
│  │ Visibility: ( ) Known  ( ) Suspected  (•) Hidden                        ││
│  │                                                                         ││
│  │ ─── Secret Behavior (visible because Hidden) ─────────────────────────  ││
│  │ │ When Probed:                                                        │ ││
│  │ │ ┌─────────────────────────────────────────────────────────────────┐ │ ││
│  │ │ │ Deflect with a sad smile; change subject to present dangers    │ │ ││
│  │ │ │                                                       [Suggest] │ │ ││
│  │ │ └─────────────────────────────────────────────────────────────────┘ │ ││
│  │ │                                                                     │ ││
│  │ │ Behavioral Tells:                                                   │ ││
│  │ │ ┌─────────────────────────────────────────────────────────────────┐ │ ││
│  │ │ │ • Avoids eye contact when past is mentioned              [X]   │ │ ││
│  │ │ │ • Tenses visibly at the word "village"                   [X]   │ │ ││
│  │ │ │ [+ Add Tell]                                        [Suggest]  │ │ ││
│  │ │ └─────────────────────────────────────────────────────────────────┘ │ ││
│  │ └─────────────────────────────────────────────────────────────────────┘ ││
│  │                                                                         ││
│  │ ─── Actantial Roles ──────────────────────────────────────────────────  ││
│  │                                                                         ││
│  │ Helpers:     [Elena (NPC)___________] "Saved my life"         [X]       ││
│  │              [Aldric (Player)_______] "Defended me"           [X]       ││
│  │              [+ Add Helper]                                             ││
│  │                                                                         ││
│  │ Opponents:   [Lord Vorn (NPC)_______] "Reminds me of crimes"  [X]       ││
│  │              [+ Add Opponent]                                           ││
│  │                                                                         ││
│  │ Sender:      [His dying captain_____] "Gave the final order"  [Clear]   ││
│  │ Receiver:    [The villagers' ghosts_] "Their peace"           [Clear]   ││
│  │                                                                         ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                             │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  SOCIAL STANCE (Aggregated)                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Allies:                                                                 ││
│  │   Elena (NPC) - "Saved my life"                                         ││
│  │   Aldric (Player) - "Defended me against Lord Vorn's accusations"       ││
│  │                                                                         ││
│  │ Enemies:                                                                ││
│  │   Lord Vorn (NPC) - "Reminds me of my crimes"                           ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Want Editor Modal (Detailed Editing)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Edit Want                                                           [X]   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Description:                                                               │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Atone for the village massacre                                          ││
│  │                                                                          ││
│  │                                                                          ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                              [Suggest]      │
│                                                                             │
│  Target Type: [Goal          ▼]                                             │
│                                                                             │
│  Target:      [Redemption                    ▼]  [Create New Goal...]       │
│               ├─ Redemption                                                 │
│               ├─ Peace                                                      │
│               ├─ Power                                                      │
│               └─ Recognition                                                │
│                                                                             │
│  Intensity:   [████████░░] 0.8                                              │
│               Mild ─────────────────────────────────────────── Obsession    │
│                                                                             │
│  Priority:    [1 - Primary ▼]                                               │
│                                                                             │
│  ─── Player Knowledge ─────────────────────────────────────────────────────│
│                                                                             │
│  Visibility:  (•) Known - Player knows this motivation                      │
│               ( ) Suspected - Player senses something                       │
│               ( ) Hidden - Player has no idea                               │
│                                                                             │
│  ─── Secret Behavior (for Suspected/Hidden) ───────────────────────────────│
│                                                                             │
│  When Probed:                                                               │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Deflect with a sad smile; change subject to present dangers            ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                              [Suggest]      │
│                                                                             │
│  Behavioral Tells:                                                          │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Avoids eye contact when past is mentioned                        [X]   ││
│  │ Tenses visibly at the word "village"                             [X]   ││
│  │ ──────────────────────────────────────────────────────────────────────  ││
│  │ [Add new tell...                                        ]               ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                              [Suggest]      │
│                                                                             │
│                                              [Cancel]  [Save Want]          │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Goal Manager (Creator Mode)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  World Goals                                                    [+ New Goal]│
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Goals define abstract desires that NPCs can pursue.                        │
│  Unlike Items or Characters, Goals represent intangible objectives.         │
│                                                                             │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │ Redemption                                                 [Edit] [X] │ │
│  │ The cleansing of past sins through noble action                       │ │
│  │ Used by: Marcus the Redeemed, Sister Elara                            │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │ Power                                                      [Edit] [X] │ │
│  │ Political or personal dominance over others                           │ │
│  │ Used by: Lord Vorn, Chancellor Graves                                 │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │ Peace                                                      [Edit] [X] │ │
│  │ An end to conflict and suffering                                      │ │
│  │ Used by: Elder Moira                                                  │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │ Recognition                                                [Edit] [X] │ │
│  │ Fame, acknowledgment, or validation from others                       │ │
│  │ Used by: (none)                                                       │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ─────────────────────────────────────────────────────────────────────────  │
│                                                                             │
│  Common Goals:                                           [Add All] [Clear]  │
│  [ ] Revenge    [ ] Justice    [ ] Love    [ ] Wealth                       │
│  [ ] Freedom    [ ] Knowledge  [ ] Honor   [ ] Survival                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Goal Editor Modal

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Create Goal                                                         [X]   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Name:                                                                      │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Redemption                                                              ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                              [Suggest]      │
│                                                                             │
│  Description:                                                               │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ The cleansing of past sins through noble action. Characters pursuing   ││
│  │ this goal seek to make amends for wrongs they have committed.          ││
│  │                                                                          ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                              [Suggest]      │
│                                                                             │
│                                              [Cancel]  [Create Goal]        │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Actantial View Quick-Add (Inline)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Add Helper for "Atone for the village massacre"                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Type: (•) NPC  ( ) Player Character                                        │
│                                                                             │
│  Character: [Search or select...                              ▼]            │
│             ├─ Elena (in scene)                                             │
│             ├─ Brother Thomas (in scene)                                    │
│             ├─ ─────────────────────────                                    │
│             ├─ Aldric [Player - Session 1]                                  │
│             ├─ Bella [Player - Session 1]                                   │
│             └─ ─────────────────────────                                    │
│                                                                             │
│  Reason:                                                                    │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Saved my life during the ambush                                         ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                              [Suggest]      │
│                                                                             │
│                                              [Cancel]  [Add Helper]         │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## LLM Prompt Format

### Character Context with Actantial Model

```
CHARACTER: Marcus the Redeemed (Mentor)
DESCRIPTION: A former mercenary seeking redemption for past sins...

CURRENT MOOD: Suspicious (toward Aldric the player)
RELATIONSHIP TO PLAYER: Acquaintance

=== MOTIVATIONS ===

KNOWN MOTIVATION:
- Help travelers on the road
  Priority: 2 (Secondary)
  Intensity: Moderate
  Helpers: None
  Opponents: Bandits of the North Road

SECRET MOTIVATION (player does not know this):
- Atone for the village massacre → [Goal: Redemption]
  Priority: 1 (Primary)
  Intensity: Very Strong (0.9)
  
  Helpers:
    - Elena (NPC): "Saved my life"
    - Aldric (Player): "Defended me against accusations"
  
  Opponents:
    - Lord Vorn (NPC): "Reminds me of my crimes"
  
  Sender: His dying captain - "Gave the final order"
  Receiver: The villagers' ghosts - "Their peace"
  
  BEHAVIORAL GUIDANCE:
  - When probed about past: Deflect with a sad smile; change subject to present dangers
  - Tells (subtle signs you may show):
    * Avoids eye contact when past is mentioned
    * Tenses visibly at the word "village"
  
  DO NOT directly reveal this motivation. You may hint through the behavioral tells
  but never state it explicitly. If pressed, use the deflection behavior.

SUSPECTED MOTIVATION (player senses something):
- Seeking something in the old ruins
  Priority: 3 (Tertiary)
  Intensity: Mild
  The player has noticed your interest but doesn't know why.
  You may be evasive but don't need to completely deny it.

=== SOCIAL STANCE ===

ALLIES (characters you trust/appreciate):
- Elena: Trusted friend who saved your life
- Aldric (Player): Has proven trustworthy by defending you

ENEMIES (characters you distrust/oppose):
- Lord Vorn: Embodiment of your past sins; his presence is painful
```

---

## Implementation Phases

### Phase 1: Goal Repository (Foundation)
**Effort**: 2 hours  
**Session**: A

#### Files to Create
- `crates/engine-adapters/src/infrastructure/persistence/goal_repository.rs`

#### Files to Modify
- `crates/engine-adapters/src/infrastructure/persistence/mod.rs`
- `crates/engine-adapters/src/infrastructure/persistence/connection.rs`

#### Deliverables
- [ ] `Neo4jGoalRepository` with full CRUD
- [ ] Schema constraint: `CREATE CONSTRAINT goal_id IF NOT EXISTS FOR (g:Goal) REQUIRE g.id IS UNIQUE`
- [ ] Index: `CREATE INDEX goal_world IF NOT EXISTS FOR (g:Goal) ON (g.world_id)`
- [ ] `goals()` accessor on `Neo4jRepository`
- [ ] Implement `GoalRepositoryPort` trait

#### Verification
```bash
cargo check --workspace && cargo xtask arch-check
```

---

### Phase 2: Domain Model & Actantial Context Service
**Effort**: 4-5 hours  
**Session**: A

#### Files to Create
- `crates/domain/src/value_objects/actantial_context.rs`
- `crates/engine-app/src/application/services/actantial_context_service.rs`

#### Files to Modify
- `crates/domain/src/entities/want.rs` - Add visibility, deflection_behavior, tells
- `crates/domain/src/value_objects/mod.rs` - Export actantial_context
- `crates/engine-ports/src/outbound/repository_port.rs` - Update actantial view signatures
- `crates/engine-adapters/src/infrastructure/persistence/character_repository.rs` - Support PC targets
- `crates/engine-app/src/application/services/mod.rs` - Export service
- `crates/engine-adapters/src/infrastructure/state/game_services.rs` - Add service
- `crates/engine-adapters/src/infrastructure/state/mod.rs` - Wire service

#### Deliverables
- [ ] `WantVisibility` enum (Known, Suspected, Hidden)
- [ ] `ActantialTarget` enum (Npc, Pc)
- [ ] `WantTarget` enum (Character, Item, Goal)
- [ ] `ActantialContext`, `WantContext`, `ActantialActor`, `SocialViewSummary` structs
- [ ] Updated `Want` entity with new fields
- [ ] Updated `add_actantial_view` to accept `ActantialTarget`
- [ ] Updated `get_actantial_views` to return both NPC and PC targets
- [ ] `get_want_target(want_id)` method in CharacterRepository
- [ ] `ActantialContextService` trait
- [ ] `ActantialContextServiceImpl` with aggregation logic
- [ ] Default generation for missing behavioral guidance
- [ ] Service wired into `GameServices`

#### Verification
```bash
cargo check --workspace && cargo xtask arch-check
```

---

### Phase 3: LLM Context Integration
**Effort**: 3 hours  
**Session**: B

#### Files to Modify
- `crates/domain/src/value_objects/llm_context.rs` - Add actantial fields
- `crates/engine-adapters/src/infrastructure/websocket_helpers.rs` - Fetch actantial context
- `crates/engine-adapters/src/run/server.rs` - Pass actantial service to workers

#### Deliverables
- [ ] `MotivationContext` struct for LLM
- [ ] `SocialStanceContext` struct for LLM
- [ ] Updated `CharacterContext` with motivations and social_stance
- [ ] `build_prompt_from_action` fetches actantial context
- [ ] Proper formatting for Known/Suspected/Hidden wants
- [ ] Behavioral guidance included for secret wants

#### Verification
```bash
cargo check --workspace && cargo xtask arch-check
```

---

### Phase 4: DM Panel Integration
**Status**: ✅ Complete
**Effort**: 4 hours  
**Session**: C
**Progress Document**: `P1.5_PHASE4_DM_PANEL_PROGRESS.md`

#### Files Created
- `crates/player-ui/src/presentation/components/creator/motivations_tab.rs` - Main UI component (~680 lines)
- `crates/player-app/src/application/services/actantial_service.rs` - HTTP client service (~230 lines)
- `crates/engine-adapters/src/infrastructure/http/want_routes.rs` - HTTP routes

#### Files Modified
- `crates/protocol/src/messages.rs` - Added all actantial DTOs and messages
- `crates/engine-adapters/src/infrastructure/websocket.rs` - Added 16 CRUD handlers
- `crates/engine-app/src/application/services/suggestion_service.rs` - Added 4 suggestion methods
- `crates/engine-app/src/application/services/llm_queue_service.rs` - Added actantial routing
- `crates/player-ui/src/presentation/components/creator/character_form.rs` - Added MotivationsTab
- `crates/player-ui/src/presentation/handlers/session_message_handler.rs` - Added handlers
- `crates/domain/src/entities/goal.rs` - Added common_goals module

#### Deliverables
- [x] `MotivationsTab` component with WantsSection, GoalsSection, SocialStanceSection
- [x] `WantEditorModal` and `GoalEditorModal` for create/edit
- [x] `ActantialActorBadge` for role visualization
- [x] HTTP routes for wants, targets, actantial views
- [x] WebSocket handlers for all actantial messages
- [x] Suggestion methods in SuggestionService
- [x] Common goals definition (12 defaults)

#### Pending Refinements
- [ ] Wire HTTP fetches in MotivationsTab (currently placeholder data)
- [ ] Add LLM suggestion button integration
- [ ] Implement target selection UI
- [ ] Add actantial view editor
- [ ] Implement session message handler state updates

#### Verification
```bash
cargo check --workspace && cargo xtask arch-check  # ✅ Passes
```

---

### Phase 5: Goal Management UI
**Effort**: 2-3 hours  
**Session**: B

#### Files to Create
- `crates/player-ui/src/presentation/components/creator/goal_manager.rs`
- `crates/player-ui/src/presentation/components/creator/goal_editor_modal.rs`
- `crates/player-ui/src/presentation/components/common/goal_picker.rs`

#### Files to Modify
- `crates/engine-adapters/src/infrastructure/http/mod.rs` - Add goal routes
- `crates/player-ui/src/presentation/components/creator/mod.rs` - Export components

#### HTTP Routes
```
GET    /api/worlds/:id/goals     - List goals for world
POST   /api/worlds/:id/goals     - Create goal
GET    /api/goals/:id            - Get goal by ID
PUT    /api/goals/:id            - Update goal
DELETE /api/goals/:id            - Delete goal
```

#### Deliverables
- [ ] `GoalManager` component for Creator Mode
- [ ] `GoalEditorModal` for create/edit
- [ ] `GoalPicker` component for Want target selection
- [ ] HTTP route handlers
- [ ] LLM suggestion for goal names and descriptions
- [ ] "Common Goals" quick-add feature

#### Verification
```bash
cargo check --workspace && cargo xtask arch-check
```

---

## Session Breakdown

### Session A: Foundation (6-7 hours)
- Phase 1: Goal Repository (2h)
- Phase 2: Domain Model & Service (4-5h)

**Goal**: Complete backend infrastructure. Actantial context can be fetched but not yet used by LLM or UI.

### Session B: Integration (5-6 hours)
- Phase 3: LLM Context Integration (3h)
- Phase 5: Goal Management UI (2-3h)

**Goal**: LLM receives full actantial context. Goals can be managed in Creator Mode.

### Session C: DM Tools (4 hours)
- Phase 4: DM Panel Integration (4h)

**Goal**: DM can view and edit NPC motivations with LLM-suggested behavioral guidance.

---

## Testing Strategy

### Unit Tests (Per Phase)
- Phase 1: Goal repository CRUD
- Phase 2: Actantial context aggregation logic
- Phase 3: LLM context formatting
- Phase 4: N/A (UI)
- Phase 5: HTTP route handlers

### Integration Tests
- Create goal → Add want targeting goal → Fetch actantial context
- Add PC as helper → Verify in LLM context
- Update want visibility → Verify behavioral guidance formatting

### Manual Testing
- Create NPCs with complex motivations in Creator Mode
- Verify LLM responses reflect secret motivations appropriately
- Test DM panel editing flows

---

## Success Criteria

### Phase 1 Complete When
- [ ] `cargo check --workspace` passes
- [ ] Goals can be created/read/updated/deleted via repository
- [ ] `Neo4jRepository.goals()` returns working repository

### Phase 2 Complete When
- [ ] Actantial context service aggregates wants, targets, and views
- [ ] NPC → PC actantial views work
- [ ] Default behavioral guidance is generated for missing fields

### Phase 3 Complete When
- [ ] LLM prompts include full motivational context
- [ ] Secret wants include behavioral guidance
- [ ] Known/Suspected/Hidden formatting is correct

### Phase 4 Complete When
- [ ] DM can view NPC motivations in panel
- [ ] DM can edit wants, targets, and views
- [ ] LLM suggestions work for deflection and tells

### Phase 5 Complete When
- [ ] Goals can be managed in Creator Mode
- [ ] Goal picker works in Want editor
- [ ] HTTP API is functional

---

## Dependencies

```
Phase 1 ─────┬────> Phase 2 ─────┬────> Phase 3
             │                   │
             │                   └────> Phase 4
             │
             └────────────────────────> Phase 5
```

- Phase 2 depends on Phase 1 (needs GoalRepository)
- Phase 3 depends on Phase 2 (needs ActantialContextService)
- Phase 4 depends on Phase 2 and Phase 3 (needs service and context format)
- Phase 5 only depends on Phase 1 (Goal CRUD)

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Token budget for full actantial context | LLM costs increase | Summarize for non-featured NPCs |
| Complex Neo4j queries for aggregation | Performance | Add indexes, consider caching |
| UI complexity for want editing | DM confusion | Good defaults, progressive disclosure |
| LLM leaking secrets despite guidance | Immersion break | Strong behavioral guidance, tells as alternatives |

---

## Post-Implementation

### Documentation Updates
- Update `docs/systems/character-system.md` with new features
- Add examples to `docs/architecture/neo4j-schema.md`
- Update `ACTIVE_DEVELOPMENT.md`

### Future Enhancements
- Revelation triggers (conditions for secrets to become known)
- Actantial view evolution based on gameplay
- LLM tools for modifying actantial relationships
- Staging influence from actantial model
