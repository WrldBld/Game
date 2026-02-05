ComfyUI Settings Plan

Scope
- Implement ComfyUI model selection, prompt injection, and pose selection with global and per-workflow overrides.
- Add workflow registry API endpoints to match the existing player workflow_service contract.
- Extend settings schema to support global, per-world, and per-workflow configuration.
- Update ComfyUI client to consume injected settings and workflow mappings.

Goals
- Provide a complete settings hierarchy for ComfyUI configuration.
- Enable prompt injection with global SCORE tokens plus per-workflow prefix/suffix/negative overrides.
- Enable model selection with clear precedence and safe defaults.
- Support pose selection using asset IDs and ControlNet mapping.
- Back existing player workflow UI with working engine endpoints and storage.

Non-Goals
- Rework the visual state generation UI beyond wiring existing controls and settings panels.
- Implement new workflow analysis algorithms beyond current shared workflow analysis helpers.
- Redesign ComfyUI node mapping schema or replace shared workflow contract types.
- Introduce new databases or replace Neo4j/SQLite.

Architecture Constraints (ADR-008/009/011)
- ADR-008 Tiered Encapsulation: workflow settings are Tier-4 simple data structs unless they enforce invariants.
- ADR-009 Port Injection: new workflow config storage must be injected via port traits, no wrapper layer.
- ADR-011 Protocol Conversion: use cases must not return wire types; settings contracts in shared are allowed.

Data Model Changes
- Shared contract: add ComfyUI workflow settings struct to settings contract module.
- AppSettings (SQLite): add global ComfyUI settings and per-workflow overrides map.
- Neo4j: persist WorkflowConfiguration nodes for workflow registry, keyed by slot.
- Queue payload: include workflow settings fields for model, prompt injection, and pose selection.

Settings Schema

Global Settings (AppSettings)
- comfyui_base_url (existing env still overrides)
- comfyui_default_model_id
- comfyui_prompt_prefix
- comfyui_prompt_suffix
- comfyui_negative_prompt
- comfyui_score_tokens_enabled (bool, default true)
- comfyui_workflow_overrides: map<string workflow_id, ComfyWorkflowConfig>

Per-World Settings (AppSettings per-world override)
- same fields as global; null means fallback to global
- per-world comfyui_workflow_overrides shadows global per workflow_id

Per-Workflow Overrides (ComfyWorkflowConfig)
- workflow_id (string key)
- model_id (optional)
- prompt_prefix (optional)
- prompt_suffix (optional)
- negative_prompt_override (optional)
- score_tokens_enabled (optional override)
- controlnet_pose_asset_id (optional asset id)
- controlnet_mappings: list of { node_id, input_key, enabled }
- prompt_mappings_override: optional override for positive/negative prompt node mappings

Settings Merge Rules
- global -> per-world -> per-workflow
- missing field means fallback to previous layer
- request-level overrides can replace workflow fields for a single generation call

Workflow Registry API Endpoints (match player workflow_service)
- GET /api/workflows
- GET /api/workflows/{slot}
- POST /api/workflows/{slot}
- DELETE /api/workflows/{slot}
- POST /api/workflows/analyze
- POST /api/workflows/{slot}/test
- PATCH /api/workflows/{slot}/defaults

Engine Implementation Notes
- Add WorkflowConfigRepo port in engine/infrastructure/ports.rs.
- Implement Neo4j WorkflowConfiguration repo with CRUD and list.
- Add use cases for workflow list/get/save/delete/analyze/test/update defaults.
- Keep workflow slot enum in shared and store only configured workflows in Neo4j.

ComfyUI Client Changes
- Extend ImageRequest to include model_id, prompt_prefix, prompt_suffix, negative_prompt, score_tokens_enabled, pose_asset_id, and controlnet mapping data.
- Move prompt injection logic into load_and_prepare_workflow using mappings, not hardcoded node IDs.
- Apply model selection by updating checkpoint loader node or mapped node input.
- Apply ControlNet pose asset mapping by updating node input with stored asset path resolved server-side.
- Preserve fallback workflow behavior when workflow slot not configured.

