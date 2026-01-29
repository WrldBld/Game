## UI Design: Story Arc - Events

User Stories: [US-NAR-001](../../../systems/narrative-system.md), [US-NAR-002](../../../systems/narrative-system.md), [US-NAR-007](../../../systems/narrative-system.md), [US-NAR-009](../../../systems/narrative-system.md)

### User Story
**DM-ARC-EVT-001**: As a DM, I can author narrative events with triggers and outcomes so that the story reacts to play.

### User Flow
1. Initial state: DM opens event library.
2. User action: DM creates or edits an event.
3. System response: Trigger builder validates and saves.
4. Final state: Event appears in library and can be linked to arcs.

### Mockups

#### State 1: Event Library (Initial)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Story Arc > Events                                         [+ New Event]   │
├─────────────────────────────────────────────────────────────────────────────┤
│  [Search...                 ] [Type: All ▼] [Status: Any ▼]                 │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Gate Rumors           Trigger: Location + Dialogue   [Edit] [Chain]    │  │
│  │ "When players ask about the gate..."                                   │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Secret Exchange      Trigger: Time + Flag           [Edit] [Chain]     │  │
│  │ "After dusk if OwesFavor is true..."                                  │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  [Import] [Export]                                                          │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Story Arc > Events.
**Interactions**:
- [+ New Event] -> Opens event editor.
- [Chain] -> Opens chain builder with event selected.

#### State 2: Event Editor (Editing)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Event Editor: Gate Rumors                                    [X]            │
├─────────────────────────────────────────────────────────────────────────────┤
│ Name: [Gate Rumors                       ]  Type: [Narrative ▼]             │
│ Description:                                                              │
│ [Input multiline...                                                      ]│
│                                                                             │
│ ─── Trigger Builder ────────────────────────────────────────────────────── │
│ [Condition] Location is Market Square                                      │
│ [AND] Dialogue includes "gate"                                             │
│ [AND] Time is Dusk                                                         │
│ [Add Condition] [Group]                                                    │
│                                                                             │
│ ─── Outcomes ───────────────────────────────────────────────────────────── │
│ Outcome 1: [Set Flag: GateRumorsHeard = true] [Edit] [Remove]               │
│ Outcome 2: [Add NPC: Guard Rook to staging] [Edit] [Remove]                 │
│                                                                             │
│ [Save Event] [Test Trigger] [Cancel]                                       │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM edits event triggers and outcomes.
**Interactions**:
- [Test Trigger] -> Runs simulation with current context.

#### State 3: Loading/Processing
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Story Arc > Events                                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                 ┌──────────────────────────────────────┐                    │
│                 │ Validating trigger rules...          │                    │
│                 │ [spinner]                             │                    │
│                 └──────────────────────────────────────┘                    │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Testing or saving triggers.
**Interactions**:
- None.

#### State 4: Error State (Invalid Trigger)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Event Editor                                                        [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Error: Trigger group missing a condition.                             │  │
│  │ [Add Condition]                                                       │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Save with an incomplete trigger group.
**Interactions**:
- [Add Condition] -> Focuses trigger builder.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| Event Library | Browse narrative events | Existing |
| Trigger Builder | Visual condition editor | Existing |
| Outcome Editor | Define effects/outcomes | New |
| Event Editor | Compose full event | Existing |

### Accessibility Considerations
- Trigger builder supports keyboard add/remove.
- Outcomes list includes text descriptions.

### Animations
- Trigger blocks animate when grouped/ungrouped.

### Responsive Considerations
- Desktop: editor split into trigger/outcomes columns.
- WASM: sections stack with collapse toggles.
