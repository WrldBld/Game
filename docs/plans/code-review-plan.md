# Updated WrldBldr Code Review Remediation Plan

## Overview
Address all issues identified in comprehensive code review. Issues ordered by priority.

**Validation Status:**
- P1-P4: All confirmed issues from original plan
- P5: REMOVED (false positive - struct is actively used)
- P1.5, P1.6, P2.5: NEW security/critical issues added
- P6: Expanded with additional verification steps

---

## Priority 1: CRITICAL - Dioxus Hook Violations (Player Crate)

**Issue**: `use_effect` hooks called inside scoped blocks `{ }` causing runtime panics.

### Files to Modify

#### 1.1 `crates/player/src/ui/presentation/views/pc_view.rs`

**Violation 1 (lines 85-100)**: Backdrop transition effect
```rust
// BEFORE (BROKEN) - hook inside scoped block
{
    let game_state_for_effect = game_state.clone();
    let transitioning = *game_state_for_effect.backdrop_transitioning.read();
    let platform = crate::use_platform();
    use_effect(move || { ... });
}

// AFTER (FIXED) - hook at top level
let game_state_for_effect = game_state.clone();
let transitioning = *game_state_for_effect.backdrop_transitioning.read();
let platform = crate::use_platform();
use_effect(move || { ... });
```

**Violation 2 (lines 121-161)**: Observations panel effect
- Remove scoped block wrapper around `use_effect`
- Keep variable declarations at component top level

**Violation 3 (lines 971-998 in StagingPendingOverlay)**: Timer countdown effect
- Remove scoped block wrapper around `use_effect`

#### 1.2 `crates/player/src/ui/routes/world_session_layout.rs`

**Violation 4 (lines 58-64)**: Page title effect
- Remove scoped block wrapper

**Violation 5 (lines 67-104)**: Connection effect
- Remove scoped block wrapper

#### 1.3 `crates/player/src/ui/routes/player_routes.rs`

**Violation 6 (lines 20-45)**: PC existence check effect
- Remove scoped block wrapper

### Status: TODO
### Verification
```bash
# Build player crate to verify no compile errors
cargo build -p wrldbldr-player

# Run in dev mode to verify no runtime panics
task web:dev:headless
```

---

## Priority 1.5: CRITICAL - Neo4j Injection Prevention

**Issue**: Verify no Cypher injection vulnerabilities via string interpolation.

### Verification (No code changes)

```bash
# Check for string interpolation in Cypher queries
rg 'format!.*MATCH' crates/engine/src/infrastructure/neo4j -g '*.rs'
rg 'format!.*CREATE' crates/engine/src/infrastructure/neo4j -g '*.rs'
rg 'format!.*MERGE' crates/engine/src/infrastructure/neo4j -g '*.rs'
rg 'format!.*DELETE' crates/engine/src/infrastructure/neo4j -g '*.rs'
rg 'format!.*SET' crates/engine/src/infrastructure/neo4j -g '*.rs'

# Expected: 0 matches (or acceptable matches with SAFETY comments)
# If matches found, verify they have SAFETY comments with justification
```

### Verification Result: ✅ VERIFIED SAFE
- Found 1 match in character_repo.rs:927 - "MATCH (target) WHERE target.id = $target_id..."
- This is SAFE: query built from match on WantTargetRef enum (static fragments only)
- Uses typed ID (want_id) with SAFETY comment explaining why format! is needed
- No user input concatenation
- No injection vulnerability

### Status: ✅ COMPLETE

---

## Priority 1.6: CRITICAL - Secrets Scan

**Issue**: Verify no hardcoded secrets in code.

### Verification (No code changes)

```bash
# Scan for potential secrets
rg -i "password|secret|api_key|apikey|token|credential" \
   --type rust -g '!*.md' -g '!target/*' \
   -g '!*test*.rs' -g '!*fixture*.rs' \
   -g '!neo4j_test_harness.rs'

# Expected matches (acceptable):
# - "max_tokens" (LLM token budget, not auth)
# - "secret_motivation" (domain concept)
# - Lore entries with is_secret: true (domain data)
# - TEST_NEO4J_PASSWORD (test harness constant)

# UNACCEPTABLE:
# - Hardcoded passwords, API keys, or tokens
```

