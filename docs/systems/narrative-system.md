# Narrative System

## Overview

## Canonical vs Implementation

This document is canonical for how the system *should* behave in gameplay.
Implementation notes are included to track current status and may lag behind the spec.

**Legend**
- **Canonical**: Desired gameplay rule or behavior (source of truth)
- **Implemented**: Verified in code and wired end-to-end
- **Planned**: Designed but not fully implemented yet


The Narrative System enables DMs to design **future events** with triggers and effects. When conditions are met (player enters a location, completes a challenge, talks to an NPC), events fire and change the game world. Events can be chained into sequences with branching paths based on outcomes.

## WebSocket Coverage

- `wrldbldr_shared::NarrativeEventRequest` now routes through `crates/engine/src/api/websocket/ws_narrative_event.rs`. The handler emits `NarrativeEventData` responses for list/get and implements create/update/delete, set active/favorite, trigger/reset, and trigger schema generation to keep the Player UI in sync.
- `EventChainRequest` passes through `crates/engine/src/api/websocket/ws_event_chain.rs`, returning `EventChainData` plus status info while allowing DM-only flow for chain manipulation (add/remove/complete/reset events, activate/favorite, etc.).
- Triggered events execute their outcome effects via `use_cases::narrative::ExecuteEffects` before broadcasting `ServerMessage::NarrativeEventTriggered`, so UI flows that call `NarrativeEventService` receive consistent state.

---

## Game Design

This system provides the scaffolding for emergent storytelling:

1. **Trigger Conditions**: Events fire when conditions are met (location, dialogue, challenge, time)
2. **Dual Detection**: Both Engine and LLM can suggest trigger opportunities
3. **DM Authority**: All triggers require DM approval before executing
4. **Branching Outcomes**: Events can have success/failure paths that chain to different events
5. **Event Effects**: Outcomes can give items, modify relationships, unlock locations, trigger scenes

---

## User Stories

### Implemented

- [x] **US-NAR-001**: As a DM, I can create narrative events with trigger conditions
  - *Implementation*: NarrativeEvent entity with NarrativeTrigger, TriggerLogic
  - *Files*: `crates/domain/src/aggregates/narrative_event.rs`

- [x] **US-NAR-002**: As a DM, I can define multiple outcomes with different effects
  - *Implementation*: EventOutcome with conditions and EventEffect list
  - *Files*: `crates/domain/src/aggregates/narrative_event.rs`

- [x] **US-NAR-003**: As a DM, I can chain events into sequences
  - *Implementation*: EventChain entity with CONTAINS_EVENT edges
  - *Files*: `crates/domain/src/entities/event_chain.rs`

- [x] **US-NAR-004**: As a DM, the Engine detects when trigger conditions are met
  - *Implementation*: Trigger evaluation runs in narrative use cases against current world state
  - *Files*: `crates/engine/src/use_cases/narrative/events.rs`

- [x] **US-NAR-005**: As a DM, the LLM can suggest triggering events during dialogue
  - *Implementation*: LLM outputs `<narrative_event_suggestion>` tags
  - *Files*: `crates/engine/src/use_cases/queues/mod.rs`

- [x] **US-NAR-006**: As a DM, I can approve/reject event triggers before they execute
  - *Implementation*: NarrativeEventSuggestionDecision WebSocket message
  - *Files*: `crates/engine/src/api/websocket/mod.rs`

- [x] **US-NAR-007**: As a DM, I can browse and manage a narrative event library
  - *Implementation*: NarrativeEventLibrary with search, filters, favorites
  - *Files*: `crates/player/src/ui/presentation/components/story_arc/narrative_event_library.rs`

- [x] **US-NAR-008**: As a DM, I can visualize event chains as flowcharts
  - *Implementation*: EventChainVisualizer component
  - *Files*: `crates/player/src/ui/presentation/components/story_arc/event_chain_visualizer.rs`

