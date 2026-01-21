# Remediation Plan

**Date:** January 21, 2026
**Status:** Pending Review
**Confirmed Issues:** 8

---

## Security Fixes

### S2: Cross-World Character Access

**File:** `crates/engine/src/api/websocket/ws_character.rs:69-93`

**Issue:** `GetCharacter` retrieves without validating world membership. A client could request characters from any world by knowing the ID.

**Fix:**
```rust
CharacterRequest::GetCharacter { character_id } => {
    let char_id = parse_character_id_for_request(&character_id, request_id)?;

    match state.app.use_cases.management.character.get(char_id).await {
        Ok(Some(character)) => {
            // ADD: World ownership validation
            if let Some(conn_world_id) = conn_info.world_id {
                if character.world_id() != conn_world_id && !conn_info.is_dm() {
                    return Err(ServerMessage::Response {
                        request_id: request_id.to_string(),
                        result: ResponseResult::error(
                            ErrorCode::Unauthorized,
                            "Character belongs to a different world"
                        ),
                    });
                }
            }
            // ... existing response code ...
        }
        // ...
    }
}
```

---

### S3: Error Message Information Leakage

**Files:** `crates/engine/src/api/websocket/ws_character.rs:64` and `:91`

**Issue:** Uses `e.to_string()` which exposes internal error details to clients. Other handlers correctly use `sanitize_repo_error()`.

**Current:**
```rust
Err(e) => Ok(ResponseResult::error(
    ErrorCode::InternalError,
    e.to_string(),  // Leaks internal details
)),
```

**Fix:**
```rust
Err(e) => {
    tracing::error!(error = %e, "Failed to list characters");
    Ok(ResponseResult::error(
        ErrorCode::InternalError,
        "Failed to retrieve characters"
    ))
}
```

---

### S4: UUID Input Length Validation

**File:** `crates/engine/src/api/websocket/mod.rs:1070-1082`

**Issue:** No explicit length check on UUID strings. Inconsistent with `request_id` validation which does have length checks.

**Fix:**
```rust
fn parse_uuid_for_request(
    id_str: &str,
    request_id: &str,
    error_msg: &str,
) -> Result<Uuid, ServerMessage> {
    if id_str.len() > 100 {
        return Err(ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(ErrorCode::BadRequest, "ID string too long"),
        });
    }

    Uuid::parse_str(id_str).map_err(|e| {
        tracing::debug!(input = %id_str, error = %e, "UUID parsing failed");
        ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(ErrorCode::BadRequest, error_msg),
        }
    })
}
```

---

## Error Handling Fixes

### E2: Character Data Fallbacks

**File:** `crates/engine/src/use_cases/staging/approve.rs:204-230, 300-314`

**Issue:** Missing characters result in empty NPC names being sent to clients.

**Current:**
```rust
Ok(None) => {
    tracing::warn!("Character not found, NPC will have incomplete data");
    (String::new(), None, None, MoodState::default(), true)
}
```

**Fix:**
```rust
let character = self.character
    .get(npc_info.character_id)
    .await?
    .ok_or(StagingError::CharacterNotFound(npc_info.character_id))?;
```

---

### E6: Session Join Silent Failures

**Files:**
- `crates/engine/src/use_cases/session/join_world_flow.rs:221`
- `crates/engine/src/use_cases/session/join_world.rs:64-67, 72-75`

**Issue:** `.ok()` discards errors silently. `LocationId::new()` creates a random ID as fallback instead of a safe empty default.

**Current:**
```rust
let location_id = self.scene.get_location(scene.id()).await
    .ok()
    .flatten()
    .unwrap_or_else(LocationId::new);  // Creates random ID
```

**Fix:**
```rust
let location_id = self.scene.get_location(scene.id()).await
    .map_err(|e| {
        tracing::error!(error = %e, scene_id = %scene.id(), "Failed to fetch location");
        e
    })?
    .ok_or_else(|| SessionError::SceneHasNoLocation(scene.id()))?;
```

---

### E7: Tool Parsing Silent Skip

**File:** `crates/engine/src/use_cases/queues/response_parser.rs:335-346`

**Issue:** Malformed LLM tool calls are silently skipped via `continue` with only a warning log.

**Current:**
```rust
Err(e) => {
    tracing::warn!(...);
    continue;
}
```

**Fix:**
```rust
Err(e) => {
    tracing::error!(
        tool_name = %name,
        json = json_str,
        error = %e,
        "Failed to parse tool arguments JSON - skipping tool"
    );
    continue;
}
```

---

## Value Object Fixes

### V1: StateName Default Violation

**File:** `crates/domain/src/value_objects/names.rs:878-879`

**Issue:** `#[derive(Default)]` creates an empty string, but `StateName::new()` validates that names cannot be empty.

**Current:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct StateName(String);
```

**Fix:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateName(String);

impl Default for StateName {
    fn default() -> Self {
        Self("Default".to_string())
    }
}
```

---

### V2: AdHocOutcomes Constructor Validation

**File:** `crates/domain/src/value_objects/ad_hoc_outcomes.rs:27-38`

**Issue:** Constructor accepts invalid input. `validate()` method exists but is never called.

**Current:**
```rust
pub fn new(...) -> Self {
    Self { /* fields */ }  // No validation
}
```

**Fix:**
```rust
pub fn new(
    success: impl Into<String>,
    failure: impl Into<String>,
    critical_success: Option<String>,
    critical_failure: Option<String>,
) -> Result<Self, DomainError> {
    let outcomes = Self {
        success: success.into(),
        failure: failure.into(),
        critical_success,
        critical_failure,
    };
    outcomes.validate()?;
    Ok(outcomes)
}
```

---

## Checklist

- [ ] S2: Add world validation to GetCharacter
- [ ] S3: Sanitize error messages in ws_character.rs (lines 64 and 91)
- [ ] S4: Add UUID length validation
- [ ] E2: Fail on missing character data in staging approval
- [ ] E6: Fix session join `.ok()` patterns
- [ ] E7: Upgrade tool parsing logging from warn to error
- [ ] V1: Fix StateName Default implementation
- [ ] V2: Add validation to AdHocOutcomes::new()
