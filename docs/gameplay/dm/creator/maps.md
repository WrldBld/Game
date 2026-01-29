## UI Design: Creator - Maps

User Stories: [US-NAV-010](../../../systems/navigation-system.md), [US-NAV-003](../../../systems/navigation-system.md)

### User Story
**DM-CRT-MAP-001**: As a DM, I can create and annotate maps so that regions and navigation are clear.

### User Flow
1. Initial state: DM opens map library.
2. User action: DM selects a location map or creates a new one.
3. System response: Map editor loads region nodes and connections.
4. Final state: Map saved and used for player navigation display.

### Mockups

#### State 1: Map Library (Initial)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Creator > Maps                                           [+ New Map]       │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌────────────────────────── Map List ──────────────────────────────┐       │
│  │ [Search...                 ] [Location: All ▼]                   │       │
│  │ ─── Location Maps ───────────────────────────────────────────── │       │
│  │ Redbridge (Town)       Nodes: 6  Connections: 8   [Edit] [View] │       │
│  │ Greyfen Marsh          Nodes: 4  Connections: 3   [Edit] [View] │       │
│  └─────────────────────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Landing view for Creator > Maps.
**Interactions**:
- [Edit] -> Opens map editor.
- [View] -> Opens preview-only mode.

#### State 2: Map Editor (Editing)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Map Editor: Redbridge                                         [X]           │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────── Canvas ────────────┐  ┌──────────── Properties ────────────┐ │
│  │ (Market Square)──(South Gate)  │  │ Background: [map_redbridge.png]    │ │
│  │       │            │           │  │ Grid: [On ▼]  Snap: [On ▼]          │ │
│  │ (Tankard)──(Back Room)         │  │ Region: [Market Square ▼]          │ │
│  │                               │  │ Label: [Market Square             ] │ │
│  │ [Add Node] [Add Link] [Delete]│  │ Icon: [Circle ▼]                    │ │
│  └───────────────────────────────┘  │ Visibility: [Player ▼]               │ │
│                                     └──────────────────────────────────────┘ │
│                                                                             │
│  [Save Map] [Publish to Players] [Cancel]                                   │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM edits map nodes and links.
**Interactions**:
- [Add Node] -> Creates a new region node.
- [Add Link] -> Connects two regions.
- [Publish to Players] -> Updates player mini-map.

#### State 3: Loading/Processing
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Creator > Maps                                                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                 ┌──────────────────────────────────────┐                    │
│                 │ Rendering map preview...             │                    │
│                 │ [spinner]                             │                    │
│                 └──────────────────────────────────────┘                    │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Preview render or save.
**Interactions**:
- None.

#### State 4: Error State (Missing Region)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Map Editor: Redbridge                                         [X]           │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Error: Linked region "Back Room" does not exist.                      │  │
│  │ [Create Region] [Unlink]                                              │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Map references a non-existent region.
**Interactions**:
- [Create Region] -> Opens region creation flow.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| Map Library | Browse location maps | New |
| Map Canvas | Node/link editor for regions | New |
| Properties Panel | Edit selected node | New |
| Map Preview | Rendered player map | New |

### Accessibility Considerations
- Canvas actions mirrored in a list for keyboard users.
- Node labels are editable via text inputs.

### Animations
- Node placement snaps with 100ms easing.
- Link creation animates a line draw.

### Responsive Considerations
- Desktop: canvas + properties side-by-side.
- WASM: canvas full width; properties in slide-up sheet.
