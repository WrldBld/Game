# WrldBldr WebSocket Migration Completion Plan

**Created**: 2025-12-26
**Status**: IN PROGRESS

## Overview

**Goal**: Complete WebSocket-first migration, remove session concept entirely, delete REST CRUD routes.

**Total Estimated Time**: 20-28 hours

---

## Phase Summary

| Phase | Description | Est. Time | Status |
|-------|-------------|-----------|--------|
| 1 | Critical Security & Bugs | 2-3 hrs | COMPLETED |
| 2 | Complete Session Removal | 10-14 hrs | COMPLETED |
| 3 | REST Route Deletion | 2-3 hrs | COMPLETED |
| 4 | Player UI WebSocket Migration | 3-4 hrs | COMPLETED |
| 5 | Implementation Gaps | 4-5 hrs | IN PROGRESS |
| 6 | Technical Debt | 3-4 hrs | NOT STARTED |
| 7 | Testing | TBD | DEFERRED |

---

## Phase 1: Critical Security & Bugs
**Status**: COMPLETED

| # | Task | Files | Status |
|---|------|-------|--------|
| 1.1 | CORS env config | `server.rs`, `config.rs` | [x] Added CORS_ALLOWED_ORIGINS env var |
| 1.2 | UUID parsing safety | `story_event_repository.rs`, `narrative_event_repository.rs` | [x] Added parse_uuid_or_nil helper with logging |
| 1.3 | Fix async blocking | `world_connection_port_adapter.rs`, `world_connection_port.rs` | [x] Made query methods async |
| 1.4 | Async mutex fixes | `client.rs` | [x] Fixed mutex held across await (others were already correct) |

---

## Phase 2: Complete Session Removal
**Status**: COMPLETED

### 2.1 Domain Layer
| File | Changes | Status |
|------|---------|--------|
| `domain/src/ids.rs` | Remove `SessionId` | [x] |
| `domain/src/entities/player_character.rs` | Remove session fields/methods | [x] |
| `domain/src/lib.rs` | Remove `SessionId` export | [x] |

### 2.2 Protocol Layer
| File | Changes | Status |
|------|---------|--------|
| `protocol/src/messages.rs` | Remove/update session messages | [x] Legacy kept for compatibility |
| `protocol/src/app_events.rs` | Replace session_id with world_id | [x] |

### 2.3 Ports Layer
| File | Changes | Status |
|------|---------|--------|
| `engine-ports/src/outbound/session_management_port.rs` | DELETE | [x] |
| `engine-ports/src/outbound/repository_port.rs` | Remove session methods | [x] |
| `engine-ports/src/outbound/queue_port.rs` | session_id -> world_id | [x] |
| `engine-ports/src/outbound/mod.rs` | Update exports | [x] |

### 2.4 Application Layer
| File | Changes | Status |
|------|---------|--------|
| `dto/queue_items.rs` | session_id -> world_id | [x] |
| `dto/player_character.rs` | Remove session_id | [x] |
| `services/player_character_service.rs` | Remove session_id | [x] |
| `services/dm_approval_queue_service.rs` | Use world_id | [x] |
| `services/player_action_queue_service.rs` | Use world_id | [x] |
| `services/dm_action_queue_service.rs` | Use world_id | [x] |
| `services/llm_queue_service.rs` | Use world_id | [x] |
| `services/tool_execution_service.rs` | Use world context | [x] |
| `services/trigger_evaluation_service.rs` | Remove _session_id | [x] |
| `handlers/request_handler.rs` | Fix ListPlayerCharacters | [x] |

### 2.5 Adapters Layer
| File | Changes | Status |
|------|---------|--------|
| `infrastructure/session/` | DELETE directory | [x] |
| `infrastructure/mod.rs` | Remove session module | [x] |
| `infrastructure/state/mod.rs` | Remove sessions field | [x] |
| `persistence/player_character_repository.rs` | Remove session queries | [x] |
| `infrastructure/queue_workers.rs` | Use world_id | [x] |
| `infrastructure/websocket.rs` | Remove session refs | [x] |
| `infrastructure/websocket_helpers.rs` | Remove session conversions | [x] |
| `infrastructure/queues/*.rs` | Update for world_id | [x] |

---

## Phase 3: REST Route Deletion
**Status**: COMPLETED

