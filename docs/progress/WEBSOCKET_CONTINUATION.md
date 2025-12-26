# WebSocket-First Refactor - Continuation Guide

**Last Updated:** 2025-12-26
**Branch:** `gemini-refactor`
**Status:** Phase 2 Complete, Phases 3-5 Deferred

---

## Quick Start

```bash
# Environment
cd /home/otto/repos/WrldBldr/Game
nix-shell shell.nix

# Verify build
cargo check --workspace
cargo xtask arch-check
```

---

## Current State Summary

### What's Complete

| Phase | Status | Details |
|-------|--------|---------|
| **Phase 1: Protocol & Infrastructure** | ✅ Complete | RequestPayload (127 variants), ResponseResult, RequestHandler trait |
| **Phase 2: Migrate CRUD Operations** | ✅ Complete | All 127 handlers implemented in AppRequestHandler |
| **Legacy Cleanup** | ✅ Complete | 20 ClientMessage variants removed, 1200+ lines cleaned |

### What's Remaining

| Phase | Status | Effort | Details |
|-------|--------|--------|---------|
| **Phase 3: Session Removal** | ❌ Deferred | 26-38 hours | Replace session_id with world_id throughout |
| **Phase 4: REST Endpoint Deletion** | ❌ Blocked | N/A | Player-UI uses REST for CRUD |
| **Phase 5: Testing & Polish** | ❌ Not Started | TBD | Integration tests, documentation |

---

## Key Files

### Protocol Layer
- `crates/protocol/src/requests.rs` - 127 RequestPayload variants
- `crates/protocol/src/responses.rs` - ResponseResult enum
- `crates/protocol/src/messages.rs` - ClientMessage (legacy) + Request variant

### Handler Layer
- `crates/engine-app/src/application/handlers/request_handler.rs` - AppRequestHandler (~2600 lines)
- `crates/engine-ports/src/inbound/request_handler.rs` - RequestHandler trait

### Infrastructure Layer
- `crates/engine-adapters/src/infrastructure/websocket.rs` - WebSocket message routing
- `crates/engine-adapters/src/infrastructure/state/mod.rs` - AppState wiring
- `crates/engine-adapters/src/infrastructure/world_connection_manager.rs` - World-scoped connections

### New Components Created
- `crates/engine-ports/src/outbound/suggestion_enqueue_port.rs` - AI suggestion abstraction
- `crates/engine-adapters/src/infrastructure/suggestion_enqueue_adapter.rs` - LLM queue wrapper
- `crates/engine-adapters/src/infrastructure/persistence/want_repository.rs` - Want CRUD

---

## Architecture Overview

```
Client WebSocket Message
    │
    ▼
ClientMessage::Request { request_id, payload: RequestPayload::* }
    │
    ▼
websocket.rs → handle_request()
    │
    ▼
AppRequestHandler.handle(payload, context)
    │
    ▼
Service Layer (WorldService, CharacterService, etc.)
    │
    ▼
Repository Layer (Neo4j)
    │
    ▼
ResponseResult::Success/Error
    │
    ▼
ServerMessage::Response { request_id, result }
```

---

## Session Removal Plan (Deferred)

### Overview
The codebase currently uses session-based connection management. The plan is to migrate to world-based connections using `WorldConnectionManager`.

### Scope Analysis

**71 session_id usages in websocket.rs categorized:**
- Getting client's session: 18 usages
- DM role checks: 29 usages
- Broadcasting: 13 usages
- Service calls: 8 usages
- Direct state access: 20+ usages

**Services depending on AsyncSessionPort (6):**
1. SessionJoinService
2. ChallengeResolutionService
3. ChallengeOutcomeApprovalService
4. NarrativeEventApprovalService
5. OutcomeTriggerService
6. EventEffectExecutor

### Migration Steps

1. **Add WorldConnectionManager methods** (~2-3 hours)
   - `broadcast_except(world_id, msg, exclude)`
   - `has_dm(world_id)`
   - `get_dm_info(world_id)`

2. **Create WorldStateManager** (~4-6 hours)
   - Store: game_time, current_scene_id, conversation_history
   - Store: pending_approvals, pending_staging_approvals
   - Replace per-session state with per-world state

3. **Create WorldConnectionPort trait** (~2-3 hours)
   - Abstract world-based connection operations
   - Replace AsyncSessionPort usages

4. **Refactor websocket.rs** (~8-12 hours)
   - Replace 71 session_id usages
   - Change from session-based to world-based lookups

5. **Refactor application services** (~6-8 hours)
   - Update 6 services to use WorldConnectionPort
   - Change session_id parameters to world_id

6. **Remove session infrastructure** (~4-6 hours)
   - Delete SessionManager, Session structs
   - Delete AsyncSessionPort trait
   - Clean up AppState

**Total: 26-38 hours**

