## UI Design: Creator - Locations

User Stories: [US-NAV-003](../../../systems/navigation-system.md), [US-NAV-004](../../../systems/navigation-system.md), [US-NAV-006](../../../systems/navigation-system.md), [US-NAV-007](../../../systems/navigation-system.md), [US-STG-009](../../../systems/staging-system.md), [US-STG-017](../../../systems/staging-system.md)

### User Story
**DM-CRT-LOC-001**: As a DM, I can build locations and regions so that staging and navigation are coherent.

### User Flow
1. Initial state: DM opens location library.
2. User action: DM selects a location or creates a new one.
3. System response: Region editor and staging defaults load.
4. Final state: Location structure saved and available to navigation/staging.

### Mockups

#### State 1: Location Library (Initial)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Creator > Locations                                       [+ New Location] │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌────────── Location Tree ──────────┐  ┌────── Location Details ─────────┐ │
│  │ World                             │  │ Name: [Redbridge             ] │ │
│  │ ├─ Redbridge (Town)               │  │ Type: [Town ▼]                │ │
│  │ │  ├─ Market Square (Region)      │  │ Description:                 │ │
│  │ │  ├─ South Gate (Region)         │  │ [Input multiline...         ] │ │
│  │ │  └─ The Crooked Tankard (Loc)   │  │                              │ │
│  │ │      ├─ Main Hall (Region)      │  │ ── Staging Defaults ───────── │ │
│  │ │      └─ Back Room (Region)      │  │ TTL Hours: [6 ▼]              │ │
│  │ └─ Greyfen Marsh (Loc)            │  │ LLM Presence: [On ▼]          │ │
│  └───────────────────────────────────┘  │ Auto-Approve: [On ▼]         │ │
│                                         └───────────────────────────────┘ │
│                                                                             │
│  ┌────────────── Regions ──────────────┐  ┌────────────── Exits ───────────┐ │
│  │ Main Hall        [Edit] [Map]       │  │ South Gate -> Greyfen Marsh    │ │
│  │ Back Room        [Edit] [Map]       │  │ Market Square -> North Road    │ │
│  │ Cellar           [Add Region]       │  │ [Add Exit]                     │ │
│  └─────────────────────────────────────┘  └───────────────────────────────┘ │
│                                                                             │
│  [Save Location] [Save & Pre-Stage] [Cancel]                                │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM browses and edits location structure.
**Interactions**:
- [+ New Location] -> Opens empty location form.
- [Edit] -> Opens region editor panel.
- [Add Exit] -> Opens exit configuration dialog.

#### State 2: Region Editor (Decision)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Region Editor: Market Square                                   [X]          │
├─────────────────────────────────────────────────────────────────────────────┤
│ Spawn Points: [Main Fountain ▼]  Capacity: [High ▼]                         │
│ Ambient Audio: [market_ambience_02 ▼]   Visual State: [Dusty Evening ▼]      │
│                                                                              │
│ Connected Regions: [South Gate] [The Crooked Tankard]                       │
│ [Add Connection] [Remove Selected]                                          │
│                                                                              │
│ [Save Region] [Cancel]                                                      │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM edits region metadata for staging and navigation.
**Interactions**:
- [Add Connection] -> Adds region link for navigation.

#### State 3: Loading/Processing
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Creator > Locations                                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                 ┌──────────────────────────────────────┐                    │
│                 │ Saving location graph...             │                    │
│                 │ [spinner]                             │                    │
│                 └──────────────────────────────────────┘                    │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Saving location graph updates.
**Interactions**:
- None.

#### State 4: Error State (Conflict)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Creator > Locations                                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Error: Region name already exists in this location.                    │  │
│  │ [Rename Region]                                                        │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Duplicate region name on save.
**Interactions**:
- [Rename Region] -> Focuses region name field.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| Location Tree | Hierarchy browser for locations/regions | New |
| Location Details Form | Core location metadata | New |
| Staging Defaults | TTL and LLM presence defaults | Existing |
| Regions List | Manage region entries | New |
| Exits Panel | Manage location exits | New |

### Accessibility Considerations
- Tree view supports keyboard expand/collapse.
- Form validation errors announce via live region.

### Animations
- Tree expand/collapse with 120ms slide.
- Save success toast fades in/out.

### Responsive Considerations
- Desktop: tree + details split; regions/exits below.
- WASM: tree becomes accordion; details full width.
