# Code Review Remediation Plan

## Big Bang Refactor

No backwards compatibility required. No production deployments or data exist.

---

# CRITICAL

## C1. ws_player.rs - User ID Spoofing

**File:** `crates/engine/src/api/websocket/ws_player.rs`

### C1a. GetMyPlayerCharacter (line 77)

**Current code:**
```rust
PlayerCharacterRequest::GetMyPlayerCharacter { world_id, user_id } => {
    // ...
    .get_by_user(world_id_typed, user_id)  // Uses untrusted user_id
```

**Fix:** Add validation before `get_by_user()`:
```rust
if user_id != conn_info.user_id.as_str() {
    return Err(ServerMessage::Response {
        request_id: request_id.to_string(),
        result: ResponseResult::error(ErrorCode::Unauthorized, "Cannot access other user's character"),
    });
}
```

### C1b. CreatePlayerCharacter (line 108)

**Current code:** Uses `data.user_id` from client payload.

**Fix:** Replace `data.user_id` with `Some(conn_info.user_id.as_str().to_string())`

### C1c. GetPlayerCharacter - Information Disclosure

**Lines:** 54, 470

**Issue:** No authorization check and exposes `user_id` in response.

**Fix:**
1. Add authorization check at line 54 (compare `pc.user_id()` with `conn_info.user_id` or require DM)
2. Remove `"user_id": pc.user_id()` from `pc_to_json()` at line 470

---

## C2. ws_creator.rs:72-74 - GetGenerationQueue

**File:** `crates/engine/src/api/websocket/ws_creator.rs`

**Current code:**
```rust
let effective_user_id = user_id
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| conn_info.user_id.to_string());
```

**Fix:** Replace entire block with:
```rust
let effective_user_id = conn_info.user_id.to_string();
```

---

## C3. ws_world.rs:147 - ExportWorld Missing DM Check

**File:** `crates/engine/src/api/websocket/ws_world.rs`

**Context:** Other sensitive operations (UpdateWorld, DeleteWorld) correctly require DM.

**Fix:** Add at line 147:
```rust
WorldRequest::ExportWorld { world_id } => {
    require_dm_for_request(conn_info, request_id)?;  // ADD THIS
```

---

## C4. connections.rs:143 - DM Slot Bypass

**File:** `crates/engine/src/api/connections.rs`

**Current code:**
```rust
if &info.user_id == joining_uid || info.user_id.is_empty() {
```

**Fix:** Remove `|| info.user_id.is_empty()`:
```rust
if &info.user_id == joining_uid {
```

---

## C5. ws_time.rs:983 - Panic Path

**File:** `crates/engine/src/api/websocket/ws_time.rs`

**Current code:**
```rust
wrldbldr_domain::CalendarId::new("gregorian").expect("gregorian is a valid calendar ID")
```

**Fix:** Change function signature to return `Result` and propagate:
```rust
pub(super) fn protocol_time_config_to_domain(
    config: &protocol::GameTimeConfig,
) -> Result<wrldbldr_domain::GameTimeConfig, String> {
    let calendar_id = wrldbldr_domain::CalendarId::new("gregorian")
        .map_err(|e| format!("Failed to initialize calendar: {}", e))?;

    Ok(wrldbldr_domain::GameTimeConfig::new(
        // ... params with calendar_id ...
    ))
}
```

Then update call sites (lines 817, 973) to handle Result.

---

# HIGH

## H1. connections.rs - Locks Across Await

**File:** `crates/engine/src/api/connections.rs`

**Locations:** Lines 296-317, 323-344, 347-368, 371-394

**Issue:** RwLock read guards held across `.await` points with 5-second timeouts. Can block all writers.

**Current pattern:**
```rust
let connections = self.connections.read().await;  // Lock acquired
// ...
match timeout(CRITICAL_SEND_TIMEOUT, sender.send(message)).await {  // .await while holding lock
```

**Fix:** Clone senders before releasing lock:
```rust
let senders: Vec<_> = {
    let connections = self.connections.read().await;
    connections.values()
        .filter(|(info, _)| /* filter condition */)
        .map(|(_, sender)| sender.clone())
        .collect()
};
// Lock released
for sender in senders {
    let _ = timeout(CRITICAL_SEND_TIMEOUT, sender.send(message.clone())).await;
}
```

