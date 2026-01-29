## UI Design: Player Route - Connection States

### User Story
No direct system user story; supports reliable session entry and recovery.

### User Flow
1. Initial state: Player enters a world route (`/worlds/:id/play`).
2. System response: `WorldSessionLayout` kicks off WebSocket join.
3. Decision points: Connected, Reconnecting, Failed, or Disconnected.
4. Final state: Player view mounts when connected.

### Mockups

#### State 1: Connection Status Bar (Connected)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [Back]                                            [Connected]              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  (Player view content loads below when connected)                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Status bar always visible in world views.
**Interactions**:
- `[Back]` -> Disconnect, clear state, return to Role Select.

#### State 2: Connecting / Reconnecting
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [Back]                                            [Connecting...]          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                               Connecting...                                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Child views do not render yet.
**Interactions**:
- `[Back]` -> Abort and return to Role Select.

#### State 3: Disconnected / Failed
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [Back]                                            [Connection Failed]      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                            Connection failed                                │
│                          (click status to retry)                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Status indicator is clickable to retry.
**Interactions**:
- Status click -> Retry connect.
- `[Back]` -> Disconnect and return.

#### State 4: Error Overlay
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Connection Error                                                    [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  A network error occurred while joining the world.                          │
│                                                                             │
│                               [Dismiss]                                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Fatal connection or session errors.
**Interactions**:
- `[Dismiss]` -> Clear error message, keep status bar for retry.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| `WorldSessionLayout` | Connection handling and gating | Yes |
| `ConnectionStatusBar` | Back/retry/status UI | Yes |
| `ErrorOverlay` | Modal for errors | Yes |

### Accessibility Considerations
- Status text visible and actionable when retry is allowed.
- Error modal traps focus and dismisses via keyboard.

### Animations
- Status indicator pulse while connecting.

### Responsive Considerations
- Status bar stays compact; error modal centers with padding on mobile.