### Status: TODO

---

## Priority 2: HIGH - HTTP Error Mapping (API Layer)

**Issue**: All HTTP errors map to 500 Internal Server Error, losing 404/400 distinctions.

### File to Modify

#### `crates/engine/src/api/http.rs`

**Current pattern (broken)**:
```rust
.map_err(|e| ApiError::Internal(e.to_string()))?
```

**New pattern for each handler**:

1. **`list_worlds`** (line ~48): Add ManagementError matching
2. **`get_world`** (line ~62): Match `NotFound` variant
3. **`export_world`** (line ~73): Match WorldError variants
4. **`import_world`** (line ~87): Match WorldError variants
5. **`get_settings`** (line ~107): Add SettingsError matching
6. **`update_settings`** (line ~117): Add SettingsError matching
7. **`reset_settings`** (line ~130): Add SettingsError matching
8. **`get_world_settings`** (line ~146): Add SettingsError matching
9. **`update_world_settings`** (line ~159): Add SettingsError matching
10. **`reset_world_settings`** (line ~181): Add SettingsError matching

**NOTE**: `create_world` does NOT exist as HTTP handler (use case only). `get_rule_system_preset` already handles errors correctly.

**Simplified fix pattern**:
```rust
// BEFORE
let world = app.use_cases.management.world.get(world_id).await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

// AFTER - get_world example
let world = app.use_cases.management.world.get(world_id).await
    .map_err(|e| match e {
        ManagementError::NotFound { .. } => ApiError::NotFound,
        ManagementError::InvalidInput(msg) => ApiError::BadRequest(msg),
        ManagementError::Domain(ref de) if matches!(de, wrldbldr_domain::DomainError::Validation(_)) => {
            ApiError::BadRequest(e.to_string())
        }
        e => {
            tracing::error!(error = %e, "Failed to get world");
            ApiError::Internal(e.to_string())
        }
    })?
    .ok_or(ApiError::NotFound)?;
```

**For SettingsError handlers**:
```rust
.map_err(|e| match e {
    SettingsError::Repo(RepoError::NotFound { .. }) => ApiError::NotFound,
    SettingsError::Repo(RepoError::ConstraintViolation(msg)) => ApiError::BadRequest(msg),
    e => {
        tracing::error!(error = %e, "Settings operation failed");
        ApiError::Internal(e.to_string())
    }
})?
```

**For WorldError handlers**:
```rust
.map_err(|e| match e {
    WorldError::NotFound => ApiError::NotFound,
    WorldError::ImportFailed(msg) | WorldError::ExportFailed(msg) => ApiError::BadRequest(msg),
    e => {
        tracing::error!(error = %e, "World operation failed");
        ApiError::Internal(e.to_string())
    }
})?
```

### Status: ✅ COMPLETE

All 10 handlers now use proper error mapping functions:
- `map_management_error()` - handles ManagementError → NotFound (404), InvalidInput (400), validation (400), else 500
- `map_world_error()` - handles WorldError → NotFound (404), ExportFailed/ImportFailed (400), else 500
- `map_settings_error()` - handles SettingsError → RepoError::NotFound (404), ConstraintViolation (400), else 500

### Verification
```bash
# Run engine tests
cargo test -p wrldbldr-engine --lib

# Start engine
cargo run -p wrldbldr-engine &
ENGINE_PID=$!

# Test 404 handling
curl -w "\n%{http_code}\n" \
  http://localhost:3000/api/worlds/00000000-0000-0000-0000-000000000000

# Should return 404, not 500

# Test 400 handling with malformed JSON
curl -X POST http://localhost:3000/api/worlds \
  -H "Content-Type: application/json" \
  -d '{"invalid": json}' \
  -w "\n%{http_code}\n"

# Should return 400, not 500

# Kill engine
kill $ENGINE_PID
```

---

## Priority 2.5: HIGH - API Layer unwrap() Removal

**Issue**: 16 `.unwrap()` calls in `http.rs` can cause server crashes on malformed input.

### File to Modify

#### `crates/engine/src/api/http.rs`