---

## H2. ws_staging.rs:71-90 - TOCTOU Race

**File:** `crates/engine/src/api/websocket/ws_staging.rs`

**Issue:** Non-atomic remove + use sequence allows double-approval.

**Current code:**
```rust
let pending = state.pending_staging_requests.remove(&request_id).await;  // CHECK
// ... later ...
if let Some(pending) = pending {
    (pending.region_id, Some(pending.location_id))  // USE
} else {
    // Falls through to treat request_id as region_id
}
```

**Fix:** Add idempotency key tracking or use atomic compare-and-swap:
```rust
let pending = state.pending_staging_requests.remove_if_present(&request_id).await;
if pending.is_none() && state.processed_staging_requests.contains(&request_id).await {
    return Some(error_response(ErrorCode::Conflict, "Already processed"));
}
```

---

## H3. DoS - Unbounded Lists

Add `limit: u32` parameter (max 200) to these handlers:

| File | Line | Handler |
|------|------|---------|
| ws_character.rs | 15 | ListCharacters |
| ws_scene.rs | 67 | ListScenes |
| ws_player.rs | 252 | GetSocialNetwork |
| ws_location.rs | 14 | ListLocations |
| ws_location.rs | 182 | ListLocationConnections |
| ws_location.rs | 284 | ListRegions |
| ws_location.rs | 502 | GetRegionConnections |
| ws_location.rs | 637 | GetRegionExits |
| ws_location.rs | 749 | ListSpawnPoints |

**Pattern:**
```rust
// Add to request type
limit: Option<u32>,
offset: Option<u32>,

// In handler
let limit = limit.unwrap_or(50).min(200);
let offset = offset.unwrap_or(0);
```

---

## H4. Cross-World Access

Add world_id validation after entity retrieval.

### ws_lore.rs
| Line | Handler |
|------|---------|
| 55 | GetLore |
| 230-241 | UpdateLoreChunk |
| 293-304 | DeleteLoreChunk |
| 331 | GrantLoreKnowledge |
| 407 | RevokeLoreKnowledge |
| 481 | GetCharacterLore |
| 503 | GetLoreKnowers |

### ws_narrative_event.rs
| Line | Handler |
|------|---------|
| 33 | GetNarrativeEvent |
| 80 | UpdateNarrativeEvent |
| 109 | DeleteNarrativeEvent |
| 127 | SetNarrativeEventActive |
| 148 | SetNarrativeEventFavorite |
| 169 | TriggerNarrativeEvent |
| 206 | ResetNarrativeEvent |

### ws_event_chain.rs
| Line | Handler |
|------|---------|
| 33 | GetEventChain |
| 82 | UpdateEventChain |
| 113 | DeleteEventChain |
| 131 | SetEventChainActive |
| 152 | SetEventChainFavorite |
| 173 | AddEventToChain |
| 200 | RemoveEventFromChain |
| 222 | CompleteChainEvent |
| 244 | ResetEventChain |

**Pattern:**
```rust
let world_id = conn_info.world_id
    .ok_or_else(|| error_response(ErrorCode::BadRequest, "Not joined to world"))?;

let entity = /* fetch entity */;
if entity.world_id() != world_id {
    return Err(error_response(ErrorCode::Forbidden, "Entity not in current world"));
}
```

---

## H5. Silent Error Swallowing

### H5a. staging/approve.rs:123-129, 155-161

**File:** `crates/engine/src/use_cases/staging/approve.rs`

**Current code:**
```rust
if let Err(e) = self.location_state.set_active(location_id, loc_state_id).await {
    tracing::warn!(error = %e, "Failed to set active location state");
}
// Continues despite error
```

**Fix:** Propagate errors:
```rust
self.location_state.set_active(location_id, loc_state_id).await?;
```

### H5b. approval/mod.rs:247-264

**File:** `crates/engine/src/use_cases/approval/mod.rs`

**Current code:** Logs dialogue recording errors but continues.

**Fix:** Propagate errors or add to result for client notification.

### H5c. ws_approval.rs:84, 98-99

