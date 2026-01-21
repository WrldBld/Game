# Configurable List Limits - Simplified Implementation

**Created:** January 20, 2026
**Updated:** January 20, 2026
**Priority:** High
**Reference:** Code review remediation C3 - DoS Prevention

---

## Problem Statement

Currently, 9 list handlers use hardcoded limits:
- **Default limit:** 50 items
- **Maximum limit:** 200 items

These limits are embedded throughout codebase, making it difficult to:
1. Allow DMs or operators to tune limits for different scenarios
2. Adjust defaults based on server capacity
3. Configure per-world or per-environment settings

---

## Architecture Discovery

### Existing Infrastructure ✅

The codebase **already has a comprehensive settings system**:

| Component | Location | Status |
|-----------|----------|--------|
| `AppSettings` | `crates/engine/src/infrastructure/app_settings.rs` | ✅ Ready |
| `SettingsRepo` | `crates/engine/src/infrastructure/ports/repos.rs` | ✅ Ready |
| `SettingsOps` | `crates/engine/src/use_cases/settings/settings_ops.rs` | ✅ Ready |
| `SettingsFieldMetadata` | `crates/shared/src/settings.rs` | ✅ Ready |
| Settings API | `crates/engine/src/api/http.rs` | ✅ Ready |

**AppSettings already includes:**
```rust
/// Default page size for list operations when no limit is specified
#[serde(default = "default_list_page_size")]
list_default_page_size: u32,

/// Maximum allowed page size for list operations (DoS prevention)
#[serde(default = "default_list_max_page_size")]
list_max_page_size: u32,
```

**Plus:**
- Token budget configuration (`ContextBudgetConfig`)
- Circuit breaker settings
- Health check settings
- Staging system settings
- Game defaults and validation limits
- Asset generation settings

### Strategy: Extend Existing System

Instead of creating new infrastructure, **extend the existing `AppSettings`**:

**Benefits:**
- ✅ Leverages battle-tested infrastructure
- ✅ Maintains consistency with existing patterns
- ✅ No new repository code needed
- ✅ Settings API already supports per-world configuration
- ✅ Simpler implementation (1-2 days vs. 6-10 days)
- ✅ Fully backward compatible

---

## Requirements

### 1. Three-Stage Retrieval System

Settings must be retrieved in order of precedence:

1. **Persistent Settings (Highest Priority)**
   - DM-specified value stored in database via `SettingsRepo`
   - World-specific override (`AppSettings.world_id = Some(world_id)`)
   - Already implemented in `SettingsOps::get_for_world()`

2. **Environment Variables (Medium Priority)**
   - Global defaults for deployment
   - Can override compiled-in constants
   - Example: `WRLDBLDR_LIST_DEFAULT_PAGE_SIZE=100`

3. **Code Constants (Fallback)**
   - Hardcoded defaults in `app_settings.rs`
   - Never used if higher-priority value exists
   - Ensure system always works even without config

### 2. Settings Fields to Add

Add to existing `AppSettings` (around line 1035):

```rust
// ============================================================================
// List Pagination Limits - Environment Overrides
// ============================================================================
/// Environment variable override for default list page size
#[serde(default)]
pub list_default_page_size_override: Option<u32>,

/// Environment variable override for max list page size
#[serde(default)]
pub list_max_page_size_override: Option<u32>,
```

### 3. Settings Metadata to Add

Add to `crates/shared/src/settings.rs` (in `SETTINGS_FIELDS` array):

```rust
SettingsFieldMetadata {
    key: "list_default_page_size",
    display_name: "List Page Size (Default)",
    description: "Number of items returned per page when no limit is specified. Can be overridden by WRLDBLDR_LIST_DEFAULT_PAGE_SIZE environment variable.",
    field_type: SettingsFieldType::Integer,
    category: "Performance",
    default_value: serde_json::json!(50),
    min_value: Some(serde_json::json!(10)),
    max_value: Some(serde_json::json!(200)),
    requires_restart: false,
},

SettingsFieldMetadata {
    key: "list_max_page_size",
    display_name: "List Page Size (Maximum)",
    description: "Maximum items allowed per page (DoS prevention). Can be overridden by WRLDBLDR_LIST_MAX_PAGE_SIZE environment variable.",
    field_type: SettingsFieldType::Integer,
    category: "Performance",
    default_value: serde_json::json!(200),
    min_value: Some(serde_json::json!(50)),
    max_value: Some(serde_json::json!(1000)),
    requires_restart: false,
},
```

### 4. Accessor Methods to Add

Add to `AppSettings` implementation (around line 1260):

