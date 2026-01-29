## UI Design: Player Route - PC Creation

### User Story
**US-CHAR-006**: As a player, I can create a PC and bind it to a session (`docs/systems/character-system.md#user-stories`).

### User Flow
1. Initial state: Player navigates to `/worlds/:id/play/create-character`.
2. User action: Complete multi-step wizard (Basics -> Sheet -> Location -> Review).
3. System response: Create PC and navigate to `/worlds/:id/play`.
4. Final state: Player enters world session view.

### Mockups

#### State 1: Step 1 - Basics
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Create Your Character                                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│  Step: [1] Basics  [2] Sheet  [3] Location  [4] Review                       │
│                                                                             │
│  Character Name *                                                          │
│  [Input field___________________________________________________]           │
│                                                                             │
│  Description (Optional)                                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  [Cancel]                                                     [Next]        │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Required name; optional description.
**Interactions**:
- `[Next]` -> Validate name and go to Sheet step.
- `[Cancel]` -> Navigate back.

#### State 2: Step 2 - Character Sheet
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Create Your Character                                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│  Step: [1] Basics  [2] Sheet  [3] Location  [4] Review                       │
│                                                                             │
│  Character Sheet                                                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ [Schema-driven fields render here]                                  │   │
│  │ STR [ 10 ]  DEX [ 12 ]  ...                                          │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  [Back]                                                       [Next]        │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Schema may load asynchronously; empty state if no schema.
**Interactions**:
- `[Next]` -> Continue even if schema missing.
- `[Back]` -> Return to Basics.

#### State 3: Step 3 - Starting Location
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Create Your Character                                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│  Step: [1] Basics  [2] Sheet  [3] Location  [4] Review                       │
│                                                                             │
│  Choose Starting Location                                                   │
│  ┌──────────────────────────────┐  ┌──────────────────────────────┐         │
│  │ Market Square                │  │ Harbor District               │         │
│  │ Exterior                      │  │ Exterior                      │         │
│  │ [Selected]                    │  │ [Select]                      │         │
│  └──────────────────────────────┘  └──────────────────────────────┘         │
│                                                                             │
│  [Back]                                                       [Next]        │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Locations list fetched from engine.
**Interactions**:
- Location card -> Select location.
- `[Next]` -> Requires a selection.

#### State 4: Step 4 - Review
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Create Your Character                                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│  Step: [1] Basics  [2] Sheet  [3] Location  [4] Review                       │
│                                                                             │
│  Name: Kira Shadowblade                                                     │
│  Description: A vengeful warrior...                                         │
│  Starting Location: Market Square                                           │
│  Character Sheet: Configured                                                │
│                                                                             │
│  [Back]                                           [Create Character]        │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Final confirmation before creation.
**Interactions**:
- `[Create Character]` -> Create PC and navigate to play view.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| `PCCreationView` | Multi-step wizard | Yes |
| `SchemaCharacterSheet` | Schema-driven sheet inputs | Yes |
| `LocationCard` | Choose start location | Yes |

### Accessibility Considerations
- Step indicator should be read as progress with current step.
- Error messages announced when validation fails.

### Animations
- Subtle step transition (fade/slide) to convey progression.

### Error States
- Missing name -> inline error banner.
- No locations -> informative message with DM contact hint.

### Responsive Considerations
- Cards stack in a single column on narrow screens.
