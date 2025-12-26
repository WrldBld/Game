# Code Quality Remediation Plan

**Created:** 2025-12-26
**Status:** Not Started
**Estimated Total Effort:** 53-80 hours
**Priority:** Post-MVP Technical Debt

This plan consolidates findings from a comprehensive code quality audit covering dead code, duplicates, architecture violations, stale references, and unimplemented methods.

---

## Executive Summary

| Category | Issues | Est. Lines | Effort |
|----------|--------|------------|--------|
| Dead/Unused Code | 44 items | ~1,106 | 4-6h |
| Unimplemented/Stubs | 19 TODOs | N/A | 8-12h |
| Duplicate Code | ~3,875 lines | 80+ files | 16-24h |
| Hexagonal Violations | 8 violations | N/A | 8-12h |
| Stale References | 25+ files | ~290 lines | 16-24h |
| Unused Dependencies | 12 deps | N/A | 1-2h |
| **TOTAL** | | **~5,300+ lines** | **53-80h** |

---

## Tier 1: Critical (Runtime Failures)

**Priority:** IMMEDIATE
**Estimated Effort:** 8-12 hours
**Blocks:** Production deployment

### T1.1: Fix Player-App REST Service Calls

**Status:** Not Started
**Effort:** 4-6 hours
**Severity:** CRITICAL - These services call deleted REST endpoints

| File | Lines | Deleted Endpoint |
|------|-------|------------------|
| `player_character_service.rs` | 107, 116, 129, 138, 148, 158, 167 | `/api/sessions/{id}/player-characters/*` |
| `world_service.rs` | 146, 159 | `/api/sessions`, `/api/worlds/{id}/sessions` |

**Root Cause:** WebSocket migration deleted REST routes but player-app services still call them.

**Fix Options:**
1. **Option A (Recommended):** Refactor services to use WebSocket `RequestPayload` instead
2. **Option B:** Delete these services if unused (verify no callers first)

**Files to modify:**
- `crates/player-app/src/application/services/player_character_service.rs`
- `crates/player-app/src/application/services/world_service.rs`

**Acceptance Criteria:**
- [ ] No REST endpoint calls in player-app services
- [ ] All functionality uses WebSocket protocol
- [ ] `cargo check --workspace` passes
- [ ] `cargo xtask arch-check` passes

---

### T1.2: Fix parse_archetype Inconsistency

**Status:** Not Started
**Effort:** 1 hour
**Severity:** HIGH - Inconsistent behavior between handlers

**Problem:** Two implementations with different case sensitivity:
- `dto/character.rs:79-91` - Case-sensitive: `"Hero"` works, `"hero"` fails
- `request_handler.rs:250-263` - Case-insensitive: both work

**Fix:**
1. Add `FromStr` impl on `CampbellArchetype` in domain
2. Use `to_lowercase()` for case-insensitive matching
3. Replace both local implementations with the canonical one

**Files to modify:**
- `crates/domain/src/value_objects/archetype.rs` (add FromStr)
- `crates/engine-app/src/application/dto/character.rs` (use FromStr)
- `crates/engine-app/src/application/handlers/request_handler.rs` (use FromStr)

**Acceptance Criteria:**
- [ ] Single `FromStr` implementation in domain
- [ ] Case-insensitive matching
- [ ] All tests pass

---

### T1.3: Fix parse_relationship_type Inconsistency

**Status:** Not Started
**Effort:** 1 hour
**Severity:** HIGH - Handler version has family relations, DTO doesn't

**Problem:**
- DTO version missing: family relationship types
- Handler version complete but duplicated

**Fix:** Same pattern as T1.2 - canonical `FromStr` in domain.

**Files to modify:**
- `crates/domain/src/entities/character.rs` (add FromStr for RelationshipType)
- `crates/engine-app/src/application/dto/character.rs`
- `crates/engine-app/src/application/handlers/request_handler.rs`

---

### T1.4: Fix Hexagonal Architecture Test Violation

**Status:** Not Started
**Effort:** 1-2 hours
**Severity:** HIGH - Breaks compile-time layer enforcement

**Problem:** `player-app/action_service.rs:84` imports from `player-adapters`:
```rust
#[cfg(test)]
mod tests {
    use wrldbldr_player_adapters::infrastructure::testing::MockGameConnectionPort;
```

