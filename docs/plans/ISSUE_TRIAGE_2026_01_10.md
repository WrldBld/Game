# Issue Triage and Implementation Plan

**Date:** 2026-01-10
**Updated:** 2026-01-10 (post PR #38 merge)
**Total Open Issues:** 9 (down from 22)
**Closed This Session:** #9, #11, #12, #13, #14, #15, #16, #17, #18, #19, #20, #21, #23, #35

## Current Status

### Completed PRs
| PR | Issues Fixed | Status |
|----|--------------|--------|
| #31 | #12, #14 | Merged - Challenge & Narrative fixes |
| #32 | #13 | Merged - Conversation test coverage |
| #33 | #17, #18, #19 | Merged - Navigation error handling |
| #34 | #9, #11 | Merged - Staging TTL config & name normalization |
| #36 | #35 | Merged - Entity wrapper refactoring (Settings/Inventory) |
| #37 | #15, #16, #23 | Merged - TimeContext + Lore Improvements (chunk order, partial revocation, DB constraint) |
| #38 | #20, #21 | Merged - Visual state fixes + Design docs for LLM resilience |

---

## Remaining Issues by Category

### Category A: Addressable Now (0 issues)

All Category A issues have been completed.

| Issue | Title | Status |
|-------|-------|--------|
| ~~#15~~ | ~~Add lore chunk order validation~~ | ✅ Closed in PR #37 |
| ~~#16~~ | ~~Implement partial lore knowledge revocation~~ | ✅ Closed in PR #37 |
| ~~#20~~ | ~~Store featured character role properly~~ | ✅ Closed in PR #38 |
| ~~#21~~ | ~~Add error handling for character relationships~~ | ✅ Closed in PR #38 |
| ~~#23~~ | ~~Implement Event/Custom TimeContext~~ | ✅ Closed in PR #37 |

### Category B: Design Complete - Ready to Implement (3 issues)

Design documents created in PR #38: `docs/designs/LLM_RESILIENCE_AND_CUSTOM_EVALUATION.md`

| Issue | Title | Design Status |
|-------|-------|---------------|
| #10 | Improve LLM failure handling in staging | ✅ Designed - exponential backoff |
| #22 | Custom scene conditions with LLM | ✅ Designed - LLM evaluation with context hints |
| #28 | Custom trigger for narrative events | ✅ Designed - same pattern as #22 |

### Category C: Major Features - Blocked (6 issues)

| Issue | Title | Blocked By |
|-------|-------|------------|
| #24 | Skill system for challenges | Requires character skill system |
| #25 | RelationshipThreshold trigger | Requires relationship level tracking |
| #26 | StatThreshold trigger | Requires character stat system |
| #27 | CombatResult trigger | Blocked by #29 (combat system) |
| #29 | Combat system | Major feature - needs full design |
| #30 | Reward/XP system | Major feature - needs inventory + leveling |

---

## Recommended PR Groupings

### ~~PR Group 1: Staging System Improvements (#9, #11)~~ ✅ COMPLETED in PR #34

---

### ~~PR Group 2: Lore System Improvements (#15, #16, #23)~~ ✅ COMPLETED in PR #37

---

### PR Group 3: Visual State Fixes (#20, #21)
**Effort:** Medium
**Theme:** Featured characters and error handling

| Issue | Summary | Files |
|-------|---------|-------|
| #20 | Store/retrieve role from FEATURES_CHARACTER edge | `scene_repo.rs:304,502`, `ports.rs:425,429` |
| #21 | Replace silent unwrap_or() with proper error handling | `narrative_event.rs`, `execute_effects.rs` |

**Implementation Details:**

**#20 - Featured Character Role:**
- Update `SceneRepo::save()` to store role from `SceneCharacter` struct
- Update `SceneRepo::set_featured_characters()` to accept `&[SceneCharacter]` instead of `&[CharacterId]`
- Update port trait methods to use `SceneCharacter` with role
- Update `get_featured_characters()` to return `Vec<SceneCharacter>`

**#21 - Error Handling:**
- Review: Many `unwrap_or(false)` in trigger matching are **intentional** (missing data = condition not met)
- Focus on: `execute_effects.rs:558` where default relationship is silently created
- Add logging when defaults are used
- Consider if relationship creation should be explicit

---

### ~~PR Group 4: TimeContext Integration (#23)~~ ✅ COMPLETED in PR #37

### ~~PR Group 5: Visual State Fixes (#20, #21)~~ ✅ COMPLETED in PR #38

---

## Recommended Implementation Order

### ~~PR 1: Staging System Improvements (#9, #11)~~ ✅ COMPLETED in PR #34

### ~~PR 2: TimeContext + Lore Improvements (#23, #15, #16)~~ ✅ COMPLETED in PR #37

### ~~PR 3: Visual State Fixes (#20, #21)~~ ✅ COMPLETED in PR #38

### Next PR: LLM Resilience (#10)
**Rationale:**
- Design complete in `docs/designs/LLM_RESILIENCE_AND_CUSTOM_EVALUATION.md`
- Foundational for #22 and #28 (custom conditions/triggers need resilient LLM calls)
- Implementation: Create `ResilientLlmClient` wrapper with exponential backoff

**Implementation Plan:**
1. Create `infrastructure/resilient_llm.rs` with `ResilientLlmClient`
2. Add `RetryConfig` to settings
3. Wire `ResilientLlmClient` to wrap `OllamaClient` in main.rs
4. Add retry metrics/logging

### Future PRs: Custom Conditions (#22, #28)
**Rationale:**
- Depends on #10 for resilient LLM calls
- Same evaluation pattern for both issues
- Implementation: Create `CustomConditionEvaluator` use case

---

## Summary

| Category | Count | Status |
|----------|-------|--------|
| A: Addressable Now | 0 | ✅ All complete |
| B: Design Complete | 3 | Ready to implement |
| C: Major Features | 6 | Blocked |
| **Total Open** | **9** | **3 ready** |

**Next recommended PR:** #10 (LLM Resilience - Exponential Backoff)
- Design complete
- Foundational for #22, #28
- Medium complexity
