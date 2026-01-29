# Visual Novel Presentation

## Goals

- Maintain cinematic continuity across region changes.
- Keep the player focused on who is speaking and what choices exist.
- Use subtle transitions to avoid disorientation.

## Scene Composition

- **Backdrop**: Full-screen image for the region or scene.
- **Sprites**: Character sprites positioned left/center/right.
- **Portrait**: Speaking character highlight.
- **Dialogue box**: Typewriter effect for lines.
- **Choices**: Contextual response buttons.
- **Action panel**: Talk, examine, travel, character sheet.

## Presentation Rhythm

- One speaker at a time with focus highlight.
- Short bursts of dialogue (1-3 sentences by default).
- Quick scene transitions on movement or narrative triggers.
- Staging overlays are brief and fade when approved.

## Transition Rules

```
[Region change]
    |
    v
[Fade out backdrop]
    |
    v
[Update scene state]
    |
    v
[Fade in new backdrop]
```

## Visual State Integration

- Location state modifies broad ambience (festival, siege).
- Region state overrides local visuals (time-of-day, aftermath).
- The resulting look is presented to the DM during staging approval.

## UI Signals for DM Approval

- **LLM processing indicator** shows the player their action is being handled.
- **Staging pending overlay** communicates the scene is being prepared.
- **Outcome pending messages** are shown after a roll until the DM approves.

## Example Layout (ASCII)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ [Backdrop Image - Region]                                                   │
│                                                                             │
│  [NPC Sprite]        [NPC Sprite - speaking]        [NPC Sprite]            │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│ [Portrait] NPC Name                                                         │
│ "Line of dialogue with typewriter effect..."                                │
├─────────────────────────────────────────────────────────────────────────────┤
│ [Choice 1]  [Choice 2]  [Choice 3]                                           │
├─────────────────────────────────────────────────────────────────────────────┤
│ [Talk] [Examine] [Travel] [Character]                                        │
└─────────────────────────────────────────────────────────────────────────────┘
```
