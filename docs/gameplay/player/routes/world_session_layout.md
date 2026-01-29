## UI Design: Player Route - World Session Layout (Player View)

### User Story
- **US-DLG-001**: Player can speak to NPCs and receive responses (`docs/systems/dialogue-system.md#user-stories`).
- **US-DLG-008**: Player sees thinking indicator (`docs/systems/dialogue-system.md#user-stories`).
- **US-DLG-017**: Player can end conversation (`docs/systems/dialogue-system.md#user-stories`).
- **US-TIME-008**: Player sees time passage notifications (`docs/systems/game-time-system.md#user-stories`).
- **US-TIME-009**: Player can see current game time (`docs/systems/game-time-system.md#user-stories`).
- **US-TIME-011**: Time-advance toast with reason (planned) (`docs/systems/game-time-system.md#user-stories`).
- **US-LORE-005/006**: Player can view lore journal with partial chunks (`docs/systems/lore-system.md#user-stories`).
- **US-INV-001/002/003/004/005**: Player inventory actions (`docs/systems/inventory-system.md#user-stories`).
- **US-NAV-008/009/010**: Player navigation + time display + minimap (`docs/systems/navigation-system.md#user-stories`).

### User Flow
1. Initial state: Connected to world; scene snapshot renders.
2. User action: Interact via Action Panel, character click, or dialogue choices.
3. System response: Dialogue updates, LLM processing indicator, time toasts, panels/modals.
4. Final state: Player continues exploration or conversation.

### Mockups

#### State 1: Core Player View (Exploration)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [Back]                                            [Connected]              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                         [Backdrop Scene]                               │ │
│  │                                                                       │ │
│  │     [NPC Sprite]          [PC Sprite]          [NPC Sprite]           │ │
│  │                                                                       │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ [Portrait]  NPC Name                                                   │ │
│  │                                                                       │ │
│  │ "Dialogue text with typewriter animation..."                           │ │
│  │ [cursor]                                                              │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐            │
│  │ "Choice option 1"│ │ "Choice option 2"│ │ "Choice option 3"│            │
│  └──────────────────┘ └──────────────────┘ └──────────────────┘            │
│                                                                             │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐               │
│  │ Talk       │ │ Examine    │ │ Travel     │ │ Character  │               │
│  │ [NPC]      │ │ [Room]     │ │ [Exit]     │ │ [Sheet]    │               │
│  └────────────┘ └────────────┘ └────────────┘ └────────────┘               │
│                                                                             │
│  Day 3, Evening  19:00                                                     │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Standard connected gameplay view with dialogue and choices.
**Interactions**:
- Character click -> Send Talk action to NPC.
- `[Choice option]` -> Send dialogue choice.
- `[Talk/Examine/Travel/Character]` -> Open action handlers/modals.

#### State 2: LLM Processing (Dialogue Thinking)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ [Portrait]  NPC Name                                                   │ │
│  │                                                                       │ │
│  │ "..."                                                                 │ │
│  │ [Thinking indicator]                                                   │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: LLM response pending.
**Interactions**:
- Action panel disabled until response returns.

#### State 3: Time Advance Toast
```
┌────────────────────────────────────────────┐
│  Time Advanced                             │
│                                            │
│  Travel to Harbor District                 │
│  +1 hour                                   │
│                                            │
│  Current time: Day 3, Evening (19:00)      │
│                                            │
│  [Dismiss]                                 │
└────────────────────────────────────────────┘
```
**Context**: Broadcast after DM approval of time suggestion.
**Interactions**:
- `[Dismiss]` -> Close toast early.

#### State 4: Lore Journal (Modal)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Lore Journal                                                       [X]    │
├─────────────────────────────────────────────────────────────────────────────┤
│  [All] [Historical] [Legend] [Secret] [Search_________]                     │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ The Fall of House Valeren                               [Historical]   │ │
│  │ You know 2 of 3 parts                                               │ │
│  │                                                                       │ │
│  │ - The Glory Days                                                     │ │
│  │ - The Betrayal                                                       │ │
│  │ - [Unknown - more to discover]                                       │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Opened from action panel (Log/Lore).
**Interactions**:
- `[X]` -> Close modal.
- Category tabs/filter/search -> Filter entries.

#### State 5: Inventory Panel (Modal)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Inventory                                                         [X]     │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ Equipped                                                              │ │
│  │  Sword of the Fallen                                    [Unequip]     │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ Rusty Key                                                [Use] [Drop] │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ Baron's Letter                                          [Use] [Drop]  │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: Opened from action panel inventory button.
**Interactions**:
- `[Use]` -> Send UseItem action.
- `[Drop]` -> Send DropItem action.
- `[Unequip]` -> Send UnequipItem action.

#### State 6: Choice Menu Emphasis (Dialogue Focus)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ "Choice option 1"                                                   │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ "Choice option 2"                                                   │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ "Choice option 3"                                                   │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
**Context**: When dialogue includes options; action panel remains visible but secondary.
**Interactions**:
- Choice card click -> Submit choice.

### Component Breakdown
| Component | Purpose | Existing? |
|-----------|---------|-----------|
| `WorldSessionLayout` | Connection + status + error overlay | Yes |
| `Backdrop` | Scene backdrop | Yes |
| `CharacterLayer` | NPC/PC sprites | Yes |
| `DialogueBox` | Speaker + typewriter text | Yes |
| `ActionPanel` | Talk/Examine/Travel/Character actions | Yes |
| `TimeAdvanceToast` | Time passage notification | Yes |
| `InventoryPanel` | Items + equip/use/drop | Yes |
| `LoreJournal` | Player codex/journal | Yes (UI pending in lore system)

### Accessibility Considerations
- Dialogue choices are keyboard reachable and read in order.
- Modals trap focus and dismiss on Escape.
- Time toast includes text alternative and manual dismiss.

### Animations
- Backdrop crossfade on region changes (500ms).
- Dialogue typewriter cursor blink.
- Time toast slide-in/out (300ms).

### Error States
- Action error toast near top-right with click-to-dismiss.
- Disconnected banner shown when connection drops.

### Responsive Considerations
- Dialogue box wraps to two lines; choice buttons stack on mobile.
- Action panel compresses to two rows with icon+label.
