# Code Review Remediation Plan

**Created:** 2025-12-19
**Status:** In Progress
**Estimated Effort:** ~13 hours

This plan addresses findings from a comprehensive code review of the Engine and Player codebases, wiring unwired code to features rather than simply deleting it.

---

## Summary of Decisions

| Item | Decision |
|------|----------|
| PresenceService | Wire with LLM + rule-based, DM confirmation via existing approval popup |
| GoalRepositoryPort | Implement (part of Actantial Model) |
| Use Cases module | Keep as architectural blueprint |
| Mobile/Compact views | Future - remove unused components for now |
| Asset storage | World data directory `./assets/` |
| Commit granularity | More granular commits |

---

## Phase 1: Critical Cleanup
**Status:** COMPLETE
**Estimated:** 30 minutes

### 1.1 Delete `websocket_handlers/` module
- **Location:** `crates/engine/src/infrastructure/websocket_handlers/`
- **Issue:** Declares 6 non-existent submodules (compilation would fail)
- **Action:** Delete entire directory and remove from `infrastructure/mod.rs`

### 1.2 Remove empty `shared/` module
- **Location:** `crates/player/src/presentation/components/shared/`
- **Action:** Delete module and remove from `components/mod.rs`

### 1.3 Remove unused Player components
- `LoadingBackdrop` (`visual_novel/backdrop.rs:64`) - never used
- `CompactActionPanel` (`action_panel.rs:237`) - mobile view deferred

### 1.4 Fix architecture violation
- **Location:** `crates/player/src/presentation/services.rs:74-88`
- **Issue:** 15 type aliases directly import `infrastructure::http_client::ApiAdapter`
- **Action:** Documented as approved violation with justification in services.rs header

---

## Phase 2: Wire SkillsDisplay Component
**Status:** COMPLETE
**Feature:** Challenge System (US-CHAL-009)
**Estimated:** 1 hour

### Implementation
- Added `on_skills` prop to ActionPanelProps
- Added Skills button with sword icon to ActionPanel
- Added skills panel state signals to PCView
- Added on_skills handler that loads world skills via skill service
- Added SkillsDisplay modal rendering with loading state
- Exported SkillsDisplay from tactical module

### Files Modified
- `crates/player/src/presentation/components/action_panel.rs`
- `crates/player/src/presentation/components/tactical/mod.rs`
- `crates/player/src/presentation/views/pc_view.rs`

---

## Phase 3: Wire PresenceService for NPC System
**Status:** Pending
**Feature:** NPC Presence (US-NPC-001 through US-NPC-005)
**Estimated:** 3 hours

### Architecture
```
Player enters region → Engine calculates presence → 
  Rule-based: Deterministic NPCs (workers, residents)
  LLM-assisted: Probabilistic NPCs (frequents_sometimes)
→ Both go to DM approval (existing popup) → DM confirms/modifies → NPCs appear in scene
```

### Tasks
1. Add PresenceService to AppState
2. Integrate with existing approval popup (ApprovalRequired message)
3. Add approval message handler for presence decisions
4. Integrate with scene resolution
5. Replace simple presence helper with service

### New Protocol Messages
```rust
// Reuse existing ApprovalRequired with new approval_type
ApprovalType::PresenceSuggestion {
    region_id: String,
    region_name: String,
    suggested_npcs: Vec<NpcPresenceSuggestion>,
}

struct NpcPresenceSuggestion {
    npc_id: String,
    npc_name: String,
    reasoning: String,
    source: PresenceSource,  // Rule | LLM
    confidence: f32,
}
```

---

## Phase 4: Complete Challenge Outcome Triggers
**Status:** Pending
**Feature:** Challenge System
**Estimated:** 3 hours

### Tasks
1. Implement EventEffectExecutor integration for OutcomeTrigger
2. Map OutcomeTrigger variants to EventEffect
3. Execute triggers after DM approval
4. Add trigger execution to narrative flow

### Trigger Mapping
| OutcomeTrigger | Implementation |
|----------------|----------------|
| RevealInformation | Add to observation + set flag |
| EnableChallenge | Update challenge active status |
| DisableChallenge | Update challenge active status |
| GiveItem | Add to player inventory (POSSESSES edge) |
| TriggerScene | Send scene change message |
| ModifyCharacterStat | Update character sheet |
| Custom | Log for DM, no auto-execution |

---

## Phase 5: Complete Asset Generation Download
**Status:** Pending
**Feature:** Asset System (US-AST-002)
**Estimated:** 2 hours

### Tasks
1. Get image URLs from ComfyUI response
2. Download images to `{world_data_dir}/assets/`
3. Create GalleryAsset records in Neo4j
4. Generate thumbnails
5. Send GenerationComplete with actual URLs

---

## Phase 6: Repository Port Implementation
**Status:** Pending
**Estimated:** 2 hours

### 6.1 Implement `ItemRepositoryPort`
**Feature:** Character Inventory (US-CHAR-007, US-CHAR-009)

### 6.2 Implement `GoalRepositoryPort`
**Feature:** Actantial Model (wants targeting abstract goals)

### 6.3 Remove `GridMapRepositoryPort`
**Feature:** Tactical Combat (Tier 5 - deferred)

### 6.4 Add `ObservationRepositoryPort`
**Feature:** Architecture consistency

---

## Phase 7: Use Cases Module Annotation
**Status:** Pending
**Estimated:** 15 minutes

Keep as architectural blueprint with clear documentation header.

---

## Phase 8: Additional Cleanup
**Status:** Pending
**Estimated:** 30 minutes

- Remove unused DMApprovalQueueService methods
- Review dead code annotations
- Documentation hygiene

---

## Phase 9: Verification & Documentation
**Status:** Pending
**Estimated:** 30 minutes

- Verify compilation of all crates
- Update ACTIVE_DEVELOPMENT.md
- Update ROADMAP.md if needed

---

## Progress Log

| Date | Phase | Task | Status |
|------|-------|------|--------|
| 2025-12-19 | - | Plan created | Done |
| 2025-12-19 | 1.1 | Delete websocket_handlers/ module | Done |
| 2025-12-19 | 1.2 | Remove empty shared/ module | Done |
| 2025-12-19 | 1.3 | Remove unused LoadingBackdrop and CompactActionPanel | Done |
| 2025-12-19 | 1.4 | Document architecture violation in services.rs | Done |
| 2025-12-19 | 1 | **Phase 1 Complete** | Done |
| 2025-12-19 | 2 | Wire SkillsDisplay to PC view | Done |
| 2025-12-19 | 2 | **Phase 2 Complete** | Done |

---

## Related Documentation

- [ACTIVE_DEVELOPMENT.md](./ACTIVE_DEVELOPMENT.md) - Current sprint work
- [ROADMAP.md](./ROADMAP.md) - Overall progress
- [MVP.md](./MVP.md) - Acceptance criteria