Prompt Injection Strategy
- Global SCORE injection: apply SCORE tokens only when score_tokens_enabled is true (default).
- Per-workflow prompt prefix and suffix appended around the user prompt.
- Per-workflow negative prompt override replaces global negative prompt unless empty.
- Request override can supply per-request prompt_prefix/suffix/negative overrides for tests or ad-hoc generation.

Model Selection Hierarchy
- Environment variable (ComfyUI checkpoint override)
- Global settings (comfyui_default_model_id)
- Per-workflow override (ComfyWorkflowConfig.model_id)
- Request override (explicit model_id on request)
- If no model_id resolved, keep workflow-embedded default model.

Pose Selection Strategy
- Use pose asset IDs stored in Neo4j assets; map to asset path at generation time.
- Store controlnet mappings per workflow, including node_id and input key for pose image.
- When pose_asset_id present, inject into mapped controlnet inputs; if absent, do not enable ControlNet.
- Validate asset ID references and ensure only engine-controlled paths are injected.

Storage Plan
- Neo4j: WorkflowConfiguration node for each configured workflow slot.
- SQLite: AppSettings and per-world settings JSON include global and per-workflow overrides.
- Keep existing settings repo; add merge logic for per-world override and workflow override in use case.

UI Changes
- Settings panels: add global and per-world ComfyUI controls for model and prompt injection defaults.
- Workflow config editor: add fields for prompt prefix/suffix/negative override and model selection.
- Workflow slot list: show configured status and current model/pose settings summary.
- Visual state generation modal: expose workflow slot selection and optional pose asset picker.

Test Plan

Unit
- Settings merge logic for global/per-world/per-workflow and request overrides.
- Workflow analysis mapping from workflow JSON to prompt/controlnet mappings.
- ComfyUI client injection helpers for prompt, model, and pose inputs.

Integration
- WorkflowConfigRepo CRUD in Neo4j with slot uniqueness.
- API endpoints for workflows: create, read, list, update defaults, delete.
- Visual state generation use case populates queue payload with workflow settings.

E2E
- Workflow upload + analyze + save + test generation through HTTP API.
- Visual state generation with configured model override and prompt injection.
- Pose selection: choose pose asset and verify controlnet input updated.

Sequencing and Dependencies

Milestone 1: Backend Settings and Workflow Registry
- Dependencies: shared settings contract updates and workflow repo port.
- Deliverables: AppSettings schema changes, workflow repo, use cases, API endpoints.
- Acceptance Criteria:
  - Workflow endpoints return data matching player workflow_service expectations.
  - Settings round-trip persists global and per-world ComfyUI settings.
  - WorkflowConfigRepo CRUD passes integration tests.

Milestone 2: Generation Pipeline and ComfyUI Client
- Dependencies: Milestone 1 workflow settings available in use case input.
- Deliverables: queue payload fields, ImageRequest fields, ComfyUI workflow injection and model selection.
- Acceptance Criteria:
  - Prompt injection uses SCORE tokens + per-workflow prefix/suffix.
  - Model selection honors hierarchy and updates workflow JSON.
  - Pose asset selection updates ControlNet inputs when configured.

Milestone 3: Frontend Wiring
- Dependencies: Milestone 1 API endpoints available.
- Deliverables: settings UI fields, workflow editor fields, generation modal workflow/pose selection.
- Acceptance Criteria:
  - Settings panels can edit global and per-world ComfyUI defaults.
  - Workflow editor can set prompt/model/pose overrides and persist them.
  - DM can select workflow and pose asset in the generation modal.

Milestone 4: Review and Validation
- Dependencies: Milestones 1-3 complete.
- Deliverables: test coverage review and validation report.
- Acceptance Criteria:
  - Unit, integration, and E2E tests listed in this plan exist and pass.
  - Architecture constraints (ADR-008/009/011) verified in code review.
  - Player workflow UI no longer calls missing endpoints.

Dependencies and Risks
- Risk: existing hardcoded node IDs in ComfyUI workflows; mitigate with mapping-driven injection.
- Risk: settings payload size growth; mitigate with workflow override map and clear merge logic.
- Risk: UI drift; mitigate by wiring existing workflow editor components to backend.
