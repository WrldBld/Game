# Configurable List Limits

**Created:** January 20, 2026
**Priority:** High
**Reference:** Code review remediation C3 - DoS Prevention

---

## Problem Statement

Currently, 9 list handlers use hardcoded limits:
- **Default limit:** 50 items
- **Maximum limit:** 200 items

These limits are embedded throughout the codebase, making it difficult to:
1. Allow DMs or operators to tune limits for different scenarios
2. Adjust defaults based on server capacity
3. Configure per-world or per-environment settings

---

## Requirements

### 1. Three-Stage Retrieval System

Settings must be retrieved in order of precedence:

1. **Persistent Settings (Highest Priority)**
   - DM-specified value stored in database
   - World-specific override
   - Per-DM personal preference

2. **Environment Variables (Medium Priority)**
   - Global defaults for deployment
   - Can override compiled-in constants
   - Example: `WRLDBLDR_DEFAULT_LIST_LIMIT=100`

3. **Code Constants (Fallback)**
   - Hardcoded defaults in source code
   - Never used if higher-priority value exists
   - Ensure system always works even without config

### 2. Settings Infrastructure

**Storage Options:**

**Option A: Neo4j Storage (Recommended)**
- Pros: Already integrated with world data
- Pros: World-specific settings naturally organized
- Pros: Single database for all configuration
- Cons: Schema changes required

**Schema additions:**
```cypher
// ListSettings node
CREATE CONSTRAINT IF NOT EXISTS FOR (w:World)-[:HAS_LIST_SETTINGS]->(:ListSettings {
    default_list_limit: Integer,
    max_list_limit: Integer,
    character_list_limit: Integer OPTIONAL,
    scene_list_limit: Integer OPTIONAL,
    region_list_limit: Integer OPTIONAL,
    location_list_limit: Integer OPTIONAL
    npc_list_limit: Integer OPTIONAL
    connection_list_limit: Integer OPTIONAL,
    exit_list_limit: Integer OPTIONAL,
    spawn_point_list_limit: Integer OPTIONAL,
    social_network_list_limit: Integer OPTIONAL
})
```

**Option B: Redis Storage**
- Pros: Fast read/write
- Pros: Easy to cache
- Pros: Supports per-world keys
- Cons: Separate infrastructure from Neo4j
- Cons: No query capability for bulk operations

**Schema:**
```
redis://
  list:settings:{world_id}
    default_limit: 50
    max_limit: 200
    character_limit: 50
    scene_limit: 50
    region_limit: 50
    location_limit: 50
    npc_limit: 50
    connection_limit: 50
    exit_limit: 50
    spawn_limit: 50
    social_network_limit: 50
```

**Option C: Configuration Service (Simple)**
- Pros: Pure application logic
- Pros: No database schema changes
- Pros: Easy to test
- Cons: Requires separate service to maintain
- Cons: Not persisted across restarts

**Recommendation:** Start with Option A (Neo4j) for production-grade persistence with schema changes.

### 3. Settings Scope Hierarchy

Settings should support multiple scopes:

```rust
pub enum SettingsScope {
    Global,              // Apply to all worlds
    World(WorldId),      // Override for specific world
    Dm(String),           // DM's personal preference
    Environment,        // Environment variable override
}
```

### 4. Settings Types

Required setting types:

| Setting Name | Type | Description | Default |
|---------------|------|-------------|---------|
| `default_list_limit` | u32 | Fallback for unconfigured endpoints | 50 |
| `max_list_limit` | u32 | Upper bound enforced by all endpoints | 200 |
| `character_list_limit` | Option<u32> | Override for ListCharacters | None |
| `scene_list_limit` | Option<u32> | Override for ListScenes | None |
| `region_list_limit` | Option<u32> | Override for ListRegions | None |
| `location_list_limit` | Option<u32> | Override for ListLocations | None |
| `npc_list_limit` | Option<u32> | Override for ListNPCs | None |
| `connection_list_limit` | Option<u32> | Override for ListRegionConnections | None |
| `exit_list_limit` | Option<u32> | Override for GetRegionExits | None |
| `spawn_point_list_limit` | Option<u32> | Override for ListSpawnPoints | None |
| `social_network_limit` | Option<u32> | Override for GetSocialNetwork | None |

### 5. Validation Rules

All settings must satisfy:
- `default_list_limit <= max_list_limit`
- All per-endpoint limits (if set) must be between default and max
- `max_list_limit >= 1`
- All limits must be `u32` values (non-negative)

---

## Implementation Plan

### Phase 1: Add Settings Fields to AppSettings

**File:** `crates/engine/src/infrastructure/app_settings.rs`

Add two new fields to existing `AppSettings` struct:

