# DM Workflows

## Overview

DM workflows mirror the approval-first philosophy across staging, dialogue, challenges, and narrative events, with game time as a pacing control.

## 1. Staging Approval (NPC Presence + Visual State)

```
[Player enters unstaged region]
        |
        v
[DM receives staging request]
        |
        v
[Review rule-based vs LLM suggestions]
        |
        v
[Adjust NPC list + TTL + visual state override]
        |
        v
[Approve staging]
```

Key decisions:

- Choose rule-based or LLM presence suggestions.
- Adjust NPC list (including hidden NPCs).
- Select visual state overrides when needed.
- Set TTL for how long this staging remains valid.

## 2. Dialogue Approval

```
[Player speaks to NPC]
        |
        v
[LLM generates response + tool suggestions]
        |
        v
[DM accepts / modifies / rejects / take over]
        |
        v
[Approved dialogue sent to player]
```

Key decisions:

- Keep responses short for tempo unless dramatic emphasis is desired.
- Approve or reject tool calls (reveal info, give item, change relationship).
- Inject directorial notes to steer tone and pacing.

## 3. Challenge Workflow

```
[AI or DM proposes challenge]
        |
        v
[DM approves and triggers]
        |
        v
[Player rolls]
        |
        v
[DM approves outcome narrative]
        |
        v
[Effects apply and scene continues]
```

Key decisions:

- Choose when to interrupt with a challenge.
- Modify DC or narrative outcomes to fit the moment.
- Approve outcome text to preserve tone and stakes.

## 4. Narrative Event Workflow

```
[System/LLM detects trigger]
        |
        v
[DM sees trigger suggestion]
        |
        v
[Approve + select outcome]
        |
        v
[Effects execute and broadcast]
```

Key decisions:

- Timing: fire now or hold for later.
- Outcome selection: success, failure, or branch selection.

## 5. Game Time Control

```
[Action suggests time cost]
        |
        v
[DM approves / modifies / skips]
        |
        v
[Game time updated]
```

Key decisions:

- Keep time progression consistent with narrative pacing.
- Pause or set time manually for scene control.

## 6. Managing AI Collaboration

- Use directorial notes to set tone, constraints, or thematic goals.
- Regenerate LLM suggestions with targeted guidance when needed.
- Override AI for pivotal moments using take over.

## Cadence Summary

- **Fast approvals** for routine dialogue and staging in low stakes scenes.
- **Deliberate approvals** for narrative events and challenge outcomes.
- **Time approvals** after meaningful actions to preserve rhythm.