```rust
/// Get effective default list page size (with environment override applied)
pub fn list_default_page_size_effective(&self) -> u32 {
    self.list_default_page_size_override
        .unwrap_or(self.list_default_page_size)
}

/// Get effective max list page size (with environment override applied)
pub fn list_max_page_size_effective(&self) -> u32 {
    self.list_max_page_size_override
        .unwrap_or(self.list_max_page_size)
}
```

---

## Implementation Plan

### Phase 1: Extend AppSettings (0.5 day)

**File:** `crates/engine/src/infrastructure/app_settings.rs`

**Task 1: Add two new fields**

Add around line 1035 (in "Validation Limits" section):

```rust
// ============================================================================
// List Pagination Limits - Environment Overrides
// ============================================================================
/// Environment variable override for default list page size
#[serde(default)]
pub list_default_page_size_override: Option<u32>,

/// Environment variable override for max list page size
#[serde(default)]
pub list_max_page_size_override: Option<u32>,
```

**Task 2: Add accessor methods**

Add around line 1260 (after existing accessors):

```rust
/// Get effective default list page size (with environment override applied)
pub fn list_default_page_size_effective(&self) -> u32 {
    self.list_default_page_size_override
        .unwrap_or(self.list_default_page_size)
}

/// Get effective max list page size (with environment override applied)
pub fn list_max_page_size_effective(&self) -> u32 {
    self.list_max_page_size_override
        .unwrap_or(self.list_max_page_size)
}
```

**Verification:**
```bash
cargo check -p wrldbldr-engine
cargo test -p wrldbldr-engine app_settings_tests
```

---

### Phase 2: Update SettingsOps for Environment Variables (0.5 day)

**File:** `crates/engine/src/use_cases/settings/settings_ops.rs`

**Task: Add environment variable loading**

Add method to apply environment variable overrides:

```rust
/// Apply list limit environment variable overrides to settings.
///
/// Supported environment variables:
/// - WRLDBLDR_LIST_DEFAULT_PAGE_SIZE: Override default list page size
/// - WRLDBLDR_LIST_MAX_PAGE_SIZE: Override max list page size
pub fn apply_env_list_limits(settings: &mut AppSettings) {
    if let Ok(val) = std::env::var("WRLDBLDR_LIST_DEFAULT_PAGE_SIZE") {
        if let Ok(size) = val.parse::<u32>() {
            if size >= 10 && size <= 200 {
                settings.list_default_page_size_override = Some(size);
            }
        }
    }

    if let Ok(val) = std::env::var("WRLDBLDR_LIST_MAX_PAGE_SIZE") {
        if let Ok(size) = val.parse::<u32>() {
            if size >= 50 && size <= 1000 {
                settings.list_max_page_size_override = Some(size);
            }
        }
    }
}
```

**Update existing `load_settings_from_env()` method:**

Already exists in settings loader. Add call to new method:

```rust
pub fn load_settings_from_env(base_settings: AppSettings) -> AppSettings {
    let mut settings = base_settings;

    // Existing env var loading (circuit breaker, health checks, etc.)
    settings = apply_env_overrides(settings);

    // NEW: Apply list limit overrides
    apply_env_list_limits(&mut settings);

    settings
}
```

**Verification:**
```bash
cargo check -p wrldbldr-engine
cargo test -p wrldbldr-engine settings_tests
```

---

### Phase 3: Add Pagination Helper (0.5 day)

**File:** `crates/engine/src/api/websocket/mod.rs` (or create `crates/engine/src/api/websocket/pagination.rs`)

**Task: Add centralized pagination helper**

```rust
/// Apply pagination limits using settings-based defaults.
///
/// Returns (limit, offset) with proper bounds:
/// - Client-provided limit is respected if specified
/// - Environment variable overrides default if set
/// - Maximum limit is always enforced (hard cap)
///
/// # Example
/// ```ignore
/// let settings = state.app.settings().await;
/// let (limit, offset) = apply_pagination_limits(&settings, client_limit, client_offset);
/// ```
pub fn apply_pagination_limits(
    settings: &AppSettings,
    client_limit: Option<u32>,
    client_offset: Option<u32>,
) -> (u32, Option<u32>) {
    let default_limit = settings.list_default_page_size_effective();
    let max_limit = settings.list_max_page_size_effective();

    // Client limit, or default, capped at max
    let limit = client_limit.unwrap_or(default_limit).min(max_limit);

    // Offset (default to 0)
    let offset = client_offset.unwrap_or(0);

    (limit, Some(offset))
}
```

**Verification:**
```bash
cargo check -p wrldbldr-engine
cargo test -p wrldbldr-engine websocket_tests
```

---

### Phase 4: Update Settings Metadata (0.5 day)

**File:** `crates/shared/src/settings.rs`

**Task: Add list limit metadata**

Add to `SETTINGS_FIELDS` array (around existing fields):

```rust
SettingsFieldMetadata {
    key: "list_default_page_size",
    display_name: "List Page Size (Default)",
    description: "Number of items returned per page when no limit is specified. Can be overridden by WRLDBLDR_LIST_DEFAULT_PAGE_SIZE environment variable.",
    field_type: SettingsFieldType::Integer,
    category: "Performance",
    default_value: serde_json::json!(50),
    min_value: Some(serde_json::json!(10)),
    max_value: Some(serde_json::json!(200)),
    requires_restart: false,
},