```rust
// In "Validation Limits" section (around line 1035)
/// Default page size for list operations when no limit is specified
#[serde(default = "default_list_page_size")]
list_default_page_size: u32,

/// Maximum allowed page size for list operations (DoS prevention)
#[serde(default = "default_list_max_page_size")]
list_max_page_size: u32,
```

Add default functions:

```rust
// Around line 1080
fn default_list_page_size() -> u32 { 50 }
fn default_list_max_page_size() -> u32 { 200 }
```

Add accessors:

```rust
pub fn list_default_page_size(&self) -> u32 {
    self.list_default_page_size
}

pub fn list_max_page_size(&self) -> u32 {
    self.list_max_page_size
}
```

**Why:** Extend existing infrastructure instead of creating new files.

---

### Phase 2: Add Settings Metadata for UI

**File:** `crates/shared/src/settings.rs`

Add metadata for UI discoverability:

```rust
// Add to SETTINGS_FIELDS array
SettingsFieldMetadata {
    key: "list_default_page_size",
    display_name: "Default List Page Size",
    description: "Number of items returned per page when no limit is specified",
    field_type: SettingsFieldType::Integer,
    category: "Performance",
    default_value: serde_json::json!(50),
    min_value: Some(serde_json::json!(10)),
    max_value: Some(serde_json::json!(200)),
    requires_restart: false,
},

SettingsFieldMetadata {
    key: "list_max_page_size",
    display_name: "Maximum List Page Size",
    description: "Maximum items allowed per page (DoS prevention)",
    field_type: SettingsFieldType::Integer,
    category: "Performance",
    default_value: serde_json::json!(200),
    min_value: Some(serde_json::json!(50)),
    max_value: Some(serde_json::json!(1000)),
    requires_restart: false,
},
```

---

### Phase 3: Add Environment Variable Support

**File:** `crates/engine/src/use_cases/settings/settings_ops.rs`

Add method to apply environment variable overrides:

```rust
/// Apply environment variable overrides to settings.
/// Environment variables take precedence over database values but not DM overrides.
///
/// Supported variables:
/// - WRLDBLDR_LIST_DEFAULT_PAGE_SIZE
/// - WRLDBLDR_LIST_MAX_PAGE_SIZE
pub fn apply_env_overrides(settings: &mut AppSettings) {
    if let Ok(val) = std::env::var("WRLDBLDR_LIST_DEFAULT_PAGE_SIZE") {
        if let Ok(size) = val.parse::<u32>() {
            settings.list_default_page_size = size;
        }
    }

    if let Ok(val) = std::env::var("WRLDBLDR_LIST_MAX_PAGE_SIZE") {
        if let Ok(size) = val.parse::<u32>() {
            settings.list_max_page_size = size;
        }
    }
}
```

Call this in `get_global()` after loading from database.

---

### Phase 4: Create Helper Function for Handlers

**File:** `crates/engine/src/api/websocket/mod.rs`

Add centralized pagination helper:

```rust
/// Apply pagination limits using settings-based defaults.
/// Returns (limit, offset) with proper bounds.
pub fn apply_pagination_limits(
    settings: &AppSettings,
    limit: Option<u32>,
    offset: Option<u32>,
) -> (Option<u32>, Option<u32>) {
    let default_size = settings.list_default_page_size();
    let max_size = settings.list_max_page_size();

    let bounded_limit = Some(limit.unwrap_or(default_size).min(max_size));
    let bounded_offset = Some(offset.unwrap_or(0));

    (bounded_limit, bounded_offset)
}
```

---

### Phase 5: Update Handler Call Sites

Replace hardcoded pattern in all 9 list handlers:

**Files to update:**

1. **ws_character.rs** (line 21):
```rust
// Before
let limit = limit.unwrap_or(50).min(200);
let offset = offset.unwrap_or(0);

// After
let settings = state.app.settings().await;
let (limit, offset) = apply_pagination_limits(&settings, limit, offset);
```

2. **ws_scene.rs** (line 69)

3. **ws_player.rs** (line 282) - GetSocialNetwork

4. **ws_location.rs** (multiple locations):
   - ListLocations (line 21)
   - ListLocationConnections (line 189)
   - ListRegions (line 291)
   - GetRegionConnections (line 509)
   - GetRegionExits (line 644)
   - ListSpawnPoints (line 756)

**Pattern for all:**
- Get settings from `state.app.settings().await`
- Call `apply_pagination_limits(&settings, limit, offset)`
- Use returned values in repository calls

---

## Testing Strategy

### 1. Unit Tests

