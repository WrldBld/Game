# WrldBldr Gameplay + AI Proposal

## Purpose

Define a playable, DM-in-the-loop loop that feels like a visual novel while preserving DM authority over AI output.

## System References

- `docs/systems/scene-system.md`
- `docs/systems/staging-system.md`
- `docs/systems/dialogue-system.md`
- `docs/systems/challenge-system.md`
- `docs/systems/narrative-system.md`
- `docs/systems/navigation-system.md`
- `docs/systems/visual-state-system.md`
- `docs/systems/game-time-system.md`
- `docs/systems/npc-system.md`
- `docs/systems/session-system.md`

## Core Vision

- Visual novel presentation with strong scene continuity.
- DM-in-the-loop AI collaboration where AI outputs are advisory until approved.
- Tight loop: enter region -> scene resolves -> staging/approval -> dialogue -> challenge/event -> time update -> repeat.
- Multi-system challenge resolution for D20, D100, and narrative systems.

## DM Approval Cadence

- Staging approval on first entry to an unstaged region.
- Dialogue approval for each AI response.
- Challenge approval for prompt and outcome text.
- Narrative event approval for trigger execution.
- Time approval when actions imply time passage.

## Document Map

- `docs/gameplay_ai_proposal/session_flow.md`
- `docs/gameplay_ai_proposal/dm_workflows.md`
- `docs/gameplay_ai_proposal/player_workflows.md`
- `docs/gameplay_ai_proposal/visual_novel_presentation.md`
- `docs/gameplay_ai_proposal/content_generation.md`
- `docs/gameplay_ai_proposal/rule_systems_matrix.md`
- `docs/gameplay_ai_proposal/demo_script_15_min.md`

## Non-Goals

- No code changes.
- No schema or protocol changes.
- No new user stories beyond the referenced system docs.
