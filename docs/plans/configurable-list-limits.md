# Configurable List Limits Implementation Plan

## Overview

Convert hardcoded list pagination defaults (`50` default, `200` max) to a configurable 3-tier settings system following the existing `AppSettings` infrastructure pattern.

**Current State:** 9 files have hardcoded `limit.unwrap_or(50).min(200)` pattern
**Target State:** Settings-based configuration with DM override → env var → constant default

---

## Scope

### Files with Hardcoded Limits (to be updated)

1. `crates/engine/src/api/websocket/ws_character.rs` (line 21)
2. `crates/engine/src/api/websocket/ws_location.rs` (multiple handlers)
3. `crates/engine/src/api/websocket/ws_player.rs` (line 282)
4. `crates/engine/src/api/websocket/ws_scene.rs` (line 69)
5. `crates/engine/src/infrastructure/neo4j/character_repo.rs` (line 572)
6. `crates/engine/src/infrastructure/neo4j/location_repo.rs` (multiple queries)
7. `crates/engine/src/infrastructure/neo4j/scene_repo.rs` (line 420)

### Existing Infrastructure (to be extended)

- `crates/engine/src/infrastructure/app_settings.rs` - Add new fields
- `crates/shared/src/settings.rs` - Add UI metadata
- `crates/engine/src/use_cases/settings/settings_ops.rs` - Add environment variable integration

---

## Implementation Plan

### Part 1: Add Settings Fields to AppSettings

**File:** `crates/engine/src/infrastructure/app_settings.rs`

Add to `AppSettings` struct (in "Validation Limits" section around line 1035):

```rust
// ============================================================================
// List Pagination Limits
// ============================================================================
/// Default page size for list operations when no limit specified
#[serde(default = "default_list_page_size")]
list_default_page_size: u32,

/// Maximum allowed page size for list operations (DoS prevention)
#[serde(default = "default_list_max_page_size")]
list_max_page_size: u32,
```

Add default functions:

```rust
fn default_list_page_size() -> u32 { 50 }
fn default_list_max_page_size() -> u32 { 200 }
```

Add accessors:

```rust
pub fn list_default_page_size(&self) -> u32 { self.list_default_page_size }
pub fn list_max_page_size(&self) -> u32 { self.list_max_page_size }
```

Add builder methods:

```rust
pub fn with_list_default_page_size(mut self, size: u32) -> Self {
    self.list_default_page_size = size;
    self
}

pub fn with_list_max_page_size(mut self, size: u32) -> Self {
    self.list_max_page_size = size;
    self
}
```

Update `Default` impl to include new fields.

---

### Part 2: Add Settings Metadata for UI

**File:** `crates/shared/src/settings.rs`

Add to `SETTINGS_FIELDS` array:

```rust
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

### Part 3: Add Environment Variable Support

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
            settings.set_list_default_page_size(size);
        }
    }
    if let Ok(val) = std::env::var("WRLDBLDR_LIST_MAX_PAGE_SIZE") {
        if let Ok(size) = val.parse::<u32>() {
            settings.set_list_max_page_size(size);
        }
    }
}
```

Call this in `get_global()` after loading from database.

---

### Part 4: Create Helper Function for Handlers

**File:** `crates/engine/src/api/websocket/mod.rs` (or new `pagination.rs`)

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

### Part 5: Update Handler Callsites

Replace hardcoded pattern in all 9 files:

**Before:**
```rust
let limit = Some(limit.unwrap_or(50).min(200));
let offset = Some(offset.unwrap_or(0));
```

**After:**
```rust
let settings = state.app.settings().await;
let (limit, offset) = apply_pagination_limits(&settings, limit, offset);
```

Note: Need to ensure `WsState` has access to settings. Check if `state.app` already provides this.

---

### Part 6: Update Repository Callsites

For repository-level defaults (character_repo, location_repo, scene_repo), the settings should be passed down from the handler layer. Do NOT add settings access to repositories (maintains clean architecture).

If a repository method is called internally (not from handler), use `None` for limit which the repository should interpret as "reasonable internal default" (keep 50/200 as constants for internal use).

---

## Resolution Order (3-Tier Cascade)

1. **Tier 1 (Highest Priority):** DM per-world setting from database
2. **Tier 2:** Environment variable (`WRLDBLDR_LIST_*`)
3. **Tier 3 (Default):** Constant in `app_settings.rs`

The existing `SettingsOps::get_for_world()` already handles Tier 1 → Tier 3 fallback. We just need to insert Tier 2 (env vars) into the resolution chain.

---

## Testing Plan

### Unit Tests

1. Test `apply_pagination_limits()` with various inputs
2. Test environment variable override application
3. Test settings field serialization/deserialization

### Integration Tests

1. Test list endpoints respect configured limits
2. Test DM override takes precedence over env var
3. Test env var takes precedence over default

### Manual Testing

1. Set `WRLDBLDR_LIST_DEFAULT_PAGE_SIZE=25` and verify lists return 25 items
2. Set per-world setting to 30 and verify it overrides env var
3. Verify max limit still prevents DoS even with high configured default

---

## Files to Modify

| File | Changes |
|------|---------|
| `crates/engine/src/infrastructure/app_settings.rs` | Add 2 fields + accessors + defaults |
| `crates/shared/src/settings.rs` | Add 2 metadata entries |
| `crates/engine/src/use_cases/settings/settings_ops.rs` | Add env var override method |
| `crates/engine/src/api/websocket/mod.rs` | Add pagination helper function |
| `crates/engine/src/api/websocket/ws_character.rs` | Use pagination helper |
| `crates/engine/src/api/websocket/ws_location.rs` | Use pagination helper |
| `crates/engine/src/api/websocket/ws_player.rs` | Use pagination helper |
| `crates/engine/src/api/websocket/ws_scene.rs` | Use pagination helper |

---

## Estimated Scope

- **New code:** ~100 lines
- **Modified code:** ~50 lines (replacing hardcoded values)
- **Test code:** ~50 lines
- **Documentation:** Update settings documentation

---

## Open Questions

1. Should there be per-entity-type limits? (e.g., different limit for characters vs locations)
   - **Recommendation:** Start with global limits, add per-type later if needed

2. Should internal repository calls (not from handlers) use settings or constants?
   - **Recommendation:** Use constants for internal calls (no settings dependency in repos)

3. Should the settings be cached in WsState or fetched per-request?
   - **Recommendation:** Cache in WsState, refresh on settings update broadcast