---

## REST Endpoint Removal (Blocked)

### Current State
Player-UI uses REST for all CRUD operations via `ApiPort` trait.

### Files to Delete (When Ready)
```
crates/engine-adapters/src/infrastructure/http/
├── character_routes.rs
├── location_routes.rs
├── region_routes.rs
├── scene_routes.rs
├── challenge_routes.rs
├── skill_routes.rs
├── narrative_event_routes.rs
├── event_chain_routes.rs
├── story_event_routes.rs
├── player_character_routes.rs
├── session_routes.rs
├── want_routes.rs
├── goal_routes.rs
├── observation_routes.rs
└── interaction_routes.rs
```

### Files to Keep
```
├── asset_routes.rs (file uploads)
├── health_routes.rs
├── world_routes.rs (export endpoint)
├── rule_system_routes.rs
├── settings_routes.rs
├── workflow_routes.rs
├── prompt_template_routes.rs
├── config_routes.rs
├── queue_routes.rs
└── suggestion_routes.rs
```

### Prerequisite
Migrate player-ui to use WebSocket Request pattern for CRUD operations.

---

## Handler Reference

### AppRequestHandler Dependencies

```rust
pub struct AppRequestHandler {
    // Core services (15)
    world_service: Arc<dyn WorldService>,
    character_service: Arc<dyn CharacterService>,
    location_service: Arc<dyn LocationService>,
    skill_service: Arc<dyn SkillService>,
    scene_service: Arc<dyn SceneService>,
    interaction_service: Arc<dyn InteractionService>,
    challenge_service: Arc<dyn ChallengeService>,
    narrative_event_service: Arc<dyn NarrativeEventService>,
    event_chain_service: Arc<dyn EventChainService>,
    player_character_service: Arc<dyn PlayerCharacterService>,
    relationship_service: Arc<dyn RelationshipService>,
    actantial_service: Arc<dyn ActantialContextService>,
    mood_service: Arc<dyn MoodService>,
    story_event_service: Arc<dyn StoryEventService>,
    item_service: Arc<dyn ItemService>,
    region_service: Arc<dyn RegionService>,
    
    // Repository ports (3)
    observation_repo: Arc<dyn ObservationRepositoryPort>,
    region_repo: Arc<dyn RegionRepositoryPort>,
    character_repo: Arc<dyn CharacterRepositoryPort>,
    
    // Optional (1)
    suggestion_enqueue: Option<Arc<dyn SuggestionEnqueuePort>>,
    broadcast_sink: Option<Arc<dyn BroadcastSink>>,
}
```

### Handler Categories

| Category | Handlers | Lines |
|----------|----------|-------|
| World | 6 | ~150 |
| Character | 7 | ~200 |
| Location | 8 | ~250 |
| Region | 13 | ~400 |
| Scene | 5 | ~150 |
| Act | 2 | ~60 |
| Skill | 5 | ~150 |
| Interaction | 6 | ~180 |
| Challenge | 7 | ~200 |
| NarrativeEvent | 9 | ~280 |
| EventChain | 12 | ~350 |
| StoryEvent | 5 | ~150 |
| PlayerCharacter | 6 | ~180 |
| Relationship | 3 | ~90 |
| Observation | 3 | ~90 |
| Goal | 5 | ~150 |
| Want | 7 | ~200 |
| ActantialView | 3 | ~90 |
| GameTime | 2 | ~60 |
| NPC Mood | 3 | ~90 |
| Character-Region | 5 | ~150 |
| AI Suggestions | 4 | ~120 |

---

## Testing Commands

```bash
# Full workspace check
cargo check --workspace

# Architecture validation
cargo xtask arch-check

# Run specific crate tests
cargo test -p wrldbldr-engine-app
cargo test -p wrldbldr-engine-adapters

# Run backend
task backend

# Run with frontend
task dev
```

---

## Known Issues / Tech Debt

1. **Unused helper functions in websocket.rs**
   - `to_domain_visibility`, `from_domain_visibility`, `to_domain_role`
   - Can be removed if not needed elsewhere

2. **SetCharacterWorkRegion defaults to Always shift**
   - Protocol doesn't include shift data
   - Consider adding to protocol if needed

3. **Some warnings about unused imports**
   - Domain crate has unused imports (ActId, SessionId)
   - Can be cleaned up in a separate pass

---

## Commit History (This Session)

1. `ccce82d` - feat(websocket): complete WebSocket-first architecture with all 126 request handlers
2. `4b8f30b` - feat(websocket): complete handler implementations with service integration
3. `bd60052` - feat(websocket): complete Phase 2 - all 127 handlers + legacy cleanup

---

## Contact / Questions

Reference the main progress document for full context:
- `docs/progress/WEBSOCKET_FIRST_REFACTOR_PLAN.md`