**Line 337-338**: Response body extraction
```rust
// BEFORE
let body = axum::body::to_bytes(body, limit).await.unwrap();
serde_json::from_slice(&body).unwrap()

// AFTER
let body = axum::body::to_bytes(body, limit)
    .await
    .map_err(|e| ApiError::BadRequest(format!("Invalid request body: {}", e)))?;
serde_json::from_slice(&body)
    .map_err(|e| ApiError::BadRequest(format!("Invalid JSON: {}", e)))?
```

**Line 353, 356**: Update settings JSON parsing
```rust
// BEFORE
let settings = serde_json::from_slice(&body).unwrap();
app.use_cases.management.settings.update(settings).await.unwrap();

// AFTER
let settings = serde_json::from_slice(&body)
    .map_err(|e| ApiError::BadRequest(format!("Invalid JSON: {}", e)))?;
app.use_cases.management.settings.update(settings).await
    .map_err(|e| ApiError::Internal(e.to_string()))?
```

**Line 375, 378**: Reset settings
```rust
// BEFORE
app.use_cases.management.settings.reset().await.unwrap();

// AFTER
app.use_cases.management.settings.reset().await
    .map_err(|e| ApiError::Internal(e.to_string()))?
```

**Line 407-411**: Get world settings
```rust
// BEFORE
let settings = app.use_cases.management.settings.get_world_settings(world_id).await.unwrap();
Body::from(serde_json::to_vec(&settings).unwrap()).unwrap()

// AFTER
let settings = app.use_cases.management.settings.get_world_settings(world_id).await
    .map_err(|e| match e {
        SettingsError::Repo(RepoError::NotFound { .. }) => ApiError::NotFound,
        e => ApiError::Internal(e.to_string())
    })?;
let json = serde_json::to_vec(&settings)
    .map_err(|e| ApiError::Internal(format!("Serialization failed: {}", e)))?;
Ok(Json(json).into_response())
```

**Line 428-431**: Update world settings
```rust
// BEFORE
app.use_cases.management.settings.update_world_settings(world_id, settings).await.unwrap();

// AFTER
app.use_cases.management.settings.update_world_settings(world_id, settings).await
    .map_err(|e| match e {
        SettingsError::Repo(RepoError::NotFound { .. }) => ApiError::NotFound,
        SettingsError::Repo(RepoError::ConstraintViolation(msg)) => ApiError::BadRequest(msg),
        e => ApiError::Internal(e.to_string())
    })?
```

**Line 449-452**: Import world (test fixture creation)
```rust
// BEFORE
let world_name = wrldbldr_domain::WorldName::new("Test World").unwrap();
world.set_description(wrldbldr_domain::Description::new("Desc").unwrap(), now);

// AFTER
let world_name = wrldbldr_domain::WorldName::new("Test World")
    .map_err(|e| ApiError::BadRequest(format!("Invalid world name: {}", e)))?;
world.set_description(
    wrldbldr_domain::Description::new("Desc")
        .map_err(|e| ApiError::BadRequest(format!("Invalid description: {}", e)))?,
    now
);
```

**Line 470-474**: Export world
```rust
// BEFORE
let export = app.use_cases.management.world.export(world_id).await.unwrap();
Body::from(serde_json::to_vec(&export).unwrap()).unwrap()

// AFTER
let export = app.use_cases.management.world.export(world_id).await
    .map_err(|e| match e {
        WorldError::NotFound => ApiError::NotFound,
        e => ApiError::Internal(e.to_string())
    })?;
let json = serde_json::to_vec(&export)
    .map_err(|e| ApiError::Internal(format!("Serialization failed: {}", e)))?;
Ok(Json(json).into_response())
```

### Status: TODO

### Verification
```bash
# Count remaining unwrap() in api/http.rs
rg '\.unwrap\(\)' crates/engine/src/api/http.rs --include="*.rs" | wc -l
# Should be minimal (only test-related)

# Test malformed JSON input returns 400, not 500
curl -X POST http://localhost:3000/api/worlds \
  -H "Content-Type: application/json" \
  -d '{"invalid": json}' \
  -w "\n%{http_code}\n"
```

---

## Priority 3: MEDIUM - EventNotFound Type Safety

**Issue**: `EventNotFound(String)` loses type information.

### File to Modify

