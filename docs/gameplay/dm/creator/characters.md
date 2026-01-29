## UI Design: Creator - Characters

User Stories: [US-NAV-003](../../../systems/navigation-system.md), [US-DLG-001](../../../systems/dialogue-system.md)

### User Story
**DM-CRT-CHAR-001**: As a DM, I can author and maintain NPCs so that dialogue and staging have consistent characters.

### User Flow
1. Initial state: DM opens character library.
2. User action: DM selects or creates a character.
3. System response: Editor loads fields and assets.
4. Final state: Character saved and available for staging/dialogue.

### Mockups

#### State 1: Character Library (Initial)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ WrldBldr  Director  Creator  Story Arc  Settings           [Connected]      │
├─────────────────────────────────────────────────────────────────────────────┤
│  Creator > Characters   [+ New Character]                                  │
│  ┌────────────────────────── Library ───────────────────────────┐          │
│  │ [Search...                 ] [Type: NPC ▼] [* Favorites]     │          │
│  │ ─── NPCs ─────────────────────────────────────────────────── │          │
│  │ * Tessa Windfall     Role: Merchant   Status: Active [Edit]  │          │
│  │   "Knows the map routes and rumors"                           │          │
│  │   Last seen: Market Square                                   │          │
│  │                                                           ┌─────────┐  │
│  │   Guard Rook         Role: Watch      Status: Hidden [Edit]│ Portrait│  │
│  │   "Always present at night shifts"                         │ Preview │  │
│  │                                                           └─────────┘  │
│  └─────────────────────────────────────────────────────────────┘          │
│                                                                             │
│  [Import NPC] [Bulk Tags] [Archive Selected]                               │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Landing view for Creator > Characters.
**Interactions**:
- [+ New Character] -> Opens empty editor in right rail.
- [Edit] -> Opens character editor with selected NPC.

#### State 2: Character Editor (Editing)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Creator > Characters                                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌────────────── Character Editor ──────────────┐  ┌────── Portrait ──────┐  │
│  │ Name:        [Tessa Windfall              ]  │  │ [Image preview      ]│  │
│  │ Archetype:   [Mentor ▼]    Mood: [Wary ▼]    │  │ [Generate] [Upload]  │  │
│  │ Tags:        [merchant] [secretive] [map]   │  └───────────────────────┘  │
│  │                                                                        │  │
│  │ Description:                                                          │  │
│  │ [Input multiline...                                                  ]│  │
│  │                                                                        │  │
│  │ Wants:     [Protect trade routes                       ] [Add]        │  │
│  │ Helpers:   [Guard Rook] [Remove]                                     │  │
│  │ Opponents: [Dock Syndicate] [Remove]                                 │  │
│  │                                                                        │  │
│  │ Default Location: [Redbridge > Market Square ▼]                       │  │
│  │ Staging Pref: [Often Present ▼]  Hidden by default: [ ]               │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│  [Save Character] [Save & Stage] [Cancel]                                   │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM edits an existing NPC or creates a new one.
**Interactions**:
- [Generate] -> Requests portrait generation.
- [Save & Stage] -> Saves and adds to staging suggestions.

#### State 3: Loading/Processing
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Creator > Characters                                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                ┌───────────────────────────────────────┐                    │
│                │ Generating portrait and metadata      │                    │
│                │ [spinner]                              │                    │
│                └───────────────────────────────────────┘                    │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Portrait generation or save in progress.
**Interactions**:
- None; overlay clears when complete.

#### State 4: Error State (Validation)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Creator > Characters                                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Error: Name cannot be empty.                                           │  │
│  │ [Fix Fields]                                                           │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: User attempts to save with missing required fields.
**Interactions**:
- [Fix Fields] -> Focuses first invalid field.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| Character Library | Searchable list of NPCs | New |
| Character Editor | Form for NPC details | New |
| Portrait Preview | Asset preview and generation | New |
| Tags/Relationships | Manage wants/helpers/opponents | New |

### Accessibility Considerations
- Editor fields are labeled and grouped by legend headings.
- Portrait actions include text labels, not icon-only.

### Animations
- New character row fades in after save.
- Portrait preview crossfades on update.

### Responsive Considerations
- Desktop: two-column list + editor layout.
- WASM: editor opens as full-screen panel with sticky Save actions.
