# Architecture & Code Quality Remediation Plan

**Created**: December 30, 2024  
**Last Updated**: December 30, 2024  
**Status**: IN PROGRESS  
**Current Architecture Score**: 92/100  
**Target Architecture Score**: 98/100

---

## Executive Summary

This is the **single consolidated plan** for all architecture remediation and code quality work. It merges content from:
- Architecture Gap Remediation Plan (Phases 1-8)
- Hexagonal Architecture Phase 2 Plan (C5-C6, M2-M4)
- Hexagonal Final Remediation Plan (Priority 1-2 items)
- Challenge Approval Refactoring Plan (Phase 11 cleanup)

**All critical hexagonal violations are resolved.** Remaining work is polish and technical debt reduction.

---

## Completed Work Summary

### Phase 1: Quick Wins ✅
- Clippy auto-fix (424→85 warnings)
- Replaced `anyhow` with `thiserror` in domain
- Fixed `derivable_impls` warnings
- Added missing crates to arch-check

### Phase 2: DTO Consolidation ✅
- `DmApprovalDecision` duplication resolved
- `SuggestionContext` unified in engine-dto

### Phase 3: God Trait Splitting ✅ (Mostly Complete)
- Split 10+ god traits into ISP-compliant sub-traits
- `PlayerCharacterRepositoryPort` → 4 sub-traits
- `SceneRepositoryPort` → 5 sub-traits
- `EventChainRepositoryPort` → 4 sub-traits
- `WorldConnectionManagerPort` → 4 sub-traits
- Remaining: `WorldStatePort` (18 methods)

### Critical Hexagonal Fixes ✅
- C1+C2: AdapterState eliminated, AppStatePort created
- C3: PlatformPort created, player-ui → player-adapters dependency removed
- C4: FixedRandomPort moved to adapters layer
- M1: Unused business logic removed from protocol crate

### Challenge Approval System ✅ (Phases 0-10)
- Critical broadcast bug fixed
- Queue persistence implemented
- Handler migrations complete
- See Phase 11 below for deferred cleanup

---

## Remaining Work

### Priority 1: Protocol Architecture (from Phase2 C5-C6)

#### C5: Remove Protocol → Domain Dependency (CRITICAL)

**Status**: PENDING  
**Effort**: 8-12 hours

**Problem**: Protocol crate depends on `wrldbldr-domain` for 16 `From<DomainEntity>` trait implementations. This:
- Forces player WASM to compile entire domain crate
- Violates API contract principle
- Creates tight coupling

**Current protocol→domain imports (16 From impls)**:
- `GalleryAsset` → `GalleryAssetResponseDto`
- `GenerationBatch` → `GenerationBatchResponseDto`
- `NpcDispositionState` → `NpcDispositionStateDto`
- `WorkflowConfiguration` → `WorkflowConfigExportDto`
- `PromptMapping` ↔ `PromptMappingDto` (bidirectional)
- `InputDefault` ↔ `InputDefaultDto` (bidirectional)
- `WorkflowInput` → `WorkflowInputDto`
- `WorkflowAnalysis` → `WorkflowAnalysisDto`
- `AdHocOutcomes` (bidirectional in messages.rs)

**Solution**:
1. Extend `EntityType` enum in domain-types (3→20 variants)
2. Move `GameTime` to domain-types as canonical source
3. Update protocol imports to use domain-types for shared vocabulary
4. Move all From<DomainEntity> implementations to engine-adapters
5. Remove `wrldbldr-domain` dependency from protocol/Cargo.toml

**Files to modify**:
- `crates/domain-types/src/asset_types.rs`
- `crates/protocol/src/responses.rs`, `dto.rs`, `messages.rs`
- `crates/protocol/Cargo.toml`
- `crates/engine-adapters/src/infrastructure/dto_conversions/` (new module)

---

#### C6: Remove Implementation Code from Ports Layer

**Status**: PENDING  
**Effort**: 2 hours

**Problem**: `workflow_service_port.rs` contains ~270 lines of implementation code:
- `analyze_workflow()`, `validate_workflow()`, `prepare_workflow()`
- `auto_detect_prompt_mappings()`, `export_workflow_configs()`, `import_workflow_configs()`
- Uses `rand` dependency (violates ports purity)

**Solution**:
1. Delete implementation code from workflow_service_port.rs (keep only trait)
2. Remove `rand` dependency from engine-ports/Cargo.toml
3. Update engine-adapters/http/workflow_routes.rs to use WorkflowService directly

---

### Priority 2: Design Improvements (from Final Remediation)

#### 2.1: Refactor workflow_routes.rs Entity Mutation

**Status**: PENDING  
**Effort**: 1.5 hours

Move entity mutation logic from HTTP handlers to `WorkflowConfigService`:
- Lines 129-158, 219-229, 284-303 in workflow_routes.rs

Add to `WorkflowServicePort`:
```rust
async fn create_or_update(&self, slot, name, workflow_json, prompt_mappings, input_defaults, locked_inputs) -> Result<WorkflowConfiguration>;
async fn update_defaults(&self, slot, input_defaults, locked_inputs) -> Result<WorkflowConfiguration>;
async fn import_configs(&self, configs, replace_existing) -> Result<ImportResult>;
```

---

#### 2.2: Align PromptContextService Interfaces

**Status**: PENDING  
**Effort**: 1 hour

**Problem**: `PromptContextServicePortAdapter` shim exists because port and app-layer traits have different signatures.

**Solution**: Align port to app-layer signature, eliminate shim.

---

