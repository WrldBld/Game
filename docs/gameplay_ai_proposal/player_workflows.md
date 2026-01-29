# Player Workflows

## Overview

Players experience a visual novel style interface with navigation, dialogue, and challenge resolution, driven by DM-approved AI suggestions.

## 1. Enter World and See Scene

```
[Join World] -> [Current Region] -> [Scene Backdrop + NPCs]
```

If staging is pending, the player sees a short loading overlay before the scene appears.

## 2. Explore Regions and Locations

```
[Navigation Options]
        |
        v
[Move to Region] or [Exit to Location]
        |
        v
[Scene changes]
```

Movement updates the scene and may prompt staging approval.

## 3. Talk to NPCs

```
[Select Talk] -> [Player line or choice]
        |
        v
[LLM processing indicator]
        |
        v
[DM-approved response appears]
```

Dialogue stays short and responsive to keep momentum.

## 4. Make Choices

```
[Dialogue choices shown] -> [Select choice] -> [DM-approved result]
```

Choices can unlock narrative events, new interactions, or challenges.

## 5. Resolve Challenges

```
[Challenge prompt] -> [Roll] -> [DM-approved outcome]
```

Challenge outcomes are narrated and may apply effects (items, flags, stat changes).

## 6. See World State Updates

- NPCs enter or leave after staging updates.
- Visual state changes update backdrops and atmosphere.
- Time changes are visible in the UI and can influence scene availability.

## Player Experience Priorities

- Fast feedback after each action.
- Clear indication of DM approval moments (thinking indicator, pending overlays).
- Consistent scene presentation with minimal visual disruption.