**File:** `crates/engine/src/api/websocket/ws_approval.rs`

**Current code:** Broadcasts with no error handling.

**Fix:** Log at warn level:
```rust
if let Err(e) = state.connections.broadcast_to_dms(world_id, dm_msg).await {
    tracing::warn!(error = %e, world_id = %world_id, "Failed to broadcast to DMs");
}
```

---

## H6. Transaction Safety

### H6a. asset_repo.rs:192-238

**File:** `crates/engine/src/infrastructure/neo4j/asset_repo.rs`

**Issue:** Deactivate + activate are separate transactions.

**Fix:** Single Cypher query:
```cypher
MATCH (old:GalleryAsset {entity_type: $entity_type, entity_id: $entity_id, asset_type: $asset_type, is_active: true})
SET old.is_active = false
WITH old
MATCH (new:GalleryAsset {id: $id})
SET new.is_active = true
RETURN new
```

### H6b. location_repo.rs:600-684

**File:** `crates/engine/src/infrastructure/neo4j/location_repo.rs`

**Issue:** Bidirectional edge creation in two operations.

**Fix:** Single Cypher with FOREACH:
```cypher
MATCH (from:Region {id: $from_id}), (to:Region {id: $to_id})
CREATE (from)-[:CONNECTS_TO {props}]->(to)
WITH from, to
FOREACH (_ IN CASE WHEN $bidirectional THEN [1] ELSE [] END |
    CREATE (to)-[:CONNECTS_TO {props}]->(from)
)
```

### H6c. staging/approve.rs:112-115

**File:** `crates/engine/src/use_cases/staging/approve.rs`

**Issue:** Save + activate not atomic.

**Fix:** Wrap in explicit transaction or add rollback on activation failure.

---

## H7. character_repo.rs:1989-2018 - Silent Type Assumption

**File:** `crates/engine/src/infrastructure/neo4j/character_repo.rs`

**Issue:** Unknown target labels default to Character.

**Current code:**
```rust
} else {
    Ok(Some(WantTarget::Character { ... }))  // Default!
}
```

**Fix:**
```rust
} else if target_labels.iter().any(|label| label == "Character") {
    Ok(Some(WantTarget::Character { ... }))
} else {
    Err(RepoError::database("query", format!("Unknown WantTarget labels: {:?}", target_labels)))
}
```

---

## H8. responses.rs:47, 74 - Serialization Failures

**File:** `crates/shared/src/responses.rs`

**Issue:** `unwrap_or_default()` silently discards serialization errors.

**Fix:** Log errors before defaulting:
```rust
pub fn success<T: Serialize>(data: T) -> Self {
    let value = match serde_json::to_value(&data) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::error!(error = %e, "Failed to serialize response data");
            None
        }
    };
    ResponseResult::Success { data: value }
}
```

---

## H9. Unbounded Input

### H9a. ws_creator.rs:617

Add upper bound:
```rust
const MAX_GRID_SIZE: u32 = 100;
if c > MAX_GRID_SIZE || r > MAX_GRID_SIZE {
    return Err(error_response(ErrorCode::BadRequest, "Grid size exceeds maximum"));
}
```

### H9b. ws_lore.rs - Lore fields

**File:** `crates/engine/src/use_cases/lore/mod.rs` lines 141-167

Add validation in handler:
- title: max 200 chars
- summary: max 1000 chars
- content per chunk: max 10,000 chars
- chunks: max 100
- tags: max 50 tags, 50 chars each

### H9c. ws_conversation.rs:175

Add length limit:
```rust
const MAX_MESSAGE_LENGTH: usize = 2000;
if message.len() > MAX_MESSAGE_LENGTH {
    return Some(error_response(ErrorCode::BadRequest, "Message too long"));
}
```

### H9d. http.rs:77-105

Add Axum body size limit middleware (10MB max).

### H9e. ws_staging.rs:172

Add guidance length limit:
```rust
const MAX_GUIDANCE_LENGTH: usize = 2000;
```

---

## H10. types.rs - Unbounded Protocol Types

**File:** `crates/shared/src/types.rs`

