## UI Design: DM Director View

User Stories: [US-DLG-002](../../systems/dialogue-system.md), [US-DLG-020](../../systems/dialogue-system.md), [US-DLG-021](../../systems/dialogue-system.md), [US-STG-002](../../systems/staging-system.md), [US-STG-008](../../systems/staging-system.md), [US-VS-007](../../systems/visual-state-system.md)

### User Story
**DM-DIR-001**: As a DM, I can monitor live gameplay, review AI outputs, and intervene quickly so that the scene stays coherent.

### User Flow
1. Initial state: DM opens Director view with live panels.
2. User action: DM reviews approvals, staging, or conversations.
3. System response: Pending items expand to detail panels and action buttons.
4. Final state: Approved changes push to players; queues clear.

### Mockups

#### State 1: Director Dashboard (Initial)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ WrldBldr  Director  Creator  Story Arc  Settings         [Connected]        │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌────────── Active Stages Selector ─────────┐  ┌─ Conversation Manager ───┐ │
│  │ Stage Set: Market Square [▼]              │  │ Active: 2                 │ │
│  │ Region Focus: Plaza [▼]   TTL: 4h         │  │ ┌──────────────────────┐ │ │
│  │ [Switch] [Pin] [New Stage]                │  │ │ Arden <-> Tessa        │ │ │
│  └───────────────────────────────────────────┘  │ │ Last: 2m ago [Open]    │ │ │
│                                                 │ ├──────────────────────┤ │ │
│  ┌──────────── Staging Overview ──────────────┐ │ │ Mira <-> Brother Kesh  │ │ │
│  │ Location: Redbridge [▼]  Region: Plaza     │ │ │ Last: 6m ago [Open]    │ │ │
│  │ ┌────────────────────────────────────────┐ │ │ └──────────────────────┘ │ │
│  │ │ NPC: Tessa (Visible)      TTL 4h       │ │ └─────────────────────────┘ │
│  │ │ NPC: Guard Rook (Hidden)  TTL 4h       │ │                             │
│  │ │ NPC: Merchant Kole        TTL 4h       │ │  ┌──────── NPC Tools ─────┐ │
│  │ └────────────────────────────────────────┘ │  │ [Summon NPC] [Reveal]   │ │
│  │ [Pre-Stage] [Edit] [History] [Expire Now]  │  │ [Hide NPC]   [Move]    │ │
│  └────────────────────────────────────────────┘  │ [Give Item]  [Notify]  │ │
│                                                 └─────────────────────────┘ │
│  ┌──────────── Scene Control ────────────────┐  ┌────── Player Status ─────┐ │
│  │ Current Scene: Market Square (Dusk)       │  │ Arden: In Dialogue       │ │
│  │ Backdrop: market_dusk_03.png [Preview]    │  │ Mira: Exploring          │ │
│  │ Visual State: Dusty Evening [Override]    │  └─────────────────────────┘ │
│  │ [Advance Time] [Trigger Event] [End Scene]│                             │
│  └───────────────────────────────────────────┘  ┌──── Directorial Notes ───┐ │
│                                                 │ [Input notes...          ]│ │
│                                                 │ [Save Notes] [Clear]      │ │
│                                                 └──────────────────────────┘ │
│                                                 ┌──────── Approval Queue ───┐ │
│                                                 │ Pending: 2  Staging: 1    │ │
│                                                 │ Dialogue: 1               │ │
│                                                 │ ┌──────────────────────┐  │ │
│                                                 │ │ 01  Dialogue: "Tessa" │  │ │
│                                                 │ │     NPC: Tessa [Review]│  │ │
│                                                 │ ├──────────────────────┤  │ │
│                                                 │ │ 02  Staging: Plaza     │  │ │
│                                                 │ │     6 NPCs  TTL: 6h    │  │ │
│                                                 │ └──────────────────────┘  │ │
│                                                 └──────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Default landing view for DM (DMViewRoute).
**Interactions**:
- [Review] -> Opens approval detail (dialogue or staging).
- [Switch] -> Changes active stage set and refreshes staging + conversation panels.
- [Pin] -> Locks stage set to prevent auto-rotation from region changes.
- [New Stage] -> Opens stage creation modal with location/region selectors.
- [Pre-Stage] -> Opens staging editor for selected region.
- [Open] -> Opens conversation detail panel.
- [Override] -> Opens visual state catalog picker.
- [Advance Time] -> Prompts for time delta and applies.

#### State 2: Approval Detail (Dialogue)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Approval: Dialogue Response                                     [X]         │
├─────────────────────────────────────────────────────────────────────────────┤
│ Context: Arden <-> Tessa  Location: Market Square  Tone: Tense              │
│                                                                               │
│ ─── AI Response ─────────────────────────────────────────────────────────── │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ "If you want the map, you will owe me a favor. No negotiations."         │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                               │
│ ─── Suggested Tools ─────────────────────────────────────────────────────── │
│ [ ] Give Item: Rusted Map  -> PC Arden                                       │
│ [ ] Set Flag: OwesFavor = true                                               │
│                                                                               │
│ ─── DM Edit (optional) ───────────────────────────────────────────────────── │
│ [Input edit text...                                                        ]│
│                                                                               │
│ [Accept] [Modify & Accept] [Reject] [Take Over]                              │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM selects a queued dialogue approval.
**Interactions**:
- [Accept] -> Sends approval; dialogue delivered to player.
- [Modify & Accept] -> Sends edited response.
- [Reject] -> Prompts for feedback and regenerates.
- [Take Over] -> Opens DM-authored response editor.

#### State 3: Loading/Processing
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Director                                                          [X]       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                        ┌─────────────────────────┐                          │
│                        │   Processing request    │                          │
│                        │   Approval queue sync   │                          │
│                        │   [spinner]             │                          │
│                        └─────────────────────────┘                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Syncing queues or applying approval decision.
**Interactions**:
- None; overlay clears on completion.

#### State 4: Error State (Disconnected)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Director                                                     [Offline]       │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Connection lost. Actions are paused.                                   │  │
│  │ [Retry Connection]  [View Cached Queue]  [Back to Role Select]         │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: WebSocket disconnect.
**Interactions**:
- [Retry Connection] -> Attempts reconnect.
- [View Cached Queue] -> Opens last known approvals read-only.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| DM Header Tabs | Switch between DM modes | Existing |
| Active Stages Selector | Switch between staged scene sets | New |
| Approval Queue Panel | List + review AI approvals | New |
| Staging Overview Panel | Pre-stage and manage region staging | Existing |
| Conversation Manager | List and open active conversations | New |
| NPC Tools Panel | Direct NPC actions | New |
| Directorial Notes | Guidance for LLM | Existing |
| Visual State Override | Choose scene state | New |

### Accessibility Considerations
- Keyboard focus cycles through panels with visible focus outline.
- Approval actions have distinct labels and aria descriptions.
- Status indicator includes text for non-color users.

### Animations
- Queue items slide-in on arrival with a 150ms stagger.
- Approval decision triggers a brief highlight pulse on the affected panel.

### Responsive Considerations
- Desktop: three-column grid with collapsible right rail.
- WASM: panels stack vertically with sticky header and floating action bar.
