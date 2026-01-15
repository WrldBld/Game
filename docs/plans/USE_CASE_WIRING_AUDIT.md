# Use Case Wiring Audit

Status: Active
Owner: Team
Last updated: 2026-01-14

Goal: identify use cases that exist but are not wired, handlers that still contain orchestration logic, and other blockers to a maintainable engine.

## Use Cases Defined (Current Wiring Status)

### Wired Today
- Movement: `enter_region`, `exit_location` are invoked from `crates/engine/src/api/websocket/mod.rs`.
- Conversation: `start`, `continue`, `end` are used in player action handling (`talk`) and approval flow in `crates/engine/src/api/websocket/mod.rs`.
- Challenge: `roll`/`resolve` are used in challenge handlers in `crates/engine/src/api/websocket/mod.rs`.
- Approval: staging/suggestion approvals are used in WS approval handling.
- Assets: `generate_asset` + `expression_sheet` used in `crates/engine/src/api/websocket/ws_creator.rs`.
- World export: used in `crates/engine/src/api/websocket/ws_core.rs` and `crates/engine/src/api/http.rs`.
- Narrative execute effects: used during narrative event approvals in `crates/engine/src/api/websocket/mod.rs`.
- Visual state resolve: used for staging approval context in `crates/engine/src/api/websocket/mod.rs`.
- Management CRUD: World/Character/Location/Region/PlayerCharacter/Relationship/Observation handlers use the management use cases in `crates/engine/src/api/websocket/ws_core.rs`, `crates/engine/src/api/websocket/ws_location.rs`, `crates/engine/src/api/websocket/ws_player.rs`.

### Defined But Not Wired
- World import use case (`use_cases::world::ImportWorld`) has no API entry point.
- Queue processing use cases (`use_cases::queues::ProcessPlayerAction`, `ProcessLlmRequest`) have no API or background runner.
- Time operations beyond suggestion (advance/set/skip) are still handler-owned; no dedicated use case API.

## Handler Logic That Should Be Extracted Into Use Cases

### WebSocket Entry Points (Main Handler Module)
- World join/session snapshot build: world/locations/characters/scenes aggregation, plus connection/user broadcasts are in `crates/engine/src/api/websocket/mod.rs` and should be a `JoinWorld` use case.
- Movement post-processing: staging approvals, rule/LLM suggestions, navigation/items retrieval, and response assembly are handled in `crates/engine/src/api/websocket/mod.rs`; move orchestration should be split into a use case that returns a response-ready model.
- Staging suggestions: `generate_rule_based_suggestions` + `generate_llm_based_suggestions` are handler-local and should live in a staging/use case layer.
- Pending approval state: approval request storage and response/broadcast logic live in the handler module.
- Visual state resolution for staging is partially use case backed but still wrapped in handler helpers; orchestration should move to a use case.
- Challenge trigger prompts: handler fetches challenge entity + pushes prompts directly to client; should be a use case.
- Time suggestion approval/advance: request tracking, validation, and broadcasts are handler-owned; should be a time use case.

### WebSocket Core Handlers
- Time config and time advancement operations in `crates/engine/src/api/websocket/ws_core.rs` directly mutate world entities.
- NPC mood/disposition/relationship and region relationship logic in `crates/engine/src/api/websocket/ws_core.rs` mixes orchestration + entity access.
- Items placement/creation in `crates/engine/src/api/websocket/ws_core.rs` uses entity APIs directly.
- Inventory requests (`GetCharacterInventory`) in `crates/engine/src/api/websocket/ws_core.rs` assemble responses directly from entities.

### WebSocket Story/Lore Handlers
- Story event get/update/list in `crates/engine/src/api/websocket/ws_story_events.rs` operate directly on entity modules.
- Lore get/update/list in `crates/engine/src/api/websocket/ws_lore.rs` operate directly on entity modules.

### HTTP API
- `crates/engine/src/api/http.rs` uses entity calls directly for listing/getting worlds; should route through a use case layer for consistency.

## Other Maintainability Blockers
- The WebSocket handler module is too large and mixes protocol routing, persistence, domain orchestration, and broadcast logic.
- Response patterns are inconsistent: some handlers return `ResponseResult`, others send bespoke `ServerMessage` responses directly.
- Request/approval state is stored in the WebSocket layer, not in a use case or service boundary.
- Time operations are partially in `use_cases::time` (helpers + suggestion), partially in handlers, making it hard to reason about time flow.
- Missing background runner for queue use cases; LLM/player action queues cannot be processed without manual triggers.

## Proposed Refactor Phases (Big Bang, No Backward Compatibility)

Phase 1: Documentation + wiring map (this file) and agreement on priority ordering.

Phase 2: Extract “session join + snapshot” into a use case and slim down `websocket/mod.rs`.

Phase 3: Extract time control (advance/pause/set/skip) into time use cases; route all time commands through them.

Phase 4: Extract staging/approval orchestration into use cases (rule/LLM suggestions, pending approvals, broadcasting). Also move challenge trigger prompts into use case.

Phase 5: Extract NPC mood/disposition, inventory/items, story events, and lore handlers into use cases and rewire HTTP routes to use use cases.

Phase 6: Wire queue use cases via background runner or explicit API endpoints.
