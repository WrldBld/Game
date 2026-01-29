## UI Design: Player Route - World Select

### User Story
No direct system user story; aligns with route flow after role selection.

### User Flow
1. Initial state: Role already chosen; world list loads.
2. User action: Select world to join/watch/continue.
3. System response: Navigates to `/worlds/:world_id/play` (or DM/spectator).
4. Final state: WorldSessionLayout handles connection.

### Mockups

#### State 1: Loading Worlds
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Select World                                                        [Back] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                            Loading worlds...                                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: List is fetched on mount.
**Interactions**:
- `[Back]` -> Route to role selection.

#### State 2: World List (Player)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Join a World                                                       [Back] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Available Worlds                                                          │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ The Rusty Anchor                                                     │ │
│  │ A coastal tavern with rumored smugglers                              │ │
│  │                                                       [Join]         │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ The Bleak Marches                                                    │ │
│  │ A war-torn frontier of ruined keeps                                 │ │
│  │                                                       [Join]         │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Player role selected; no create button.
**Interactions**:
- `[Join]` -> Save `LAST_WORLD`, navigate to `/worlds/:id/play`.
- `[Back]` -> Route to role selection.

#### State 3: World List (DM)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Select World                                                       [Back] │
├─────────────────────────────────────────────────────────────────────────────┤
│  Your Worlds                                                  [+ Create]    │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ The Rusty Anchor                                                     │ │
│  │ A coastal tavern with rumored smugglers                              │ │
│  │                                              [Enter World ->]         │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: DM sees create option and enter action.
**Interactions**:
- `[+ Create]` -> Show create world form.
- `[Enter World ->]` -> Navigate to DM view.

#### State 4: Empty State
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Join a World                                                       [Back] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  No worlds available.                                                      │
│  Ask your DM to create a world first.                                      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: `worlds` list empty.
**Interactions**:
- `[Back]` -> Route to role selection.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| `WorldSelectView` | List worlds, handle role-specific actions | Yes |
| `WorldCard` | World summary with action button | Yes |
| `CreateWorldForm` | DM-only create flow | Yes |

### Accessibility Considerations
- World cards are button-like; ensure full row is focusable.
- Error banner should be announced to screen readers.

### Animations
- Subtle list fade-in after loading.

### Responsive Considerations
- List becomes single-column on small screens; action buttons remain visible.