SettingsFieldMetadata {
    key: "list_max_page_size",
    display_name: "List Page Size (Maximum)",
    description: "Maximum items allowed per page (DoS prevention). Can be overridden by WRLDBLDR_LIST_MAX_PAGE_SIZE environment variable.",
    field_type: SettingsFieldType::Integer,
    category: "Performance",
    default_value: serde_json::json!(200),
    min_value: Some(serde_json::json!(50)),
    max_value: Some(serde_json::json!(1000)),
    requires_restart: false,
},
```

**Verification:**
```bash
cargo check -p wrldbldr-shared
cargo test -p wrldbldr-shared settings_tests
```

---

### Phase 5: Update List Handlers (2-3 days)

**Files to Update:**

1. `crates/engine/src/api/websocket/ws_character.rs` (line ~21)
2. `crates/engine/src/api/websocket/ws_scene.rs` (line ~69)
3. `crates/engine/src/api/websocket/ws_player.rs` (line ~282) - GetSocialNetwork
4. `crates/engine/src/api/websocket/ws_location.rs` (multiple lines):
   - ListLocations (line ~21)
   - ListLocationConnections (line ~189)
   - ListRegions (line ~291)
   - GetRegionConnections (line ~509)
   - GetRegionExits (line ~644)
   - ListSpawnPoints (line ~756)

**Pattern to Apply:**

```rust
// BEFORE (hardcoded):
CharacterRequest::ListCharacters { world_id, limit: client_limit, offset } => {
    let limit = client_limit.unwrap_or(50).min(200);
    let offset = offset.unwrap_or(0);
    // ... rest of handler
}

// AFTER (settings-based):
CharacterRequest::ListCharacters { world_id, limit: client_limit, offset } => {
    let world_id_typed = parse_world_id_for_request(&world_id, request_id)?;

    // Get settings (cached in WsState)
    let settings = state.app.settings().await;

    // Apply pagination limits with environment override support
    let (limit, offset) = apply_pagination_limits(&settings, client_limit, offset);

    // ... use limit, offset in repository call
    match state.app.use_cases.character.list(..., limit, offset).await {
        // ...
    }
}
```

**Important Notes:**
- `state.app.settings().await` already caches settings in `WsState`
- No performance impact (cached after first call)
- Settings automatically refreshed when updated via API

**Verification per file:**
```bash
cargo check -p wrldbldr-engine
cargo test -p wrldbldr-engine ws_character_tests
cargo test -p wrldbldr-engine ws_location_tests
cargo test -p wrldbldr-engine ws_scene_tests
cargo test -p wrldbldr-engine ws_player_tests
```

---

### Phase 6: Add Unit Tests (0.5 day)

**File:** `tests/engine_tests/list_limits_tests.rs` (new file)

**Tests to Add:**

```rust
#[cfg(test)]
mod tests {
    use wrldbldr_engine::api::websocket::pagination::apply_pagination_limits;
    use wrldbldr_engine::infrastructure::app_settings::AppSettings;

    #[tokio::test]
    fn test_apply_pagination_limits_with_defaults() {
        let settings = AppSettings::default();
        let (limit, offset) = apply_pagination_limits(&settings, None, None);

        assert_eq!(limit, 50);
        assert_eq!(offset, Some(0));
    }

    #[tokio::test]
    fn test_apply_pagination_limits_with_client_limit() {
        let settings = AppSettings::default();
        let (limit, offset) = apply_pagination_limits(&settings, Some(100), None);

        assert_eq!(limit, 100);
        assert_eq!(offset, Some(0));
    }

    #[tokio::test]
    fn test_apply_pagination_limits_max_enforced() {
        let settings = AppSettings::default();
        let (limit, offset) = apply_pagination_limits(&settings, Some(1000), None);

        // Client limit (1000) should be capped at max (200)
        assert_eq!(limit, 200);
    }

