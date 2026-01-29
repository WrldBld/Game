## UI Design: Settings - Skills

User Stories: (no explicit US IDs referenced for skills settings)

### User Story
**DM-SET-SKILL-001**: As a DM, I can configure the skill list and modifiers so that challenges align with my system.

### User Flow
1. Initial state: DM opens skills settings.
2. User action: DM edits skill list or values.
3. System response: Skill rules validate and persist.
4. Final state: Challenge rolls reference updated skills.

### Mockups

#### State 1: Skills Settings (Initial)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Settings > Skills                                                        [x]│
├─────────────────────────────────────────────────────────────────────────────┤
│  Game System Preset: [PF2E ▼]   Custom Skills: [On ▼]                       │
│                                                                             │
│  ─── Skills List ───────────────────────────────────────────────────────── │
│  * Athletics      Base: [0]  Type: [Physical ▼]   [Edit] [Disable]          │
│  * Arcana         Base: [0]  Type: [Knowledge ▼]  [Edit] [Disable]          │
│  Deception        Base: [0]  Type: [Social ▼]     [Edit] [Disable]          │
│  Survival         Base: [0]  Type: [Practical ▼]  [Edit] [Disable]          │
│                                                                             │
│  [Add Skill] [Import from System]                                           │
│                                                                             │
│  ─── Roll Display ──────────────────────────────────────────────────────── │
│  Show skill tags in challenges: [x]   Show base modifier: [x]               │
│                                                                             │
│  [Save] [Reset]                                                            │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Settings > Skills.
**Interactions**:
- [Edit] -> Opens skill edit dialog.
- [Add Skill] -> Adds a new skill row.

#### State 2: Skill Edit Dialog (Decision)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Edit Skill: Arcana                                           [X]            │
├─────────────────────────────────────────────────────────────────────────────┤
│ Name: [Arcana                         ]                                    │
│ Type: [Knowledge ▼]   Abbrev: [Arc]   Color Tag: [Teal ▼]                   │
│ Description:                                                              │
│ [Input multiline...                                                      ]│
│                                                                             │
│ [Save Skill] [Cancel]                                                      │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM edits a skill definition.
**Interactions**:
- [Save Skill] -> Updates the skill list.

#### State 3: Error State (Duplicate Skill)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Settings > Skills                                                        [!]│
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Error: Skill name already exists.                                     │  │
│  │ [Rename Skill]                                                        │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Adding a duplicate skill name.
**Interactions**:
- [Rename Skill] -> Focuses the name field.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| Skills List | View and edit skill definitions | New |
| Skill Edit Dialog | Edit skill metadata | New |
| Preset Selector | Load ruleset defaults | Existing |

### Accessibility Considerations
- Skill rows are navigable by keyboard and announce state.
- Dialogs trap focus and return focus to triggering row.

### Animations
- Skill rows animate height on expand/collapse.

### Responsive Considerations
- Desktop: full list with inline controls.
- WASM: list items open in full-screen editor.
