# Session Flow

## Overview

This describes the intended end-to-end session loop using the existing systems:

- Navigation (region movement)
- Staging (NPC presence + visual state approval)
- Scene (visual novel presentation)
- Dialogue (LLM responses with DM approval)
- Challenge (rolls and outcomes)
- Narrative events (triggered story effects)
- Game time (DM-controlled time progression)

## Core Loop (Player + DM)

```
[Player enters world]
        |
        v
[Player chooses region]
        |
        v
[Staging request created]
        |
        v
[DM approves staging + visual state]
        |
        v
[Scene resolves + NPCs appear]
        |
        v
[Player talks / chooses / examines]
        |
        v
[AI proposes dialogue/challenge/event]
        |
        v
[DM approves/edits/rejects]
        |
        v
[Outcome shown + state updated]
        |
        v
[Time suggestion -> DM decision]
        |
        v
  (loop back to movement or dialogue)
```

## Approval Cadence (Golden Path)

Approval exists to preserve DM authority and tone. The cadence is predictable:

1. **Staging approval** when a player enters an unstaged region.
2. **Dialogue approval** for every AI response.
3. **Challenge approval** for AI suggestions and for outcome text.
4. **Narrative event approval** for triggers suggested by AI or system detection.
5. **Time suggestion approval** when actions imply time passage (if time mode is suggested).

## Scene Pacing Rules

- **Region changes are scene cuts.** Each region is a screen with a distinct backdrop.
- **Dialogue occurs inside the scene.** NPC speaking focus highlights the portrait or sprite.
- **Short loops for momentum.** Dialogue responses default to 1-3 sentences.
- **Challenges interrupt and resolve quickly.** Roll -> DM approves outcome -> continue.
- **Narrative events are punctuations.** Triggered events change the scene or unlock options.

## Session Stages

### 1. Arrival and Orientation

- Player joins world and receives initial location, region, and scene.
- If staging is needed, player sees a short loading overlay.

### 2. Exploration Loop

- Player navigates between regions and exits to connected locations.
- Each movement can create a staging request and/or time suggestion.

### 3. Interaction Loop

- Player talks to NPCs, examines objects, or selects dialogue choices.
- AI proposes dialogue and optionally suggests challenges or narrative triggers.
- DM approves or edits responses before the player sees them.

### 4. Resolution and Progress

- Challenge resolution applies outcomes and updates character state.
- Narrative events apply effects (flags, items, scene triggers).
- Game time advances if DM accepts a suggestion or manually sets time.

## End-to-End Flow (Message Perspective)

```
[Player Action]
        |
        v
[Engine builds context from graph]
        |
        v
[LLM suggests response/tools]
        |
        v
[DM approval decision]
        |
        v
[State update + broadcast]
        |
        v
[Player UI updates scene]
```
