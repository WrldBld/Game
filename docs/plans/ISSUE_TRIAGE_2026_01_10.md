# Issue Triage and Implementation Plan

**Date:** 2026-01-10
**Updated:** 2026-01-10 (post PR #37 merge)
**Total Open Issues:** 11 (down from 22)
**Closed This Session:** #9, #11, #12, #13, #14, #15, #16, #17, #18, #19, #23, #35

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

---

## Remaining Issues by Category

### Category A: Addressable Now (2 issues)

| Issue | Title | Type | Complexity |
|-------|-------|------|------------|
| ~~#15~~ | ~~Add lore chunk order validation~~ | ~~Enhancement~~ | ✅ Closed in PR #37 |
| ~~#16~~ | ~~Implement partial lore knowledge revocation~~ | ~~Enhancement~~ | ✅ Closed in PR #37 |
| #20 | Store featured character role properly | Enhancement | Medium |
| #21 | Add error handling for character relationships | Bug | Medium |
| ~~#23~~ | ~~Implement Event/Custom TimeContext~~ | ~~Enhancement~~ | ✅ Closed in PR #37 |

### Category B: Requires Design/Discussion (3 issues)

| Issue | Title | Blocker |
|-------|-------|---------|
| #10 | Improve LLM failure handling in staging | Needs design: retry strategy, circuit breaker |
| #22 | Custom scene conditions with LLM | Needs design: condition expression schema |
| #28 | Custom trigger for narrative events | Needs design: trigger expression schema |

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

---

## Recommended Implementation Order

### ~~PR 1: Staging System Improvements (#9, #11)~~ ✅ COMPLETED in PR #34

### ~~PR 2: TimeContext + Lore Improvements (#23, #15, #16)~~ ✅ COMPLETED in PR #37

### Next PR: Visual State Fixes (#20, #21)
**Rationale:**
- Last addressable issues remaining
- #20 involves storing role on FEATURES_CHARACTER edge
- #21 needs careful analysis of intentional vs accidental unwrap_or usage

---

## Summary

| Category | Count | Status |
|----------|-------|--------|
| A: Addressable Now | 2 | Ready to implement |
| B: Needs Design | 3 | Deferred |
| C: Major Features | 6 | Blocked |
| **Total Open** | **11** | **2 ready** |

**Next recommended PR:** #20 + #21 (Visual State Fixes)
- Last addressable issues
- Medium complexity
- Clear implementation path
