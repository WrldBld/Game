# Clean Codebase Remediation Plan

Status: Active
Owner: Team
Last updated: 2026-01-17

Goal: remediate all findings from the latest full code review and establish a clean baseline with no new tech debt.

## Status Legend
- [ ] Pending
- [~] In progress
- [x] Done

## Scope (Findings to Remediate)
1) Domain purity breach: `serde_json` dependency in domain.
2) Aggregate mutations returning `()` instead of domain events.
3) Stringly-typed fields bypassing value objects.
4) Domain validation returning `Result<(), String>` instead of `DomainError`.
5) UUID parse fallback masking errors.
6) Port trait sprawl beyond the intended ~10-15 boundary traits.
7) Value objects with public fields and mutable setters (immutability violations).
8) Raw `Uuid` usage inside domain value objects.
9) Silent data drops during repository hydration.
10) Boolean flags used where enum state machines are required.
11) Non-domain concerns living in the domain crate (UI metadata, LLM prompt templates).
12) Runtime panics/unwraps in production UI/service code.
13) Game system schema and workflow configuration (UI/infra) types embedded in domain.
14) Use cases swallowing repository failures and defaulting silently.
15) Use cases returning JSON blobs or raw serde types instead of domain/use-case DTOs.
16) Dioxus hook misuse (hooks inside RSX/conditionals) causing runtime panics.
17) LLM response parsing defaults hide invalid tool call payloads (should fail fast).
18) External importers default missing fields without validation (risk of partial data ingestion).
19) Use cases default on infrastructure errors (queue depth, repo lists, health checks) instead of propagating.
20) API/WebSocket handlers default invalid inputs rather than validating and returning errors.
21) Protocol/translation layers default on serialization failures, hiding data loss.
22) Repository wrapper naming diverges from `*Repository` convention.
23) Cross-layer error taxonomy/mapping is inconsistent and lacks user-facing standards.
24) Observability tech debt: missing contextual logging/telemetry in error paths.
25) Dependency hygiene tech debt: unused deps and inconsistent linting/formatting baselines.
26) Security tech debt: secrets scanning and authz audit not formalized.

## Phase 1: Domain Purity + Error Types

### 1.1 Remove `serde_json` from domain dependencies
- [~] Audit all `serde_json` usage in `crates/domain` and classify:
  - [x] Data model dependency (needs redesign)
  - [x] Tests/fixtures (can move or gate)
  - [x] Serde roundtrip tests (can move or use `serde_json` only in tests)
- [x] Redesign plan for each `serde_json::Value` usage:
  - Character sheet/value maps: move `serde_json::Value` payloads to `protocol` DTOs and keep domain as typed enums + newtypes only.
  - Workflow/config JSON blobs: move config storage to `engine` (infra or use-case layer), keep domain with IDs + metadata only.
  - Content/traits/spells JSON: move JSON serialization/fixtures to `protocol` or `engine` tests; domain should hold typed structs.
  - Any truly opaque blob required by domain: wrap as `OpaqueJson` value object in `engine`, not domain.
- [x] Replace domain fields accordingly:
  - Convert domain fields to typed structs/enums.
  - Add conversion layers in `protocol` and `engine` mapping code.
- [x] Move UI/schema and workflow config types out of domain:
  - [x] `crates/domain/src/character_sheet.rs` schema types -> `protocol` (wire format).
  - [x] `crates/domain/src/types/workflow.rs` + `crates/domain/src/entities/workflow_config.rs` -> `engine` (infra/use-case) + `protocol` DTOs.
- [x] Move non-domain UI/LLM metadata out of domain:
  - [x] `PromptTemplateMetadata` and prompt strings -> `engine` (LLM orchestration) or `protocol`.
  - [x] `SettingsFieldMetadata` (UI) -> `protocol` or `player`, keep domain with raw config only.
- [x] Update domain crate dependencies to remove `serde_json`.
- [x] Ensure domain still serializes/deserializes using `serde` only.

Acceptance:
- `crates/domain/Cargo.toml` has no `serde_json` dependency.
- `rg "serde_json" crates/domain` returns only test-only or zero hits.