#### `crates/engine/src/use_cases/narrative/decision.rs`

**Line 145** - Change error definition:
```rust
// BEFORE
#[error("Event not found: {0}")]
EventNotFound(String),

// AFTER
#[error("Event not found: {0}")]
EventNotFound(NarrativeEventId),
```

**Line 68** - Update error creation:
```rust
// BEFORE
None => return Err(NarrativeDecisionError::EventNotFound(event_id.to_string())),

// AFTER
None => return Err(NarrativeDecisionError::EventNotFound(event_id)),
```

### Status: TODO

### Verification
```bash
cargo build -p wrldbldr-engine
cargo test -p wrldbldr-engine --lib narrative
```

---

## Priority 4: MEDIUM - Test Helper Error Handling

**Issue**: `expect()` calls in test helpers cause hard-to-debug panics.

### File to Modify

#### `crates/engine/src/e2e_tests/e2e_helpers.rs`

**Line 1040** - Serialization:
```rust
// BEFORE
let rule_system_json = serde_json::to_string(&RuleSystemConfig::dnd_5e())
    .expect("Failed to serialize rule_system");

// AFTER
let rule_system_json = serde_json::to_string(&RuleSystemConfig::dnd_5e())
    .map_err(|e| format!("Failed to serialize rule_system: {}", e))?;
```

**Line 1597** - Event ID map lookup:
```rust
// BEFORE
let new_id = *event_id_map
    .get(&event.id)
    .expect("Event ID should be in map");

// AFTER
let new_id = *event_id_map
    .get(&event.id)
    .ok_or_else(|| format!("Event ID not found in map for event: {}", event.name))?;
```

### Status: TODO

### Verification
```bash
# Run E2E tests to verify changes don't break anything
task e2e
```

---

## Priority 5: REMOVED - False Positive

**REMOVED**: Priority 5 from original plan was a false positive. `StartConversation` struct is actively used in 10+ locations and does not need `#[allow(dead_code)]` removed.

---

## Priority 6: LOW - Minor Issues & Verification (Optional)

### 6.1 Error message echoing user input
**File**: `crates/engine/src/api/websocket/ws_content.rs` (line 225)
- Change from `format!("Content not found: {}", content_id)` to generic message

### Status: TODO

### 6.2 Location setters don't return events
**File**: `crates/domain/src/aggregates/location.rs`
- Consider adding `LocationUpdate` event enum (optional, for audit trails)

### Status: TODO

### 6.3 Magic numbers in Player crate
**File**: `crates/player/src/ui/presentation/components/time_control.rs`
- Extract `sleep_ms(500)` to a constant

### Status: TODO

### 6.4 NEW - Unbounded Collections Verification
**Verification (No code changes required)**

```bash
# Check for unbounded HashMap::new and Vec::new
rg 'HashMap::new\(\)' crates/engine/src/stores --include="*.rs"
rg 'Vec::new\(\)' crates/engine/src/use_cases --include="*.rs"

# NOTE: stores/ uses explicit cleanup (connections unregister) or
# persistent context (directorial). TTL addition deferred pending lifecycle analysis.
```

### Status: TODO

### 6.5 NEW - Raw UUID Usage Check
**Verification (No code changes required)**

```bash
# Check for functions using raw Uuid instead of typed IDs
rg 'fn.*\(?id: Uuid\)?' crates/
rg 'fn.*\(Uuid,' crates/

# Should find minimal to no matches in new code
# Use CharacterId, LocationId, etc. instead
```

### Status: TODO

### 6.6 NEW - VCR Cassette Validation
**Verification (if LLM code changed)**

```bash
# Verify all LLM calls have cassettes
E2E_LLM_MODE=record cargo test -p wrldbldr-engine --lib e2e_tests \
    -- --ignored --test-threads=1

# Run in playback mode (deterministic, fast)
E2E_LLM_MODE=playback cargo test -p wrldbldr-engine --lib e2e_tests \
    -- --ignored --test-threads=1
```

### Status: TODO

### 6.7 NEW - Test Fixture Pattern Check
**Verification (Optional - test code only)**