#### 2.3: Move Config Types from player-ports

**Status**: PENDING  
**Effort**: 30 minutes

Move `ShellKind` and `RunnerConfig` from `player-ports/src/lib.rs` to `player-runner`.

---

### Priority 3: File Decomposition

#### 3.1: app_state.rs Factory Extraction

**Status**: PENDING  
**Effort**: 2-3 hours

Extract from `new_app_state()` (~1313 lines → ~600-700 lines):
- Repository factory function (lines ~317-400)
- Queue infrastructure factory (lines ~613-660)
- WorkerServices creation (lines ~1275-1291)

---

#### 3.2: Large Repository Decomposition

**Status**: PENDING  
**Effort**: 4-6 hours

| File | Lines | Split Strategy |
|------|-------|----------------|
| `character_repository.rs` | 2073 | npc.rs, pc.rs, common.rs |
| `narrative_event_repository.rs` | 2005 | crud.rs, query.rs, trigger.rs |
| `story_event_repository.rs` | 1814 | crud.rs, edge.rs, timeline.rs |

---

#### 3.3: request_handler.rs God Object

**Status**: PENDING  
**Effort**: 4-6 hours

Split 3,497 lines into domain-specific modules:
```
handlers/
├── mod.rs (dispatcher)
├── world_handler.rs
├── character_handler.rs
├── location_handler.rs
├── scene_handler.rs
├── challenge_handler.rs
├── narrative_handler.rs
├── inventory_handler.rs
├── generation_handler.rs
└── admin_handler.rs
```

---

### Priority 4: God Trait Completion

#### 4.1: Split WorldStatePort (18 methods → 6 traits)

**Status**: PENDING  
**Effort**: 1.5 hours

| New Trait | Methods |
|-----------|---------|
| `WorldTimeStatePort` | get_game_time, set_game_time, advance_game_time |
| `WorldConversationStatePort` | add_conversation, get_conversation_history, clear |
| `WorldApprovalStatePort` | add_pending_approval, remove, get_pending |
| `WorldSceneStatePort` | get_current_scene, set_current_scene |
| `WorldDirectorialStatePort` | get/set/clear directorial_context |
| `WorldLifecyclePort` | initialize_world, cleanup_world, is_world_initialized |

---

#### 4.2: Remaining God Traits (7 with 15+ methods)

**Status**: PENDING  
**Effort**: 6-8 hours

| Trait | Methods | Priority |
|-------|---------|----------|
| LocationRepositoryPort | 27 | Medium |
| RegionRepositoryPort | 19 | Medium |
| InteractionRepositoryPort | 17 | Low |
| AssetRepositoryPort | 17 | Low |

---

### Priority 5: Code Quality

#### 5.1: Manual Clippy Fixes (85 warnings)

**Status**: PENDING  
**Effort**: 4-6 hours

| Warning Type | Count | Effort |
|--------------|-------|--------|
| `too_many_arguments` | 31 | HIGH |
| `result_large_err` | 13 | MEDIUM |
| `large_enum_variant` | 4 | MEDIUM |
| `type_complexity` | 3 | LOW |

**Priority fixes** (10+ args):
- `challenge_resolution_service` constructor (14 args)
- `story_event_service` constructor (13 args)
- `llm_queue_service` constructor (11 args)

---

#### 5.2: Error Handling Audit

**Status**: PENDING  
**Effort**: 2-3 hours

**Critical**: `staging_service.rs:616` - `extract_json_array(response).unwrap()` panics on malformed LLM response

**Pattern review**: 88 instances of `let _ = potentially_failing_operation();`

---

#### 5.3: Challenge Approval Phase 11 Cleanup (Deferred)

**Status**: DEFERRED  
**Effort**: 2-3 hours

Remaining items from Challenge Approval refactoring:
- Remove `WorldConnectionPort` from `ChallengeOutcomeApprovalService`
- Remove in-memory HashMap cache alongside queue
- Remove protocol imports from `challenge_outcome_approval_service.rs`
- Add unit tests for queue operations

---

### Priority 6: Documentation

#### 6.1: Document PromptMappingDto Separation

**Status**: PENDING  
**Effort**: 30 minutes

Add comments explaining intentional separation (camelCase vs snake_case).

---

#### 6.2: Add Deprecation Notices

**Status**: PENDING  
**Effort**: 30 minutes

Add `#[deprecated]` to legacy monolithic traits pointing to split modules.

---

## Effort Summary

| Priority | Description | Effort | Status |
|----------|-------------|--------|--------|
| 1 | Protocol Architecture (C5, C6) | 10-14h | PENDING |
| 2 | Design Improvements | 3h | PENDING |
| 3 | File Decomposition | 10-15h | PENDING |
| 4 | God Trait Completion | 8-10h | PENDING |
| 5 | Code Quality | 8-12h | PENDING |
| 6 | Documentation | 1h | PENDING |
| **Total** | | **40-55h** | |

---

## Verification Checklist

After each change:
```bash
cargo check --workspace
cargo test --workspace
cargo xtask arch-check
cargo clippy --workspace
```

Final verification targets:
- `app_state.rs` < 700 lines
- Clippy warnings < 50
- No files > 1500 lines
- No traits > 12 methods

---

## Change Log

| Date | Changes |
|------|---------|
| Dec 30, 2024 | Consolidated from 3 separate architecture plans |
| Dec 30, 2024 | Added Challenge Approval Phase 11 cleanup |
| Dec 30, 2024 | Phases 1-3 marked complete |
| Dec 30, 2024 | C1-C4 marked complete |