- [x] **US-NAR-009**: As a DM, I can use a visual builder for trigger conditions
  - *Implementation*: TriggerBuilder component with schema-driven form generation
  - *Files*: `crates/player/src/ui/presentation/components/story_arc/trigger_builder.rs`, `crates/protocol/src/types.rs` (TriggerSchema)

### Pending

- [x] **US-NAR-010**: SetFlag effect with flag storage system
  - *Status*: Implemented - Flag repository with Neo4j storage
  - *Files*: `crates/engine/src/repositories/flag.rs`, `crates/engine/src/infrastructure/neo4j/flag_repo.rs`

- [ ] **US-NAR-011**: StartCombat effect requires combat system
  - *Notes*: Effect type exists but execution is gated; DM sees a warning and no state changes occur until combat is implemented

- [ ] **US-NAR-012**: AddReward effect requires reward/XP system  
  - *Notes*: Effect type exists but execution is gated; DM sees a warning and no state changes occur until rewards are implemented

---

## UI Mockups

### Narrative Event Library

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Narrative Events                                                [+ Create] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  [🔍 Search...          ]  [Status: All ▼]  [★ Favorites]  [Active Only]   │
│                                                                             │
│  Active: 5   Triggered: 3   Pending: 8                                      │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ★ The Baron's Arrival              Priority: High    [Active]       │   │
│  │   "The Baron unexpectedly arrives at the tavern"                    │   │
│  │   Triggers: Enter Tavern, Talk to Bartender                         │   │
│  │   [Edit] [Trigger Now]                                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │   Market Day Festival              Priority: Medium   [Triggered]   │   │
│  │   "The annual festival brings crowds to the market"                 │   │
│  │   Triggered: Day 3, Morning                                         │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented

### Event Chain Visualizer

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  The Baron's Downfall                                               [Edit]  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│     ┌─────────────────┐                                                     │
│     │ Baron's Arrival │ ◀── START                                           │
│     └────────┬────────┘                                                     │
│              │                                                              │
│      ┌───────┴───────┐                                                      │
│      │               │                                                      │
│      ▼               ▼                                                      │
│  [Confronted]   [Ignored]                                                   │
│      │               │                                                      │
│      ▼               ▼                                                      │
│  ┌──────────┐   ┌──────────┐                                               │
│  │ Baron's  │   │ Baron's  │                                               │
│  │ Threat   │   │ Plot     │                                               │
│  └────┬─────┘   └────┬─────┘                                               │
│       │              │                                                      │
│       ▼              ▼                                                      │
│   [Success]      [Discovered]                                               │
│       │              │                                                      │
│       ▼              ▼                                                      │
│  ┌──────────┐   ┌──────────┐                                               │
│  │ Baron    │   │ Baron    │                                               │
│  │ Defeated │   │ Escapes  │                                               │
│  └──────────┘   └──────────┘                                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented (basic)

### Visual Trigger Condition Builder

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Event: The Baron's Arrival - Trigger Conditions                    [Save]  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Logic: [All must be true ▼]  (AND / OR / At least N)                       │
│                                                                             │
│  ─── Conditions ────────────────────────────────────────────────────────── │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 1. [Enter Region ▼]                                                  │   │
│  │    Region: [The Rusty Anchor - Bar Counter ▼]                        │   │
│  │    ☑ Required                                               [🗑️]     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 2. [Time of Day ▼]                                                   │   │
│  │    Time: [Evening ▼]                                                 │   │
│  │    ☐ Required                                               [🗑️]     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 3. [Talk to NPC ▼]                                                   │   │
│  │    NPC: [Marcus the Bartender ▼]                                     │   │
│  │    ☑ Required                                               [🗑️]     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  [+ Add Condition]                                                          │
│                                                                             │
│  ─── Available Trigger Types ───────────────────────────────────────────── │
│  Enter Location | Enter Region | Exit Location | Talk to NPC               │
│  Challenge Complete | Item Acquired | Item Used | Relationship Threshold   │
│  Time of Day | Game Day | Flag Set | Stat Threshold | Event Complete       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented (US-NAR-009)

