# Playable MVP Plan

## MVP Definition
Goal: a reliable DM-in-the-loop visual novel demo where a player can enter a world, move between regions, see NPCs and backdrops, talk to an NPC, receive DM-approved dialogue, face a challenge, and see the outcome without getting stuck.

Minimum flows:
- Player joins world, selects PC, sees current scene.
- Player moves to a region, staging runs, DM approves, player receives scene change and NPCs.
- Player starts dialogue, DM approves response, player sees reply.
- DM triggers challenge, player rolls, DM approves, player sees result.

Non-negotiables for MVP:
- Staging approval completes the region transition for waiting PCs.
- Visual state defaults exist for worlds with no authored states.
- Demo world is seeded with content that guarantees the above flows.

## Critical Path
1. Fix staging completion and visual state defaults (blocks region movement).
2. Harden interaction auth and join identity (blocks safe multiplayer demo).
3. Seed demo world + ComfyUI assets (blocks playability without authoring UI).
4. Validate player UI stability (panic removal) and queue error visibility.
5. End-to-end walkthrough and verification.

## Blockers
P0 - Gameplay
- Staging approval does not complete player region transition.
- Staging approval requires active visual state, but authoring UI is missing.

P0 - Security / Access
- Interaction handler lacks world/PC ownership verification.
- Join flow trusts client-provided user_id.

P1 - Reliability
- Queue depth lookup errors are swallowed.
- processed_ids storage has no eviction path.
- Player UI conversion can panic on invalid outcomes.

## Architecture Fixes
These changes reduce coupling and avoid protocol boundary violations for MVP stability.

1) ADR-011 breach: use case returns wire-format DTOs
- Fix by introducing a use-case DTO for observation summaries and converting to protocol types at the API boundary.
- Files: `crates/engine/src/use_cases/management/observation.rs`, relevant API handler under `crates/engine/src/api/websocket/`.

2) Player/domain coupling
- Replace direct `wrldbldr_domain` imports in player with `wrldbldr_shared` re-exports or player-local DTOs.
- Files: `crates/player/src/ui/presentation/components/visual_novel/choice_menu.rs`, `crates/player/src/application/services/npc_service.rs`, `crates/player/src/ui/presentation/state/dialogue_state.rs`.

3) Domain UI leakage
- Decide whether `EventChain.color` is domain-significant; if UI-only, move to view model or a presentation DTO. If domain-significant, introduce a validated value object (ColorCode).
- Files: `crates/domain/src/aggregates/event_chain.rs` and any readers.

## Code Review Fixes
1) Interaction authorization
- Validate interaction ownership by world/PC before execution. Prefer a use case that enforces authorization centrally.
- File: `crates/engine/src/api/websocket/ws_conversation.rs`.

2) Queue error handling
- Replace `unwrap_or(1)` with explicit logging and a safe fallback or error response.
- File: `crates/engine/src/api/websocket/ws_conversation.rs`.

3) processed_ids eviction
- Replace HashSet with TTL cache or bounded LRU; prune on cleanup.
- File: `crates/engine/src/api/websocket/mod.rs`.

4) Join identity trust
- Ignore client user_id unless validated; use server-issued identity or token verification.
- File: `crates/engine/src/api/websocket/ws_session.rs`.

5) Player panic risk
- Change `ProtoAdHocOutcomes::new` to return Result and surface validation errors to UI.
- File: `crates/player/src/session_types.rs`.

## Gameplay Blockers
1) Staging completion
- On staging approval, send `SceneChanged` and `SceneUpdate` to waiting PCs using stored pending data, or trigger server-side re-enter region logic.
- Files: `crates/engine/src/api/websocket/ws_staging.rs`, `crates/engine/src/api/websocket/ws_movement.rs`, `crates/player/src/ui/presentation/handlers/session_message_handler.rs`.

2) Visual state defaults
- Allow staging approval when no visual states exist, or auto-create a default state per location/region. Seed defaults for demo worlds.
- Files: `crates/engine/src/use_cases/staging/approve.rs`, world seeding scripts.

