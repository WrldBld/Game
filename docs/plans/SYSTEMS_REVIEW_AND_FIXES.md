# Systems Review and Fixes

Purpose: Capture gaps found by reviewing `docs/systems/*` first, then auditing implementation and tests in code. This file is a running log of discoveries and the follow-up plan.

## Scope
- Documentation-first review: `docs/systems/*.md`
- Code review targets: engine (WS/API + use cases + entities + repos), protocol, player as relevant
- Focus: implementation gaps, bugs, duplications, missing tests

## Discoveries (Docs →)

### docs/systems/

- [staging-system.md] Doc inconsistency: US-STG-007/008 are marked implemented (and point to UI files), but the later “Pre-Staging UI (Location View)” section says **Status: Pending**. Decide whether to update docs or complete missing UI.
- [staging-system.md] Spec says LLM staging suggestions should consider active narrative events + recent dialogues (“story-aware LLM”). Need to verify `engine/src/entities/staging.rs` prompt/context actually includes these.
- [staging-system.md] TTL language mixes “configurable TTL hours” with UI mock showing “until 10:30 PM game time”. Verify whether TTL is real-time vs game-time; align implementation + docs.
- [staging-system.md] Hidden-from-players NPCs “can still interact via DM-triggered approach events”. Verify the engine has an explicit DM-triggered path for hidden NPC interactions (or update docs).

- [game-time-system.md] Spec explicitly says “Staging TTL expires after in-game hours, not real minutes”. Verify staging expiry logic is keyed off `World.game_time` vs wall-clock (`Utc::now()`); if wall-clock is used, either adjust implementation or update docs.
- [game-time-system.md] Doc marks time suggestions (US-TIME-007) as pending, but the engine appears to support a DM approval flow (`GameTimeAdvanced` etc.) and we have WS integration tests around DM approving a time suggestion. Align docs vs reality.

- [dialogue-system.md] Doc notes OCCURRED_IN_SCENE / OCCURRED_AT graph edges are “defined but not yet created during save”. Verify current narrative persistence, decide whether to implement edge creation now or explicitly defer with tracking.

- [narrative-system.md] US-NAR-011/012 are documented as “effect type exists but execution stubs”. Verify stubs are explicit (return error) vs silently ignored; add targeted tests to prevent accidental no-op execution.

- [navigation-system.md] Claims `AdvanceGameTime` “invalidates presence cache”. Verify staging cache invalidation behavior exists (or clarify docs if TTL is purely time-based and doesn’t require explicit invalidation).
- [navigation-system.md] Notes `region_items` are “hardcoded to empty in build_prompt_from_action”. If true, this is a clear implementation gap for LLM context; decide whether to implement or keep deferred.
- [scene-system.md] Doc contradiction: US-SCN-009 is marked implemented above, but later the “Scene Entry Conditions Editor” section claims evaluation not implemented. Verify engine `Scene::resolve_scene()` actually evaluates conditions; update docs accordingly.
- [visual-state-system.md] Docs state LocationState is not implemented (only RegionState exists) and that staging approval UI doesn’t include resolved visual states yet. Track as explicit gap (likely larger than this review cycle).

## Discoveries (Code →)

- [crates/player-ui/src/presentation/components/dm_panel/location_staging.rs] Pre-staging location view exists, so staging docs “Status: Pending” is likely outdated. Implementation looks partial: `expires_in_hours` is hard-coded to `24.0` and there’s no explicit “Expired” state wired (only None/Pending/Active).

- Staging TTL semantics look consistent with docs: `EnterRegion` pulls `world_data.game_time.current()` and passes it through to staging resolution/expiry checks.

## Plan (Fixes + Tests)

## Implemented

- Updated scene system docs to remove contradiction about entry-condition evaluation (engine evaluates built-in conditions; UI is pending). See docs/systems/scene-system.md.
- Updated staging system docs to reflect that pre-staging UI + modal and the player staging-pending overlay exist (noting implementation is “basic”). See docs/systems/staging-system.md.

