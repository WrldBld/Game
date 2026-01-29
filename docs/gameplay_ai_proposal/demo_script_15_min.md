# 15-Minute Demo Script

## Scenario Setup

- Location: Rusty Anchor Tavern
- Region: Bar Counter
- NPCs: Marcus (bartender), Old Sal (regular)
- Goal: Learn about a smuggler and unlock a dock lead

## Timeline

### 0:00 - 2:00: Join and Stage

- Player joins world and appears at Bar Counter.
- Staging request appears for DM with rule-based and LLM options.
- DM selects Marcus and Old Sal, sets TTL to 3 hours, approves visual state "Evening".
- Player sees scene resolve with tavern backdrop and NPC sprites.

### 2:00 - 5:00: First Dialogue

- Player selects Talk -> Marcus.
- LLM generates a short response about the smuggler rumor.
- DM approves response and a tool suggestion: RevealInfo("dock meeting at night").
- Player sees response in dialogue box.

### 5:00 - 7:00: Choice and Hook

- LLM proposes two dialogue choices.
- Player chooses "Can you introduce me?"
- DM approves a brief response that sets a narrative event trigger.
- Narrative event suggestion appears for "Marcus arranges a contact." DM approves.

### 7:00 - 10:00: Challenge Moment

- DM triggers a Persuasion challenge to convince Old Sal to share details.
- Player rolls; result is shown.
- DM approves the outcome text: partial success with a complication.
- Outcome applies a flag: "owed_favor".

### 10:00 - 12:00: Time and Mood Shift

- System suggests time +1 hour for the conversation.
- DM approves time suggestion to advance toward night.
- Visual state remains in "Evening"; atmosphere text updates slightly.

### 12:00 - 14:00: Movement Teaser

- Player selects Travel -> Exit to Docks.
- Staging request appears for Docks region.
- DM accepts rule-based staging with a hidden NPC (the smuggler).

### 14:00 - 15:00: Cliffhanger

- Scene resolves at the docks.
- DM triggers an approach event: "A hooded figure gestures from the shadows."
- Player is left with a clear next action and a sense of forward momentum.

## Demo Success Criteria

- Every AI output is DM-approved before reaching the player.
- Scene transitions are smooth and readable.
- At least one challenge is rolled and resolved.
- At least one narrative event is approved and executed.
- Time advancement is visible and contextual.