### 1.2 Normalize domain validation errors
- [ ] Replace `Result<(), String>` in domain validation with `Result<(), DomainError>`.
- [ ] Add/extend `DomainError` variants as needed for validation.
- [ ] Update callers and tests to assert `DomainError` variants.
- [ ] Apply same rule to workflow validation helpers in `crates/domain/src/types/workflow.rs`.

Acceptance:
- No `Result<(), String>` validation methods in domain.
- Validation errors are typed and consistent.

### 1.3 Remove IO/JSON errors from domain traits
- [ ] Replace `ContentError::IoError` and `ContentError::JsonError` with domain-only errors.
- [ ] Move IO/JSON parsing to `engine` or `protocol`, leaving domain with pure validation only.

Acceptance:
- Domain error enums do not depend on `std::io` or `serde_json` error types.
- `crates/domain` compiles without IO/JSON error variants.

### 1.4 Enforce value object immutability + private fields
- [ ] Inventory value objects with public fields or `&mut self` setters.
- [ ] Convert public fields to private with read-only accessors.
- [ ] Replace setters with builder-style `with_*` methods that consume `self`.
- [ ] Ensure all value objects are valid by construction (constructor validates).

Acceptance:
- No public fields in value objects.
- No `&mut self` mutation methods on value objects (builder patterns only).

### 1.5 Replace raw Uuid usage in domain value objects
- [ ] Inventory `Uuid` usage in domain value objects.
- [ ] Replace with typed IDs (`CharacterId`, `PlayerCharacterId`, etc.) or newtypes.
- [ ] Update serde mappings to use `try_from`/`into` to preserve wire compatibility.

Acceptance:
- Domain value objects do not expose raw `Uuid` fields.
- Typed IDs are used throughout domain value objects.

### 1.6 Encapsulate domain entities (non-aggregate structs)
- [ ] Inventory `crates/domain/src/entities` for public fields.
- [ ] Convert entities to private fields with read-only accessors and builder-style constructors.
- [ ] Replace stringly-typed IDs and labels in entities with value objects or typed IDs where applicable.
- [ ] Replace boolean state flags with enums where they represent mutually exclusive states.

Acceptance:
- Domain entities no longer expose public fields.
- Domain entities use typed IDs or value objects for identifiers and validated strings.
- State machine flags are expressed as enums, not multiple booleans.

## Phase 2: Domain Events for Mutations

### 2.1 Inventory and catalog of mutation methods returning `()`
- [ ] Enumerate all public `&mut self` methods on aggregates that return `()`.
- [ ] For each, define a specific domain event enum or reuse an existing update enum.
- [ ] Update method signatures to return the event.
- [ ] Update all call sites to handle the event or explicitly ignore it.
- [ ] Add tests asserting the correct event is emitted.

Acceptance:
- All aggregate mutation methods return domain events.
- `rg "fn .*\\(&mut self" crates/domain/src/aggregates` yields no `()` returns.

## Phase 3: Layer Responsibility Cleanup

### 3.1 Remove protocol types from use cases
- [ ] Inventory `wrldbldr_protocol` usage in `crates/engine/src/use_cases`.
- [ ] Move protocol conversions into API/websocket/http layers.
- [ ] Adjust use cases to return domain types or use-case-specific DTOs (domain-only).
- [ ] Update tests to reflect domain-type outputs.
- [ ] Targeted cleanup:
  - `engine/src/use_cases/session/directorial.rs`
  - `engine/src/use_cases/session/join_world_flow.rs`
  - `engine/src/use_cases/time/mod.rs`
  - `engine/src/use_cases/staging/approve.rs`
  - `engine/src/use_cases/staging/request_approval.rs`
  - `engine/src/use_cases/staging/types.rs`

Acceptance:
- `rg "wrldbldr_protocol" crates/engine/src/use_cases` returns zero hits.

### 3.2 Stop swallowing repository failures in use cases
- [ ] Replace `ok().flatten()` and `unwrap_or_default()` in use-case repository calls with explicit error handling.
- [ ] Propagate or log errors with context (entity + operation).
- [ ] Add tests for error propagation in staging/time/visual-state flows.
- [ ] Targeted cleanup (use-case defaults that should fail fast):
  - `engine/src/use_cases/session/join_world.rs` (repo list + current scene defaults).
  - `engine/src/use_cases/narrative_operations.rs` (flag repo defaults).
  - `engine/src/use_cases/player_action/mod.rs` (queue depth fallback).
  - `engine/src/use_cases/assets/mod.rs` (health check fallback).
  - `engine/src/use_cases/queues/mod.rs` (queue context defaults where required).

