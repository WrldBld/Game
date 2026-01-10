# Issue Triage and Implementation Plan

**Date:** 2026-01-10
**Updated:** 2026-01-10 (post PR #31, #32, #33 merge)
**Total Open Issues:** 16 (down from 22)
**Closed This Session:** #12, #13, #14, #17, #18, #19

## Current Status

### Completed PRs
| PR | Issues Fixed | Status |
|----|--------------|--------|
| #31 | #12, #14 | Merged - Challenge & Narrative fixes |
| #32 | #13 | Merged - Conversation test coverage |
| #33 | #17, #18, #19 | Merged - Navigation error handling |

---

## Remaining Issues by Category

### Category A: Addressable Now (7 issues)

| Issue | Title | Type | Complexity |
|-------|-------|------|------------|
| #9 | Make staging TTL values configurable | Enhancement | Small |
| #11 | Fix name mismatch in LLM NPC matching | Bug | Small |
| #15 | Add lore chunk order validation | Enhancement | Medium |
| #16 | Implement partial lore knowledge revocation | Enhancement | Medium |
| #20 | Store featured character role properly | Enhancement | Medium |
| #21 | Add error handling for character relationships | Bug | Medium |
| #23 | Implement Event/Custom TimeContext | Enhancement | Small |

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

### PR Group 1: Staging System Improvements (#9, #11)
**Effort:** Small-Medium
**Theme:** Configuration and NPC matching bug fix

| Issue | Summary | Files |
|-------|---------|-------|
| #9 | Replace hardcoded 30s timeout and 24h TTL with settings | `staging/mod.rs:32,202,1022` |
| #11 | Add whitespace normalization to LLM NPC name matching | `staging/mod.rs:946` |

**Implementation Details:**

**#9 - TTL Configuration:**
- Settings already exist: `staging_timeout_seconds` (line 119), `default_presence_cache_ttl_hours` (line 110)
- Replace `DEFAULT_STAGING_TIMEOUT_SECONDS` constant usage
- Replace hardcoded `24` in `default_ttl_hours` and `ttl_hours`
- Fetch from world settings in `RequestStagingApproval` and `AutoApproveStagingTimeout`

**#11 - Name Normalization:**
- Current: `c.name.to_lowercase() == suggestion.name.to_lowercase()`
- Fix: Add normalize function: `trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")`
- Handles: extra spaces, leading/trailing whitespace, multiple consecutive spaces

---

### PR Group 2: Lore System Improvements (#15, #16)
**Effort:** Medium
**Theme:** Chunk ordering and API symmetry

| Issue | Summary | Files |
|-------|---------|-------|
| #15 | Validate chunk order uniqueness, re-index on delete | `use_cases/lore/mod.rs:133-227` |
| #16 | Add partial revocation to match partial grant | `lore_repo.rs`, `ports.rs`, `ws_lore.rs:316-356` |

**Implementation Details:**

**#15 - Order Validation:**
- `add_chunk()`: Validate order is unique, or auto-assign next sequential
- `update_chunk()`: Validate new order doesn't conflict
- `delete_chunk()`: Re-index remaining chunks to maintain sequential order (0,1,2...)

**#16 - Partial Revocation:**
- Add `remove_chunks_from_knowledge()` to `LoreRepo` trait (ports.rs)
- Implement in `lore_repo.rs` (similar to `add_chunks_to_knowledge`)
- Add `LoreKnowledge::remove_chunks()` method to domain
- Update `revoke_knowledge()` in use case to accept `chunk_ids: Option<Vec<LoreChunkId>>`
- Remove explicit rejection in `ws_lore.rs:325-331`

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

### PR Group 4: TimeContext Integration (#23)
**Effort:** Small
**Theme:** Enable time-based trigger evaluation

| Issue | Summary | Files |
|-------|---------|-------|
| #23 | Wire GameTime through trigger evaluation to populate time_context | `narrative.rs:577` |

**Implementation Details:**
- In `evaluate_triggers_for_world()`, fetch current scene's TimeContext
- Convert `TimeContext` to string: `"Morning"`, `"Afternoon"`, `"Evening"`, `"Night"`, or custom
- Set `time_context: Some(time_string)` in TriggerContext construction
- Infrastructure exists - just needs wiring

---

## Recommended Implementation Order

### Next PR: Staging System Improvements (#9, #11)
**Rationale:**
- Smallest scope, highest confidence
- #11 is a bug fix (higher priority)
- Settings infrastructure already exists
- Isolated changes, low risk

### Then: TimeContext Integration (#23)
**Rationale:**
- Small, isolated change
- Enables time-based triggers to work
- Infrastructure already exists

### Then: Lore System Improvements (#15, #16)
**Rationale:**
- Medium complexity
- Independent from other systems
- Improves API consistency

### Finally: Visual State Fixes (#20, #21)
**Rationale:**
- Most complex of remaining issues
- #21 needs careful analysis of intentional vs accidental unwrap_or usage

---

## Summary

| Category | Count | Status |
|----------|-------|--------|
| A: Addressable Now | 7 | Ready to implement |
| B: Needs Design | 3 | Deferred |
| C: Major Features | 6 | Blocked |
| **Total Open** | **16** | **7 ready** |

**Next recommended PR:** #9 + #11 (Staging System Improvements)
- Small scope
- Bug fix included (#11)
- Clear implementation path
