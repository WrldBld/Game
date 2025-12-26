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
| 3 | REST Route Deletion | 2-3 hrs | NOT STARTED |
| 4 | Player UI WebSocket Migration | 3-4 hrs | NOT STARTED |
| 5 | Implementation Gaps | 4-5 hrs | NOT STARTED |
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
**Status**: NOT STARTED

### 2.1 Domain Layer
| File | Changes | Status |
|------|---------|--------|
| `domain/src/ids.rs` | Remove `SessionId` | [ ] |
| `domain/src/entities/player_character.rs` | Remove session fields/methods | [ ] |
| `domain/src/lib.rs` | Remove `SessionId` export | [ ] |

### 2.2 Protocol Layer
| File | Changes | Status |
|------|---------|--------|
| `protocol/src/messages.rs` | Remove/update session messages | [ ] |
| `protocol/src/app_events.rs` | Replace session_id with world_id | [ ] |

### 2.3 Ports Layer
| File | Changes | Status |
|------|---------|--------|
| `engine-ports/src/outbound/session_management_port.rs` | DELETE | [ ] |
| `engine-ports/src/outbound/repository_port.rs` | Remove session methods | [ ] |
| `engine-ports/src/outbound/queue_port.rs` | session_id -> world_id | [ ] |
| `engine-ports/src/outbound/mod.rs` | Update exports | [ ] |

### 2.4 Application Layer
| File | Changes | Status |
|------|---------|--------|
| `dto/queue_items.rs` | session_id -> world_id | [ ] |
| `dto/player_character.rs` | Remove session_id | [ ] |
| `services/player_character_service.rs` | Remove session_id | [ ] |
| `services/dm_approval_queue_service.rs` | Use world_id | [ ] |
| `services/player_action_queue_service.rs` | Use world_id | [ ] |
| `services/dm_action_queue_service.rs` | Use world_id | [ ] |
| `services/llm_queue_service.rs` | Use world_id | [ ] |
| `services/tool_execution_service.rs` | Use world context | [ ] |
| `services/trigger_evaluation_service.rs` | Remove _session_id | [ ] |
| `handlers/request_handler.rs` | Fix ListPlayerCharacters | [ ] |

### 2.5 Adapters Layer
| File | Changes | Status |
|------|---------|--------|
| `infrastructure/session/` | DELETE directory | [ ] |
| `infrastructure/mod.rs` | Remove session module | [ ] |
| `infrastructure/state/mod.rs` | Remove sessions field | [ ] |
| `persistence/player_character_repository.rs` | Remove session queries | [ ] |
| `infrastructure/queue_workers.rs` | Use world_id | [ ] |
| `infrastructure/websocket.rs` | Remove session refs | [ ] |
| `infrastructure/websocket_helpers.rs` | Remove session conversions | [ ] |
| `infrastructure/queues/*.rs` | Update for world_id | [ ] |

---

## Phase 3: REST Route Deletion
**Status**: NOT STARTED

### Files to DELETE (18)
| File | Status |
|------|--------|
| `http/world_routes.rs` | [ ] |
| `http/character_routes.rs` | [ ] |
| `http/location_routes.rs` | [ ] |
| `http/region_routes.rs` | [ ] |
| `http/scene_routes.rs` | [ ] |
| `http/interaction_routes.rs` | [ ] |
| `http/goal_routes.rs` | [ ] |
| `http/want_routes.rs` | [ ] |
| `http/challenge_routes.rs` | [ ] |
| `http/skill_routes.rs` | [ ] |
| `http/story_event_routes.rs` | [ ] |
| `http/narrative_event_routes.rs` | [ ] |
| `http/event_chain_routes.rs` | [ ] |
| `http/player_character_routes.rs` | [ ] |
| `http/observation_routes.rs` | [ ] |
| `http/session_routes.rs` | [ ] |
| `http/sheet_template_routes.rs` | [ ] |
| `http/suggestion_routes.rs` | [ ] |

### Files to KEEP (7)
- `export_routes.rs` - large file downloads
- `asset_routes.rs` - multipart file uploads
- `workflow_routes.rs` - file upload, ComfyUI
- `queue_routes.rs` - health check, admin
- `settings_routes.rs` - configuration
- `prompt_template_routes.rs` - LLM config
- `rule_system_routes.rs` - read-only reference

### Infrastructure Updates
| File | Status |
|------|--------|
| `http/mod.rs` | [ ] |
| `run/server.rs` | [ ] |

---

## Phase 4: Player UI WebSocket Migration
**Status**: NOT STARTED

### Player Services
| File | Status |
|------|--------|
| `player-app/services/world_service.rs` | [ ] |
| `player-app/services/player_character_service.rs` | [ ] |
| `player-app/services/character_service.rs` | [ ] |
| Other entity services | [ ] |

### Player UI Handlers
| File | Status |
|------|--------|
| `session_message_handler.rs` - WorldJoined | [ ] |
| `session_message_handler.rs` - Response | [ ] |
| `session_message_handler.rs` - EntityChanged | [ ] |

---

## Phase 5: Implementation Gaps
**Status**: NOT STARTED

### Critical Fixes
| Gap | File | Status |
|-----|------|--------|
| PC Data in WorldJoined | `websocket.rs` | [ ] |
| Fix Spectate Target | `websocket.rs` | [ ] |
| DirectorialUpdate persist | `websocket.rs` | [ ] |
| Wire NPC Mood Panel | `session_message_handler.rs` | [ ] |

### Feature Implementation
| Feature | Status |
|---------|--------|
| Region Item Placement | [ ] |
| Staging Status API | [ ] |
| Character Stat Updates | [ ] |

---

## Phase 6: Technical Debt
**Status**: NOT STARTED

| Task | Status |
|------|--------|
| Split websocket.rs | [ ] |
| Error handling audit | [ ] |
| Remove unused code | [ ] |

---

## Execution Log

### 2025-12-26
- Created completion plan
- Starting Phase 1...