---

## UI Design: DM Story Arc Enhancements (Event Chains)

### User Story
**US-NAR-013**: As a DM, I can assess, edit, and validate story arcs quickly so that narrative pacing and gameplay consequences stay coherent during play.

### User Flow
1. Initial state: DM opens Story Arc panel with chain list and quick status cues.
2. User action: DM selects a chain → opens editor and visualizer.
3. System response: Loads chain graph, computes readiness, blocked dependencies, and effects summary.
4. Final state: DM validates or updates the chain, then publishes or pauses it.

### Mockups

#### State 1: Event Chain List + Quick Status
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Story Arcs (Event Chains)                                      [+ Create]  │
├─────────────────────────────────────────────────────────────────────────────┤
│  [🔍 Search chains...        ]  [Status: All ▼]  [Act: Any ▼]  [★ Favorites] │
│  Sort: [Progress ▼]  View: [Compact ▼]  [Validate Selected]                  │
│                                                                             │
│  ─── Active & At Risk ────────────────────────────────────────────────────  │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ★ The Baron's Downfall           3/8  37%      Next: "Baron's Threat"│   │
│  │    Ready: 2  Blocked: 1  Pending: 1   Risk: High  Act II             │   │
│  │    Triggers: Enter Tavern, Talk to Bartender                          │   │
│  │    Effects: +Rep, Unlock Location                                     │   │
│  │    [Open] [Pause] [Preview] [⋯]                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ─── Stable ──────────────────────────────────────────────────────────────  │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Market Day Festival              6/6  100%     Completed  Day 3 AM   │   │
│  │    Ready: 0  Blocked: 0  Pending: 0   Risk: Low   Act I              │   │
│  │    [Open] [Archive]                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM needs a high-signal overview to choose where to intervene.
**Interactions**:
- [Open] → opens Editor + Visualizer split view.
- [Validate Selected] → runs readiness/consistency scan and reports issues.
- [Preview] → opens read-only runtime projection (what’s next, likely outcomes).

