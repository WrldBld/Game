## UI Design: Creator - Items

User Stories: [US-INV-006](../../../systems/inventory-system.md), [US-INV-007](../../../systems/inventory-system.md)

### User Story
**DM-CRT-ITEM-001**: As a DM, I can author items and place them in the world so that rewards and loot feel intentional.

### User Flow
1. Initial state: DM opens item library.
2. User action: DM creates or edits an item template.
3. System response: Item is saved and available for placement.
4. Final state: Items can be granted or placed in regions.

### Mockups

#### State 1: Item Library (Initial)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Creator > Items                                          [+ New Item]      │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌────────────────────────── Library ───────────────────────────────┐       │
│  │ [Search...                 ] [Type: All ▼] [Rarity: Any ▼]        │       │
│  │ ─── Items ────────────────────────────────────────────────────── │       │
│  │ * Rusted Map        Type: Quest     Unique: Yes   [Edit] [Place] │       │
│  │   "Leads to the drowned vault."                                  │       │
│  │   Last used: Market Square                                      │       │
│  │                                                                  │       │
│  │ Silver Dagger     Type: Weapon  Unique: No    [Edit] [Place]    │       │
│  │   "Balanced blade with an inlayed hilt."                        │       │
│  └──────────────────────────────────────────────────────────────────┘       │
│                                                                             │
│  [Import Items] [Bulk Tags] [Archive Selected]                               │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Landing view for Creator > Items.
**Interactions**:
- [Edit] -> Opens item editor.
- [Place] -> Opens region placement dialog.

#### State 2: Item Editor (Editing)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Item Editor: Rusted Map                                        [X]          │
├─────────────────────────────────────────────────────────────────────────────┤
│ Name: [Rusted Map                         ]  Type: [Quest ▼]                │
│ Rarity: [Unique ▼]   Stackable: [ ]   Quantity: [1]                          │
│ Tags: [map] [quest] [antique]                                               │
│                                                                              │
│ Description:                                                                │
│ [Input multiline...                                                        ]│
│                                                                              │
│ Properties:                                                                 │
│ - Readable Text: [The ink is faded...                         ]             │
│ - Grants Knowledge: [Drowned Vault Location ▼]                              │
│                                                                              │
│ [Save Item] [Save & Place] [Cancel]                                         │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM edits item template details.
**Interactions**:
- [Save & Place] -> Opens region placement after save.

#### State 3: Placement Dialog (Decision)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Place Item: Rusted Map                                       [X]            │
├─────────────────────────────────────────────────────────────────────────────┤
│ Location: [Redbridge ▼]   Region: [Market Square ▼]                          │
│ Quantity: [1]   Visibility: [Visible ▼]                                     │
│ Placement Notes: [Hidden beneath the fountain...            ]               │
│                                                                              │
│ [Confirm Placement] [Cancel]                                                │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM places item in a region or gives to a PC.
**Interactions**:
- [Confirm Placement] -> Item added to region inventory.

#### State 4: Error State (Invalid)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Item Editor                                                          [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Error: Item type is required.                                         │  │
│  │ [Fix Fields]                                                          │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Save attempted with missing fields.
**Interactions**:
- [Fix Fields] -> Focuses the first invalid field.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| Item Library | Browse and filter item templates | New |
| Item Editor | Edit item metadata and properties | New |
| Placement Dialog | Place item in region or give to PC | Existing |

### Accessibility Considerations
- Form fields include clear labels and helper text.
- Placement dialog supports keyboard escape to cancel.

### Animations
- Placement confirmation toast slides in from bottom.
- Library rows highlight on hover/focus.

### Responsive Considerations
- Desktop: list + editor modal.
- WASM: editor and placement are full-screen sheets.
