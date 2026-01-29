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
- [x] P2 Lore pipeline complete (backend)
- [x] P2 Lore pipeline complete (frontend)
  - Implemented `LoreDiscovered`, `LoreRevoked`, `LoreUpdated`, `CharacterLoreResponse` translation in `message_translator.rs`
  - Created `LoreService` with `get_character_lore` and `get_lore_entry` methods
  - Added `lore` to Services struct initialization
  - Added `apply_lore_journal` method to `LoreState`
  - Added `LoreService` to `LoreJournalEntryData` re-export
  - Added `use_lore_service` hook to presentation services
  - Added `get_character_lore` convenience re-export
  - Added `get_character_lore` and `get_lore_entry` to ClientMessageBuilder
  - Added Journal button to ActionPanel with "book" icon
  - Added Journal modal handler to PCView
  - Implemented `handle_open_lore_journal` with lore fetching and error handling
  - LoreJournal modal wiring complete with proper type conversions
- [x] P2 Creator/NPC refresh complete (backend)
- [x] P2 Creator/NPC refresh complete (frontend)
   - Normalized relationship entity_id format: `rel_npc_{npc_id}_pc_{pc_id}` for NPC-PC relationships
   - Normalized region relationship entity_id format: `rel_{type}_{char_id}_{region_id}` for home/work relationships
   - Updated `ws_npc.rs` to emit EntityChanged with consistent payload containing npc_id, pc_id, relationship, disposition, reason
   - Fixed broken test code in `creator_flow_npc.rs`
   - Added `npc_set_relationship_emits_entity_changed_broadcast` test
   - Added `npc_set_disposition_emits_entity_changed_broadcast_with_normalized_id` test
   - Updated existing tests to verify normalized entity_id format
- [x] P2 Creator/NPC refresh complete (frontend)
   - Added `npc_dispositions_refresh_counter: Signal<u32>` to GameState
   - Added `trigger_npc_dispositions_refresh()` method to GameState
   - Updated `trigger_entity_refresh()` to call `trigger_npc_dispositions_refresh()` for "Relationship" entity_type
   - Added `use_npc_service()` import to content.rs
   - Added `npc_service` hook to DirectorModeContent component
   - Added `dispositions_loading` signal for loading state
   - Subscribed to `npc_dispositions_refresh_counter` with `use_effect` to refetch dispositions when changed
   - Replaced `ClientMessageBuilder::set_npc_disposition` with `npc_service.set_disposition`
   - Replaced `ClientMessageBuilder::set_npc_relationship` with `npc_service.set_relationship`
   - Removed unused `command_bus_disposition` and `command_bus_rel` signals
- [ ] Code review + architecture review
- [x] Checks/tests clean