Add handler-level validation for `ApprovalDecision` variants (lines 47-67):
- `modified_dialogue`, `feedback`, `dm_response`: max 5000 chars
- `approved_tools`, `rejected_tools`: max 50 items, 100 chars each
- `item_recipients`: max 20 items, max 10 recipients per item

---

## H11. mod.rs:719-732 - Request ID Validation

**File:** `crates/engine/src/api/websocket/mod.rs`

Add at function entry:
```rust
if request_id.is_empty() || request_id.len() > 100 {
    return Some(ServerMessage::Response {
        request_id: "invalid".to_string(),
        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid request_id"),
    });
}
```

---

## H12. Cross-World Access in Use Cases (NEW)

Additional cross-world vulnerabilities in management use cases.

### management/location.rs:65-99
`update_location` fetches by ID without world_id check.

### management/scene.rs:64-98
`update` and `delete` lack world_id validation.

### management/character.rs:76-150
`update` operation can modify characters from other worlds.

### actantial/mod.rs:103-151
Goal `update` and `delete` operations lack world_id check.

### narrative/events.rs:123-175
`update`, `delete`, `set_active`, `set_favorite` operations lack world_id validation.

**Pattern:** After fetching entity, validate:
```rust
if entity.world_id() != conn_info.world_id.expect("joined to world") {
    return Err(/* unauthorized */);
}
```

---

# MEDIUM

## M1. Error Information Leakage

Replace `e.to_string()` with generic error message:

| File | Lines |
|------|-------|
| ws_npc.rs | 52, 117, 156, 228, 266, 304, 358, 402, 489, 531 |
| ws_world.rs | 164, 194 |
| ws_content.rs | 154 |

**Pattern:**
```rust
Err(e) => {
    tracing::error!(error = %e, "Operation failed");
    return Ok(ResponseResult::error(ErrorCode::InternalError, "Operation failed"));
}
```

---

## M2. Type Safety

### M2a. observation.rs:267
Change `pub npc_id: String` to `pub npc_id: CharacterId`

### M2b. region.rs - RegionConnection
Create `LockState` enum:
```rust
pub enum LockState {
    Unlocked,
    Locked { description: String },
}
```
Replace `is_locked: bool` + `lock_description: Option<String>`.

### M2c. stat_block.rs:104, 107
Create `StatName` newtype for HashMap keys.

### M2d. game_time.rs:183-194
Change `cost_for_action` to return `Option<u32>` or `Result<u32, Error>` for unknown actions.

### M2e. narrative_event.rs
Create `Keyword` newtype for trigger keywords.

### M2f. types.rs
Change String IDs to typed IDs where serde support exists.

---

## M3. Unchecked Numeric Casts

Use `try_into()` with error handling:

| File | Lines |
|------|-------|
| location_repo.rs | 81-84, 97, 156, 165-168 |
| challenge_repo.rs | 264, 269 |
| narrative_repo.rs | 1412-1414 |
| lore_repo.rs | 77 |
| interaction_repo.rs | 124 |
| scene_repo.rs | 213 |
| act_repo.rs | 70 |
| fivetools.rs | 1228 |
| character_sheet/mod.rs | 594, 600, 630, 636, 666 |
| narrative/execute_effects.rs | 887, 891 |
| assets/expression_sheet.rs | 92-93 |
| types.rs | 203 |
| staging/types.rs | 160 |

**Pattern:**
```rust
let value: u32 = i64_value.try_into()
    .map_err(|_| RepoError::data("value out of range"))?;
```

---

## M4. Domain Logic

### M4a. character.rs:374
Validate `amount > 0` in `apply_damage` or use unsigned type.

### M4b. character.rs:510
Document integer division behavior or use explicit rounding:
```rust
(max / 2).max(1)  // Document: rounds down
// OR
((max + 1) / 2).max(1)  // Rounds up
```

### M4c. challenge.rs:602-626
Return `Result<Self, DomainError>` from `dc()` and `percentage()` instead of `assert!`.

### M4d. challenge.rs:704-712
Consider private fields with builder if invariants added later.

### M4e. challenge.rs:683-696
Add same validation as `dc()` and `percentage()` constructors in `parse()`.

