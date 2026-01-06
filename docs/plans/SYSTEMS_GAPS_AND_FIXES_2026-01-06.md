# Systems / Testing Gaps & Fixes (2026-01-06)

Goal: a single checkbox-driven list of all discovered gaps (tests + implementation), ordered by impact, so we can track progress via completions.

## P0 — Correctness bugs (likely real user-visible failures)

- [x] **Staging WS request correlation is broken**
  - Symptom: `ServerMessage::StagingApprovalRequired` sends a random `request_id`, but engine `handle_staging_approval` parses the incoming `request_id` as a `RegionId`.
  - Impact: DM approvals can fail in real use unless client sends `region_id` as `request_id`.
  - Fix: store `request_id -> region_id` mapping when emitting `StagingApprovalRequired`; look up on approval. Keep backward-compat fallback (parse `request_id` as region UUID) if mapping missing.
  - Tests:
    - WS integration: DM approves using `request_id` and player receives `StagingReady` for the correct `region_id`.

- [x] **Staging `StagingReady.region_id` should be the real region id**
  - Symptom: `handle_staging_approval` currently uses `request_id` as `region_id` in `StagingReady`.
  - Impact: player UI/state can attach staging to the wrong region.
  - Fix: broadcast `region_id` as region UUID string.
  - Tests: covered by the WS integration test above.

- [ ] **Staging approval does not persist per-NPC properties**
  - Symptom: approval use case only stages NPC IDs (`stage_npc`), which (Neo4j impl) sets `is_hidden_from_players=false` and does not record `mood`/`reasoning` chosen at approval time.
  - Impact: hidden NPCs and DM-set moods/reasoning cannot survive beyond the broadcast; subsequent reads/joins can’t reconstruct them.
  - Fix direction:
    - Prefer: approval creates/saves a `domain::Staging` with NPC edge properties and activates it (`CURRENT_STAGING`).
    - Alternative: add repo methods to update edge properties for existing staging.
  - Tests:
    - Repo/integration: approve staging with hidden/mood/reasoning and verify subsequent `get_active_staging/get_staged_npcs` returns correct flags.

- [ ] **StagingApprovalRequired.game_time is real-time, not world game time**
  - Symptom: uses `Utc::now()` to populate `game_time` fields.
  - Impact: DM decisions refer to wrong in-world time.
  - Fix: populate from `world.game_time.current()`.
  - Tests: WS integration assertion on game_time content.

## P1 — Behavior gaps (missing tests for key flows)

- [ ] **Movement use cases have thin test coverage**
  - Missing tests:
    - `EnterRegion`: blocked movement (no path), wrong location, movement blocked by rule.
    - `ExitLocation`: similar invalid transitions.

- [ ] **Conversation start use case has no direct tests**
  - Missing tests:
    - Rejects when NPC not staged in player region.
    - Happy path enqueues correct `PlayerActionData` fields.

- [ ] **Approval (LLM suggestion) flow lacks end-to-end tests**
  - Missing tests:
    - DM Accept/Reject/Modify updates queue state correctly and produces expected broadcast(s).

- [ ] **PreStageRegion / StagingRegenerateRequest flows not covered**
  - Missing tests:
    - Pre-stage sets staging such that later player entry yields `Ready` without DM intervention.
    - Regenerate produces updated suggestions and doesn’t mutate active staging.

## P2 — Quality / regression prevention

- [ ] **Protocol crate lacks serde regression tests for high-traffic messages**
  - Add minimal serde round-trip tests for staging/time/approval message enums.

- [ ] **Player services tests are placeholder-level**
  - Example: some tests only assert “doesn’t panic” rather than validating payload correctness.
  - Upgrade: assert `RequestPayload` values and response parsing behavior.

## Notes / Constraints

- Hexagonal architecture: only infra boundaries use traits; internal engine code stays concrete.
- Prefer small, high-signal tests that fail for real regressions.