**Fix Options:**
1. **Option A (Recommended):** Move `MockGameConnectionPort` to `player-ports` with `#[cfg(test)]` feature
2. **Option B:** Add `wrldbldr-player-adapters` as `[dev-dependencies]` in player-app

**Files to modify:**
- `crates/player-ports/src/outbound/game_connection_port.rs` (add mock)
- `crates/player-app/src/application/services/action_service.rs` (update import)
- `crates/player-app/Cargo.toml` (if Option B)

---

## Tier 2: High Priority (Technical Debt)

**Priority:** HIGH
**Estimated Effort:** 16-20 hours
**Blocks:** Maintainability

### T2.1: Delete Dead Code Modules

**Status:** Not Started
**Effort:** 2-3 hours
**Lines to Remove:** ~680

| Module | Location | Lines |
|--------|----------|-------|
| `json_exporter.rs` | `engine-adapters/infrastructure/export/` | ~367 |
| `config_routes.rs` | `engine-adapters/infrastructure/http/` | ~47 |
| `tool_parser.rs` (unused functions) | `engine-app/services/llm/` | ~250 |
| `common_goals` module | `domain/entities/goal.rs` | ~60 |

**Tasks:**
- [ ] Delete `json_exporter.rs` (world_snapshot.rs is the active exporter)
- [ ] Delete `config_routes.rs` (routes never registered)
- [ ] Remove unused functions from `tool_parser.rs`:
  - `parse_tool_calls()` (line 23)
  - `parse_single_tool()` (line 153)
  - `validate_tool_calls()` (line 437)
- [ ] Delete `common_goals` module in `goal.rs` (lines 44-107)
- [ ] Update `mod.rs` files to remove declarations
- [ ] Remove corresponding tests if any

---

### T2.2: Create Shared Row Converters Module

**Status:** Not Started
**Effort:** 3-4 hours
**Lines to Consolidate:** ~400

**Problem:** `row_to_item()` duplicated in 4 repositories (~40 lines each):
- `item_repository.rs:309`
- `character_repository.rs:1610`
- `player_character_repository.rs:454`
- `region_repository.rs:700`

Also duplicated: `row_to_region()`, `row_to_character()`

**Fix:**
1. Create `crates/engine-adapters/src/infrastructure/persistence/converters.rs`
2. Move all `row_to_*` functions to this module
3. Make them `pub(crate)` for internal use
4. Update all repository imports

**Files to create:**
- `crates/engine-adapters/src/infrastructure/persistence/converters.rs`

**Files to modify:**
- `crates/engine-adapters/src/infrastructure/persistence/mod.rs`
- All 4 repository files listed above

---

### T2.3: Wire LLM Context TODOs

**Status:** Not Started
**Effort:** 7-8 hours
**Severity:** HIGH - Affects LLM prompt quality

**Problem:** `build_prompt_from_action` in `websocket_helpers.rs` has 4 TODOs:

| Line | TODO | Impact |
|------|------|--------|
| 73 | `region_items: Vec::new()` | LLM can't see items in region |
| 199 | `current_mood: None` | NPC responses ignore mood |
| 200 | `motivations: None` | NPC responses ignore actantial context |
| 298 | `featured_npc_names: Vec::new()` | Narrative events missing NPC names |

**Tasks:**
- [ ] Wire `region_items` - call `fetch_region_items()` in prompt builder
- [ ] Wire `current_mood` - requires PC context, call `mood_service.get_npc_mood(npc_id, pc_id)`
- [ ] Wire `motivations` - call `actantial_service.get_llm_context()`, convert to `MotivationsContext`
- [ ] Wire `featured_npc_names` - call `narrative_event_service.get_featured_npcs()` for each event

**Files to modify:**
- `crates/engine-adapters/src/infrastructure/websocket_helpers.rs`

---

### T2.4: Move DTOs from Ports to App/Domain

**Status:** Not Started
**Effort:** 4-6 hours
**Severity:** MAJOR - Architecture violation

