## UI Design: Player Route - Main Menu

### User Story
No direct system user story; onboarding entry point before role/world selection.

### User Flow
1. Initial state: App launches on `MainMenuRoute`.
2. System response: Loads last server URL or default, configures engine URL.
3. Final state: Auto-redirects to role selection (web) or waits for manual connect (native).

### Mockups

#### State 1: Auto-Redirect (Web)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  WrldBldr                                                               [X] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                           Loading WrldBldr...                               │
│                                                                             │
│                      Using last server or default                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Web flow auto-configures server URL and pushes `/roles`.
**Interactions**:
- None (system-driven redirect).

#### State 2: Manual Connect (Native/Desktop)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  WrldBldr                                                               [X] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                          WrldBldr                                           │
│                         TTRPG Game Client                                   │
│                                                                             │
│  Server Address                                                             │
│  [ws://localhost:3000/ws___________________________________________]        │
│                                                                             │
│                               [Connect to Server]                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Desktop flow can still allow explicit server entry.
**Interactions**:
- `[Connect to Server]` -> Save server URL, configure engine URL, navigate to Role Select.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| `MainMenu` | Server URL input and connect button | Yes |

### Accessibility Considerations
- Keyboard focus on input and connect button.
- Input label paired with field for screen readers.

### Animations
- None required; keep redirect fast and calm.

### Responsive Considerations
- Centered card scales to 90% width on small screens.
