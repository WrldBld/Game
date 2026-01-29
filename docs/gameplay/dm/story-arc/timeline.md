## UI Design: Story Arc - Timeline

User Stories: [US-NAR-001](../../../systems/narrative-system.md), [US-NAR-007](../../../systems/narrative-system.md)

### User Story
**DM-ARC-TIME-001**: As a DM, I can view and plan story beats over time so that pacing stays intentional.

### User Flow
1. Initial state: DM opens timeline for the active world.
2. User action: DM adds or edits a story beat.
3. System response: Timeline updates and propagates to events/quests.
4. Final state: Timeline reflects planned and completed beats.

### Mockups

#### State 1: Timeline Overview (Initial)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Story Arc > Timeline                                       [+ New Beat]    │
├─────────────────────────────────────────────────────────────────────────────┤
│  [Filter: All ▼]  [Arc: Redbridge ▼]  [Status: Any ▼]                       │
│                                                                             │
│  ─── Day 12, Dusk ──────────────────────────────────────────────────────── │
│  o "Market tensions rise"           Status: Planned   [Edit] [Link Event]  │
│  │  Notes: Guard patrols increase.                                         │
│  │  Linked: Event "Gate Rumors"                                            │
│                                                                             │
│  ─── Day 13, Night ─────────────────────────────────────────────────────── │
│  o "Secret exchange at the Tankard" Status: Draft    [Edit] [Link Event]   │
│  │  Notes: Reveal map if conditions met.                                   │
│                                                                             │
│  [Jump to Today] [Export]                                                   │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Story Arc > Timeline.
**Interactions**:
- [+ New Beat] -> Opens beat editor.
- [Link Event] -> Opens event picker.

#### State 2: Beat Editor (Decision)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Story Beat Editor                                           [X]             │
├─────────────────────────────────────────────────────────────────────────────┤
│ Title: [Market tensions rise                   ]                            │
│ Time:  [Day 12 ▼]  [Dusk ▼]   Status: [Planned ▼]                            │
│ Arc:   [Redbridge ▼]                                                       │
│ Notes:                                                                    │
│ [Input multiline...                                                      ]│
│ Linked Events: [Gate Rumors] [Add]                                         │
│                                                                             │
│ [Save Beat] [Cancel]                                                       │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Add/edit a beat.
**Interactions**:
- [Add] -> Opens event library selector.

#### State 3: Loading/Processing
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Story Arc > Timeline                                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                 ┌──────────────────────────────────────┐                    │
│                 │ Updating timeline...                 │                    │
│                 │ [spinner]                             │                    │
│                 └──────────────────────────────────────┘                    │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Timeline save or refresh.
**Interactions**:
- None.

#### State 4: Error State (Conflict)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Story Arc > Timeline                                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Error: Beat time overlaps an existing locked beat.                    │  │
│  │ [Adjust Time]                                                         │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Collision with locked timeline item.
**Interactions**:
- [Adjust Time] -> Opens beat editor with time focused.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| Timeline List | Chronological beats | New |
| Beat Editor | Create/edit a beat | New |
| Event Linker | Attach narrative events | New |

### Accessibility Considerations
- Timeline items are keyboard selectable.
- Filter controls have clear labels.

### Animations
- New beat insertion animates a brief highlight.

### Responsive Considerations
- Desktop: full-width timeline with sticky filters.
- WASM: timeline entries expand into stacked cards.