**Problem:** Ports layer contains concrete implementations:
- `engine-ports/queue_port.rs:35-51` - `QueueItem<T>` with UUID/timestamp generation
- `engine-ports/llm_port.rs:37-96` - `LlmRequest` builder pattern
- `engine-ports/request_handler.rs:70-168` - `RequestContext` validation logic
- `engine-ports/use_cases.rs:49-182` - DTOs that should be in app layer

**Fix:**
1. Move `QueueItem<T>` to `engine-app/src/application/dto/queue.rs`
2. Move `LlmRequest`, `ChatMessage` to domain or protocol
3. Move `RequestContext` validation to `engine-app`
4. Move use-case DTOs to appropriate layers

**Files to modify:**
- `crates/engine-ports/src/outbound/queue_port.rs`
- `crates/engine-ports/src/outbound/llm_port.rs`
- `crates/engine-ports/src/inbound/request_handler.rs`
- `crates/engine-ports/src/inbound/use_cases.rs`
- Various engine-app files (new imports)

---

## Tier 3: Medium Priority (Code Quality)

**Priority:** MEDIUM
**Estimated Effort:** 12-16 hours
**Blocks:** Developer productivity

### T3.1: Remove Unused Cargo Dependencies

**Status:** Not Started
**Effort:** 1-2 hours

| Crate | Unused Deps |
|-------|-------------|
| `wrldbldr-domain` | `anyhow`, `serde_json` |
| `wrldbldr-engine-adapters` | `tower`, `futures-channel`, `rand` |
| `wrldbldr-player-ports` | `url` |
| `wrldbldr-player-adapters` | `futures-channel` |
| `wrldbldr-player-ui` | `thiserror`, `serde-wasm-bindgen`, `wasm-bindgen-futures`, `gloo-timers`, `gloo-net` |

**Tasks:**
- [ ] Remove each dependency
- [ ] Verify `cargo check --workspace`
- [ ] Verify `cargo test --workspace`

---

### T3.2: Consolidate Duplicate Type Definitions

**Status:** Not Started
**Effort:** 4-6 hours

**Problem:** Types defined in both domain and protocol:

| Type | Domain | Protocol |
|------|--------|----------|
| `CampbellArchetype` | `value_objects/archetype.rs:5-29` | `types.rs:118-133` |
| `GameTime` | `game_time.rs:29-32` | `types.rs:169-189` |
| `MonomythStage` | value_objects | `types.rs:142-161` |

**Fix Options:**
1. **Option A:** Protocol re-exports from domain (protocol depends on domain)
2. **Option B:** Create `wrldbldr-types` crate for shared types
3. **Option C:** Domain re-exports from protocol (reverse dependency)

**Recommended:** Option A - Protocol depends on domain for shared enums.

---

### T3.3: Remove Legacy Protocol Messages

**Status:** Not Started
**Effort:** 2-3 hours

**Problem:** Deprecated messages still handled:
- `JoinSession` - Returns deprecation warning
- `SessionJoined` - Legacy flow
- `PlayerJoined` / `PlayerLeft` - Legacy flow

**Preferred:** `JoinWorld`, `WorldJoined`, `UserJoined`, `UserLeft`

**Tasks:**
- [ ] Remove `JoinSession` handler (or keep warning for migration period)
- [ ] Remove `SessionJoined`, `PlayerJoined`, `PlayerLeft` handlers
- [ ] Update player UI to not use legacy messages
- [ ] Add deprecation timeline comment if keeping

---

### T3.4: Update Stale Documentation

**Status:** Not Started
**Effort:** 2-3 hours

**Files referencing deleted REST routes:**

| File | Issue |
|------|-------|
| `docs/systems/navigation-system.md:344-345` | References deleted route files |
| `docs/systems/observation-system.md:195` | References `observation_routes.rs` |
| `docs/systems/narrative-system.md:354` | References `narrative_event_routes.rs` |
| `docs/systems/challenge-system.md:299` | References `challenge_routes.rs` |
| `docs/systems/character-system.md:318-345` | REST endpoint table |
| `docs/systems/scene-system.md:270-278` | REST endpoint table |
| `docs/progress/SPRINT_6_TIER2_PLAN.md:269-306` | Session broadcast pattern |

**Tasks:**
- [ ] Update each system doc to reflect WebSocket-first architecture
- [ ] Remove or update REST endpoint tables
- [ ] Update code examples to show WebSocket patterns