### Files DELETED (16)
| File | Status |
|------|--------|
| `http/world_routes.rs` | [x] |
| `http/character_routes.rs` | [x] |
| `http/location_routes.rs` | [x] |
| `http/region_routes.rs` | [x] |
| `http/scene_routes.rs` | [x] |
| `http/interaction_routes.rs` | [x] |
| `http/goal_routes.rs` | [x] |
| `http/want_routes.rs` | [x] |
| `http/challenge_routes.rs` | [x] |
| `http/skill_routes.rs` | [x] |
| `http/story_event_routes.rs` | [x] |
| `http/narrative_event_routes.rs` | [x] |
| `http/event_chain_routes.rs` | [x] |
| `http/observation_routes.rs` | [x] |
| `http/sheet_template_routes.rs` | [x] |
| `http/suggestion_routes.rs` | [x] |

Note: `player_character_routes.rs` and `session_routes.rs` were deleted in Phase 2.

### Files KEPT (8)
- `export_routes.rs` - large file downloads
- `asset_routes.rs` - multipart file uploads
- `workflow_routes.rs` - file upload, ComfyUI
- `queue_routes.rs` - health check, admin
- `settings_routes.rs` - configuration
- `prompt_template_routes.rs` - LLM config
- `rule_system_routes.rs` - read-only reference
- `config_routes.rs` - app configuration

### Infrastructure Updates
| File | Status |
|------|--------|
| `http/mod.rs` | [x] Cleaned up module declarations and route registrations |
| `run/server.rs` | [x] Already using create_routes() |

---

## Phase 4: Player UI WebSocket Migration
**Status**: COMPLETED

### Connection State Updates
| File | Status |
|------|--------|
| `connection_state.rs` | [x] Added world_id, world_role, connected_users |
| `session_state.rs` | [x] Added world_id, world_role, connected_users accessors |
| `game_state.rs` | [x] Added trigger_entity_refresh() method |

### Message Handlers
| File | Handler | Status |
|------|---------|--------|
| `session_message_handler.rs` | WorldJoined | [x] Full implementation |
| `session_message_handler.rs` | WorldJoinFailed | [x] Error state propagation |
| `session_message_handler.rs` | UserJoined | [x] Connected users management |
| `session_message_handler.rs` | UserLeft | [x] Connected users management |
| `session_message_handler.rs` | Response | [x] Logging (correlation handled by RequestManager) |
| `session_message_handler.rs` | EntityChanged | [x] Cache invalidation triggers |
| `session_message_handler.rs` | SpectateTargetChanged | [x] Log entry |

---

## Phase 5: Implementation Gaps
**Status**: IN PROGRESS

### Critical Fixes
| Gap | File | Status |
|-----|------|--------|
| PC Data in WorldJoined | `websocket.rs` | [x] Fetches PC when role=Player |
| PC Data in UserJoined | `websocket.rs` | [x] Includes PC in broadcast |
| Fix SetSpectateTarget | `websocket.rs` | [x] Full implementation |
| WorldConnectionManager.set_spectate_target | `world_connection_manager.rs` | [x] Added method |
| DirectorialUpdate persist | `websocket.rs` | [ ] TODO - currently in-memory only |
| Wire NPC Mood Panel | `session_message_handler.rs` | [x] **DONE** - handlers exist, DM panel wired |

### Feature Implementation
| Feature | Status |
|---------|--------|
| Region Item Placement | [~] Partial - read/drop works, need place_item API |
| Staging Status API | [~] Partial - messages exist, need GetStagingStatus request |
| Character Stat Updates | [x] **DONE** - CharacterStatUpdated handler implemented |

---

## Phase 6: Technical Debt
**Status**: NOT STARTED

| Task | Status |
|------|--------|
| Split websocket.rs (~3700 lines) | [ ] |
| Error handling audit | [ ] |
| Remove unused code (warnings) | [ ] |

---

## Commits Made This Session

1. `b306f1d` - feat: complete Phase 1 & 2 - security fixes and session removal
2. `c0f7fb8` - refactor: delete CRUD REST routes - Phase 3
3. `1343c94` - feat: implement WebSocket-first protocol handlers in Player UI - Phase 4
4. `7442f31` - feat: implement PC data in JoinWorld and SetSpectateTarget - Phase 5 partial

---

## Code Removed

- **~5700 lines** of REST route handlers (Phase 3)
- **~4400 lines** of session infrastructure (Phase 2)
- **Total**: ~10,100 lines removed

---

## Remaining Work

### High Priority
- DirectorialUpdate persistence (currently in-memory only)
- Region Item Placement - complete the `place_item()` API
- Staging Status API - add `GetStagingStatus` request/response

### Completed (2025-12-26 review)
- ~~NPC Mood Panel wiring~~ - handlers exist, DM panel fully wired
- ~~Character Stat Updates~~ - CharacterStatUpdated handler implemented

### Low Priority (Phase 6)
- Split websocket.rs into modules
- Clean up unused code warnings
- Error handling audit
