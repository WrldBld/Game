# Staging System

## Overview

The Staging System manages **which NPCs are present in a region** at any given time, combining rule-based logic with LLM reasoning and requiring DM approval. Unlike simple presence calculation, staging provides a complete workflow where the DM reviews and approves NPC presence before players see them, with results cached based on configurable TTL. The term "staging" comes from theatre, representing "who is on stage" in a scene.

---

## Game Design

The Staging System creates a living, coherent world while maintaining DM control over narrative pacing:

1. **DM-Approved Presence**: Every NPC appearance goes through DM approval, ensuring narrative consistency
2. **Dual Decision Modes**: Rule-based (deterministic) and LLM-enhanced (contextual) options give DMs flexibility
3. **Pre-Staging**: DMs can set up regions before players arrive for seamless gameplay
4. **Smart Caching**: Approved stagings persist with configurable TTL to reduce repetitive approvals
5. **Story-Aware LLM**: The LLM considers active narrative events and recent dialogues when suggesting presence
6. **Background Workflow**: Players see a brief loading state while DM approves, minimizing interruption

### Theatre Language

WrldBldr uses theatre and story terminology throughout:
- **Staging**: The configuration of NPCs present in a region (who's "on stage")
- **Pre-staging**: Setting up a scene before the audience (players) arrives
- **Scene**: The visual novel presentation (backdrop, sprites, dialogue)

---

## User Stories

### Implemented (Engine Complete, UI Partial)

- [x] **US-STG-001**: As a player, I see NPCs appear after entering a region when the DM approves
  - *Implementation*: Background approval workflow with StagingPending → StagingReady messages
  - *Files*: `crates/engine-adapters/src/infrastructure/websocket.rs`, `crates/player-ui/src/presentation/views/pc_view.rs`

- [x] **US-STG-002**: As a DM, I see a staging approval popup when a player enters an unstaged region
  - *Implementation*: StagingApprovalRequired message triggers popup with rule/LLM options
  - *Files*: `crates/player-ui/src/presentation/components/dm_panel/staging_approval.rs`

- [x] **US-STG-003**: As a DM, I can choose between rule-based and LLM-based NPC suggestions
  - *Implementation*: Both options shown side-by-side with reasoning
  - *Files*: `crates/engine-app/src/application/services/staging_service.rs`

- [x] **US-STG-004**: As a DM, I can customize which NPCs are present by toggling checkboxes
  - *Implementation*: Manual override of any suggestion before approval
  - *Files*: `crates/player-ui/src/presentation/components/dm_panel/staging_approval.rs`

- [x] **US-STG-005**: As a DM, I can regenerate LLM suggestions with additional guidance
  - *Implementation*: Text field for DM guidance, re-query LLM with context
  - *Files*: `crates/engine-app/src/application/services/staging_service.rs`

- [x] **US-STG-006**: As a DM, I can use the previous staging if it's still relevant
  - *Implementation*: Previous staging shown with "Use Previous" button
  - *Files*: `crates/player-ui/src/presentation/components/dm_panel/staging_approval.rs`

- [x] **US-STG-007**: As a DM, I can pre-stage regions before players arrive
  - *Implementation*: Dedicated pre-staging UI in location view
  - *Files*: `crates/player-ui/src/presentation/components/dm_panel/location_staging.rs`

- [x] **US-STG-008**: As a DM, I can view and manage stagings for all regions in a location
  - *Implementation*: Location staging tab showing all regions with status
  - *Files*: `crates/player-ui/src/presentation/components/dm_panel/location_staging.rs`

- [x] **US-STG-009**: As a DM, I can configure default staging TTL per location
  - *Implementation*: Location settings with `presence_cache_ttl_hours` field
  - *Files*: `crates/player-app/src/application/services/location_service.rs` (LocationFormData)

- [x] **US-STG-010**: As a DM, I can set the cache duration when approving a staging
  - *Implementation*: TTL dropdown in approval popup
  - *Files*: `crates/player-ui/src/presentation/components/dm_panel/staging_approval.rs`

- [x] **US-STG-011**: As a DM, I can view staging history for a region
  - *Implementation*: History list in pre-staging UI (via StagingRepository.get_history)
  - *Files*: `crates/engine-adapters/src/infrastructure/persistence/staging_repository.rs`

- [x] **US-STG-012**: As a player, I see a loading indicator while staging is pending
  - *Implementation*: Dimmed backdrop with "Setting the scene..." overlay
  - *Files*: `crates/player-ui/src/presentation/views/pc_view.rs`

- [x] **US-STG-013**: As a DM, I can stage NPCs as present but hidden from players
  - *Implementation*: Hidden NPCs do not appear in player presence payloads (`SceneChanged`, `StagingReady`)
  - *Implementation*: Hidden NPCs can still interact via DM-triggered approach events
  - *Completed 2025-12-25*:
    - Added `is_hidden_from_players` per NPC in staging (per region per staging entry)
    - Persisted flag on `INCLUDES_NPC` edge
    - Filtered hidden NPCs out of player-facing `npcs_present`
    - DM UI shows hidden state and allows toggling
  - *Key files*:
    - `crates/domain/src/entities/staging.rs`
    - `crates/engine-adapters/src/infrastructure/persistence/staging_repository.rs`
    - `crates/engine-adapters/src/infrastructure/websocket.rs`
    - `crates/player-ui/src/presentation/components/dm_panel/staging_approval.rs`

---

## UI Mockups

### DM Staging Approval Popup

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  🎭 Stage the Scene                                              [X]        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  📍 The Bar Counter                                                         │
│     Rusty Anchor Tavern                                                     │
│  🕐 Day 3, Evening (7:30 PM)                                                │
│                                                                             │
│  👤 Waiting: Aldric the Ranger                                              │
│                                                                             │
│  ─── Previous Staging ────────────────────────────────────────────────────  │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ⏱️ Approved 4.5 hours ago (Day 3, Afternoon 3:00 PM)                │   │
│  │ 📋 Marcus ✓, Old Sal ✓, Mysterious Stranger ✗                       │   │
│  │                                              [Use Previous]          │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ─── Choose Staging Method ───────────────────────────────────────────────  │
│                                                                             │
│  ┌─────────────────────────────────┐ ┌─────────────────────────────────┐   │
│  │ 📋 RULES-BASED                  │ │ 🤖 LLM-ENHANCED                 │   │
│  │                                 │ │                                 │   │
│  │ [✓] Marcus the Bartender        │ │ [✓] Marcus the Bartender        │   │
│  │     Works here (Evening)        │ │     "Working his usual shift"   │   │
│  │                                 │ │                                 │   │
│  │ [✓] Old Sal                     │ │ [✓] Old Sal                     │   │
│  │     Frequents (Often, Evening)  │ │     "A regular, here as always" │   │
│  │                                 │ │                                 │   │
│  │ [ ] Mysterious Stranger         │ │ [ ] Mysterious Stranger         │   │
│  │     Frequents (Sometimes) - 40% │ │     "Said he'd be at the docks" │   │
│  │     ↳ Rolled: Not present       │ │                                 │   │
│  │                                 │ │                                 │   │
│  │         [Use Rules]             │ │         [Use LLM]               │   │
│  └─────────────────────────────────┘ └─────────────────────────────────┘   │
│                                                                             │
│  ─── Or Customize ────────────────────────────────────────────────────────  │
│                                                                             │
│  Toggle NPCs manually:                                                      │
│  [✓] Marcus    [✓] Old Sal    [ ] Mysterious Stranger                       │
│                                                                             │
│  ─── Cache Duration ──────────────────────────────────────────────────────  │
│  Valid for: [▼ 3 hours ] (until 10:30 PM game time)                         │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                        ✓ Approve Staging                              │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ─── Regenerate with Guidance ────────────────────────────────────────────  │
│  [Consider that the party just had a loud fight outside...              ]   │
│  [🔄 Regenerate LLM]                                                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ⏳ Pending

### Pre-Staging UI (Location View)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  📍 The Rusty Anchor Tavern                                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  [Overview] [Regions] [NPCs] [🎭 Staging] [Settings]                        │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Pre-stage regions before players arrive.                                   │
│  Current game time: Day 3, Evening (7:30 PM)                                │
│                                                                             │
│  ─── Regions ─────────────────────────────────────────────────────────────  │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 🎭 Bar Counter                                                       │   │
│  │                                                                      │   │
│  │ Current Staging: ✓ Active (expires in 2.5 hours)                     │   │
│  │ NPCs: Marcus ✓, Old Sal ✓                                            │   │
│  │                                                                      │   │
│  │ [View/Edit Staging]                   [Clear Staging]                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 🎭 Tables                                                            │   │
│  │                                                                      │   │
│  │ Current Staging: ⚠️ None (will prompt on player entry)               │   │
│  │                                                                      │   │
│  │ [Pre-Stage Now]                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 🔒 Back Room                                                         │   │
│  │                                                                      │   │
│  │ Current Staging: ⏸️ Expired (was set 6 hours ago)                    │   │
│  │ Previous: Shady Dealer ✓                                             │   │
│  │                                                                      │   │
│  │ [Refresh Staging]                     [View Previous]                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ⏳ Pending

### Pre-Staging Editor Modal

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  🎭 Pre-Stage: Tables                                            [X]        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Set up NPCs before players arrive at this region.                          │
│  Current game time: Day 3, Evening (7:30 PM)                                │
│                                                                             │
│  ─── Quick Options ───────────────────────────────────────────────────────  │
│                                                                             │
│  [📋 Generate from Rules]    [🤖 Generate with LLM]    [📝 Manual Setup]   │
│                                                                             │
│  ─── NPCs Who Could Be Here ──────────────────────────────────────────────  │
│                                                                             │
│  Workers:                                                                   │
│  [ ] Serving Wench Mira (Evening shift)                                     │
│                                                                             │
│  Regulars:                                                                  │
│  [✓] Drunk Sailor Pete (Frequents Often)                                    │
│  [✓] Card Shark Vince (Frequents Sometimes)                                 │
│  [ ] Nervous Merchant (Frequents Rarely)                                    │
│                                                                             │
│  Residents:                                                                 │
│  [ ] (none)                                                                 │
│                                                                             │
│  ─── Duration ────────────────────────────────────────────────────────────  │
│  Valid for: [▼ 3 hours ] (until 10:30 PM game time)                         │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                        💾 Save Pre-Staging                            │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ⏳ Pending

### Location Settings (TTL Configuration)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  📍 The Rusty Anchor Tavern - Settings                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ─── Basic Info ──────────────────────────────────────────────────────────  │
│  Name: [The Rusty Anchor Tavern                                 ]           │
│  Type: [▼ Interior    ]                                                     │
│                                                                             │
│  ─── Staging Settings ────────────────────────────────────────────────────  │
│                                                                             │
│  Default staging duration: [▼ 3 hours    ]                                  │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ℹ️  How long NPC presence approvals remain valid (in game time).    │   │
│  │                                                                      │   │
│  │    Quick presets:                                                    │   │
│  │    • Busy venue (tavern, market): 1-2 hours                         │   │
│  │    • Calm location (shop, home): 3-4 hours                          │   │
│  │    • Static location (dungeon, ruins): 8-24 hours                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  [✓] Use LLM for staging decisions                                          │
│      When enabled, an AI considers story context to suggest NPC presence.   │
│      When disabled, only rule-based logic is used.                          │
│                                                                             │
│  ┌────────────────────┐                                                     │
│  │ 💾 Save Settings   │                                                     │
│  └────────────────────┘                                                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ⏳ Pending

### Player Loading State

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                      │   │
│  │                      [BACKDROP IMAGE - dimmed]                       │   │
│  │                    The Rusty Anchor Tavern                           │   │
│  │                                                                      │   │
│  │                                                                      │   │
│  │                      ┌─────────────────────┐                         │   │
│  │                      │  🎭                  │                         │   │
│  │                      │  Setting the scene...│                         │   │
│  │                      │  [spinner]           │                         │   │
│  │                      └─────────────────────┘                         │   │
│  │                                                                      │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      [Dialogue box - empty]                          │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ⏳ Pending

---

## Data Model

### Neo4j Nodes

```cypher
// Staging - a persisted approval of NPC presence for a region
(:Staging {
    id: "uuid",
    region_id: "uuid",
    location_id: "uuid",
    world_id: "uuid",
    game_time: datetime,        // Game time when approved
    approved_at: datetime,      // Real time when approved
    ttl_hours: 3,               // How long valid in game hours
    approved_by: "client_id",   // Who approved
    source: "llm",              // "rule" | "llm" | "custom" | "prestaged"
    dm_guidance: null,          // Optional guidance text for regeneration
    is_active: true             // Current active staging for region
})

// Location enhancement for staging settings
(:Location {
    // ... existing fields ...
    presence_cache_ttl_hours: 3,  // Default TTL for this location
    use_llm_presence: true        // Whether to use LLM suggestions
})
```

### Neo4j Edges

```cypher
// Staging includes NPCs with presence status
(staging:Staging)-[:INCLUDES_NPC {
    is_present: true,
    reasoning: "Works here during evening shift"
}]->(character:Character)

// Quick lookup: current staging for a region
(region:Region)-[:CURRENT_STAGING]->(staging:Staging)

// History: all stagings for a region
(region:Region)-[:HAS_STAGING]->(staging:Staging)
```

### Staging Context (LLM Input)

The LLM receives context to make informed decisions:

```rust
pub struct StagingContext {
    // Region information
    pub region_name: String,
    pub region_description: String,
    pub location_name: String,
    pub time_of_day: TimeOfDay,
    pub time_display: String,
    
    // Story context
    pub active_events: Vec<ActiveEventContext>,
    pub npc_dialogues: Vec<NpcDialogueContext>,
    
    // Extensible
    pub additional_context: HashMap<String, String>,
}
```

---

## API

### REST Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | `/api/regions/{id}/staging` | Get current staging | ⏳ |
| GET | `/api/regions/{id}/staging/history` | Get staging history | ⏳ |
| POST | `/api/regions/{id}/staging` | Create/approve staging | ⏳ |
| DELETE | `/api/regions/{id}/staging` | Clear current staging | ⏳ |
| PUT | `/api/locations/{id}` | Update location (incl. TTL settings) | ✅ (needs fields) |

### WebSocket Messages

#### Client → Server (DM only)

| Message | Fields | Purpose |
|---------|--------|---------|
| `StagingApprovalResponse` | `request_id`, `approved_npcs`, `ttl_hours`, `source` | DM approves staging |
| `StagingRegenerateRequest` | `request_id`, `guidance` | DM requests new LLM suggestions |
| `PreStageRegion` | `region_id`, `npcs`, `ttl_hours` | DM pre-stages before player arrives |

#### Server → Client (DM)

| Message | Fields | Purpose |
|---------|--------|---------|
| `StagingApprovalRequired` | `request_id`, `region_id`, `region_name`, `location_name`, `game_time_display`, `previous_staging`, `rule_based_npcs`, `llm_based_npcs`, `default_ttl_hours`, `waiting_pcs` | Player entered unstaged region |

#### Server → Client (Player)

| Message | Fields | Purpose |
|---------|--------|---------|
| `StagingPending` | `region_id` | Staging approval in progress |
| `StagingReady` | `region_id`, `npcs_present` | Staging approved, show NPCs |

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| Staging Entity | ✅ | - | `entities/staging.rs` with hidden NPC support |
| StagingContext VO | ✅ | - | `value_objects/staging_context.rs` |
| Location TTL fields | ✅ | - | Added to Location entity |
| StagingRepository | ✅ | - | Neo4j persistence (CURRENT_STAGING, HAS_STAGING edges) |
| StagingService | ✅ | - | Core logic + LLM with configurable prompts |
| StagingContextProvider | ✅ | - | Builds context for LLM queries |
| PromptBuilder | ✅ | - | Uses PromptTemplateService |
| StagingRepositoryPort | ✅ | - | Port trait defined |
| Protocol Messages | ⏳ | ⏳ | Partial - needs staging-specific messages |
| WebSocket Integration | ⏳ | - | Approval workflow pending |
| Staging State | - | ⏳ | `game_state.rs` |
| Message Handlers | - | ⏳ | Handle staging messages |
| StagingApproval Component | - | ✅ | DM approval popup exists |
| LocationStaging Component | - | ⏳ | Pre-staging UI pending |
| StagingPending Overlay | - | ⏳ | Player loading state pending |
| Location Settings UI | - | ⏳ | TTL configuration pending |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/entities/staging.rs` | Staging entity |
| Domain | `crates/domain/src/value_objects/staging_context.rs` | LLM context types |
| Application | `crates/engine-app/src/application/services/staging_service.rs` | Core staging logic |
| Application | `crates/engine-app/src/application/services/staging_context_provider.rs` | Context provider |
| Application | `crates/engine-ports/src/outbound/staging_repository_port.rs` | Repository trait |
| Infrastructure | `crates/engine-adapters/src/infrastructure/persistence/staging_repository.rs` | Neo4j implementation |
| Infrastructure | `crates/engine-adapters/src/infrastructure/websocket.rs` | Staging message handlers |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Protocol | `crates/protocol/src/messages.rs` | Staging DTOs/messages |
| Presentation | `crates/player-ui/src/presentation/state/game_state.rs` | Staging state signals |
| Presentation | `crates/player-ui/src/presentation/handlers/session_message_handler.rs` | Handle staging messages |
| Presentation | `crates/player-ui/src/presentation/components/dm_panel/staging_approval.rs` | Approval popup |
| Presentation | `crates/player-ui/src/presentation/components/dm_panel/location_staging.rs` | Pre-staging UI |
| Presentation | `crates/player-ui/src/presentation/views/pc_view.rs` | StagingPending overlay |
| Presentation | `crates/player-ui/src/presentation/components/creator/location_form.rs` | TTL settings |

---

## Related Systems

- **Depends on**: [NPC System](./npc-system.md) (NPC-Region relationships), [Navigation System](./navigation-system.md) (region movement), [Dialogue System](./dialogue-system.md) (conversation history for LLM context), [Narrative System](./narrative-system.md) (active events for LLM context), [Prompt Template System](./prompt-template-system.md) (configurable staging prompts)
- **Replaces**: PresenceService (simple rule-based presence calculation)
- **Used by**: [Scene System](./scene-system.md) (NPCs in scene)

---

## LLM Prompt Structure

The LLM receives a structured prompt combining rules and context:

```
You are helping determine which NPCs are present in a location for a TTRPG game.

## Location
The Bar Counter (Rusty Anchor Tavern)
A worn wooden counter with brass fittings. The barkeep polishes glasses.
Time: Evening (7:30 PM)

## Rule-Based Suggestions
The game rules suggest:
- Marcus the Bartender: PRESENT (Works here, Evening shift)
- Old Sal: PRESENT (Frequents here Often, Evening)
- Mysterious Stranger: ABSENT (Frequents Sometimes - 40% chance, rolled absent)

## Your Role
You may AGREE with or OVERRIDE the rules based on narrative considerations.
Consider: story reasons, interesting opportunities, conflicts, current context.

## Active Story Elements
- "The Festival Begins" event is active at this location
  Relevance: The tavern is busier than usual

## Recent NPC Interactions
- Mysterious Stranger: Last spoke to party 2 hours ago
  Summary: "Told the party he would meet them at the docks at sunset"
  Mentioned locations: ["The Docks"]

## DM Guidance (if provided)
"Consider that the party just had a loud fight outside"

## Response Format
[
  {
    "name": "Marcus the Bartender",
    "is_present": true,
    "reasoning": "Agree with rules - Marcus is working his shift"
  },
  {
    "name": "Mysterious Stranger",
    "is_present": false,
    "reasoning": "Override rules - He told the party he'd be at the docks"
  }
]
```

---

## Revision History

| Date | Change |
|------|--------|
| 2025-12-26 | Marked US-STG-013 (hidden NPCs) as complete |
| 2025-12-24 | Updated status: Engine complete, UI partial |
| 2025-12-19 | Initial version - Phase 3 planning |