Acceptance:
- Use cases do not silently default on repo errors.
- Error handling is explicit and test-covered.

### 3.3 Remove `serde_json::Value`/JSON building from use cases
- [ ] Inventory use cases returning `serde_json::Value` or building JSON snapshots.
- [ ] Replace with domain/use-case DTOs; move JSON serialization to API/protocol layers.
- [ ] Update callers (API/websocket) to perform serialization and mapping.

Acceptance:
- `rg "serde_json::Value|serde_json::json" crates/engine/src/use_cases` returns only test-only hits.

## Phase 4: Value Objects for Stringly-Typed Fields

### 4.1 Define value objects for validated strings
- [ ] Identify string fields in aggregates that represent domain concepts (names, descriptions, tags, asset paths, notes).
- [ ] For each, create a value object with validation and serde try_from/into.
- [ ] Replace raw `String` fields with value objects across aggregates.
- [ ] Redesign guidance for common string fields:
  - Descriptions/notes: `Description` newtype with length bounds and trimming.
  - Tags: `Tag` newtype + `Vec<Tag>` (lowercased, no empty/whitespace).
  - Asset paths/IDs: `AssetId` or `AssetPath` newtype (non-empty, max length).
  - User identifiers: `UserId` newtype (non-empty, max length).
  - Settings metadata: replace `field_type`/`category` strings with enums.
- [ ] Update repositories, use cases, and protocol mappings accordingly.

Acceptance:
- No raw `String` fields for validated concepts in domain aggregates.
- Constructors validate all newtypes and return `DomainError` on invalid input.

## Phase 5: UUID Parsing and Data Integrity

### 5.1 Remove UUID nil fallbacks
- [ ] Replace `parse_uuid_or_nil` with fallible parsing returning `Result<Uuid, RepoError>`.
- [ ] Update call sites to handle parse errors explicitly (propagate or return validation errors).
- [ ] Add logging with context (entity type, field) on parse failures.
- [ ] Add tests for invalid UUID handling paths.

Acceptance:
- No silent UUID fallback to `Uuid::nil()` in persistence code.
- Invalid UUIDs surface as errors (not silently accepted).

### 5.2 Eliminate silent data drops during repository hydration
- [ ] Inventory all `unwrap_or_default`, `unwrap_or_*`, and `filter_map(...ok())` in repo hydration code.
- [ ] Replace silent defaults with explicit `RepoError` for invalid stored data.
- [ ] Update tests to expect failures on corrupted data.
- [ ] Replace `get_json_or_default`/`get_string_or` usage in hydration paths for required fields with strict parsing.
- [ ] Targeted cleanup (fail-fast on invalid stored values):
  - `engine/src/infrastructure/neo4j/scene_repo.rs` (entry_conditions, featured_characters, time_context).
  - `engine/src/infrastructure/neo4j/narrative_repo.rs` (triggers/outcomes/tags JSON, trigger_logic, event lists).
  - `engine/src/infrastructure/neo4j/character_repo.rs` (default_mood, expression_config, name/description).
  - `engine/src/infrastructure/neo4j/player_character_repo.rs` (sheet_data, status flags, description).
  - `engine/src/infrastructure/neo4j/location_repo.rs` (map_bounds/parent_map_bounds JSON, default_region_id).
  - `engine/src/infrastructure/neo4j/challenge_repo.rs` (difficulty/outcomes/trigger JSON).
  - `engine/src/infrastructure/neo4j/world_repo.rs` (created_at/updated_at/game_time).
  - `engine/src/infrastructure/neo4j/location_state_repo.rs` and `engine/src/infrastructure/neo4j/region_state_repo.rs`.
  - `engine/src/infrastructure/neo4j/interaction_repo.rs`, `content_repo.rs`, `goal_repo.rs`, `asset_repo.rs`, `staging_repo.rs`, `observation_repo.rs`, `item_repo.rs`, `lore_repo.rs`.
  - `engine/src/infrastructure/neo4j/helpers.rs` (row_to_item defaults, Node/RowExt fallbacks).
  - `engine/src/infrastructure/neo4j/location_repo.rs` (region/location connection defaults).
  - `engine/src/infrastructure/neo4j/lore_repo.rs` (tags/knowledge JSON parsing fallbacks).
  - `engine/src/infrastructure/neo4j/content_repo.rs` (skill category parsing defaults).
  - `engine/src/infrastructure/neo4j/interaction_repo.rs` (interaction type/target/conditions JSON parse defaults).
  - `engine/src/infrastructure/neo4j/scene_repo.rs` (missing location_id placeholder, entry condition drops).
  - `engine/src/infrastructure/neo4j/asset_repo.rs` (entity_type default fallback).
  - `engine/src/infrastructure/neo4j/location_state_repo.rs` and `engine/src/infrastructure/neo4j/region_state_repo.rs` (activation rules/logic JSON defaults).
  - `engine/src/infrastructure/neo4j/narrative_repo.rs` (conversation turn row defaults, completed/events list defaults).
  - `engine/src/infrastructure/neo4j/flag_repo.rs` (query result defaults for boolean state).
