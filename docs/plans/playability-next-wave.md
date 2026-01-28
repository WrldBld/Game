**Created:** January 27, 2026
**Status:** Draft
**Owner:** OpenCode
**Scope:** Next wave playability plan (validated)

---

## Validated Gaps (summary)

### Dialogue
- Conversation persistence is missing: `record_dialogue_context`, `get_dialogues_with_npc`, `update_spoke_to` are no‑ops.
- Conversation scoping is partial: UI does not filter by `conversation_id` and approvals/LLMProcessing lack `conversation_id`.

### Staging + Visual State
- Staging approval uses world defaults; location defaults not respected (`use_llm_presence`, TTL).
- Visual state soft‑rule LLM evaluation missing; details service mismatch fixed but pre‑stage still lacks selection.

### Challenge
- Skill binding not persisted (`skill_id` dropped on create/update).
- Partial/critical outcomes not supported in CRUD/UI.
- Stat updates not broadcast to clients.
- Region availability edges not implemented.

### Lore
- Player lore pipeline missing (translator + services + UI wiring).
- Lore linking (ABOUT_* edges) and COMMON_LORE not implemented.
- LLM discover lore tool not wired.

### Creator/Character/NPC
- EntityChanged refresh doesn’t handle Relationship changes.
- Director still uses direct message builder for NPC disposition/relationship.

---

## Prioritized Execution (backend → frontend → review)

1) **Dialogue persistence + scoping** (P0)
   - Implement conversation graph persistence in narrative_repo and use cases.
   - Add conversation_id to approval + LLMProcessing messages.
   - UI filter dialogue state by active conversation_id.

2) **Staging defaults + LLM presence gating** (P1)
   - Use location staging defaults (TTL + use_llm_presence) in request_approval.
   - If use_llm_presence=false, skip LLM suggestions.

3) **Challenge skill binding + stat sync** (P1)
   - Persist skill_id/check_stat in challenge CRUD.
   - Broadcast CharacterStatUpdated and handle in client.

4) **Lore player pipeline** (P2)
   - Translate lore messages to PlayerEvents; add LoreService; wire journal UI.

5) **Creator/NPC refresh** (P2)
   - Route Relationship EntityChanged into refresh signals.
   - Replace direct message builder usage with NpcService in director UI.

---

## Progress

- [x] P0 Dialogue persistence + scoping complete (backend)
  - ConversationSpeaker enum added for type-safe speaker identification
  - create_conversation uses MATCH/MERGE (fail-fast on missing nodes)
  - add_dialogue_turn auto-computes order (turn_counter)
  - DialogueTurn stores game_time as epoch seconds (i64)
  - Optional context edges (IN_SCENE/AT_LOCATION/AT_REGION) fixed
  - Backend review + checks complete
- [x] P0 Dialogue persistence + scoping complete (frontend)
  - ApprovalRequired and LLMProcessing include conversation_id
  - Dialogue state scoped by conversation_id
  - ConversationEnded clears only matching conversation
- [x] P1 Staging defaults + LLM gating complete (backend)
  - Added optional `presence_cache_ttl_hours` and `use_llm_presence` to `StagingApprovalInput`
  - Location defaults are fetched and merged with input overrides (input takes precedence)
  - LLM suggestions are gated based on effective `use_llm_presence` flag
  - Effective TTL is used in result instead of settings default
  - Added unit tests:
    - `test_llm_presence_disabled_skips_llm_calls`: Verifies LLM port is not called when disabled
    - `test_location_defaults_applied_when_input_omitted`: Verifies location defaults are applied
    - `test_input_overrides_take_precedence`: Verifies input overrides take precedence over location defaults
- [x] P1 Staging defaults + LLM gating complete (review)
  - Added validation for `presence_cache_ttl_hours` (effective value must be > 0)
  - Added unit tests for invalid TTL (zero and negative values)
  - Mapped `StagingError` to proper `ErrorCode` in `ws_movement.rs` (ValidationError, NotFound)
  - Fixed `staging_game_time_to_protocol` to guard against day=0 causing negative total_seconds
- [ ] P1 Staging defaults + LLM gating complete (frontend)
- [ ] P1 Challenge skill binding + stat sync complete
- [ ] P2 Lore pipeline complete
- [ ] P2 Creator/NPC refresh complete
- [x] Code review + architecture review
- [x] Checks/tests clean
