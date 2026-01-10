# Issue Triage and Implementation Plan

**Date:** 2026-01-10
**Updated:** 2026-01-10 (after reviewer validation)
**Total Open Issues:** 22 (#9-#30)

## Issue Categories

### Category A: Addressable Now (14 issues)
Small to medium fixes that don't require new systems or major design work.

### Category B: Requires Design/Discussion (2 issues)
Need architectural decisions before implementation.

### Category C: Major Features - Blocked (6 issues)
Large features that depend on systems not yet implemented.

---

## Category A: Addressable Now

### PR Group 1: Navigation System Fixes
**Issues:** #17, #18, #19
**Effort:** Medium (2-3 days)
**Theme:** Error handling and validation in navigation/exit system

| Issue | Title | Type | Summary |
|-------|-------|------|---------|
| #17 | Return errors for missing navigation exits | Enhancement | Silent `tracing::warn` + skip → return errors |
| #18 | Add bidirectional exit validation | Enhancement | Validate return path exists for bidirectional exits |
| #19 | Replace .ok() in SceneChangeBuilder | Enhancement | 5+ `.ok()` calls silently ignore errors |

**Dependencies:**
- #18 is independent (creation-time validation)
- #17 → #19: If `get_exits()` return type changes, `scene_change.rs:96` must update
- Can implement #18 separately; #17 and #19 should be done together

**Files:** `location.rs`, `region.rs`, `scene_change.rs`

---

### PR Group 2: Lore System Improvements
**Issues:** #15, #16
**Effort:** Medium (2-2.5 days)
**Theme:** Chunk ordering and partial revocation

| Issue | Title | Type | Summary |
|-------|-------|------|---------|
| #15 | Add lore chunk order validation | Enhancement | No validation of unique/sequential orders, no re-index on delete |
| #16 | Implement partial lore knowledge revocation | Enhancement | Grant supports partial, revoke doesn't - asymmetric API |

**Dependencies:** None - these are independent (different concerns: ordering vs knowledge tracking)
**Files:** `use_cases/lore/mod.rs`, `ws_lore.rs`, `lore_repo.rs`, `ports.rs`

---

### PR Group 3: Staging System Improvements
**Issues:** #9, #11
**Effort:** Medium (2-2.5 days)
**Theme:** Configuration and NPC matching

| Issue | Title | Type | Summary |
|-------|-------|------|---------|
| #9 | Make staging TTL values configurable | Enhancement | 2 hardcoded TTLs (30s timeout, 24h TTL) → settings |
| #11 | Fix name mismatch in LLM NPC matching | Bug | Case-insensitive but no whitespace normalization |

**#11 Implementation:** Strict normalized matching (trim, lowercase, collapse whitespace).
The LLM prompt already asks for "exact name from the list" - fix enables compliance.

**Note:** #10 (LLM failure handling) deferred - requires more design for retry/circuit breaker patterns.

**Files:** `staging/mod.rs`, `settings.rs`

---

### PR Group 4: Visual State Fixes
**Issues:** #20, #21
**Effort:** Medium (2 days)
**Theme:** Featured characters and error handling

| Issue | Title | Type | Summary |
|-------|-------|------|---------|
| #20 | Store featured character role properly | Enhancement | Role hardcoded to 'Secondary', never read back |
| #21 | Add error handling for character relationships | Bug | Silent `unwrap_or()` on relationship data |

**Files:** `scene_repo.rs`, `character_repo.rs`, `ports.rs`

---

### PR Group 5: Challenge & Narrative Fixes
**Issues:** #12, #14
**Effort:** Small-Medium (1-2 days)
**Theme:** Queue cleanup and effect validation

| Issue | Title | Type | Summary |
|-------|-------|------|---------|
| #12 | Fix silent queue cleanup failure | Bug | `mark_complete()` errors logged but not propagated |
| #14 | Validate combat/reward effects before execution | Enhancement | Invalid params accepted, fail silently at runtime |

**Files:** `challenge/mod.rs`, `narrative/execute_effects.rs`

---

### PR Group 6: Conversation Testing
**Issues:** #13
**Effort:** Small (1 day)
**Theme:** Test coverage

| Issue | Title | Type | Summary |
|-------|-------|------|---------|
| #13 | Add tests for ContinueConversation and EndConversation | Enhancement | Zero test coverage for these use cases |

**Files:** `conversation/continue_conversation.rs`, `conversation/end.rs`

---

### PR Group 7: TimeContext Integration (NEW)
**Issues:** #23
**Effort:** Small (1-2 days)
**Theme:** Enable time-based trigger evaluation

| Issue | Title | Type | Summary |
|-------|-------|------|---------|
| #23 | Implement Event/Custom TimeContext | Enhancement | `TriggerContext.time_context` exists but never populated |

**Implementation:** Pass `GameTime` through trigger evaluation pipeline, convert to `time_of_day().display_name()`.
Infrastructure already exists - just needs wiring.

**Files:** `narrative.rs`, `enter_region.rs`, `exit_location.rs`

---

## Category B: Requires Design/Discussion

| Issue | Title | Blocker |
|-------|-------|---------|
| #10 | Improve LLM failure handling in staging | Needs design: retry strategy, circuit breaker, timeout policy |
| #22 | Custom scene conditions with LLM | Needs design: condition expression schema, LLM prompt format |
| #28 | Custom trigger for narrative events | Needs design: trigger expression schema |

---

## Category C: Major Features - Blocked

These require new systems that don't exist yet:

| Issue | Title | Blocked By |
|-------|-------|------------|
| #24 | Skill system for challenges | Requires character skill system |
| #25 | RelationshipThreshold trigger | Requires relationship level tracking |
| #26 | StatThreshold trigger | Requires character stat system |
| #27 | CombatResult trigger | Blocked by #29 (combat system) |
| #29 | Combat system | Major feature - needs full design |
| #30 | Reward/XP system | Major feature - needs inventory + leveling |

---

## Recommended Implementation Order

Based on dependencies, effort, and impact:

### Phase 1: Quick Wins (1-2 PRs)
1. **PR: Challenge & Narrative Fixes** (#12, #14)
   - Small, isolated fixes
   - Improves reliability

2. **PR: Conversation Testing** (#13)
   - Adds coverage for existing code
   - No behavior changes

### Phase 2: Core Improvements (2-3 PRs)
3. **PR: Navigation System Fixes** (#17, #18, #19)
   - High impact on user experience
   - Fixes silent failures

4. **PR: Staging System Improvements** (#9, #11)
   - Configuration flexibility
   - Fixes NPC matching bug

### Phase 3: Feature Completeness (2 PRs)
5. **PR: Lore System Improvements** (#15, #16)
   - API symmetry
   - Data integrity

6. **PR: Visual State Fixes** (#20, #21)
   - Data correctness
   - Error visibility

---

## Summary

| Category | Count | Addressable |
|----------|-------|-------------|
| A: Addressable Now | 14 | Yes |
| B: Needs Design | 3 | After discussion |
| C: Major Features | 5 | No (blocked) |
| **Total** | **22** | **14 ready** |

**Recommended first PR:** #12 + #14 (Challenge & Narrative Fixes)
- Small scope, high impact
- Fixes data integrity issues
- Good validation of the PR workflow

---

## Reviewer Validation Notes (2026-01-10)

### Validated Claims
- ✓ #15 and #16 are independent (different concerns)
- ✓ #23 should move from "Blocked" to "Addressable" (infrastructure exists)
- ✓ #9 effort estimate 2-2.5 days with #11
- ✓ #13 effort estimate 1-1.5 days

### Corrected Claims
- ✗ #17, #18, #19 are NOT fully independent: #18 is independent, but #17→#19 have coupling
- ✗ #11 does NOT need design decision: strict normalized matching is the clear path