- [ ] Replace `unwrap_or`/`unwrap_or_default` fallbacks in row conversions with explicit `RepoError` for required fields.

Acceptance:
- Repository hydration fails fast on invalid stored values rather than silently dropping data.

## Phase 6: Port Trait Reduction

### 6.1 Port trait audit and consolidation
- [ ] Inventory all port traits in `crates/engine/src/infrastructure/ports.rs`.
- [ ] Classify each trait as:
  - Real swap boundary (keep)
  - Internal service/repo abstraction (convert to concrete type)
  - Test-only seam (replace with mockable repository/service)
- [ ] Merge or remove non-boundary traits.
- [ ] Update use cases and repositories to depend on concrete wrappers.
- [ ] Update App wiring and tests.

Acceptance:
- Port traits reduced to ~10-15 boundary traits.
- Use cases depend on repositories/services, not port traits.

## Phase 7: Regression Coverage

Note: Performance benchmarking happens in testing and is tracked separately.

### 7.1 Add verification tests
- [ ] Add/extend tests for:
  - New value object validation.
  - Mutation event emission.
  - UUID parse error propagation.
- [ ] Ensure any removed or moved domain types still serialize via protocol or engine layers.

Acceptance:
- Tests cover new behavior and fail on regression.

## Phase 8: Runtime Safety (Player/UI)

### 8.1 Remove panic paths in production UI/service code
- [ ] Replace `use_context::<T>()` panics with fallible accessors returning `Option<T>` or `Result<T, UiError>`.
- [ ] Remove `unwrap()` usage in UI event handlers (e.g., navigation handlers), replace with guarded control flow.
- [ ] Move any hook calls (`use_signal`, `use_context`, `use_navigator`) out of conditional/RSX blocks into top-level component scope.
- [ ] Add tests or assertions that ensure required contexts are provided at the app composition root.

Acceptance:
- No `unwrap()` in non-test player UI/service code.
- Context access failures are handled explicitly (no panics).
- All Dioxus hooks are called unconditionally at the top level of components.

## Phase 9: External Input Validation (LLM + Importers)

### 9.1 Enforce strict LLM response parsing
- [ ] Replace `unwrap_or_default` on LLM responses and tool call argument parsing.
- [ ] Treat missing choices, missing content, and invalid tool call JSON as errors.
- [ ] Add tests/VCR cassettes to ensure invalid responses surface as `LlmError::InvalidResponse`.

### 9.2 Validate external content imports
- [ ] Identify required fields in `crates/engine/src/infrastructure/importers/fivetools.rs`.
- [ ] Replace default fallbacks for required fields with explicit parse/validation errors.
- [ ] Decide and document which fields are optional vs required for ingestion.
- [ ] Add tests to ensure invalid source data fails fast.

Acceptance:
- LLM responses do not silently default or drop tool call arguments.
- Importers fail fast on missing required fields and document optional fallbacks.

## Phase 10: API Input Validation

### 10.1 Validate user input in websocket/http handlers
- [ ] Replace default parsing on invalid inputs (e.g., numeric fields, IDs) with explicit errors.
- [ ] Ensure handler-level validation rejects malformed payloads rather than silently defaulting.
- [ ] Add tests for invalid input cases where defaults were previously applied.