    #[tokio::test]
    fn test_apply_pagination_limits_with_offset() {
        let settings = AppSettings::default();
        let (limit, offset) = apply_pagination_limits(&settings, None, Some(25));

        assert_eq!(limit, 50);
        assert_eq!(offset, Some(25));
    }

    #[tokio::test]
    fn test_env_override_default() {
        let mut settings = AppSettings::default();
        settings.list_default_page_size_override = Some(75);

        let (limit, offset) = apply_pagination_limits(&settings, None, None);

        assert_eq!(limit, 75);
    }

    #[tokio::test]
    fn test_env_override_max() {
        let mut settings = AppSettings::default();
        settings.list_max_page_size_override = Some(500);

        let (limit, offset) = apply_pagination_limits(&settings, Some(1000), None);

        assert_eq!(limit, 500);
    }

    #[tokio::test]
    fn test_both_env_overrides() {
        let mut settings = AppSettings::default();
        settings.list_default_page_size_override = Some(75);
        settings.list_max_page_size_override = Some(500);

        let (limit, offset) = apply_pagination_limits(&settings, Some(1000), None);

        assert_eq!(limit, 500); // Capped at max (500), not default (75)
    }
}
```

**Verification:**
```bash
cargo test -p wrldbldr-engine list_limits_tests
```

---

### Phase 7: Update Documentation (0.5 day)

**Tasks:**

1. **Update `README.md`** (if applicable):
   ```markdown
   ## List Limit Configuration

   The server supports configurable list pagination limits to prevent DoS attacks
   while allowing flexibility for different deployment scenarios.

   ### Three-Tier Configuration

   1. **Per-World Settings** (highest priority)
      Configure via Settings API: `PUT /api/worlds/{id}/settings`

   2. **Environment Variables** (medium priority)
      ```bash
      export WRLDBLDR_LIST_DEFAULT_PAGE_SIZE=50
      export WRLDBLDR_LIST_MAX_PAGE_SIZE=200
      ```

   3. **Code Defaults** (fallback)
      Default: 50 items
      Maximum: 200 items

   ### Environment Variables

   | Variable | Description | Default | Range |
   |-----------|-------------|---------|--------|
   | `WRLDBLDR_LIST_DEFAULT_PAGE_SIZE` | Default list page size | 50 | 10-200 |
   | `WRLDBLDR_LIST_MAX_PAGE_SIZE` | Maximum list page size | 200 | 50-1000 |
   ```

2. **Update API Documentation** (if exists):
   Document new pagination helper function and environment variables

**Verification:**
- Documentation builds without errors
- Examples are clear and accurate

---

## Testing Strategy

### 1. Unit Tests

**File:** `tests/engine_tests/list_limits_tests.rs` (created in Phase 6)

Test coverage:
- ✅ Default limits work without any configuration
- ✅ Client-provided limits are respected
- ✅ Maximum limit is hard cap
- ✅ Environment variable overrides work
- ✅ Combined overrides work correctly
- ✅ Offset handling is correct

### 2. Integration Tests

**File:** `tests/engine_tests/list_limits_integration_tests.rs` (new)

```rust
#[tokio::test]
async fn test_full_settings_flow() {
    // Create test world with custom settings
    let world_id = create_test_world().await;

    // 1. Verify default limits work
    let mut app = create_test_app().await;
    let result = get_characters(&app, world_id, None, None).await;
    assert!(result.len() <= 50);

    // 2. Test that environment variables override defaults
    std::env::set_var("WRLDBLDR_LIST_DEFAULT_PAGE_SIZE", "25");
    let result = get_characters(&app, world_id, None, None).await;
    assert!(result.len() <= 25);
}

#[tokio::test]
async fn test_settings_api_respects_limits() {
    // Test that Settings API can configure list limits
    // (When per-world storage is added in future)
}

