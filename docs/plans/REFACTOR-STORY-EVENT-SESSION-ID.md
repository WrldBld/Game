# Refactor: Remove session_id from Story Events

**Created**: 2025-12-25  
**Status**: **COMPLETE**  
**Estimated Effort**: ~3.5 hours  
**Completed**: 2025-12-25

---

## Background

Story events were incorrectly scoped to sessions (a bad design decision where worlds might be recyclable - they aren't). Story events should be **world-scoped only**.

### Current State Analysis

| Layer | `session_id` Type | Notes |
|-------|-------------------|-------|
| **Domain Entity** | Graph edge (optional) | `OCCURRED_IN_SESSION` edge |
| **Engine DTO** (`StoryEventResponseDto`) | `Option<String>` | Correct - optional |
| **Player App DTO** (`StoryEventData`) | `String` (non-optional) | **BUG** - should be `Option<String>` |
| **Repository** | Has `list_by_session`, `set_session`, `get_session` | To be removed |
| **API** | Optional `?session_id=` query param | To be removed |

### What We're Removing

Remove `session_id` from story events only. Keep it for:
- `PlayerCharacter.session_id` - Still needed (binds PC to active game)
- `SessionState` / connection management - Still needed
- Queue items - Still needed (routes messages to correct session)

---

## Phase 1: Backend - Remove session_id from Story Event Service (~1.5h)

### 1.1 Engine Service Layer

**File:** `crates/engine-app/src/application/services/story_event_service.rs`

Remove `session_id: SessionId` parameter from ALL `record_*` methods:

| Method | Line | Change |
|--------|------|--------|
| `record_dialogue_exchange()` | ~53 | Remove `session_id` param |
| `record_challenge_attempted()` | ~106 | Remove `session_id` param |
| `record_scene_transition()` | ~170 | Remove `session_id` param |
| `record_dm_marker()` | ~215 | Remove `session_id` param |
| `record_information_revealed()` | ~268 | Remove `session_id` param |
| `record_relationship_changed()` | ~321 | Remove `session_id` param |
| `record_item_acquired()` | ~369 | Remove `session_id` param |
| `record_narrative_event_triggered()` | ~417 | Remove `session_id` param |
| `record_session_started()` | ~467 | Remove `session_id` param |
| `record_session_ended()` | ~497 | Remove `session_id` param |
| `list_by_session()` | ~530 | Remove entire method |

Remove calls to `self.repository.set_session()` from all these methods.

### 1.2 Engine DTOs

**File:** `crates/engine-app/src/application/dto/story_event.rs`

```rust
// Remove from ListStoryEventsQueryDto (line 13):
pub session_id: Option<String>,  // DELETE

// Remove from CreateDmMarkerRequestDto (line 29):
pub session_id: String,  // DELETE

// Remove from StoryEventResponseDto (line 148):
pub session_id: Option<String>,  // DELETE

// Update StoryEventResponseDto::with_edges() - remove session_id param
// Update StoryEventResponseDto::from() - remove session_id field
```

### 1.3 Repository Port

**File:** `crates/engine-ports/src/outbound/repository_port.rs`

Remove from `StoryEventRepositoryPort` trait:
```rust
// DELETE these methods:
async fn list_by_session(&self, session_id: SessionId) -> Result<Vec<StoryEvent>>;
async fn set_session(&self, event_id: StoryEventId, session_id: SessionId) -> Result<bool>;
async fn get_session(&self, event_id: StoryEventId) -> Result<Option<SessionId>>;
```

### 1.4 Neo4j Repository Implementation

**File:** `crates/engine-adapters/src/infrastructure/persistence/story_event_repository.rs`

Remove implementations:
- `list_by_session()` (~lines 1010-1027)
- `set_session()` (~lines 1270-1285)
- `get_session()` (~lines 1287-1300)

### 1.5 HTTP Routes

**File:** `crates/engine-adapters/src/infrastructure/http/story_event_routes.rs`

- Remove `session_id` handling from `list_story_events()` (lines 40-48)
- Remove `session_id` parsing from `create_dm_marker()` (lines 147-149)

### 1.6 Callers of Story Event Service

**File:** `crates/engine-app/src/application/services/dm_approval_queue_service.rs`
- Update call to `record_dialogue_exchange()` - remove `session_id` argument

**Search for other callers:**
```bash
grep -r "record_dialogue_exchange\|record_challenge_attempted\|record_dm_marker" crates/engine-app/
```

---

## Phase 2: Frontend - Remove session_id from Player App (~1h)

### 2.1 Player App Service

**File:** `crates/player-app/src/application/services/story_event_service.rs`

```rust
// Change signature (line ~47):
pub async fn list_story_events(&self, world_id: &str, session_id: Option<&str>) -> ...
// To:
pub async fn list_story_events(&self, world_id: &str) -> ...

// Remove session_id query param building
```

### 2.2 Player App DTO

**File:** `crates/player-app/src/application/dto/world_snapshot.rs`

```rust
// Line 835 - DELETE:
pub session_id: String,
```

### 2.3 UI Components

**File:** `crates/player-ui/src/presentation/components/story_arc/add_dm_marker.rs`
- Remove `session_id` prop (line 12)
- Remove from request body

**File:** `crates/player-ui/src/presentation/components/story_arc/timeline_view.rs`
- Remove `session_id` from `TimelineViewProps` (line 93)
- Update component usage

**File:** `crates/player-ui/src/presentation/views/director/content.rs`
- Update DM marker creation to not require session_id

---

## Phase 3: Cleanup & Verification (~1h)

1. Run `cargo check --workspace` to find any missed references
2. Run `cargo xtask arch-check` to verify architecture
3. Update any tests that use session_id for story events
4. Remove unused imports

---

## Files Summary

| Layer | File | Changes |
|-------|------|---------|
| **Engine Service** | `story_event_service.rs` | Remove `session_id` from all `record_*` methods |
| **Engine DTO** | `story_event.rs` | Remove from DTOs |
| **Repository Port** | `repository_port.rs` | Remove session methods |
| **Neo4j Adapter** | `story_event_repository.rs` | Remove session edge methods |
| **HTTP Routes** | `story_event_routes.rs` | Remove session filtering |
| **DM Approval** | `dm_approval_queue_service.rs` | Update caller |
| **Player Service** | `story_event_service.rs` | Remove session_id param |
| **Player DTO** | `world_snapshot.rs` | Remove `session_id` field |
| **UI Components** | `add_dm_marker.rs`, `timeline_view.rs`, `director/content.rs` | Remove session_id |

---

## Notes

- Existing `OCCURRED_IN_SESSION` edges in Neo4j can remain - they're harmless but unused
- Session entity is still needed for PlayerCharacter binding and live game coordination
- This is a breaking API change but we control both frontend and backend