Acceptance:
- No API handlers silently default invalid user inputs.

### 10.2 Fail fast on protocol/message serialization errors
- [ ] Replace `serde_json::to_value(...).unwrap_or_default()` with explicit error handling.
- [ ] Return structured errors when serialization fails (protocol + message translation).
- [ ] Add tests for invalid/unsupported payloads.

Acceptance:
- Protocol/message layers do not silently drop/empty payloads on serialization failures.

## Phase 11: Repository Naming + Layer Standards

### 11.1 Enforce `*Repository` wrapper naming
- [ ] Rename repository wrapper structs to `*Repository` for consistency.
- [ ] Update module exports and app wiring to use renamed types.
- [ ] Update tests and mocks referencing renamed wrappers.

Acceptance:
- All repository wrappers in `crates/engine/src/repositories` follow `*Repository` naming.

## Phase 12: Error Taxonomy + Mapping Standards

### 12.1 Define cross-layer error mapping guidelines
- [ ] Define standard error mapping between domain -> use case -> API/protocol.
- [ ] Ensure error variants carry context (entity, operation, identifiers).
- [ ] Replace ad-hoc string errors in engine/player with typed errors.
- [ ] Add tests for error mapping in API/websocket handlers.

Acceptance:
- Error mapping is consistent across layers and user-facing errors are typed.

## Phase 13: Observability Tech Debt

### 13.1 Add contextual logging in error paths
- [ ] Audit error paths that currently drop context (repo hydration, API validation, LLM parsing).
- [ ] Add structured logs with identifiers and operation names.
- [ ] Ensure logging does not leak secrets or user PII.

Acceptance:
- Error paths log context for troubleshooting without leaking sensitive data.

## Phase 14: Dependency + Lint Hygiene

### 14.1 Clean unused deps and standardize linting
- [x] Audit crates for unused dependencies; remove or gate test-only deps.
- [ ] Standardize clippy/lint warnings as errors where appropriate.
- [x] Ensure formatting/lint baselines are documented in `docs/architecture/review.md`.

Acceptance:
- No unused dependencies in workspace; lint/format baselines documented.

## Phase 15: Security Tech Debt Checks

### 15.1 Add security review tasks
- [ ] Add secrets scan to review checklist or CI script.
- [ ] Audit auth/authz boundaries and document expected access controls.
- [ ] Ensure user-input validation is explicit at API boundaries (aligns with Phase 10).

Acceptance:
- Security review tasks are documented and repeatable.

## Progress Tracking Checklist

- [x] Phase 1.1 complete
- [ ] Phase 1.2 complete
- [ ] Phase 1.3 complete
- [ ] Phase 1.4 complete
- [ ] Phase 1.5 complete
- [ ] Phase 1.6 complete
- [ ] Phase 2.1 complete
- [ ] Phase 3.1 complete
- [ ] Phase 3.2 complete
- [ ] Phase 3.3 complete
- [ ] Phase 4.1 complete
- [ ] Phase 5.1 complete
- [ ] Phase 5.2 complete
- [ ] Phase 6.1 complete
- [ ] Phase 7.1 complete
- [ ] Phase 8.1 complete
- [ ] Phase 9.1 complete
- [ ] Phase 9.2 complete
- [ ] Phase 10.1 complete
- [ ] Phase 10.2 complete
- [ ] Phase 11.1 complete
- [ ] Phase 12.1 complete
- [ ] Phase 13.1 complete
- [~] Phase 14.1 complete
- [ ] Phase 15.1 complete


## Validity Review (Self-check)

This plan maps the last full review findings and adds the missing tech debt scope identified in follow-up review.
Each phase has explicit tasks and acceptance criteria. Progress can be tracked via the checklist above.
Risks:
- Refactoring string fields to newtypes will ripple across protocol, repositories, and UI mappings.
- Removing `serde_json` from domain may require moving several data-only structs to non-domain crates.
- Tightening repo hydration will surface inconsistencies in seed/test data that must be fixed or re-seeded.
- Repository renames will ripple through use cases, wiring, and tests.
- Stricter error mapping and logging may require new error types across layers.
Mitigations:
- Execute phases in order to reduce churn.
- Add tests alongside each change to prevent regressions.
- Stage repository renames with type aliases if needed.
- Document error mapping rules before refactors.