#[tokio::test]
async fn test_client_limits_with_settings() {
    // Test that client-provided limits interact correctly with settings
    let app = create_test_app().await;

    // Client limit > env var > default
    let result = get_characters(&app, world_id, Some(1000), None).await;
    assert!(result.len() <= 200); // Hard cap enforced
}
```

### 3. Manual Testing Checklist

- [ ] Default limits (50) work without any settings
- [ ] Environment variables override defaults
- [ ] Per-endpoint limits enforced (all 9 handlers)
- [ ] Max limit (200) is hard cap
- [ ] Invalid environment values are rejected (out of range)
- [ ] Settings API shows new fields
- [ ] All list operations respect configured limits

---

## Migration Path

### Rollout Strategy

1. **Phase 1 (Infrastructure)**
   - Extend existing AppSettings (Phase 1)
   - Add env var support (Phase 2)
   - Add pagination helper (Phase 3)
   - Add unit tests (Phase 6)
   - No database changes required
   - Deploy to staging

2. **Phase 2 (API Layer)**
   - Update settings metadata (Phase 4)
   - Update all 9 list handlers (Phase 5)
   - Add integration tests (Phase 7)
   - Update documentation (Phase 7)
   - Deploy to production

### Backward Compatibility

✅ **Fully Backward Compatible:**
- Existing worlds without custom settings will continue using defaults (50/200)
- Environment variables are optional (no breaking changes if not set)
- No database migration required for initial implementation
- Settings API automatically supports new fields
- Client protocol unchanged

---

## Success Criteria

Implementation is complete when:

1. ✅ `list_default_page_size_override` and `list_max_page_size_override` added to `AppSettings`
2. ✅ Accessor methods `*_effective()` added to `AppSettings`
3. ✅ `apply_env_list_limits()` added to `SettingsOps`
4. ✅ Pagination helper created in `websocket/mod.rs`
5. ✅ Settings metadata added to `shared/src/settings.rs`
6. ✅ All 9 list handlers updated to use pagination helper
7. ✅ Unit tests cover pagination helper and env overrides
8. ✅ Integration tests verify end-to-end flow
9. ✅ Manual testing confirms behavior
10. ✅ Documentation updated (README, API docs)

---

## Files Summary

| Phase | Files to Modify/Add | Lines | Effort |
|--------|---------------------|-------|----------|
| 1: Extend AppSettings | crates/engine/src/infrastructure/app_settings.rs | +10 | 0.5 day |
| 2: SettingsOps Env Support | crates/engine/src/use_cases/settings/settings_ops.rs | +30 | 0.5 day |
| 3: Pagination Helper | crates/engine/src/api/websocket/mod.rs | +30 | 0.5 day |
| 4: Settings Metadata | crates/shared/src/settings.rs | +50 | 0.5 day |
| 5: Handler Updates | ws_character, ws_scene, ws_player, ws_location (4 files) | ~150 | 2-3 days |
| 6: Unit Tests | tests/engine_tests/list_limits_tests.rs | +100 | 0.5 day |
| 7: Integration Tests | tests/engine_tests/list_limits_integration_tests.rs | +80 | 0.5 day |
| 8: Documentation | README.md, API docs | ~100 | 0.5 day |
| **Total** | **~550 lines** | **4.5-6.5 days** |

---

## Open Questions

1. **Per-endpoint limits?**
   - Should we support per-endpoint limits (e.g., different limit for characters vs. locations)?
   - **Recommendation:** Start with global limits, add per-type later if needed (more complexity)

2. **World-specific storage?**
   - When should we add per-world storage for list limits?
   - **Recommendation:** Add after Phase 7, as enhancement to `SettingsRepo`

3. **Dynamic limit adjustment?**
   - Should limits be adjustable at runtime without restart?
   - **Recommendation:** Already supported via Settings API, no restart needed for env vars

4. **Rate limiting?**
   - Should we add per-user or per-IP rate limiting for list operations?
   - **Recommendation:** Add to Phase 7 if DoS attacks detected, as enhancement

---

## Next Steps

1. ✅ **Review and approve this plan** with architecture/tech lead
2. ✅ **Create GitHub issue** tracking implementation phases
3. **Assign Phase 1** (Extend AppSettings) to engineering team
4. **Complete Phases 2-4** sequentially after Phase 1 approval
5. **Update client documentation** with new environment variables after completion
6. **Consider Phase 8 enhancements** (per-world storage, rate limiting) for future iteration

---

## Comparison: Simplified vs. Original Plan

| Metric | Original Plan | Simplified Approach |
|--------|---------------|---------------------|
| New Infrastructure Files | 3 (SettingsRepo, SettingsReader, etc.) | 0 (extend existing) |
| Database Schema Changes | Yes (Neo4j) | No (leverage existing) |
| Total Lines of Code | ~500 lines | ~550 lines |
| Estimated Time | 6-10 days | 4.5-6.5 days |
| Risk Level | Medium (new code) | Low (extends battle-tested code) |
| Complexity | Higher (new patterns) | Lower (follows existing) |
| Backward Compatibility | Requires migration | Seamless (defaults work immediately) |

**Decision:** Simplified approach is recommended due to:
- ✅ Faster implementation
- ✅ Lower risk
- ✅ Consistent with existing architecture
- ✅ No database schema changes needed initially