## Demo World Seeding
Seed a stable demo world so the MVP is playable without creator tools.

Minimum content:
- 1 world, 1 location, 2 regions, 1 player character, 2 NPCs, 1 challenge.
- Default location/region visual states (backdrop + optional NPC sprites).
- At least 1 conversation starter and 1 interaction.

Seeding approach:
- Add a deterministic seed script or fixture that inserts the above content into Neo4j.
- Use stable IDs for repeatability in demo and tests.

## ComfyUI Image Seeding Workflow
Objective: generate consistent backdrops and NPC sprites for the demo world.

Inputs:
- Prompt inputs: location description, region description, NPC archetype, style guide.
- Optional negative prompts and seed values for consistency.

Outputs:
- Backdrops: one per location and region.
- NPC portraits/sprites: at least one per NPC.

Folder conventions (example):
- `assets/demo/worlds/<world_id>/backdrops/`
- `assets/demo/worlds/<world_id>/npcs/`
- `assets/demo/worlds/<world_id>/thumbnails/`

Workflow:
1) Prepare prompt inputs and optional fixed seeds.
2) Run ComfyUI generation for backdrops and NPCs.
3) Save outputs using the folder conventions above.
4) Update seeded world records to reference the asset paths.

## Tests and Verification
Do not run tests in this task; include verification steps for later execution.

Backend:
- Unit tests for interaction authorization and processed_ids eviction.
- WebSocket handler tests for staging completion and join identity validation.
- Use case tests for observation summary DTOs and staging approval fallback.

Frontend:
- Unit test for `ProtoAdHocOutcomes` error path.
- Manual QA: DM approves staging, player receives scene change and sees NPCs.

End-to-end:
- Demo world walkthrough covering join, move, stage, talk, challenge.

## Milestones and Sequencing
Sequencing rule: backend -> frontend -> review.

Milestone 1: Core backend blockers resolved
- Scope: staging completion, visual state defaults, interaction authorization, join identity trust.
- Acceptance criteria:
  - Staging approval results in `SceneChanged` and `SceneUpdate` for waiting PCs.
  - Worlds without authored visual states can still stage.
  - Interaction execution rejects cross-world or cross-PC access.
  - Join flow does not trust client user_id.
- Dependencies: none.

Milestone 2: Backend reliability and architecture fixes
- Scope: queue error handling, processed_ids eviction, ADR-011 breach fix, domain UI leakage decision.
- Acceptance criteria:
  - Queue count errors are visible and not silently swallowed.
  - processed_ids storage is bounded or TTL-evicted.
  - Use case returns a non-wire DTO for observations; conversion happens in API.
  - EventChain color decision implemented and documented in code.
- Dependencies: Milestone 1.

Milestone 3: Player frontend stability and boundary cleanup
- Scope: player/domain coupling removal, panic risk removal.
- Acceptance criteria:
  - Player no longer imports `wrldbldr_domain` in listed modules.
  - Invalid ad-hoc outcomes do not panic UI.
- Dependencies: Milestone 1 and Milestone 2 (for API contract alignment).

Milestone 4: Demo world + asset seeding
- Scope: demo world seed data and ComfyUI assets wired.
- Acceptance criteria:
  - Demo world exists with minimum content.
  - Visual states reference existing assets.
  - Staging can succeed with seeded defaults.
- Dependencies: Milestone 1 (staging completion + defaults) and Milestone 2 (observation DTO if used by demo tools).

Milestone 5: Review and playthrough verification
- Scope: end-to-end walkthrough and review against MVP definition.
- Acceptance criteria:
  - Player can join, move, see NPCs, talk, and face a challenge without getting stuck.
  - DM can approve staging and dialogue, and trigger challenge.
- Dependencies: Milestones 1-4.

## What We Will Not Do For MVP
- Full visual state authoring UI.
- Creator-mode CRUD for all content types.
- Comprehensive re-skinning or UI polish beyond stability fixes.
- Multi-world account security or SSO auth; MVP uses server-issued identity only.
- Large-scale content catalogs or procedural generation beyond the demo world.
