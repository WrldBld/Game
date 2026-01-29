## UI Design: Story Arc - Chains

User Stories: [US-NAR-003](../../../systems/narrative-system.md), [US-NAR-008](../../../systems/narrative-system.md)

### User Story
**DM-ARC-CHN-001**: As a DM, I can sequence narrative events into chains so that the story escalates in a controlled way.

### User Flow
1. Initial state: DM opens chain list.
2. User action: DM creates or edits a chain.
3. System response: Flowchart updates and validates.
4. Final state: Chain appears in library and can be triggered.

### Mockups

#### State 1: Chain Library (Initial)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Story Arc > Chains                                        [+ New Chain]    │
├─────────────────────────────────────────────────────────────────────────────┤
│  [Search...                 ] [Arc: Redbridge ▼]                            │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Chain: Market Unrest     Steps: 4   Status: Active   [Edit] [View]     │  │
│  │ "Escalates from rumors to confrontation."                              │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  [Import] [Export]                                                          │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Story Arc > Chains.
**Interactions**:
- [+ New Chain] -> Opens chain editor.
- [Edit] -> Opens flowchart editor.

#### State 2: Chain Builder (Editing)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Chain Builder: Market Unrest                              [X]              │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────────────── Flowchart ───────────────────────────┐          │
│  │ (Gate Rumors) -> (Secret Exchange) -> (Guard Crackdown)      │          │
│  │        |                         \-> (Player Choice)         │          │
│  │ [Add Node] [Add Link] [Delete]                               │          │
│  └──────────────────────────────────────────────────────────────┘          │
│                                                                             │
│  ┌──────────── Chain Properties ────────────┐                               │
│  │ Name: [Market Unrest                  ] │                               │
│  │ Arc:  [Redbridge ▼]                    │                               │
│  │ Trigger: [On Event Completed ▼]        │                               │
│  │ Start Event: [Gate Rumors ▼]           │                               │
│  └────────────────────────────────────────┘                               │
│                                                                             │
│  [Save Chain] [Validate] [Cancel]                                           │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM links events in a sequence.
**Interactions**:
- [Add Node] -> Inserts an event from library.
- [Validate] -> Checks for loops or missing nodes.

#### State 3: Loading/Processing
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Story Arc > Chains                                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                 ┌──────────────────────────────────────┐                    │
│                 │ Validating chain...                  │                    │
│                 │ [spinner]                             │                    │
│                 └──────────────────────────────────────┘                    │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Chain validation in progress.
**Interactions**:
- None.

#### State 4: Error State (Loop Detected)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Chain Builder                                                   [X]         │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Error: Loop detected between "Secret Exchange" and "Guard Crackdown"  │  │
│  │ [Break Link] [View Loop]                                              │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Chain validation fails due to loop.
**Interactions**:
- [Break Link] -> Removes the offending connection.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| Chain Library | Browse story chains | New |
| Chain Builder | Visual sequence editor | Existing |
| Chain Properties | Edit chain metadata | New |

### Accessibility Considerations
- Flowchart nodes accessible via a list sidebar.
- Validation errors are announced with clear labels.

### Animations
- Links draw with 120ms easing on creation.
- Node selection glow for focus state.

### Responsive Considerations
- Desktop: flowchart plus properties panel.
- WASM: flowchart full width; properties in slide-over.
