## UI Design: Settings - Workflows

User Stories: [US-STG-016](../../../systems/staging-system.md), [US-STG-017](../../../systems/staging-system.md), [US-DLG-009](../../../systems/dialogue-system.md)

### User Story
**DM-SET-WF-001**: As a DM, I can configure approval and staging workflows so that AI output fits my pacing.

### User Flow
1. Initial state: DM opens workflow settings.
2. User action: DM edits workflow values.
3. System response: Values validate and persist.
4. Final state: Approval and staging behavior updates immediately.

### Mockups

#### State 1: Workflow Settings (Initial)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Settings > Workflows                                                     [x]│
├─────────────────────────────────────────────────────────────────────────────┤
│  ─── Approval Queue ────────────────────────────────────────────────────── │
│  Max Pending Items: [50]    Auto-Sort: [Newest First ▼]                    │
│  Notification Level: [High ▼]   Play Sound: [x]                            │
│                                                                             │
│  ─── Staging Workflow ──────────────────────────────────────────────────── │
│  Auto-Approve Timeout (sec): [30]                                          │
│  Auto-Approve on Timeout: [x]                                              │
│  Default TTL (hours): [6 ▼]                                                │
│  LLM Presence Suggestions: [On ▼]                                          │
│                                                                             │
│  ─── Dialogue Workflow ─────────────────────────────────────────────────── │
│  Token Budget Preset: [Balanced ▼]                                        │
│  Max Regeneration Attempts: [3]                                           │
│  Allow DM Takeover: [x]                                                   │
│                                                                             │
│  [Save] [Reset]                                                            │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Settings > Workflows.
**Interactions**:
- [Save] -> Persists settings to world.
- [Reset] -> Restores defaults.

#### State 2: Loading/Processing
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Settings > Workflows                                                     [ ]│
├─────────────────────────────────────────────────────────────────────────────┤
│                 ┌──────────────────────────────────────┐                    │
│                 │ Saving workflow settings...          │                    │
│                 │ [spinner]                             │                    │
│                 └──────────────────────────────────────┘                    │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Save in progress.
**Interactions**:
- None.

#### State 3: Error State (Validation)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Settings > Workflows                                                     [!]│
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Error: Auto-approve timeout must be between 5 and 300 seconds.        │  │
│  │ [Fix Value]                                                           │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Invalid numeric input.
**Interactions**:
- [Fix Value] -> Focuses the invalid input.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| Workflow Form | Configure approval and staging defaults | Existing |
| Settings Field Help | Show inline guidance | Existing |
| Save/Reset Bar | Primary settings actions | Existing |

### Accessibility Considerations
- All fields have labels and helper text.
- Save button disabled until changes detected.

### Animations
- Success toast slides in after save.

### Responsive Considerations
- Desktop: grouped sections in one column.
- WASM: sections stack with collapsible headers.
