# Content Generation Philosophy

## AI Collaboration Model

WrldBldr uses AI as a proposer, not an executor. All AI output is advisory until the DM approves. This mirrors the Dialogue System and Staging System design.

## What AI Generates

- NPC dialogue responses (short, contextual)
- Suggested challenges and narrative event triggers
- Staging recommendations (who is present)
- Visual state suggestions (when soft rules are used)

## What AI Never Does Without Approval

- Executes tool calls (items, flags, relationship changes)
- Triggers events or challenges
- Applies outcomes or time changes

## Context Assembly (Based on System Docs)

```
[World + Location + Region]
        |
        v
[Active events + recent dialogue]
        |
        v
[NPC wants + relationships + schedules]
        |
        v
[Scene + visual state + time of day]
        |
        v
[LLM prompt]
```

Key sources:

- Dialogue context includes wants, relationships, location, narrative events.
- Staging context includes region, time of day, active events, recent NPC interactions.
- Visual state resolution uses activation rules and optional LLM evaluation.

## Response Shape and Constraints

- Dialogue responses default to 1-3 sentences.
- Tool calls must be explicit and individually approved.
- LLM reasoning is visible to the DM and hidden from players.
- Token budgets are enforced per context category; overflow is summarized.

## Regeneration and DM Steering

- DM can regenerate LLM outputs with guidance.
- DM can take over and author final text.
- Directorial notes act as persistent context for tone and pacing.

## Failure Handling

- If LLM fails, fall back to rule-based staging or DM-authored responses.
- If AI suggestions are rejected repeatedly, DM can proceed manually.