#### State 2: Event Chain Editor (Metadata + Event Roster)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Chain Editor: The Baron's Downfall                           [Save] [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│  Name: [The Baron's Downfall                ]   Act: [Act II ▼]             │
│  Tags: [political] [revenge] [court]  [+ Add]  Color: [#FF5733]             │
│  Status: (●) Active  ( ) Paused  ( ) Archived  Auto-advance: [✓]            │
│                                                                             │
│  ─── Event Roster ────────────────────────────────────────────────────────  │
│  ┌────┬─────────────────────────┬──────────────┬────────────┬───────────┐   │
│  │ #  │ Event                    │ Triggers     │ Outcomes   │ Status    │   │
│  ├────┼─────────────────────────┼──────────────┼────────────┼───────────┤   │
│  │ 1  │ Baron's Arrival          │ 2 ready      │ 2 branches │ Complete  │   │
│  │ 2  │ Baron's Threat           │ 1 blocked    │ 1 branch   │ Current   │   │
│  │ 3  │ Baron's Plot             │ 0 ready      │ 1 branch   │ Pending   │   │
│  └────┴─────────────────────────┴──────────────┴────────────┴───────────┘   │
│  [Add Event] [Auto-Link] [Reorder] [Validate] [Preview Effects]             │
│                                                                             │
│  ─── Validation ──────────────────────────────────────────────────────────  │
│  ⚠ Baron's Threat: "Talk to Marcus" trigger missing NPC link               │
│  ⚠ Baron's Plot: Outcome references missing location                        │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM manages sequencing, status, and content consistency.
**Interactions**:
- [Add Event] → opens event picker and inserts into roster.
- [Auto-Link] → suggests chain edges based on outcomes and triggers.
- [Validate] → runs consistency checks and surfaces actionable warnings.

#### State 3: Event Chain Visualizer + Inspector
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Visualizer: The Baron's Downfall         [Fit] [Zoom +] [Zoom -] [Layout▼] │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ START                                                               │   │
│  │   ┌──────────────────┐                                               │   │
│  │   │ Baron's Arrival  │  ✅ Completed  Triggers: 2/2 ready            │   │
│  │   └─────────┬────────┘                                               │   │
│  │             │                                                        │   │
│  │   ┌─────────▼─────────┐      ┌───────────────┐                        │   │
│  │   │ Baron's Threat    │      │ Baron's Plot  │                        │   │
│  │   │ 📍 Current        │      │ 🔒 Blocked    │                        │   │
│  │   │ Triggers: 1/2     │      │ Missing NPC   │                        │   │
│  │   └─────────┬─────────┘      └───────────────┘                        │   │
│  │             │                                                        │   │
│  │        [Success]                                                     │   │
│  │             ▼                                                        │   │
│  │   ┌──────────────────┐                                               │   │
│  │   │ Baron Defeated   │  Pending  Effects: +Rep, Unlock Location      │   │
│  │   └──────────────────┘                                               │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ─── Inspector ───────────────────────────────────────────────────────────  │
│  Event: Baron's Threat                                                     │
│  Triggers: Talk to Marcus (missing link), Enter Tavern (ready)             │
│  Outcomes: Success → Baron Defeated                                        │
│  Effects: ModifyRelationship +0.3, UnlockLocation: Courtyard               │
│  [Open Event] [Fix Link] [Simulate Outcome]                                │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM sees narrative structure, blockers, and consequences.
**Interactions**:
- Click node → loads Inspector with triggers, effects, and links.
- [Fix Link] → jumps to missing entity picker (NPC/location).
- [Simulate Outcome] → preview effect list and expected world impact.

#### State 4: Loading / Error
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Loading chain data...                                                     │
│  [spinner]  Fetching triggers, outcomes, and dependencies                   │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│  ⚠ Unable to load chain                                                   │
│  Reason: missing narrative event data                                      │
│  [Retry] [Open Logs]                                                       │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Network or data integrity issues.
**Interactions**:
- [Retry] → refetches chain data.
- [Open Logs] → shows error details for debugging.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| EventChainList | Overview list with status and progress | Yes (enhance) |
| EventChainCard | Per-chain summary with quick actions | Yes (enhance) |
| ChainStatusSummary | Ready/Blocked/Pending counters | New |
| ChainRiskBadge | Highlights pacing or dependency risk | New |
| EventChainEditor | Chain metadata and roster manager | Yes (enhance) |
| EventRosterTable | Tabular roster with status + reorder | New |
| ValidationBanner | Displays readiness/errors in context | New |
| EventChainVisualizer | Graph view with state badges | Yes (enhance) |
| EventInspectorPanel | Details for selected node | New |
| EffectSummaryChip | Quick effect visibility per node | New |
| DependencyLinker | Quick fix for missing NPC/location | New |

### Accessibility Considerations
- Keyboard navigation across list rows, toolbar buttons, and graph nodes.
- High-contrast status chips; non-color cues (icons + labels) for Ready/Blocked.
- Inspector actions reachable via tab order and shortcut hints (e.g., `E` open event).

### Animations
- Graph layout transition: smooth reflow when auto-linking or reordering.
- Node highlight pulse on selection and when a blocked dependency resolves.
- Progress bar easing on completion updates.

---

## Data Model

### Neo4j Nodes

```cypher
(:NarrativeEvent {
    id: "uuid",
    name: "The Baron's Arrival",
    description: "The Baron unexpectedly arrives at the tavern",
    scene_direction: "The door swings open, and silence falls...",
    suggested_opening: "Well, well... what have we here?",
    is_active: true,
    is_triggered: false,
    is_repeatable: false,
    trigger_count: 0,
    delay_turns: 0,
    priority: 10,
    is_favorite: true,
    created_at: datetime()
})

(:EventChain {
    id: "uuid",
    name: "The Baron's Downfall",
    description: "Events leading to the Baron's defeat",
    is_active: true,
    current_position: 0,
    color: "#FF5733",
    is_favorite: true
})
```

### Neo4j Edges

```cypher
// Event tied to location/region/scene
(event:NarrativeEvent)-[:TIED_TO_LOCATION]->(location:Location)
(event:NarrativeEvent)-[:TIED_TO_REGION]->(region:Region)
(event:NarrativeEvent)-[:TIED_TO_SCENE]->(scene:Scene)
(event:NarrativeEvent)-[:BELONGS_TO_ACT]->(act:Act)

// Story events (including dialogue turns) anchor to conversation, scene, and time
(story:StoryEvent)-[:PART_OF_CONVERSATION]->(conversation:Conversation)
(story:StoryEvent)-[:OCCURRED_IN_SCENE]->(scene:Scene)
(story:StoryEvent)-[:OCCURRED_AT]->(time:GameTime)

// Event features NPCs
(event:NarrativeEvent)-[:FEATURES_NPC {
    role: "Primary"
}]->(character:Character)

// Event chain membership
(chain:EventChain)-[:CONTAINS_EVENT {
    position: 1,
    is_completed: false
}]->(event:NarrativeEvent)

// Event chaining
(event:NarrativeEvent)-[:CHAINS_TO {
    delay_turns: 2,
    chain_reason: "Baron retaliates after being exposed"
}]->(next:NarrativeEvent)

// Trigger conditions
(event:NarrativeEvent)-[:TRIGGERED_BY_ENTERING_LOCATION]->(location:Location)
(event:NarrativeEvent)-[:TRIGGERED_BY_ENTERING_REGION]->(region:Region)
(event:NarrativeEvent)-[:TRIGGERED_BY_TALKING_TO]->(character:Character)
(event:NarrativeEvent)-[:TRIGGERED_BY_CHALLENGE_COMPLETE {success_required: true}]->(challenge:Challenge)

// Effect edges
(event:NarrativeEvent)-[:EFFECT_GIVES_ITEM {outcome: "success", quantity: 1}]->(item:Item)
(event:NarrativeEvent)-[:EFFECT_MODIFIES_RELATIONSHIP {sentiment_change: 0.3}]->(character:Character)
```

### Trigger Types

```rust
pub enum NarrativeTriggerType {
    EnterLocation { location_id: String },
    EnterRegion { region_id: String },
    ExitLocation { location_id: String },
    TalkToNpc { npc_id: String },
    ChallengeComplete { challenge_id: String, success_required: bool },
    ItemAcquired { item_id: String },
    ItemUsed { item_id: String },
    RelationshipThreshold { npc_id: String, min_sentiment: f32 },
    TimeOfDay { time: TimeOfDay },
    GameDay { day: u32 },
    FlagSet { flag_name: String, value: bool },
    StatThreshold { stat_name: String, min_value: i32 },
    EventComplete { event_id: String, outcome_required: Option<String> },
    Custom { condition: String },
}

pub enum TriggerLogic {
    All,          // AND - all triggers must be true
    Any,          // OR - any trigger can be true
    AtLeast(u32), // N of M triggers must be true
}
```

### Effect Types

```rust
pub enum EventEffect {
    GiveItem { item_id: String, quantity: u32 },
    TakeItem { item_id: String, quantity: u32 },
    ModifyRelationship { npc_id: String, sentiment_change: f32 },
    SetFlag { flag_name: String, value: bool },
    ModifyStat { stat_name: String, amount: i32 },
    UnlockLocation { location_id: String },
    EnableChallenge { challenge_id: String },
    DisableChallenge { challenge_id: String },
    TriggerScene { scene_id: String },
    PlayDialogue { dialogue_id: String },
    SpawnNpc { npc_id: String, location_id: String },
    Custom { action: String },
}
```

---

## API

### REST Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | `/api/worlds/{id}/narrative-events` | List events | ✅ |
| POST | `/api/worlds/{id}/narrative-events` | Create event | ✅ |
| GET | `/api/narrative-events/{id}` | Get event | ✅ |
| PUT | `/api/narrative-events/{id}` | Update event | ✅ |
| DELETE | `/api/narrative-events/{id}` | Delete event | ✅ |
| PUT | `/api/narrative-events/{id}/active` | Toggle active | ✅ |
| PUT | `/api/narrative-events/{id}/favorite` | Toggle favorite | ✅ |
| POST | `/api/narrative-events/{id}/trigger` | Manual trigger | ✅ |
| GET | `/api/worlds/{id}/event-chains` | List chains | ✅ |
| POST | `/api/worlds/{id}/event-chains` | Create chain | ✅ |
| POST | `/api/event-chains/{id}/events` | Add to chain | ✅ |

### WebSocket Messages

#### Client → Server

| Message | Fields | Purpose |
|---------|--------|---------|
| `NarrativeEventSuggestionDecision` | `event_id`, `approved`, `selected_outcome` | DM approves trigger |

#### Server → Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `NarrativeEventTriggered` | `event_id`, `name`, `description`, `effects` | Event fired |

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| NarrativeEvent Entity | ✅ | ✅ | Full trigger/outcome support |
| EventChain Entity | ✅ | ✅ | Sequencing, branching |
| NarrativeEvent Repository | ✅ | - | Neo4j with all edges |
| TriggerEvaluationService | ✅ | - | Evaluate against state |
| EventEffectExecutor | ⏳ | - | StartCombat/AddReward are stubbed |
| WebSocket Request Handlers | ⏳ | - | NarrativeEvent/EventChain requests not wired |
| LLM Event Suggestions | ✅ | - | Parse XML tags |
| Event Library UI | - | ✅ | Search, filter, favorites |
| Event Chain Editor | - | ✅ | Add/remove events |
| Event Chain Visualizer | - | ✅ | Flowchart view |
| Pending Events Widget | - | ✅ | Director sidebar |
| Visual Trigger Builder | ✅ | ✅ | Schema endpoint + TriggerBuilder component |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/aggregates/narrative_event.rs` | Event entity, EventEffect enum |
| Domain | `crates/domain/src/entities/event_chain.rs` | Chain entity |
| Use Case | `crates/engine/src/use_cases/narrative/events.rs` | Narrative operations, trigger checks |
| Use Case | `crates/engine/src/use_cases/narrative/execute_effects.rs` | Execute all effect types |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/narrative_repo.rs` | Neo4j repo |
| Infrastructure | `crates/engine/src/infrastructure/ports.rs` | NarrativeRepo trait |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Application | `crates/player/src/application/services/narrative_event_service.rs` | API calls |
| Presentation | `crates/player/src/ui/presentation/components/story_arc/narrative_event_library.rs` | Library |
| Presentation | `crates/player/src/ui/presentation/components/story_arc/event_chain_editor.rs` | Editor |
| Presentation | `crates/player/src/ui/presentation/components/story_arc/event_chain_visualizer.rs` | Visualizer |
| Presentation | `crates/player/src/ui/presentation/components/story_arc/pending_events_widget.rs` | Widget |

---

## Related Systems

- **Depends on**: [Navigation System](./navigation-system.md) (location triggers), [Challenge System](./challenge-system.md) (challenge triggers), [Character System](./character-system.md) (NPC features)
- **Used by**: [Dialogue System](./dialogue-system.md) (LLM suggestions), [Scene System](./scene-system.md) (scene triggers)

---

## Revision History

| Date | Change |
|------|--------|
| 2026-01-05 | US-NAR-009 complete - Visual Trigger Condition Builder |
| 2025-12-18 | Initial version extracted from MVP.md |
