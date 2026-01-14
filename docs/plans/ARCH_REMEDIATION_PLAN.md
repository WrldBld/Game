# Architecture Remediation Plan

## Scope
Address architecture debt identified in `docs/architecture/review.md` review:
- Domain purity violations (time sources in domain).
- Domain types used as DTOs (LLM context, queue payloads) with public fields/raw IDs.
- Aggregate naming validation gaps (raw `String` names).
- Use cases depending directly on port traits instead of repositories.
- Port trait sprawl beyond the intended ~10 boundaries.

## Phase 0: Validation Baseline
- Capture findings and confirm current violations are real and in active code paths.
- Criteria to proceed:
  - `rg "Utc::now\(\)" crates/domain/src` shows in non-doc contexts.
  - `wrldbldr_domain::GamePromptRequest` and queue payloads are only used in engine + tests.
  - Use cases directly depend on port traits (not repositories).

## Phase 1: Domain Purity + DTO Relocation
### Work
1) Remove time sources from domain code.
   - Replace `Utc::now()` in domain aggregates/entities with explicit `now: DateTime<Utc>` inputs.
   - Update callers in engine to pass `ClockPort::now()`.
2) Relocate DTO-like LLM context + queue payloads out of `domain`.
   - Move `value_objects/llm_context.rs` and `value_objects/queue_data.rs` into `crates/engine/src/llm_context.rs` and `crates/engine/src/queue_types.rs` (or a new `engine/src/types/` module).
   - Update imports and re-exports (remove from `domain::lib.rs`, update engine use cases/tests).
   - Keep serde derives in engine where they are used.
3) Introduce name newtypes for aggregates with raw `String` names.
   - Add `SceneName` and `NarrativeEventName` (validated) in `domain/src/value_objects/names.rs`.
   - Update aggregates + any call sites.

### Exit criteria
- `rg "Utc::now\(\)" crates/domain/src` returns zero non-doc hits.
- `domain` no longer exports LLM/queue DTOs.
- Aggregates with `name: String` use `*Name` newtypes.
- `cargo check --workspace` passes.

## Phase 2: Repository Alignment + Port Pruning (Engine)
### Work
1) Replace direct port-trait usage in use cases with repository wrappers.
   - For use cases that need non-CRUD functions, extend repository APIs rather than injecting port traits.
2) Consolidate port traits toward boundary-only set.
   - Evaluate repos like `ActRepo`, `SkillRepo`, `InteractionRepo`, etc. for consolidation (e.g., grouped `ContentRepo`).
   - `SkillRepo` consolidated into `ContentRepo` (completed).
   - Keep only realistic swap boundaries.

### Exit criteria
- No use case struct holds `Arc<dyn *Repo>` directly.
- `ports.rs` list is reduced or consolidated to the agreed boundary set.
- `cargo check --workspace` passes.

## Phase 3: Cleanup + Tests
### Work
- Update docs to reflect new placement of LLM/queue DTOs.
- Add/adjust unit tests for new name newtypes.
- Run targeted tests for queue/LLM request flows if available.

### Exit criteria
- Documentation references updated paths.
- Tests for name validation pass.

## Phase 4: 5e Primitives Coverage (Content + Tests)
### Work
- Complete support for all D&D 5e primitives in content persistence and retrieval.
  - Ensure storage and CRUD for feats, spells, classes, subclasses, races, backgrounds, items, proficiencies, and related metadata.
  - Wire content access through `ContentRepo` and `ContentService` where applicable.
- Add tests covering each primitive:
  - Importer coverage for 5etools data.
  - Repository CRUD for each primitive (Neo4j integration tests).
  - Use case/API coverage for listing/getting primitives.

### Exit criteria
- All 5e primitives are persisted and queryable via `ContentRepo`.
- Tests exist for each primitive (import + repository + API/use case where applicable).
- `cargo test --workspace` passes with required test dependencies.

## Risks / Mitigations
- API surface changes: mitigate with type aliases or transitional re-exports inside engine.
- Refactor ripple: perform per-phase commits and keep changes scoped.
- Potential cyclic dependencies: ensure engine owns DTO types to avoid domain contamination.

## Notes
- Each phase will be committed independently.
- If a phase requires follow-on fixes, add a small “Phase 1.x” sub-phase rather than mixing concerns.
