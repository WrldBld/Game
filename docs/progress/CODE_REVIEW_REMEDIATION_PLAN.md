# Code Review Remediation Plan

**Created:** 2025-12-19
**Status:** In Progress
**Estimated Effort:** ~25 hours (revised after Phase 3 expansion)

This plan addresses findings from a comprehensive code review of the Engine and Player codebases, wiring unwired code to features rather than simply deleting it.

---

## Summary of Decisions

| Item | Decision |
|------|----------|
| PresenceService | **Replaced by Staging System** - Full DM approval workflow with rule+LLM, pre-staging UI, configurable TTL |
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

## Phase 3: Staging System (Replaces PresenceService)
**Status:** COMPLETE (testing deferred)
**Feature:** NPC Presence with DM Approval Workflow
**Estimated:** 16.5 hours

> **Full details:** See [STAGING_IMPLEMENTATION_PLAN.md](./STAGING_IMPLEMENTATION_PLAN.md)

### Overview

The Staging System replaces the simple PresenceService with a comprehensive workflow:
- **DM always approves** NPC presence before players see them
- **Rule-based and LLM-based** options shown side-by-side
- **Pre-staging UI** for DMs to set up regions before players arrive
- **Configurable TTL** per location
- **Persistent staging history** in Neo4j
- **Background workflow** - player sees loading while DM approves

### Sub-Parts

| Part | Description | Est. | Status |
|------|-------------|------|--------|
| A | Dialogue Tracking Enhancement (dependency) | 2.5h | ✅ |
| B | Staging Domain (entities, value objects) | 2h | ✅ |
| C | Staging Infrastructure (repository, protocol) | 3h | ✅ |
| D | Staging Service (core logic) | 2h | ✅ |
| E | Engine Integration (WebSocket changes) | 1.5h | ✅ |
| F | Player UI (approval popup, pre-staging, settings) | 4.5h | ✅ |
| G | Finalization (cleanup done, testing deferred) | 1h | ✅ |

### Key Files to Create/Modify

**Engine:**
- `entities/staging.rs` (new)
- `value_objects/staging_context.rs` (new)
- `services/staging_service.rs` (new)
- `persistence/staging_repository.rs` (new)
- `websocket.rs` (modify for staging flow)

**Player:**
- `dm_panel/staging_approval.rs` (new)
- `dm_panel/location_staging.rs` (new)
- `views/pc_view.rs` (add StagingPending overlay)
- `creator/location_editor.rs` (add TTL settings)

**Protocol:**
- `messages.rs` (add staging messages)

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
| 2025-12-19 | 3 | Phase 3 planning complete | Done |
| 2025-12-19 | 3 | Created STAGING_IMPLEMENTATION_PLAN.md | Done |
| 2025-12-19 | 3 | Created staging-system.md | Done |
| 2025-12-19 | 3 | Updated npc-system.md | Done |
| 2025-12-19 | 3 | Updated dialogue-system.md | Done |
| 2025-12-19 | 3.A | Dialogue Tracking Enhancement | Done |
| 2025-12-19 | 3.B | Staging Domain | Done |
| 2025-12-19 | 3.C | Staging Infrastructure | Done |
| 2025-12-19 | 3.D | Staging Service | Done |
| 2025-12-19 | 3.E | Engine Integration | Done |
| 2025-12-19 | 3.F | Player UI (F1-F6 complete) | Done |
| 2025-12-19 | 3.G | Remove PresenceService, update docs | Done |
| 2025-12-19 | 3 | **Phase 3 Complete** | Done |

---

## Related Documentation

- [STAGING_IMPLEMENTATION_PLAN.md](./STAGING_IMPLEMENTATION_PLAN.md) - Detailed Phase 3 plan
- [staging-system.md](../systems/staging-system.md) - Staging system specification
- [ACTIVE_DEVELOPMENT.md](./ACTIVE_DEVELOPMENT.md) - Current sprint work
- [ROADMAP.md](./ROADMAP.md) - Overall progress
- [MVP.md](./MVP.md) - Acceptance criteria