---

### T3.5: Remove Unused Struct Fields

**Status:** Not Started
**Effort:** 1-2 hours

| File | Struct | Unused Field |
|------|--------|--------------|
| `actantial_context_service.rs:198` | `ActantialContextServiceImpl` | `item_repo` |
| `generation_service.rs:116` | `BatchTracker` | `completed_count` |
| `scene_resolution_service.rs:59` | `SceneResolutionServiceImpl` | `character_repository` |
| `trigger_evaluation_service.rs:200-201` | `TriggerEvaluationService` | `challenge_repo`, `character_repo` |
| `memory_queue.rs:22` | `InMemoryQueue` | `queue_name` |

**Tasks:**
- [ ] Remove each unused field
- [ ] Update constructors
- [ ] Verify no runtime impact

---

## Tier 4: Low Priority (Polish)

**Priority:** LOW
**Estimated Effort:** 4-6 hours
**Blocks:** Nothing

### T4.1: Remove Empty Modules

**Status:** Not Started
**Effort:** 30 minutes

| Module | Location |
|--------|----------|
| `aggregates` | `domain/src/aggregates/mod.rs` (empty, Phase 3.1 DDD) |
| `inbound` | `player-ports/src/inbound/mod.rs` (empty) |

**Decision:** Remove or add `#[allow(unused)]` with future phase comment.

---

### T4.2: Remove Unused ID Type

**Status:** Not Started
**Effort:** 15 minutes

**Problem:** `WorkflowId` in `domain/src/ids.rs` is defined but never used.

**Fix:** Delete or mark with `#[allow(dead_code)]` if planned for future.

---

### T4.3: Consolidate MOOD_OPTIONS Constant

**Status:** Not Started
**Effort:** 30 minutes

**Problem:** Two different `MOOD_OPTIONS` constants:
- `npc_mood_panel.rs:9` - Includes "Depressed", "Appreciative", "Amused"
- `npc_motivation.rs:36` - Includes "Greedy", "Dutiful", "Conflicted"

**Fix:** Create single authoritative list, re-export where needed.

---

### T4.4: Clean Up Stale Trait Default Stubs

**Status:** Not Started
**Effort:** 30 minutes

**Problem:** `repository_port.rs:1676-1696` has default implementations returning "not implemented" errors, but Neo4j implementation exists.

**Fix:** Update comment or change default to `unimplemented!()` macro.

---

## Tier 5: Future Consideration

These items were identified but deferred for strategic reasons.

### T5.1: Player-App DTO Layer Refactoring

**Status:** Deferred (Phase 16.3)
**Location:** `player-app/src/application/dto/`

**Problem:** `world_snapshot.rs` (1,123 lines) duplicates engine-app DTOs.

**Future Fix:** Use protocol types directly or create shared DTO crate.

---

### T5.2: Repository CRUD Pattern Macros

**Status:** Deferred
**Lines Affected:** ~825

**Problem:** Every repository has similar CRUD Cypher patterns.

**Future Fix:** Create macro or trait for common CRUD operations.

---

### T5.3: Error Mapping Extension Traits

**Status:** Deferred
**Occurrences:** 100+

**Problem:** Repeated `.map_err(|e| ...)` patterns.

**Future Fix:** Create `ResultExt` trait for common error transformations.

---

## Verification Checklist

After each tier is complete:

- [ ] `cargo check --workspace`
- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets`
- [ ] `cargo xtask arch-check`
- [ ] Manual smoke test of affected features

---

## Progress Log

| Date | Tier | Task | Status | Commit |
|------|------|------|--------|--------|
| 2025-12-26 | - | Plan created | Done | - |

---

## Related Documentation

- [CODE_REVIEW_REMEDIATION_PLAN.md](./CODE_REVIEW_REMEDIATION_PLAN.md) - Previous remediation plan
- [WEBSOCKET_MIGRATION_COMPLETION.md](./WEBSOCKET_MIGRATION_COMPLETION.md) - WebSocket migration status
- [HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md](./HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md) - Architecture enforcement
- [ACTIVE_DEVELOPMENT.md](./ACTIVE_DEVELOPMENT.md) - Current sprint work