**File:** `tests/engine_tests/settings_tests.rs` (new)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    fn test_apply_pagination_limits_with_defaults() {
        let settings = AppSettings::default();
        let (limit, offset) = apply_pagination_limits(&settings, None, None);

        assert_eq!(limit, Some(50));
        assert_eq!(offset, Some(0));
    }

    #[tokio::test]
    fn test_apply_pagination_limits_with_client_limit() {
        let settings = AppSettings::default();
        let (limit, offset) = apply_pagination_limits(&settings, Some(100), None);

        assert_eq!(limit, Some(100));
        assert_eq!(offset, Some(0));
    }

    #[tokio::test]
    fn test_apply_pagination_limits_max_enforced() {
        let settings = AppSettings::default();
        let settings = settings.with_list_max_page_size(500); // Set max lower than default

        let (limit, offset) = apply_pagination_limits(&settings, Some(1000), None);

        // Client limit (1000) should be capped at max (500)
        assert_eq!(limit, Some(500));
    }

    #[tokio::test]
    fn test_apply_pagination_limits_with_offset() {
        let settings = AppSettings::default();
        let (limit, offset) = apply_pagination_limits(&settings, None, Some(25));

        assert_eq!(limit, Some(50));
        assert_eq!(offset, Some(25));
    }
}
```

### 2. Integration Tests

**File:** `tests/engine_tests/settings_integration_tests.rs` (new)

```rust
#[tokio::test]
async fn test_full_settings_flow() {
    // Create test world with custom settings
    let world_id = WorldId::new_v4();

    // 1. Verify default limits work
    let mut app = create_test_app().await;
    let result = get_characters(&app, world_id, None, None).await;
    assert!(result.len() <= 50);

    // 2. Test that handlers respect settings
    // Update app settings via environment or API (if implemented)
    let result = get_characters(&app, world_id, Some(100), None).await;
    assert!(result.len() <= 100);
}
```

### 3. Manual Testing Checklist

- [ ] Default limits (50) work without any settings
- [ ] Environment variables override defaults
- [ ] Per-endpoint limits enforced
- [ ] Max limit (200) is hard cap
- [ ] Invalid settings rejected with validation error

---

## Migration Path

### Rollout Strategy

1. **Phase 1 (Infrastructure)**
   - Extend existing AppSettings
   - Add settings metadata for UI
   - Add environment variable support
   - Add unit tests
   - No database schema changes required (simple approach)
   - Deploy settings to existing worlds

2. **Phase 2 (Service Layer)**
   - Implement pagination helper
   - Add env override integration
   - Update all 9 list handlers
   - Add integration tests
   - Deploy to staging

3. **Phase 3 (Validation)**
   - Run full test suite
   - Load test with existing worlds
   - Performance testing with large result sets
   - Update documentation

### Backward Compatibility

**Important:** This is a simple approach that maintains backward compatibility:
- Existing worlds without custom settings will continue using defaults (50/200)
- Environment variables can override defaults without any changes
- No breaking changes to client protocol
- No database migration required (using existing AppSettings)

---

## Success Criteria

Implementation is complete when:

1. ✅ `list_default_page_size` and `list_max_page_size` fields added to AppSettings
2. ✅ Settings metadata added to shared crate
3. ✅ `apply_env_overrides` function implemented in settings_ops.rs
4. ✅ `apply_pagination_limits` helper created in mod.rs
5. ✅ All 9 list handlers updated to use pagination helper
6. ✅ Unit tests cover pagination helper and env overrides
7. ✅ Integration tests verify end-to-end flow
8. ✅ Manual testing confirms behavior
9. ✅ Documentation updated (API docs, settings docs)

---

## Files Summary

| Phase | Files to Modify | Lines |
|--------|------------------|--------|
| 1: Settings Fields | crates/engine/src/infrastructure/app_settings.rs | +10 |
| 2: Settings Metadata | crates/shared/src/settings.rs | +50 |
| 3: Env Support | crates/engine/src/use_cases/settings/settings_ops.rs | +20 |
| 4: Helper Function | crates/engine/src/api/websocket/mod.rs | +30 |
| 5: Handler Updates | ws_character, ws_scene, ws_player, ws_location | ~200 |
| 6: Unit Tests | tests/engine_tests/settings_tests.rs | +100 |
| 7: Integration Tests | tests/engine_tests/settings_integration_tests.rs | +80 |
| **Total** | **~500 lines** |

---

## Open Questions

1. Should there be per-entity-type limits? (e.g., different limit for characters vs locations)
   - **Recommendation:** Start with global limits, add per-type later if needed

2. Should limits be cached in WsState or fetched per-request?
   - **Recommendation:** Fetch per-request from AppSettings (fast, already cached)

3. Should we add rate limiting for list operations?
   - **Recommendation:** Add to Phase 3 if DoS attacks detected

---

## Next Steps

1. Review and approve this plan with architecture/tech lead
2. Create GitHub issue tracking implementation phases
3. Assign Phase 1 to engineering team
4. Schedule phases 2-3 after Phase 1 completion
5. Update client documentation with new environment variables