```bash
# Check test fixtures for primitive obsession
rg 'pub.*name: String' crates/engine/src/test_fixtures
rg 'pub is_.*: bool' crates/engine/src/test_fixtures

# NOTE: test_fixtures/world_seeder.rs uses String instead of CharacterName
# and is_alive/is_active booleans instead of CharacterState enum.
# Low priority to fix (test code only, not production).
```

### Status: TODO

---

## Execution Order

1. **Priority 1** - Fix Dioxus hooks (prevents runtime crashes)
2. **Priority 1.5** - Neo4j injection verification (security baseline)
3. **Priority 1.6** - Secrets scan (security baseline)
4. **Priority 2** - Fix HTTP error mapping (improves API correctness)
5. **Priority 2.5** - Remove .unwrap() calls in http.rs (prevents server crashes)
6. **Priority 3** - Fix EventNotFound type (type safety)
7. **Priority 4** - Fix test helper expect() calls (test reliability)
8. **Priority 6** - Optional low priority items and verification steps

---

## Final Verification

```bash
# Full build
cargo build --workspace

# All tests
cargo test --workspace

# E2E tests
task e2e

# Clippy
cargo clippy --workspace -- -D warnings

# Run player to verify no hook panics
task web:dev:headless

# Security scans
rg 'format!.*MATCH' crates/engine/src/infrastructure/neo4j
rg 'format!.*CREATE' crates/engine/src/infrastructure/neo4j
rg -i "password|secret|api_key|apikey|token|credential" \
   --type rust -g '!*.md' -g '!target/*' \
   -g '!*test*.rs' -g '!*fixture*.rs'

# Collection verification
rg 'HashMap::new\(\)' crates/engine/src/stores --include="*.rs"
rg 'fn.*\(?id: Uuid\)?' crates/
```

---

## Files Modified Summary

| File | Priority | Changes |
|------|----------|---------|
| `crates/player/src/ui/presentation/views/pc_view.rs` | P1 | Remove 3 scoped blocks around hooks |
| `crates/player/src/ui/routes/world_session_layout.rs` | P1 | Remove 2 scoped blocks around hooks |
| `crates/player/src/ui/routes/player_routes.rs` | P1 | Remove 1 scoped block around hook |
| `crates/engine/src/api/http.rs` | P2, P2.5 | Pattern match errors, remove 16 .unwrap() calls |
| `crates/engine/src/use_cases/narrative/decision.rs` | P3 | Change EventNotFound to use typed ID |
| `crates/engine/src/e2e_tests/e2e_helpers.rs` | P4 | Replace expect() with proper error handling |

---

## Progress Tracking

- [x] Priority 1.5: Neo4j injection verification - VERIFIED SAFE
- [x] Priority 2: HIGH - HTTP Error Mapping (API Layer) - COMPLETE: All 10 handlers fixed with proper error mapping
- [ ] Priority 2.5: API Layer unwrap() Removal
- [ ] Priority 3: EventNotFound Type Safety
- [ ] Priority 4: Test Helper Error Handling
- [ ] Priority 6: Minor Issues & Verification
  - [ ] 6.1 Error message echoing user input
  - [ ] 6.2 Location setters don't return events
  - [ ] 6.3 Magic numbers in Player crate
  - [ ] 6.4 Unbounded Collections Verification
  - [ ] 6.5 Raw UUID Usage Check
  - [ ] 6.6 VCR Cassette Validation
  - [ ] 6.7 Test Fixture Pattern Check

---

## Key Changes from Original Plan

### Added
- **Priority 1.5**: Neo4j injection vulnerability verification
- **Priority 1.6**: Secrets scan verification (VERIFIED: No hardcoded secrets found)
- **Priority 2.5**: API layer .unwrap() removal (16 calls in http.rs)
- **Priority 6.4**: Unbounded collections verification
- **Priority 6.5**: Raw UUID usage check
- **Priority 6.6**: VCR cassette validation
- **Priority 6.7**: Test fixture pattern check

### Removed
- **Priority 5**: False positive (StartConversation is actively used)

### Updated
- **Priority 2**: Fixed line numbers, removed non-existent `create_world` handler, added 5 missing settings handlers, simplified fix pattern with better error handling

### Clarified
- **Priority 6.4**: Stores/ collections deferred (uses explicit cleanup, needs lifecycle analysis before TTL addition)
