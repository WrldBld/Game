# WrldBldr Gameplay + AI Proposal

## Purpose

This proposal defines a cohesive gameplay experience for WrldBldr that aligns with the system specifications in:

- `docs/systems/scene-system.md`
- `docs/systems/staging-system.md`
- `docs/systems/dialogue-system.md`
- `docs/systems/challenge-system.md`
- `docs/systems/narrative-system.md`
- `docs/systems/navigation-system.md`
- `docs/systems/visual-state-system.md`
- `docs/systems/game-time-system.md`
- `docs/systems/npc-system.md`

The intent is to describe a playable, DM-in-the-loop loop that feels like a visual novel, while preserving DM authority over AI output.

## Core Vision

- Visual novel presentation with strong scene continuity and cinematic pacing.
- DM-in-the-loop AI collaboration where all AI outputs are advisory until approved.
- A tight gameplay loop: enter region -> scene resolves -> staging/approval -> dialogue -> challenge/event -> time and state update -> repeat.
- Multi-system challenge resolution that maps cleanly to D20, D100, and narrative systems.

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