### M4f. event_chain.rs:341
Use `.round()` before casting:
```rust
format!("{}%", (self.progress() * 100.0).round() as u32)
```

---

## M5. Silent Data Loss

### M5a. player_character_repo.rs:370
Log JSON parse failures.

### M5b. helpers.rs:131-137, 271-277
Add warn logging to `get_json_or_default`:
```rust
fn get_json_or_default<T: DeserializeOwned + Default>(&self, field: &str) -> T {
    self.get::<String>(field)
        .ok()
        .filter(|s| !s.is_empty())
        .and_then(|s| match serde_json::from_str(&s) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::warn!(field = %field, error = %e, "JSON parse failed, using default");
                None
            }
        })
        .unwrap_or_default()
}
```

### M5c. story_events/mod.rs:93, narrative/chains.rs:114,164
Log dropped tags:
```rust
.filter_map(|s| match Tag::new(&s) {
    Ok(tag) => Some(tag),
    Err(e) => {
        tracing::warn!(tag = %s, error = %e, "Dropping invalid tag");
        None
    }
})
```

### M5d. skill.rs - base_attribute parsing
Log when `base_attribute` parsing fails (content_repo.rs:56-58).

### M5e. region_state.rs:69,201 + location_state.rs:67,199
Return `Result` from constructors or log validation failures.

### M5f. character_repo.rs:338-342
Log archetype_history JSON parse failures.

### M5g. *_repo.rs - Invalid UUIDs
Log before returning None:
```rust
Uuid::parse_str(&id).ok().or_else(|| {
    tracing::warn!(id = %id, "Invalid UUID");
    None
})
```

---

## M6. connections.rs:53 - Memory Leak

**Issue:** `directorial_contexts` DashMap has no TTL or cleanup.

**Fix:**
1. Add `clear_directorial_context(world_id)` call in `leave_world()` when last DM disconnects
2. Consider TTL-based expiration for idle worlds

---

## M7. Error Context - .ok() Without Logging

Add warn logging before `.ok()`:

| File | Lines |
|------|-------|
| ws_movement.rs | 550, 707, 719 |
| movement/mod.rs | 82-99 |
| scene_repo.rs | 584 |
| join_world_flow.rs | 221 |
| challenge/mod.rs | 774 |
| management/skill.rs | 58, 103 |

**Pattern:**
```rust
match operation().await {
    Ok(v) => Some(v),
    Err(e) => {
        tracing::warn!(error = %e, "Operation failed");
        None
    }
}
```

---

## M8. ws_creator.rs:311-325 - Suggestion Deletion

Return NotFound when deletion fails:
```rust
if deleted {
    Ok(ResponseResult::success_empty())
} else {
    Ok(ResponseResult::error(ErrorCode::NotFound, "Suggestion not found"))
}
```

---

## M9. character_repo.rs - Invalid Enums Silent

**Lines:** 1627, 1651, 1655, 1916-1922

Log when enum parsing fails:
```rust
.and_then(|s| match s.parse::<RegionShift>() {
    Ok(v) => Some(v),
    Err(e) => {
        tracing::warn!(value = %s, error = %e, "Invalid RegionShift");
        None
    }
})
```

---

# LOW

## L1. Comment Additions

Add `// Intentionally ignore - <reason>` to intentional error discards:
- queues/mod.rs:847-858 (already has good comment)
- Test files with intentional `.ok()` usage

---

## L2. FlagName Newtype

**File:** `crates/domain/src/entities/game_flag.rs`

Create newtype:
```rust
pub struct FlagName(String);

impl FlagName {
    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let s = name.into();
        if s.is_empty() || s.len() > 128 {
            return Err(DomainError::validation("Invalid flag name"));
        }
        Ok(Self(s))
    }
}
```

---

## L3. Testing

Replace `.unwrap()` with `.expect("message")` in test_fixtures:
- image_mocks.rs
- world_seeder.rs
- mod.rs
- llm_integration.rs

---

## L4. item.rs:49-52 - Container Properties

Create enum:
```rust
pub enum ContainerProperties {
    NotContainer,
    Container { limit: Option<u32> },
}
```

Replace `can_contain_items: bool` + `container_limit: Option<u32>`.

---

# Verification

After each change:
```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```
