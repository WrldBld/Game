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
    - Fix ToolArgs::UpdateCharacterStat.character_id type (CharacterId → PlayerCharacterId).
    - Persist stat updates via PlayerCharacterRepo::modify_stat with return values.
    - Extend StatUpdate struct with old_value, delta, and character_name.
    - Remove repo lookup and computed delta from ws_approval handler.
    - Add TODO comment for N+1 skill_id query in ChallengeOps::list.

4) **Lore player pipeline** (P2)
   - Translate lore messages to PlayerEvents; add LoreService; wire journal UI.

5) **Creator/NPC refresh** (P2)
   - Route Relationship EntityChanged into refresh signals.
   - Replace direct message builder usage with NpcService in director UI.

---

## Progress

- [x] P1 Staging defaults + LLM gating complete (backend)
- [x] P1 Staging defaults + LLM gating complete (review)
- [x] P1 Staging defaults + LLM gating complete (frontend)
- [x] P1 Challenge skill binding + stat sync complete (backend)
- [x] P1 Challenge skill binding + stat sync complete (frontend)
  - Implemented `CharacterStatUpdated` message translation in `message_translator.rs`
  - Handler already existed in `session_message_handler.rs` (logs stat change + triggers refresh)
  - Challenge UI already displays skill name via skill_id-to-skill_name translation:
    - `ChallengeLibrary` builds `skills_map` from skills prop
    - `ChallengeList` displays skill name via `skills_map.get(&challenge.skill_id)`
    - `ChallengeEditor` has skill selector dropdown with skill_id binding
    - `ChallengeRollModal` displays `skill_name` from challenge prompt
- [ ] P2 Lore pipeline complete
- [ ] P2 Creator/NPC refresh complete
- [x] Code review + architecture review
- [x] Checks/tests clean
